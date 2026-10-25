//! Lower validated Rivet operations to LLVM, then build an independent native artifact.
use crate::language::{CompiledProgram, EntryInfo};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildProfile {
    Size,
    Baseline,
}
impl BuildProfile {
    pub fn name(self) -> &'static str {
        match self {
            Self::Size => "size",
            Self::Baseline => "baseline",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "size" => Ok(Self::Size),
            "baseline" => Ok(Self::Baseline),
            _ => Err("Profile must be size or baseline".into()),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct NativeArtifact {
    pub path: PathBuf,
    pub llvm_path: PathBuf,
    pub assembly_path: PathBuf,
    pub metadata_path: PathBuf,
    pub size_bytes: u64,
    pub profile: BuildProfile,
    pub target: String,
    pub build_ms: u128,
    pub entry: EntryInfo,
}
pub fn unique_directory(parent: &Path, prefix: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    for _ in 0..10 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let path = parent.join(format!(
            "{prefix}-{}-{stamp}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return fs::canonicalize(path).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Cannot allocate a unique artifact directory".into())
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn build_native(
    program: &CompiledProgram,
    output: &Path,
    profile: BuildProfile,
) -> Result<NativeArtifact, String> {
    let started = Instant::now();
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create output directory: {e}"))?;
    let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let filename = output.file_name().ok_or("Output needs a filename")?;
    let output = parent.join(filename);
    let sidecar = |suffix: &str| {
        let mut path = output.as_os_str().to_os_string();
        path.push(suffix);
        PathBuf::from(path)
    };
    let llvm_path = sidecar(".ll");
    let assembly_path = sidecar(".s");
    let metadata_path = sidecar(".build.json");
    for path in [&output, &llvm_path, &assembly_path, &metadata_path] {
        if path.symlink_metadata().is_ok() {
            return Err(format!(
                "Output already exists: {}. Choose a new output path or remove the previous generated artifact.",
                path.display()
            ));
        }
    }
    let stage = Stage(unique_directory(&parent, ".rivet-stage")?);
    let ll = stage.0.join("program.ll");
    let asm = stage.0.join("program.s");
    let object = stage.0.join("program.o");
    let runtime = stage.0.join("runtime.c");
    let runtime_object = stage.0.join("runtime.o");
    let executable = stage.0.join("program");
    fs::write(&ll, program.emit_llvm()).map_err(|e| e.to_string())?;
    fs::write(&runtime, include_str!("../native/runtime.c")).map_err(|e| e.to_string())?;
    fs::write(
        stage.0.join("runtime.h"),
        include_str!("../native/runtime.h"),
    )
    .map_err(|e| e.to_string())?;
    let clang = std::env::var_os("RIVET_CLANG").unwrap_or_else(|| "clang".into());
    let version = checked(Command::new(&clang).arg("--version"))?;
    let target = checked(Command::new(&clang).arg("-dumpmachine"))?
        .trim()
        .to_owned();
    let optimization = match profile {
        BuildProfile::Size => "-Oz",
        BuildProfile::Baseline => "-O0",
    };
    checked(
        Command::new(&clang)
            .arg("-c")
            .arg(&ll)
            .arg(optimization)
            .arg("-o")
            .arg(&object),
    )?;
    checked(
        Command::new(&clang)
            .arg("-S")
            .arg(&ll)
            .arg(optimization)
            .arg("-o")
            .arg(&asm),
    )?;
    checked(
        Command::new(&clang)
            .args([
                "-c",
                "-std=c11",
                "-D_POSIX_C_SOURCE=200809L",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-ffunction-sections",
                "-fdata-sections",
                optimization,
            ])
            .arg(&runtime)
            .arg("-o")
            .arg(&runtime_object),
    )?;
    let mut link = Command::new(&clang);
    link.arg(&object).arg(&runtime_object);
    if matches!(profile, BuildProfile::Size) {
        link.arg(if cfg!(target_os = "macos") {
            "-Wl,-dead_strip"
        } else {
            "-Wl,--gc-sections"
        });
    }
    checked(link.arg("-o").arg(&executable))?;
    let artifact = NativeArtifact {
        path: output,
        llvm_path,
        assembly_path,
        metadata_path,
        size_bytes: fs::metadata(&executable).map_err(|e| e.to_string())?.len(),
        profile,
        target,
        build_ms: started.elapsed().as_millis(),
        entry: program.entry_info(),
    };
    let runtime_abi = if matches!(artifact.entry, EntryInfo::Compute) {
        2
    } else {
        1
    };
    let metadata = serde_json::json!({"format":"rivet-native-build-v1","program":program.name(),"artifact":artifact,"backend":version.lines().next().unwrap_or("unknown"),"optimization":optimization,"runtime_abi":runtime_abi,"generated_from":"validated typed register IR","source_or_interpreter_required_at_run_time":false});
    // Publish only fully linked artifacts; generated siblings are inspection data, never runtime inputs.
    fs::rename(&ll, &artifact.llvm_path).map_err(|e| e.to_string())?;
    fs::rename(&asm, &artifact.assembly_path).map_err(|e| e.to_string())?;
    fs::write(
        &artifact.metadata_path,
        serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&executable, &artifact.path).map_err(|e| e.to_string())?;
    Ok(artifact)
}

fn checked(command: &mut Command) -> Result<String, String> {
    let description = format!("{command:?}");
    let mut child=command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|format!("Cannot launch native backend ({description}): {e}. Install Clang/Xcode command-line tools or set RIVET_CLANG."))?;
    let stdout = child.stdout.take().expect("stdout configured");
    let stderr = child.stderr.take().expect("stderr configured");
    fn drain(mut input: impl Read) -> Vec<u8> {
        let mut result = Vec::new();
        let mut buf = [0; 4096];
        while let Ok(n) = input.read(&mut buf) {
            if n == 0 {
                break;
            }
            let keep = n.min(1024 * 1024 - result.len());
            result.extend_from_slice(&buf[..keep]);
        }
        result
    }
    let out = thread::spawn(move || drain(stdout));
    let err = thread::spawn(move || drain(stderr));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(30) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out.join();
                    let _ = err.join();
                    return Err(format!("Native backend exceeded 30 seconds: {description}"));
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    };
    let stdout = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    if !status.success() {
        return Err(format!(
            "Native backend failed ({status}): {stderr}\n{description}"
        ));
    }
    Ok(stdout)
}
