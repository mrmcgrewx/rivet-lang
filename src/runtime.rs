//! Loopback-only HTTP support. The language supplies every successful route response.
use crate::http::{Response, Server};
use crate::language::Route;
use serde::Serialize;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct ProgramServer {
    address: SocketAddr,
    paths: Vec<String>,
    requests: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerInfo {
    pub address: String,
    pub requests: u64,
    pub routes: Vec<String>,
    pub build_id: u64,
}

#[derive(Debug, Serialize)]
pub struct HttpResult {
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub elapsed_us: u128,
}

impl ProgramServer {
    pub fn start(port: u16, routes: Vec<Route>) -> Result<Self, String> {
        let listener = Server::http(("127.0.0.1", port))
            .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
        let address = listener.address();
        let paths = routes.iter().map(|r| r.path.clone()).collect();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicU64::new(0));
        let stop_worker = stop.clone();
        let count_worker = requests.clone();
        // Serialize once after execution, before serving; requests never run host-language demo code.
        let responses: Vec<_> = routes
            .into_iter()
            .map(|r| (r.path, r.status, r.body.to_string()))
            .collect();
        let worker = thread::Builder::new()
            .name("rivet-program-http".into())
            .spawn(move || {
                while !stop_worker.load(Ordering::Acquire) {
                    let request =
                        match listener.recv_timeout(Duration::from_millis(25), &stop_worker) {
                            Ok(Some(r)) => r,
                            Ok(None) => continue,
                            Err(_) => break,
                        };
                    count_worker.fetch_add(1, Ordering::Relaxed);
                    let path = request.url().split('?').next().unwrap_or(request.url());
                    let (status, body) = if request.method() != "GET" {
                        (405, "{\"error\":\"method not allowed\"}".to_owned())
                    } else if let Some((_, status, body)) =
                        responses.iter().find(|(route, _, _)| route == path)
                    {
                        (*status, body.clone())
                    } else {
                        (404, "{\"error\":\"not found\"}".to_owned())
                    };
                    let mut response = Response::from_string(body)
                        .with_status(status)
                        .with_header(header("Content-Type", "application/json; charset=utf-8"))
                        .with_header(header("Cache-Control", "no-store"))
                        .with_header(header("Connection", "close"));
                    if status == 405 {
                        response.add_header(header("Allow", "GET"));
                    }
                    let _ = request.respond(response);
                }
            })
            .map_err(|e| format!("Cannot start server worker: {e}"))?;
        Ok(Self {
            address,
            paths,
            requests,
            stop,
            worker: Some(worker),
        })
    }

    pub fn info(&self, build_id: u64) -> ServerInfo {
        ServerInfo {
            address: format!("http://{}", self.address),
            requests: self.requests.load(Ordering::Relaxed),
            routes: self.paths.clone(),
            build_id,
        }
    }

    pub fn request(&self, path: &str) -> Result<HttpResult, String> {
        request_http(self.address, path)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "Program server worker panicked".to_owned())?;
        }
        Ok(())
    }
}

impl Drop for ProgramServer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub fn header(name: &str, value: &str) -> (String, String) {
    (name.to_owned(), value.to_owned())
}

/// A bounded real HTTP request to a listener created by this process, never an arbitrary URL.
pub fn request_http(address: SocketAddr, path: &str) -> Result<HttpResult, String> {
    if !address.ip().is_loopback() {
        return Err("Requests must remain on loopback".into());
    }
    if !path.starts_with('/')
        || path.len() > 2048
        || !path.is_ascii()
        || path.bytes().any(|b| b <= b' ' || b == 127 || b == b'#')
    {
        return Err("Request path must start with / and contain no spaces, controls, fragment, or non-ASCII characters (maximum 2048 bytes)".into());
    }
    let started = Instant::now();
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|e| format!("Connect failed: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .take(131073)
        .read_to_end(&mut response)
        .map_err(|e| format!("Read failed: {e}"))?;
    if response.len() > 131072 {
        return Err("HTTP response exceeds 128 KiB".into());
    }
    let text = String::from_utf8(response).map_err(|_| "Non-UTF-8 HTTP response")?;
    let (headers, body) = text
        .split_once("\r\n\r\n")
        .ok_or("Malformed HTTP response")?;
    let status = headers
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or("Missing HTTP status")?;
    let content_type = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default();
    Ok(HttpResult {
        status,
        content_type,
        body: body.into(),
        elapsed_us: started.elapsed().as_micros(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn routes() -> Vec<Route> {
        vec![Route {
            method: "GET".into(),
            path: "/hello".into(),
            status: 200,
            body: json!({"message":"hello world"}),
        }]
    }
    #[test]
    fn repeated_requests_shutdown_and_rebind() {
        let mut server = ProgramServer::start(0, routes()).unwrap();
        let port = server.address.port();
        for _ in 0..8 {
            let response = server.request("/hello?probe=1").unwrap();
            assert_eq!(response.status, 200);
            assert_eq!(response.content_type, "application/json; charset=utf-8");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
                json!({"message":"hello world"})
            );
        }
        assert_eq!(server.request("/missing").unwrap().status, 404);
        assert_eq!(server.info(1).requests, 9);
        assert!(ProgramServer::start(port, routes()).is_err());
        assert!(server.request("/hello\r\nInjected: yes").is_err());
        server.stop().unwrap();
        assert!(TcpStream::connect(server.address).is_err());
        let mut replacement = ProgramServer::start(port, routes()).unwrap();
        replacement.stop().unwrap();
    }
    #[test]
    fn rejects_non_get() {
        let server = ProgramServer::start(0, routes()).unwrap();
        let mut stream = TcpStream::connect(server.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        write!(
            stream,
            "POST /hello HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            server.address
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 405"));
        assert!(response.contains("Allow: GET"));
    }
}
