use rivet::workbench::Workbench;
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
        let server = Workbench::bind(0).unwrap();
        let address = server.address();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let worker = thread::spawn(move || server.serve(signal).unwrap());
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
    fn call(&self, method: &str, path: &str, body: Value) -> (u16, Value) {
        self.custom(method, path, body, "")
    }
    fn custom(&self, method: &str, path: &str, body: Value, extra: &str) -> (u16, Value) {
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let body = body.to_string();
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-Rivet-Request: 1\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (
            head.split_whitespace().nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(body).unwrap(),
        )
    }
    fn build(&self, source: &str) -> u64 {
        let (status, result) = self.call("POST", "/api/build", json!({"source":source}));
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

#[test]
fn console_edit_error_and_stale_build_paths() {
    let app = Harness::new();
    let (status, examples) = app.call("GET", "/api/examples", json!({}));
    assert_eq!(status, 200);
    assert_eq!(examples["examples"].as_array().unwrap().len(), 3);
    let source = include_str!("../examples/hello.rivet.json");
    let id = app.build(source);
    let (status, result) = app.call("POST", "/api/run", json!({"build_id":id}));
    assert_eq!(status, 200);
    assert_eq!(result["stdout"], "Hello, world!\n");
    let changed = source.replace("world!", "engineer!");
    let next_id = app.build(&changed);
    assert!(next_id > id);
    assert_eq!(app.call("POST", "/api/run", json!({"build_id":id})).0, 409);
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":next_id})).1["stdout"],
        "Hello, engineer!\n"
    );
    let (status, result) = app.call("POST", "/api/build", json!({"source":"{}"}));
    assert_eq!(status, 400);
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(
        app.call("POST", "/api/run", json!({"build_id":next_id})).0,
        409
    );
    assert!(app.call("GET", "/api/status", json!({})).1["build"].is_null());
}

#[test]
fn program_server_lifecycle_and_independent_running_artifact() {
    let app = Harness::new();
    let mut source: Value =
        serde_json::from_str(include_str!("../examples/server.rivet.json")).unwrap();
    source["entry"]["port"] = json!(0);
    let id = app.build(&source.to_string());
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
        assert_eq!(
            serde_json::from_str::<Value>(result["body"].as_str().unwrap()).unwrap(),
            json!({"message":"hello world"})
        );
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
    assert_eq!(
        app.call("POST", "/api/build", json!({"source":"invalid"}))
            .0,
        400
    );
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
    let changed = source
        .to_string()
        .replace("hello world", "changed response");
    let id = app.build(&changed);
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
fn workbench_rejects_foreign_origins_and_invalid_request_fields() {
    let app = Harness::new();
    assert_eq!(
        app.custom(
            "POST",
            "/api/build",
            json!({"source":"{}"}),
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
