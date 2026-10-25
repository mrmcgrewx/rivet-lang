//! Local editor service. Build artifacts and live server state are deliberately separate.
use crate::http::{Request, Response, Server};
use crate::{
    language::{self, CompiledProgram, Execution},
    runtime::{self, ProgramServer},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Read,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_BODY: u64 = 512 * 1024;

#[derive(Clone, Debug, Serialize)]
struct BuildInfo {
    id: u64,
    name: String,
    instructions: usize,
    ir: String,
}
struct Build {
    info: BuildInfo,
    program: CompiledProgram,
}

pub struct Workbench {
    listener: Server,
    address: SocketAddr,
    build: Option<Build>,
    next_build: u64,
    running: Option<(u64, ProgramServer)>,
    last_stdout: String,
}

impl Workbench {
    pub fn bind(port: u16) -> Result<Self, String> {
        let listener = Server::http(("127.0.0.1", port))
            .map_err(|e| format!("Cannot start workbench on 127.0.0.1:{port}: {e}"))?;
        let address = listener.address();
        Ok(Self {
            listener,
            address,
            build: None,
            next_build: 1,
            running: None,
            last_stdout: String::new(),
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
    fn status(&self) -> Value {
        json!({"ok":true,"build":self.build.as_ref().map(|b| &b.info),"server":self.running.as_ref().map(|(id,s)|s.info(*id)),"last_stdout":self.last_stdout})
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
                        {"id":"hello","name":"Hello world","source":include_str!("../examples/hello.rivet.json")},
                        {"id":"server","name":"HTTP server","source":include_str!("../examples/server.rivet.json")},
                        {"id":"computed","name":"Computed values","source":include_str!("../examples/computed.rivet.json")}
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
        let content_type = request_header("Content-Type").unwrap_or_default();
        if request_header("X-Rivet-Request").as_deref() != Some("1")
            || !content_type
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("application/json")
        {
            send_json(
                request,
                403,
                error("API mutations require JSON and X-Rivet-Request: 1"),
            );
            return;
        }
        if request.body_length().is_some_and(|n| n as u64 > MAX_BODY) {
            send_json(request, 413, error("API request exceeds 512 KiB"));
            return;
        }
        let mut body = String::new();
        match request
            .as_reader()
            .take(MAX_BODY + 1)
            .read_to_string(&mut body)
        {
            Ok(_) if body.len() as u64 <= MAX_BODY => (),
            _ => {
                send_json(
                    request,
                    400,
                    error("Cannot read bounded UTF-8 request body"),
                );
                return;
            }
        }
        let (status, response) = self.dispatch(&path, &body);
        send_json(request, status, response);
    }
    fn dispatch(&mut self, path: &str, body: &str) -> (u16, Value) {
        match path {
            "/api/build" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Input {
                    source: String,
                }
                let input: Input = match serde_json::from_str(body) {
                    Ok(v) => v,
                    Err(e) => return (400, error(&format!("Invalid build request: {e}"))),
                };
                // Failure invalidates the build, but an already-running server owns its own artifact.
                self.build = None;
                match language::compile(&input.source) {
                    Ok(program) => {
                        let info = BuildInfo {
                            id: self.next_build,
                            name: program.name().to_string(),
                            instructions: program.instruction_count(),
                            ir: program.disassemble(),
                        };
                        self.next_build += 1;
                        let response = json!({"ok":true,"build":info,"diagnostics":[]});
                        self.build = Some(Build { info, program });
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
                }
                let input: Input = match serde_json::from_str(body) {
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
                let started = Instant::now();
                match build.program.execute() {
                    Ok(Execution::Console { stdout }) => {
                        self.last_stdout = stdout.clone();
                        (
                            200,
                            json!({"ok":true,"kind":"console","stdout":stdout,"elapsed_us":started.elapsed().as_micros()}),
                        )
                    }
                    Ok(Execution::Http { port, routes }) => {
                        match ProgramServer::start(port, routes) {
                            Ok(server) => {
                                let info = server.info(build.info.id);
                                self.running = Some((build.info.id, server));
                                (200, json!({"ok":true,"kind":"http","server":info}))
                            }
                            Err(e) => (400, error(&e)),
                        }
                    }
                    Err(e) => (400, error(&e)),
                }
            }
            "/api/stop" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Input {}
                if serde_json::from_str::<Input>(body).is_err() {
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
                let input: Input = match serde_json::from_str(body) {
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
