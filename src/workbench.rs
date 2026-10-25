//! Local editor service. Build artifacts and live server state are deliberately separate.
use crate::http::{Request, Response, Server};
use crate::{
    language::{self, EntryInfo},
    native::{self, BuildProfile, NativeArtifact},
    native_process::{self, NativeServer},
    runtime,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const MAX_BODY: u64 = 512 * 1024;
// JSON can encode one input byte as six ASCII bytes (e.g. a whitespace escape).
const MAX_RUN_BODY: u64 = 6 * native_process::MAX_INPUT_BYTES as u64 + 1024;

#[derive(Clone, Debug, Serialize)]
struct BuildInfo {
    id: u64,
    name: String,
    instructions: usize,
    ir: String,
    artifact: Value,
    llvm_ir: String,
    assembly: String,
    build_ms: u128,
}
struct Build {
    info: BuildInfo,
    artifact: NativeArtifact,
}

pub struct Workbench {
    listener: Server,
    address: SocketAddr,
    build: Option<Build>,
    next_build: u64,
    running: Option<(u64, NativeServer)>,
    artifact_root: PathBuf,
    artifacts: BTreeMap<u64, NativeArtifact>,
    last_error: Option<String>,
    last_stdout: String,
}

impl Workbench {
    pub fn bind(port: u16) -> Result<Self, String> {
        let listener = Server::http_with_body_limit(("127.0.0.1", port), MAX_RUN_BODY as usize)
            .map_err(|e| format!("Cannot start workbench on 127.0.0.1:{port}: {e}"))?;
        let address = listener.address();
        Ok(Self {
            listener,
            address,
            build: None,
            next_build: 1,
            running: None,
            last_stdout: String::new(),
            artifact_root: native::unique_directory(Path::new(".rivet/builds"), "session")?,
            artifacts: BTreeMap::new(),
            last_error: None,
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn serve(mut self, stop: Arc<AtomicBool>) -> Result<(), String> {
        while !stop.load(Ordering::Acquire) {
            if let Some(request) = self
                .listener
                .recv_timeout(Duration::from_millis(25), &stop)
                .map_err(|e| e.to_string())?
            {
                self.handle(request);
            }
        }
        if let Some((_, mut server)) = self.running.take() {
            server.stop()?;
        }
        Ok(())
    }
    fn status(&mut self) -> Value {
        self.check_child();
        json!({"ok":true,"build":self.build.as_ref().map(|b| &b.info),"server":self.running.as_ref().map(|(id,s)|s.info(*id)),"last_stdout":self.last_stdout,"last_error":self.last_error})
    }
    fn check_child(&mut self) {
        if let Some((_, server)) = self.running.as_mut()
            && let Some(error) = server.poll_exit()
        {
            self.last_error = Some(error.message);
            self.running = None;
        }
    }
    fn handle(&mut self, mut request: Request) {
        let allowed_hosts = [
            self.address.to_string(),
            format!("localhost:{}", self.address.port()),
        ];
        let request_header = |name: &str| {
            request
                .headers()
                .iter()
                .find(|h| h.0.eq_ignore_ascii_case(name))
                .map(|h| h.1.to_owned())
        };
        let host = request_header("Host").unwrap_or_default();
        let origin = request_header("Origin");
        if !allowed_hosts.contains(&host)
            || origin
                .as_ref()
                .is_some_and(|o| !allowed_hosts.iter().any(|h| o == &format!("http://{h}")))
        {
            send_json(
                request,
                403,
                error("Workbench accepts only its local Host and Origin"),
            );
            return;
        }
        let path = request.url().to_owned();
        if request.method() == "GET" {
            if let Some(id) = path
                .strip_prefix("/api/artifacts/")
                .and_then(|value| value.parse::<u64>().ok())
            {
                if let Some(artifact) = self.artifacts.get(&id) {
                    match fs::read(&artifact.path) {
                        Ok(bytes) => {
                            let _ = request.respond(
                                Response::from_bytes(bytes)
                                    .with_header(runtime::header(
                                        "Content-Type",
                                        "application/octet-stream",
                                    ))
                                    .with_header(runtime::header(
                                        "Content-Disposition",
                                        "attachment; filename=\"rivet-program\"",
                                    ))
                                    .with_header(runtime::header("Cache-Control", "no-store"))
                                    .with_header(runtime::header(
                                        "X-Content-Type-Options",
                                        "nosniff",
                                    )),
                            );
                        }
                        Err(e) => {
                            send_json(request, 404, error(&format!("Artifact unavailable: {e}")))
                        }
                    }
                } else {
                    send_json(request, 404, error("Unknown artifact"));
                }
                return;
            }

            if let Some(id) = path.strip_prefix("/api/examples/") {
                let example: Option<(&[u8], &str)> = match id {
                    "hello" => Some((include_bytes!("../examples/hello.rvb"), "hello.rvb")),
                    "server" => Some((include_bytes!("../examples/server.rvb"), "server.rvb")),
                    "computed" => {
                        Some((include_bytes!("../examples/computed.rvb"), "computed.rvb"))
                    }
                    "two-sum" => Some((include_bytes!("../examples/two-sum.rvb"), "two-sum.rvb")),
                    _ => None,
                };
                if let Some((bytes, filename)) = example {
                    let _ = request.respond(
                        Response::from_bytes(bytes.to_vec())
                            .with_header(runtime::header(
                                "Content-Type",
                                "application/octet-stream",
                            ))
                            .with_header(runtime::header(
                                "Content-Disposition",
                                &format!("attachment; filename=\"{filename}\""),
                            ))
                            .with_header(runtime::header("Cache-Control", "no-store"))
                            .with_header(runtime::header("X-Content-Type-Options", "nosniff")),
                    );
                } else {
                    send_json(request, 404, error("Unknown binary example"));
                }
                return;
            }
            match path.as_str() {
                "/" => send(
                    request,
                    200,
                    "text/html; charset=utf-8",
                    include_str!("../web/index.html").to_owned(),
                ),
                "/app.js" => send(
                    request,
                    200,
                    "text/javascript; charset=utf-8",
                    include_str!("../web/app.js").to_owned(),
                ),
                "/style.css" => send(
                    request,
                    200,
                    "text/css; charset=utf-8",
                    include_str!("../web/style.css").to_owned(),
                ),
                "/api/status" => send_json(request, 200, self.status()),
                "/api/examples" => send_json(
                    request,
                    200,
                    json!({"examples":[
                        {"id":"hello","name":"Hello world","url":"/api/examples/hello","filename":"hello.rvb"},
                        {"id":"server","name":"HTTP server","url":"/api/examples/server","filename":"server.rvb"},
                        {"id":"computed","name":"Computed values","url":"/api/examples/computed","filename":"computed.rvb"},
                        {"id":"two-sum","name":"Two Sum","url":"/api/examples/two-sum","filename":"two-sum.rvb","input":"4 2 7 11 15 9\n"}
                    ]}),
                ),
                _ => send_json(request, 404, error("Not found")),
            }
            return;
        }
        if request.method() != "POST" {
            send_json(request, 405, error("Use GET or POST"));
            return;
        }
        let binary_source = matches!(path.as_str(), "/api/build" | "/api/inspect");
        let expected_type = if binary_source {
            "application/octet-stream"
        } else {
            "application/json"
        };
        let content_type = request_header("Content-Type").unwrap_or_default();
        if request_header("X-Rivet-Request").as_deref() != Some("1")
            || !content_type
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case(expected_type)
        {
            send_json(
                request,
                403,
                error(&format!(
                    "This endpoint requires {expected_type} and X-Rivet-Request: 1"
                )),
            );
            return;
        }
        if path == "/api/build" {
            self.build = None;
        }
        let limit = if binary_source {
            65536
        } else if path == "/api/run" {
            MAX_RUN_BODY
        } else {
            MAX_BODY
        };
        if request.body_length().is_some_and(|n| n as u64 > limit) {
            send_json(
                request,
                413,
                error(&format!("Request body exceeds {limit} bytes")),
            );
            return;
        }
        let mut body = Vec::new();
        match request.as_reader().take(limit + 1).read_to_end(&mut body) {
            Ok(_) if body.len() as u64 <= limit => (),
            _ => {
                send_json(request, 400, error("Cannot read bounded request body"));
                return;
            }
        }
        let (status, response) = self.dispatch(&path, &body);
        send_json(request, status, response);
    }
    fn dispatch(&mut self, path: &str, body: &[u8]) -> (u16, Value) {
        self.check_child();
        match path {
            "/api/inspect" => match language::inspect_binary(body) {
                Ok(inspection) => (200, json!({"ok":true,"inspection":inspection})),
                Err(diagnostics) => (
                    400,
                    json!({"ok":false,"error":"Invalid binary source","diagnostics":diagnostics}),
                ),
            },
            "/api/build" => {
                // Failure invalidates the build, but an already-running server owns its own artifact.
                self.build = None;
                match language::compile_binary(body) {
                    Ok(program) => {
                        let id = self.next_build;
                        self.next_build += 1;
                        let output = self.artifact_root.join(id.to_string()).join("program");
                        let artifact =
                            match native::build_native(&program, &output, BuildProfile::Size) {
                                Ok(artifact) => artifact,
                                Err(e) => return (400, error(&e)),
                            };
                        let llvm_ir = match fs::read_to_string(&artifact.llvm_path) {
                            Ok(value) => value,
                            Err(e) => return (500, error(&e.to_string())),
                        };
                        let assembly = match fs::read_to_string(&artifact.assembly_path) {
                            Ok(value) => value,
                            Err(e) => return (500, error(&e.to_string())),
                        };
                        let info = BuildInfo {
                            id,
                            name: program.name().to_owned(),
                            instructions: program.instruction_count(),
                            ir: program.disassemble(),
                            artifact: json!({"path":artifact.path,"size_bytes":artifact.size_bytes,"profile":artifact.profile,"target":artifact.target,"download_url":format!("/api/artifacts/{id}")}),
                            llvm_ir,
                            assembly,
                            build_ms: artifact.build_ms,
                        };
                        self.last_error = None;
                        let response = json!({"ok":true,"build":info,"diagnostics":[]});
                        self.artifacts.insert(id, artifact.clone());
                        self.build = Some(Build { info, artifact });
                        (200, response)
                    }
                    Err(diagnostics) => (
                        400,
                        json!({"ok":false,"error":"Build failed","diagnostics":diagnostics}),
                    ),
                }
            }
            "/api/run" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Input {
                    build_id: u64,
                    #[serde(default)]
                    input: String,
                }
                let input: Input = match serde_json::from_slice(body) {
                    Ok(v) => v,
                    Err(e) => return (400, error(&format!("Invalid run request: {e}"))),
                };
                if self.running.is_some() {
                    return (
                        409,
                        error("Stop the running server before running another program"),
                    );
                }
                let Some(build) = &self.build else {
                    return (409, error("Build a valid program first"));
                };
                if build.info.id != input.build_id {
                    return (409, error("Build is stale; build the current source again"));
                }
                if input.input.len() > native_process::MAX_INPUT_BYTES {
                    return (413, error("Runtime input exceeds 262144 bytes"));
                }
                if !input.input.is_empty() && !matches!(build.artifact.entry, EntryInfo::Compute) {
                    return (400, error("This program does not accept runtime input"));
                }
                self.last_error = None;
                match &build.artifact.entry {
                    EntryInfo::Console | EntryInfo::Compute => {
                        match native_process::run_console_with_input(&build.artifact, &input.input)
                        {
                            Ok(result) => {
                                self.last_stdout = result.stdout.clone();
                                (
                                    200,
                                    json!({"ok":true,"kind":"console","stdout":result.stdout,"stderr":result.stderr,"exit_code":result.exit_code,"elapsed_us":result.elapsed_us}),
                                )
                            }
                            Err(e) => (
                                400,
                                json!({"ok":false,"error":e.message,"stderr":e.stderr,"exit_code":e.exit_code,"diagnostics":[]}),
                            ),
                        }
                    }
                    EntryInfo::Http { .. } => match NativeServer::start(&build.artifact) {
                        Ok(server) => {
                            let info = server.info(build.info.id);
                            self.running = Some((build.info.id, server));
                            (200, json!({"ok":true,"kind":"http","server":info}))
                        }
                        Err(e) => (
                            400,
                            json!({"ok":false,"error":e.message,"stderr":e.stderr,"exit_code":e.exit_code,"diagnostics":[]}),
                        ),
                    },
                }
            }
            "/api/stop" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Input {}
                if serde_json::from_slice::<Input>(body).is_err() {
                    return (400, error("Stop expects an empty JSON object"));
                }
                let stopped = self.running.is_some();
                if let Some((_, mut server)) = self.running.take()
                    && let Err(e) = server.stop()
                {
                    return (500, error(&e));
                }
                (200, json!({"ok":true,"stopped":stopped}))
            }
            "/api/request" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Input {
                    path: String,
                }
                let input: Input = match serde_json::from_slice(body) {
                    Ok(v) => v,
                    Err(e) => return (400, error(&format!("Invalid request: {e}"))),
                };
                let Some((_, server)) = &self.running else {
                    return (409, error("Start a server before sending a request"));
                };
                match server.request(&input.path) {
                    Ok(response) => (
                        200,
                        json!({"ok":true,"status":response.status,"content_type":response.content_type,"body":response.body,"elapsed_us":response.elapsed_us}),
                    ),
                    Err(e) => (400, error(&e)),
                }
            }
            _ => (404, error("Unknown API endpoint")),
        }
    }
}

fn error(message: &str) -> Value {
    json!({"ok":false,"error":message,"diagnostics":[]})
}
fn send_json(request: Request, status: u16, value: Value) {
    send(
        request,
        status,
        "application/json; charset=utf-8",
        value.to_string(),
    );
}
fn send(request: Request, status: u16, content_type: &str, body: String) {
    let response = Response::from_string(body).with_status(status)
        .with_header(runtime::header("Content-Type",content_type))
        .with_header(runtime::header("Cache-Control","no-store"))
        .with_header(runtime::header("Connection","close"))
        .with_header(runtime::header("X-Content-Type-Options","nosniff"))
        .with_header(runtime::header("X-Frame-Options","DENY"))
        .with_header(runtime::header("Referrer-Policy","no-referrer"))
        .with_header(runtime::header("Content-Security-Policy","default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'; form-action 'none'"));
    let _ = request.respond(response);
}
