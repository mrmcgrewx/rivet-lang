use rivet::{
    language::{self, Execution},
    runtime::ProgramServer,
    workbench::Workbench,
};
use std::{
    env,
    fs::File,
    io::Read,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rivet: {error}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] || args == ["help"] {
        println!(
            "Rivet 0.1 — typed dataflow language\n\n  rivet ui [--port PORT]    Open local workbench (default 7331)\n  rivet check FILE          Compile and print register IR\n  rivet run FILE            Execute; HTTP programs stop with Ctrl-C\n\nAll listeners bind to 127.0.0.1. Port 0 selects a free port."
        );
        return Ok(());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Release)).map_err(|e| e.to_string())?;
    match args[0].as_str() {
        "ui" => {
            let port = match args.as_slice() {
                [_] => 7331,
                [_, flag, port] if flag == "--port" => port
                    .parse::<u16>()
                    .map_err(|_| "Port must be an integer from 0 to 65535")?,
                _ => return Err("Usage: rivet ui [--port PORT]".into()),
            };
            let workbench = Workbench::bind(port)?;
            println!("Rivet workbench: http://{}", workbench.address());
            println!("Press Ctrl-C to stop the workbench and its program server.");
            workbench.serve(stop)
        }
        "check" | "run" if args.len() == 2 => {
            let mut source = String::new();
            File::open(&args[1])
                .map_err(|e| format!("{}: {e}", args[1]))?
                .take(65537)
                .read_to_string(&mut source)
                .map_err(|e| format!("Cannot read UTF-8 program: {e}"))?;
            let program = language::compile(&source).map_err(|errors| {
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            if args[0] == "check" {
                println!("{}", program.disassemble());
                return Ok(());
            }
            match program.execute()? {
                Execution::Console { stdout } => {
                    print!("{stdout}");
                    Ok(())
                }
                Execution::Http { port, routes } => {
                    let mut server = ProgramServer::start(port, routes)?;
                    let info = server.info(0);
                    println!("Rivet program server: {}", info.address);
                    for path in info.routes {
                        println!("GET {}{path}", info.address);
                    }
                    while !stop.load(Ordering::Acquire) {
                        thread::sleep(Duration::from_millis(25));
                    }
                    server.stop()
                }
            }
        }
        _ => Err("Unknown command or arguments. Run rivet --help.".into()),
    }
}
