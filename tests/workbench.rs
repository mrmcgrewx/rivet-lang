use rivet::{language, workbench::Workbench};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
struct Harness {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Harness {
    fn new() -> Self {
        let app = Workbench::bind(0).unwrap();
        let address = app.address();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let worker = thread::spawn(move || app.serve(signal).unwrap());
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
    fn raw(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
        content_type: &str,
        extra: &str,
    ) -> (u16, Vec<u8>) {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: {content_type}\r\nX-Rivet-Request: 1\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",self.address,body.len()).unwrap();
        stream.write_all(body).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let end = response.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&response[..end]);
        (
            head.split_whitespace().nth(1).unwrap().parse().unwrap(),
            response[end + 4..].to_vec(),
        )
    }
    fn call(&self, method: &str, path: &str, body: Value) -> (u16, Value) {
        let (status, bytes) = self.raw(
            method,
            path,
            body.to_string().as_bytes(),
            "application/json",
            "",
        );
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    fn binary(&self, path: &str, source: &[u8]) -> (u16, Value) {
        let (status, bytes) = self.raw("POST", path, source, "application/octet-stream", "");
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    fn build(&self, source: &[u8]) -> u64 {
        let (status, result) = self.binary("/api/build", source);
        assert_eq!(status, 200, "{result}");
        result["build"]["id"].as_u64().unwrap()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
fn edit_text(source: &[u8], index: usize, value: &str) -> Vec<u8> {
    let info = language::inspect_binary(source).unwrap();
    let node = &info.instructions[index];
    let mut source = source.to_vec();
    source.splice(
        node.payload_offset..node.payload_offset + node.payload_length,
        value.bytes(),
    );
    source[node.offset + 2..node.offset + 4].copy_from_slice(&(value.len() as u16).to_le_bytes());
    let length = (source.len() - 16) as u32;
    source[12..16].copy_from_slice(&length.to_le_bytes());
    source
}
fn server_source() -> Vec<u8> {
    let mut source = include_bytes!("../examples/server.rvb").to_vec();
    let offset = language::inspect_binary(&source).unwrap().entry_offset;
    source[offset..offset + 2].copy_from_slice(&0u16.to_le_bytes());
    source
}
#[test]
fn binary_examples_and_inspection_are_exact_and_reject_text_source() {
    let app = Harness::new();
    let (status, examples) = app.call("GET", "/api/examples", json!({}));
    assert_eq!(status, 200);
    assert_eq!(examples["examples"].as_array().unwrap().len(), 3);
    assert!(examples["examples"][0].get("source").is_none());
    let (status, bytes) = app.raw(
        "GET",
        "/api/examples/hello",
        b"",
        "application/octet-stream",
        "",
    );
    assert_eq!(status, 200);
    assert_eq!(bytes, include_bytes!("../examples/hello.rvb"));
    let (status, result) = app.binary("/api/inspect", &bytes);
    assert_eq!(status, 200);
    assert_eq!(result["inspection"]["source_bytes"], 49);
    assert_eq!(result["inspection"]["instructions"][0]["offset"], 16);
    assert!(
        result["inspection"]["ir"]
            .as_str()
            .unwrap()
            .contains("concat")
    );
    assert_eq!(
        app.call("POST", "/api/build", json!({"source":"{}"})).0,
        403
    );
    let (status, result) = app.binary(
        "/api/build",
        include_bytes!("fixtures/legacy/hello.rivet.json"),
    );
    assert_eq!(status, 400);
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(app.binary("/api/inspect", &vec![0; 65537]).0, 413);
}
#[test]
fn console_binary_edit_error_and_stale_build_paths() {
    let app = Harness::new();
    let source = include_bytes!("../examples/hello.rvb");
    let id = app.build(source);
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":id})).1["stdout"],
        "Hello, world!\n"
    );
    let changed = edit_text(source, 1, "engineer!");
    let next = app.build(&changed);
    assert!(next > id);
    assert_eq!(app.call("POST", "/api/run", json!({"build_id":id})).0, 409);
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":next})).1["stdout"],
        "Hello, engineer!\n"
    );
    let (status, result) = app.binary("/api/build", b"RVB\0");
    assert_eq!(status, 400);
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":next})).0,
        409
    );
    assert!(app.call("GET", "/api/status", json!({})).1["build"].is_null());
}
#[test]
fn binary_server_lifecycle_and_independent_running_artifact() {
    let app = Harness::new();
    let source = server_source();
    let id = app.build(&source);
    let (status, result) = app.call("POST", "/api/run", json!({"build_id":id}));
    assert_eq!(status, 200, "{result}");
    let address: SocketAddr = result["server"]["address"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    for _ in 0..4 {
        let (status, result) = app.call("POST", "/api/request", json!({"path":"/hello"}));
        assert_eq!(status, 200);
        assert_eq!(result["status"], 200);
        assert_eq!(result["content_type"], "application/json; charset=utf-8");
        assert_eq!(result["body"], "{\"message\":\"hello world\"}");
    }
    assert_eq!(
        app.call("POST", "/api/request", json!({"path":"/missing"}))
            .1["status"],
        404
    );
    assert_eq!(
        app.call(
            "POST",
            "/api/request",
            json!({"path":"http://example.com/"})
        )
        .0,
        400
    );
    assert_eq!(app.call("POST", "/api/run", json!({"build_id":id})).0, 409);
    assert_eq!(app.binary("/api/build", b"invalid").0, 400);
    assert_eq!(
        app.call("POST", "/api/request", json!({"path":"/hello"})).1["status"],
        200
    );
    assert_eq!(
        app.call("GET", "/api/status", json!({})).1["server"]["build_id"],
        id
    );
    assert_eq!(app.call("POST", "/api/stop", json!({})).1["stopped"], true);
    assert!(TcpStream::connect(address).is_err());
    assert_eq!(app.call("POST", "/api/stop", json!({})).1["stopped"], false);
    assert_eq!(
        app.call("POST", "/api/request", json!({"path":"/hello"})).0,
        409
    );
    let id = app.build(&edit_text(&source, 0, "changed response"));
    assert_eq!(app.call("POST", "/api/run", json!({"build_id":id})).0, 200);
    assert!(
        app.call("POST", "/api/request", json!({"path":"/hello"})).1["body"]
            .as_str()
            .unwrap()
            .contains("changed response")
    );
    let latest = app.call("GET", "/api/status", json!({})).1["server"]["address"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .parse::<SocketAddr>()
        .unwrap();
    drop(app);
    assert!(TcpStream::connect(latest).is_err());
}
#[test]
fn workbench_rejects_foreign_origins_and_invalid_control_fields() {
    let app = Harness::new();
    assert_eq!(
        app.raw(
            "POST",
            "/api/build",
            include_bytes!("../examples/hello.rvb"),
            "application/octet-stream",
            "Origin: https://evil.example\r\n"
        )
        .0,
        403
    );
    assert_eq!(app.call("POST", "/api/stop", json!({"extra":true})).0, 400);
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":0,"extra":true}))
            .0,
        400
    );
    let mut stream = TcpStream::connect(app.address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write!(
        stream,
        "GET /api/status HTTP/1.1\r\nHost: evil.example\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"));
}
