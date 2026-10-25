//! OS-process lifecycle for compiled artifacts. No interpreter participates in these paths.
use crate::{
    language::EntryInfo,
    native::NativeArtifact,
    runtime::{HttpResult, request_http},
};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Read},
    net::SocketAddr,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[derive(Debug, Serialize)]
pub struct RunError {
    pub message: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}
impl RunError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            stderr: String::new(),
            exit_code: None,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct ConsoleResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub elapsed_us: u128,
}
#[derive(Clone, Debug, Serialize)]
pub struct ServerInfo {
    pub address: String,
    pub requests: u64,
    pub routes: Vec<String>,
    pub build_id: u64,
    pub pid: u32,
}
pub struct NativeServer {
    child: Child,
    address: SocketAddr,
    routes: Vec<String>,
    requests: Arc<AtomicU64>,
    stderr: Arc<Mutex<Vec<u8>>>,
    readers: Vec<JoinHandle<()>>,
    exited: bool,
}
fn spawn(artifact: &NativeArtifact) -> Result<Child, RunError> {
    Command::new(&artifact.path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("RIVET_CONTROL", "1")
        .spawn()
        .map_err(|e| {
            RunError::new(format!(
                "Cannot launch executable {}: {e}",
                artifact.path.display()
            ))
        })
}
fn read_bounded(mut input: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    while let Ok(n) = input.read(&mut buffer) {
        if n == 0 {
            break;
        }
        let count = n.min(131072 - bytes.len());
        bytes.extend_from_slice(&buffer[..count]);
    }
    bytes
}
pub fn run_console(artifact: &NativeArtifact) -> Result<ConsoleResult, RunError> {
    if !matches!(artifact.entry, EntryInfo::Console) {
        return Err(RunError::new("Artifact is an HTTP server"));
    }
    let started = Instant::now();
    let mut child = spawn(artifact)?;
    let stdout = child.stdout.take().expect("stdout configured");
    let stderr = child.stderr.take().expect("stderr configured");
    let out = thread::spawn(move || read_bounded(stdout));
    let err = thread::spawn(move || read_bounded(stderr));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() > Duration::from_secs(5) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out.join();
                    let _ = err.join();
                    return Err(RunError::new("Console executable exceeded five seconds"));
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::new(e.to_string()));
            }
        }
    };
    let stdout = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    if !status.success() {
        return Err(RunError {
            message: format!("Native executable failed ({status}): {}", stderr.trim()),
            stderr,
            exit_code: status.code(),
        });
    }
    Ok(ConsoleResult {
        stdout,
        stderr,
        exit_code: status.code().unwrap_or(0),
        elapsed_us: started.elapsed().as_micros(),
    })
}
impl NativeServer {
    pub fn start(artifact: &NativeArtifact) -> Result<Self, RunError> {
        let EntryInfo::Http { routes, .. } = &artifact.entry else {
            return Err(RunError::new("Artifact is a console program"));
        };
        let mut child = spawn(artifact)?;
        let stdout = child.stdout.take().expect("stdout configured");
        let stderr_pipe = child.stderr.take().expect("stderr configured");
        let requests = Arc::new(AtomicU64::new(0));
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let (sender, receiver) = mpsc::channel();
        // Runtime stdout has a single short readiness line; never accept a host outside loopback.
        let out = thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(address) = line.strip_prefix("Rivet program server: http://") {
                    let _ = sender.send(address.to_string());
                }
            }
        });
        let counts = requests.clone();
        let logs = stderr.clone();
        let err = thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines().map_while(Result::ok) {
                if line == "RIVET_REQUEST" {
                    counts.fetch_add(1, Ordering::Relaxed);
                } else if let Ok(mut bytes) = logs.lock() {
                    let remaining = 65536 - bytes.len();
                    bytes.extend_from_slice(&line.as_bytes()[..line.len().min(remaining)]);
                    if bytes.len() < 65536 {
                        bytes.push(b'\n');
                    }
                }
            }
        });
        let mut server = Self {
            child,
            address: "127.0.0.1:0".parse().expect("literal"),
            routes: routes.clone(),
            requests,
            stderr,
            readers: vec![out, err],
            exited: false,
        };
        let start = Instant::now();
        loop {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(address) => match address.parse::<SocketAddr>() {
                    Ok(address) if address.ip().is_loopback() && address.port() != 0 => {
                        server.address = address;
                        return Ok(server);
                    }
                    _ => {
                        let _ = server.stop();
                        return Err(RunError::new(
                            "Native server reported an invalid listener address",
                        ));
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => (),
            }
            if let Some(error) = server.poll_exit() {
                return Err(error);
            }
            if start.elapsed() > Duration::from_secs(3) {
                let _ = server.stop();
                return Err(RunError {
                    message: "Native server did not become ready within three seconds".into(),
                    stderr: server.error_text(),
                    exit_code: None,
                });
            }
        }
    }
    fn error_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
    }
    fn join_readers(&mut self) {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
    pub fn poll_exit(&mut self) -> Option<RunError> {
        if self.exited {
            return Some(RunError::new("Native server has exited"));
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.exited = true;
                self.join_readers();
                Some(RunError {
                    message: format!(
                        "Native server exited ({status}): {}",
                        self.error_text().trim()
                    ),
                    stderr: self.error_text(),
                    exit_code: status.code(),
                })
            }
            Ok(None) => None,
            Err(e) => Some(RunError::new(e.to_string())),
        }
    }
    pub fn info(&self, build_id: u64) -> ServerInfo {
        ServerInfo {
            address: format!("http://{}", self.address),
            requests: self.requests.load(Ordering::Relaxed),
            routes: self.routes.clone(),
            build_id,
            pid: self.child.id(),
        }
    }
    pub fn request(&self, path: &str) -> Result<HttpResult, String> {
        request_http(self.address, path)
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.exited {
            if self.child.try_wait().map_err(|e| e.to_string())?.is_none() {
                // PID belongs to the unreaped Child; it cannot be reused before wait.
                unsafe {
                    libc::kill(self.child.id() as libc::pid_t, libc::SIGTERM);
                }
                let start = Instant::now();
                loop {
                    if self.child.try_wait().map_err(|e| e.to_string())?.is_some() {
                        break;
                    }
                    if start.elapsed() > Duration::from_secs(2) {
                        self.child.kill().map_err(|e| e.to_string())?;
                        self.child.wait().map_err(|e| e.to_string())?;
                        break;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
            self.exited = true;
        }
        self.join_readers();
        Ok(())
    }
}
impl Drop for NativeServer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
