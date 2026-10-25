use rivet::{
    language::{self, Execution},
    native::{self, BuildProfile},
    native_process::NativeServer,
    runtime::request_http,
    workbench::Workbench,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(native::unique_directory(&std::env::temp_dir(), "rivet-native-test").unwrap())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct App {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl App {
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
    fn raw(&self, method: &str, path: &str, body: Value) -> (u16, Vec<u8>) {
        let mut client = TcpStream::connect(self.address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let body = body.to_string();
        write!(client,"{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-Rivet-Request: 1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
        let mut bytes = Vec::new();
        client.read_to_end(&mut bytes).unwrap();
        let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let header = String::from_utf8_lossy(&bytes[..end]);
        let status = header.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, bytes[end + 4..].to_vec())
    }
    fn call(&self, path: &str, body: Value) -> Value {
        let (status, bytes) = self.raw("POST", path, body);
        let result: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(status, 200, "{result}");
        result
    }
    fn build(&self, source: &str) -> Value {
        self.call("/api/build", json!({"source":source}))["build"].clone()
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
fn assert_native(bytes: &[u8]) {
    #[cfg(target_os = "macos")]
    assert_eq!(&bytes[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
    #[cfg(target_os = "linux")]
    assert_eq!(&bytes[..4], b"\x7fELF");
}
#[test]
fn ui_artifact_download_and_console_survive_workbench_shutdown() {
    let app = App::new();
    let build = app.build(include_str!("../examples/hello.rivet.json"));
    let path = PathBuf::from(build["artifact"]["path"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    assert_native(&bytes);
    assert_eq!(
        bytes.len() as u64,
        build["artifact"]["size_bytes"].as_u64().unwrap()
    );
    assert!(build["llvm_ir"].as_str().unwrap().contains("@rv_concat"));
    assert!(build["assembly"].as_str().unwrap().contains("rv_print"));
    let (status, download) = app.raw(
        "GET",
        build["artifact"]["download_url"].as_str().unwrap(),
        json!({}),
    );
    assert_eq!(status, 200);
    assert_eq!(download, bytes);
    let result = app.call("/api/run", json!({"build_id":build["id"]}));
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["stdout"], "Hello, world!\n");
    let isolated = Temp::new();
    let copy = isolated.0.join("hello");
    fs::copy(&path, &copy).unwrap();
    drop(app);
    assert!(path.exists());
    fs::remove_file(path).unwrap();
    let output = Command::new(&copy)
        .current_dir(&isolated.0)
        .env_clear()
        .env("PATH", "/nonexistent")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Hello, world!\n");
    assert!(output.stderr.is_empty());
}
#[test]
fn server_artifact_runs_alone_after_workbench_and_managed_child_stop() {
    let app = App::new();
    let mut source: Value =
        serde_json::from_str(include_str!("../examples/server.rivet.json")).unwrap();
    source["entry"]["port"] = json!(0);
    let build = app.build(&source.to_string());
    let path = PathBuf::from(build["artifact"]["path"].as_str().unwrap());
    let running = app.call("/api/run", json!({"build_id":build["id"]}));
    assert!(running["server"]["pid"].as_u64().unwrap() > 0);
    let managed: SocketAddr = running["server"]["address"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    let isolated = Temp::new();
    let copy = isolated.0.join("server");
    fs::copy(&path, &copy).unwrap();
    drop(app);
    assert!(TcpStream::connect(managed).is_err());
    assert!(path.exists());
    fs::remove_file(path).unwrap();
    let mut child = Command::new(&copy)
        .current_dir(&isolated.0)
        .env_clear()
        .env("PATH", "/nonexistent")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    reader.read_line(&mut ready).unwrap();
    let address: SocketAddr = ready
        .trim()
        .strip_prefix("Rivet program server: http://")
        .unwrap()
        .parse()
        .unwrap();
    for _ in 0..5 {
        let response = request_http(address, "/hello").unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        assert_eq!(response.body, "{\"message\":\"hello world\"}");
    }
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    assert!(child.wait().unwrap().success());
    assert!(TcpStream::connect(address).is_err());
}
#[test]
fn both_profiles_match_reference_http_json_and_native_failure() {
    let source=json!({"version":1,"name":"native JSON boundary","instructions":[
 {"id":"max","op":"int","value":i64::MAX},{"id":"min","op":"int","value":i64::MIN},
 {"id":"text","op":"text","value":"snow ☃\0 quote \" slash \\ newline\n"},{"id":"b","op":"bool","value":true},
 {"id":"maxtext","op":"to_text","value":"max"},{"id":"nested","op":"object","fields":{"z":"max","a":"min"}},
 {"id":"body","op":"object","fields":{"z":"nested","a\0key":"text","bool":"b","quoted":"maxtext"}}],
 "entry":{"kind":"http","port":0,"routes":[{"method":"GET","path":"/values","status":201,"body":"body"}]}}).to_string();
    let program = language::compile(&source).unwrap();
    let Execution::Http { routes, .. } = program.execute().unwrap() else {
        panic!()
    };
    let expected = routes[0].body.to_string();
    let temp = Temp::new();
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        let artifact =
            native::build_native(&program, &temp.0.join(profile.name()), profile).unwrap();
        let mut server = NativeServer::start(&artifact).unwrap();
        let response = server.request("/values?case=1").unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.body, expected);
        server.stop().unwrap();
    }
    let overflow = source.replace(&i64::MAX.to_string(), "9223372036854775808");
    assert!(language::compile(&overflow).is_err());
}
#[test]
fn cli_build_is_native_and_backend_failure_is_reported_without_artifact() {
    let temp = Temp::new();
    let hello = temp.0.join("hello");
    let cli = env!("CARGO_BIN_EXE_rivet");
    let output = Command::new(cli)
        .args(["build", "examples/hello.rivet.json", "-o"])
        .arg(&hello)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_native(&fs::read(&hello).unwrap());
    let failed = temp.0.join("bad");
    let output = Command::new(cli)
        .env("RIVET_CLANG", "/nonexistent/rivet-clang")
        .args(["build", "examples/hello.rivet.json", "-o"])
        .arg(&failed)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!failed.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot launch native backend"));
    let overwrite = Command::new(cli)
        .args(["build", "examples/hello.rivet.json", "-o"])
        .arg(&hello)
        .output()
        .unwrap();
    assert!(!overwrite.status.success());
    assert_eq!(
        Command::new(&hello).output().unwrap().stdout,
        b"Hello, world!\n"
    );
}
