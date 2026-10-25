//! Bounded HTTP/1.x transport for loopback experiments, one request per connection.
//! Socket ownership stays in the serving thread, so shutdown closes the listener synchronously.
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
const MAX_HEADERS: usize = 8192;
const MAX_BODY: usize = 512 * 1024;
const REQUEST_DEADLINE: Duration = Duration::from_secs(2);

pub struct Server {
    listener: TcpListener,
}
pub struct Request {
    stream: TcpStream,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}
impl Response {
    pub fn from_string(body: String) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body,
        }
    }
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    pub fn with_header(mut self, header: (String, String)) -> Self {
        self.headers.push(header);
        self
    }
    pub fn add_header(&mut self, header: (String, String)) {
        self.headers.push(header);
    }
}
impl Server {
    pub fn http(address: (&str, u16)) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }
    pub fn address(&self) -> SocketAddr {
        self.listener.local_addr().expect("bound TCP address")
    }
    pub fn recv_timeout(
        &self,
        timeout: Duration,
        stop: &AtomicBool,
    ) -> io::Result<Option<Request>> {
        let until = Instant::now() + timeout;
        loop {
            if stop.load(Ordering::Acquire) {
                return Ok(None);
            }
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false)?;
                    stream.set_read_timeout(Some(Duration::from_millis(25)))?;
                    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
                    match read_request(&mut stream, stop) {
                        Ok((method, url, headers, body)) => {
                            return Ok(Some(Request {
                                stream,
                                method,
                                url,
                                headers,
                                body,
                            }));
                        }
                        Err((status, message)) => {
                            if !stop.load(Ordering::Acquire) {
                                let body=serde_json::json!({"ok":false,"error":message,"diagnostics":[]}).to_string();
                                let _ = write_response(
                                    &mut stream,
                                    Response::from_string(body)
                                        .with_status(status)
                                        .with_header((
                                            "Content-Type".into(),
                                            "application/json; charset=utf-8".into(),
                                        )),
                                );
                            }
                            return Ok(None);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= until {
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => return Err(e),
            }
        }
    }
}
impl Request {
    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
    pub fn body_length(&self) -> Option<usize> {
        Some(self.body.len())
    }
    pub fn as_reader(&mut self) -> &[u8] {
        &self.body
    }
    pub fn respond(mut self, response: Response) -> io::Result<()> {
        write_response(&mut self.stream, response)
    }
}
type Parsed = (String, String, Vec<(String, String)>, Vec<u8>);
type Failure = (u16, &'static str);
fn read_request(stream: &mut TcpStream, stop: &AtomicBool) -> Result<Parsed, Failure> {
    let deadline = Instant::now() + REQUEST_DEADLINE;
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            if index + 4 > MAX_HEADERS {
                return Err((431, "HTTP headers exceed 8 KiB"));
            }
            break index + 4;
        }
        if bytes.len() >= MAX_HEADERS {
            return Err((431, "HTTP headers exceed 8 KiB"));
        }
        read_more(stream, &mut bytes, MAX_HEADERS, deadline, stop)?;
    };
    let mut slots = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut slots);
    match request.parse(&bytes[..header_end]) {
        Ok(httparse::Status::Complete(_)) => (),
        _ => return Err((400, "Malformed HTTP headers")),
    }
    let method = request.method.ok_or((400, "Missing method"))?.to_owned();
    let url = request.path.ok_or((400, "Missing target"))?.to_owned();
    if !url.starts_with('/') || url.len() > 2048 || !url.is_ascii() {
        return Err((
            400,
            "Use an ASCII origin-form request target of at most 2048 bytes",
        ));
    }
    let mut headers = Vec::new();
    let mut content_length = None;
    let mut has_host = false;
    for h in request.headers.iter() {
        let value = std::str::from_utf8(h.value)
            .map_err(|_| (400, "Non-UTF-8 header"))?
            .trim()
            .to_owned();
        if h.name.eq_ignore_ascii_case("transfer-encoding") {
            return Err((501, "Transfer-Encoding is unsupported; send Content-Length"));
        }
        if h.name.eq_ignore_ascii_case("expect") {
            return Err((417, "Expect is unsupported"));
        }
        if h.name.eq_ignore_ascii_case("host") {
            if has_host {
                return Err((400, "Duplicate Host header"));
            }
            has_host = true;
        }
        let security_header = h.name.eq_ignore_ascii_case("origin")
            || h.name.eq_ignore_ascii_case("x-rivet-request")
            || h.name.eq_ignore_ascii_case("content-type");
        if security_header
            && headers
                .iter()
                .any(|(name, _): &(String, String)| name.eq_ignore_ascii_case(h.name))
        {
            return Err((400, "Duplicate security-sensitive header"));
        }
        if h.name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err((400, "Invalid or duplicate Content-Length"));
            }
            let length = value
                .parse::<usize>()
                .map_err(|_| (413, "HTTP body exceeds 512 KiB"))?;
            if length > MAX_BODY {
                return Err((413, "HTTP body exceeds 512 KiB"));
            }
            content_length = Some(length);
        }
        headers.push((h.name.to_owned(), value));
    }
    let length = content_length.unwrap_or(0);
    let total = header_end + length;
    while bytes.len() < total {
        read_more(stream, &mut bytes, total, deadline, stop)?;
    }
    Ok((method, url, headers, bytes[header_end..total].to_vec()))
}
fn read_more(
    stream: &mut TcpStream,
    bytes: &mut Vec<u8>,
    limit: usize,
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<(), Failure> {
    let mut buffer = [0u8; 4096];
    loop {
        if stop.load(Ordering::Acquire) {
            return Err((503, "Server stopping"));
        }
        if Instant::now() >= deadline {
            return Err((408, "HTTP request deadline exceeded"));
        }
        let length = buffer.len().min(limit.saturating_sub(bytes.len()));
        match stream.read(&mut buffer[..length]) {
            Ok(0) => return Err((400, "Incomplete HTTP request")),
            Ok(n) => {
                bytes.extend_from_slice(&buffer[..n]);
                return Ok(());
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(_) => return Err((400, "Cannot read HTTP request")),
        }
    }
}
fn write_response(stream: &mut TcpStream, response: Response) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Content Too Large",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Response",
    };
    write!(
        stream,
        "HTTP/1.1 {} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.body.len()
    )?;
    for (name, value) in response.headers {
        if !name.eq_ignore_ascii_case("connection") && !name.eq_ignore_ascii_case("content-length")
        {
            write!(stream, "{name}: {value}\r\n")?;
        }
    }
    write!(stream, "\r\n{}", response.body)?;
    stream.flush()
}
