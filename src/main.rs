use rivet::{
    language::{self, CompiledProgram, Execution},
    native::{self, BuildProfile},
    native_process::{self, NativeServer},
    runtime::ProgramServer,
    workbench::Workbench,
};
use std::{
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
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
fn read_program(file: &str) -> Result<CompiledProgram, String> {
    let mut source = Vec::new();
    File::open(file)
        .map_err(|e| format!("{file}: {e}"))?
        .take(65537)
        .read_to_end(&mut source)
        .map_err(|e| format!("Cannot read binary program: {e}"))?;
    language::compile_binary(&source).map_err(|errors| {
        errors
            .iter()
            .map(|e| format!("{}: {}", e.path, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] || args == ["help"] {
        println!(
            "Rivet — binary source native compiler (RVB1/RVB2)\n\n  rivet build FILE -o PATH [--profile size|baseline|speed]\n                            Build a standalone executable and inspectable LLVM/assembly\n  rivet run FILE            Build and launch native executable\n  rivet interpret FILE      Run the reference interpreter\n  rivet inspect FILE        Decode binary source to a readable view\n  rivet check FILE          Validate and print typed register IR\n  rivet ui [--port PORT]    Start local workbench (default 7331)\n\nSource files are binary RVB (.rvb). JSON/text source is not accepted.\nCompute programs read runtime data from stdin (finish with EOF).\nTwo Sum input: count, array elements, target; e.g. '4 2 7 11 15 9'.\nBuild requires Clang/Xcode command-line tools. Executables need only system libraries.\nAll listeners bind 127.0.0.1; port 0 chooses a free port. Ctrl-C stops managed servers."
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
            println!("Build creates persistent native artifacts. Ctrl-C stops managed processes.");
            workbench.serve(stop)
        }
        "build" if args.len() >= 4 => {
            let program = read_program(&args[1])?;
            let mut output: Option<PathBuf> = None;
            let mut profile = BuildProfile::Size;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "-o" if i + 1 < args.len() && output.is_none() => {
                        output = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--profile" if i + 1 < args.len() => {
                        profile = BuildProfile::parse(&args[i + 1])?;
                        i += 2;
                    }
                    _ => {
                        return Err(
                            "Usage: rivet build FILE -o PATH [--profile size|baseline|speed]"
                                .into(),
                        );
                    }
                }
            }
            let output = output.ok_or("Build requires -o PATH")?;
            let artifact = native::build_native(&program, &output, profile)?;
            println!(
                "Executable: {}\nLLVM IR: {}\nAssembly: {}\nBuild metadata: {}\n{} bytes · {} · {} ms",
                artifact.path.display(),
                artifact.llvm_path.display(),
                artifact.assembly_path.display(),
                artifact.metadata_path.display(),
                artifact.size_bytes,
                artifact.target,
                artifact.build_ms
            );
            Ok(())
        }
        "inspect" | "check" | "run" | "interpret" if args.len() == 2 => {
            let program = read_program(&args[1])?;
            if args[0] == "check" || args[0] == "inspect" {
                println!("{}", program.disassemble());
                return Ok(());
            }
            let mut input = String::new();
            if matches!(program.entry_info(), language::EntryInfo::Compute) {
                std::io::stdin()
                    .take(native_process::MAX_INPUT_BYTES as u64 + 1)
                    .read_to_string(&mut input)
                    .map_err(|e| format!("Cannot read runtime input: {e}"))?;
                if input.len() > native_process::MAX_INPUT_BYTES {
                    return Err("Runtime input exceeds 262144 bytes".into());
                }
            }
            if args[0] == "interpret" {
                return match program.execute_with_input(&input)? {
                    Execution::Console { stdout } => {
                        print!("{stdout}");
                        Ok(())
                    }
                    Execution::Http { port, routes } => {
                        let mut server = ProgramServer::start(port, routes)?;
                        println!("Rivet reference server: {}", server.info(0).address);
                        while !stop.load(Ordering::Acquire) {
                            thread::sleep(Duration::from_millis(25));
                        }
                        server.stop()
                    }
                };
            }
            let directory = native::unique_directory(Path::new(".rivet/cli"), "run")?;
            let artifact =
                native::build_native(&program, &directory.join("program"), BuildProfile::Size)?;
            match artifact.entry {
                language::EntryInfo::Console | language::EntryInfo::Compute => {
                    let result = native_process::run_console_with_input(&artifact, &input)
                        .map_err(|e| e.message)?;
                    print!("{}", result.stdout);
                    eprint!("{}", result.stderr);
                    Ok(())
                }
                language::EntryInfo::Http { .. } => {
                    let mut server = NativeServer::start(&artifact).map_err(|e| e.message)?;
                    println!("Rivet program server: {}", server.info(0).address);
                    while !stop.load(Ordering::Acquire) {
                        if let Some(error) = server.poll_exit() {
                            return Err(error.message);
                        }
                        thread::sleep(Duration::from_millis(25));
                    }
                    server.stop()
                }
            }
        }
        _ => Err("Unknown command or arguments. Run rivet --help.".into()),
    }
}
