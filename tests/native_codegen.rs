use rivet::language::{EntryInfo, Execution, compile};
use rivet::native::{BuildProfile, build_native};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const HELLO: &str = include_str!("../examples/hello.rivet.json");
const SERVER: &str = include_str!("../examples/server.rivet.json");
const COMPUTED: &str = include_str!("../examples/computed.rivet.json");
static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "rivet-codegen-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn console(instructions: Value, output: &str) -> String {
    json!({
        "version": 1,
        "name": "native test",
        "instructions": instructions,
        "entry": {"kind":"console", "value":output}
    })
    .to_string()
}

#[test]
fn entry_metadata_does_not_execute_failing_instructions() {
    let source = console(
        json!([
            {"id":"max", "op":"int", "value":i64::MAX},
            {"id":"one", "op":"int", "value":1},
            {"id":"overflow", "op":"add", "left":"max", "right":"one"},
            {"id":"out", "op":"text", "value":"unreachable"}
        ]),
        "out",
    );
    let program = compile(&source).unwrap();
    assert!(program.execute().is_err());
    assert!(matches!(program.entry_info(), EntryInfo::Console));
    assert_eq!(
        serde_json::to_value(program.entry_info()).unwrap(),
        json!({"kind":"console"})
    );
    assert_eq!(
        serde_json::to_value(compile(SERVER).unwrap().entry_info()).unwrap(),
        json!({"kind":"http", "port":8787, "routes":["/hello"]})
    );
    let ir = program.emit_llvm();
    assert!(ir.contains("@llvm.sadd.with.overflow.i64(i64 9223372036854775807, i64 1)"));
    assert!(ir.contains("br i1 %overflow2, label %failed2, label %next2"));
    assert!(ir.find("failed2:").unwrap() < ir.find("; %3: out").unwrap());
    assert!(ir.contains("unreachable\nnext2:"));
}

#[test]
fn literals_are_escaped_bytes_with_lengths_and_extra_terminators() {
    let text = "a\0\"\\\n雪🦀\n}\ndefine i32 @injected() {";
    let mut source: Value = serde_json::from_str(&console(
        json!([{"id":"out", "op":"text", "value":text}]),
        "out",
    ))
    .unwrap();
    source["name"] = json!("unsafe\n}\ndefine i32 @injected() {");
    let ir = compile(&source.to_string()).unwrap().emit_llvm();
    let escaped = text
        .bytes()
        .map(|byte| format!("\\{byte:02X}"))
        .collect::<String>();
    assert!(ir.contains(&format!(
        "constant [{} x i8] c\"{escaped}\\00\"",
        text.len() + 1
    )));
    assert!(ir.contains(&format!("call ptr @rv_text(ptr @c0, i64 {})", text.len())));
    assert!(!ir.contains("@injected"));
    assert_eq!(ir.matches("define i32 @main").count(), 1);
}

#[test]
fn native_lowering_is_deterministic_and_covers_the_operation_abi() {
    let source = console(
        json!([
            {"id":"a", "op":"int", "value":20},
            {"id":"b", "op":"int", "value":22},
            {"id":"sum", "op":"add", "left":"a", "right":"b"},
            {"id":"number", "op":"to_text", "value":"sum"},
            {"id":"truth", "op":"bool", "value":true},
            {"id":"bool_text", "op":"to_text", "value":"truth"},
            {"id":"text", "op":"text", "value":"π\0"},
            {"id":"alias", "op":"to_text", "value":"text"},
            {"id":"out", "op":"concat", "values":["alias", "number", "bool_text"]},
            {"id":"empty", "op":"object", "fields":{}},
            {"id":"object", "op":"object", "fields":{"z":"empty", "n":"sum", "b":"truth", "a":"out"}}
        ]),
        "out",
    );
    let program = compile(&source).unwrap();
    let ir = program.emit_llvm();
    assert_eq!(ir, compile(&source).unwrap().emit_llvm());
    for call in [
        "call ptr @rv_i64_text(i64 %r2)",
        "zext i1 true to i8",
        "call ptr @rv_bool_text(i8 zeroext %bool5)",
        "call ptr @rv_concat(ptr %concat8, i64 3)",
        "call ptr @rv_json_text(ptr %r8)",
        "call ptr @rv_json_i64(i64 %r2)",
        "call ptr @rv_json_bool(i8 zeroext %bool10.1)",
        "call ptr @rv_object(ptr null, ptr null, ptr null, i64 0)",
        "call void @rv_print(ptr %r8)",
        "call void @rv_cleanup()",
    ] {
        assert!(ir.contains(call), "missing {call} in:\n{ir}");
    }
    assert!(ir.contains("store ptr %r9, ptr %object10.3"));
}

#[test]
fn http_lowering_registers_program_routes_and_returns_native_exit_status() {
    let ir = compile(SERVER).unwrap().emit_llvm();
    assert!(ir.contains("call void @rv_add_route(ptr"));
    assert!(ir.contains("i64 6, i32 200, ptr %r1)"));
    assert!(ir.contains("%exit = call i32 @rv_serve(i32 8787)"));
    assert!(ir.contains("call void @rv_cleanup()\n  ret i32 %exit"));
    let empty = json!({"version":1,"name":"empty","instructions":[],"entry":{"kind":"http","port":0,"routes":[]}});
    let ir = compile(&empty.to_string()).unwrap().emit_llvm();
    assert!(!ir.contains("call void @rv_add_route"));
    assert!(ir.contains("%exit = call i32 @rv_serve(i32 0)"));
}

#[test]
fn clang_accepts_emitted_modules_for_examples_and_empty_values() {
    let scratch = Scratch::new();
    let empty = console(
        json!([
            {"id":"out","op":"concat","values":[]},
            {"id":"obj","op":"object","fields":{}}
        ]),
        "out",
    );
    let escaped = console(
        json!([
            {"id":"out","op":"text","value":"\0\"\\\n雪🦀"},
            {"id":"obj","op":"object","fields":{"\0\n雪":"out"}}
        ]),
        "out",
    );
    for (index, source) in [HELLO, SERVER, COMPUTED, &empty, &escaped]
        .iter()
        .enumerate()
    {
        let llvm = scratch.0.join(format!("program{index}.ll"));
        let object = scratch.0.join(format!("program{index}.o"));
        fs::write(&llvm, compile(source).unwrap().emit_llvm()).unwrap();
        let result = Command::new("clang")
            .args(["-Wno-override-module", "-c"])
            .arg(&llvm)
            .arg("-o")
            .arg(&object)
            .output()
            .expect("native codegen verification requires Clang");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(object.metadata().unwrap().len() > 0);
    }
}

fn assert_console_native_matches(source: &str, error_category: Option<&str>) {
    let program = compile(source).unwrap();
    let reference = program.execute();
    let scratch = Scratch::new();
    for profile in [BuildProfile::Baseline, BuildProfile::Size] {
        let artifact = build_native(&program, &scratch.0.join(profile.name()), profile).unwrap();
        let output = Command::new(&artifact.path).output().unwrap();
        match &reference {
            Ok(Execution::Console { stdout }) => {
                assert!(error_category.is_none());
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(&output.stdout, stdout.as_bytes());
                assert!(output.stderr.is_empty());
            }
            Err(message) => {
                let category = error_category.expect("expected reference success");
                assert!(message.contains(category), "reference error: {message}");
                assert!(!output.status.success());
                assert!(
                    output.stdout.is_empty(),
                    "failed evaluation must not print output"
                );
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.contains(category), "native error: {stderr}");
            }
            other => panic!("expected a console program: {other:?}"),
        }
    }
}

#[test]
fn both_profiles_match_reference_console_values_and_boundaries() {
    let scalar = console(
        json!([
            {"id":"min", "op":"int", "value":i64::MIN},
            {"id":"max", "op":"int", "value":i64::MAX},
            {"id":"sum", "op":"add", "left":"min", "right":"max"},
            {"id":"min_text", "op":"to_text", "value":"min"},
            {"id":"max_text", "op":"to_text", "value":"max"},
            {"id":"sum_text", "op":"to_text", "value":"sum"},
            {"id":"yes", "op":"bool", "value":true},
            {"id":"no", "op":"bool", "value":false},
            {"id":"yes_text", "op":"to_text", "value":"yes"},
            {"id":"no_text", "op":"to_text", "value":"no"},
            {"id":"space", "op":"text", "value":" \0雪🦀\"\\\n"},
            {"id":"copy", "op":"to_text", "value":"space"},
            {"id":"out", "op":"concat", "values":["min_text", "copy", "max_text", "copy", "sum_text", "copy", "yes_text", "copy", "no_text"]},
            {"id":"empty", "op":"object", "fields":{}},
            {"id":"obj", "op":"object", "fields":{"\0雪":"out", "n":"min", "b":"no", "nested":"empty"}}
        ]),
        "out",
    );
    let empty = console(json!([{"id":"out", "op":"concat", "values":[]}]), "out");
    // Text is 65535 bytes; the mandatory newline exactly fills the output bound.
    let boundary = console(
        json!([
            {"id":"chunk", "op":"text", "value":"x".repeat(16383)},
            {"id":"tail", "op":"text", "value":"xxx"},
            {"id":"out", "op":"concat", "values":["chunk","chunk","chunk","chunk","tail"]}
        ]),
        "out",
    );
    for source in [HELLO, &scalar, &empty, &boundary] {
        assert_console_native_matches(source, None);
    }
}

#[test]
fn both_profiles_preserve_unused_overflows_and_first_failure_order() {
    for (left, right) in [(i64::MAX, 1), (i64::MIN, -1)] {
        let source = console(
            json!([
                {"id":"left", "op":"int", "value":left},
                {"id":"right", "op":"int", "value":right},
                {"id":"unused", "op":"add", "left":"left", "right":"right"},
                {"id":"chunk", "op":"text", "value":"x".repeat(16384)},
                {"id":"too_large", "op":"concat", "values":["chunk","chunk","chunk","chunk","chunk"]},
                {"id":"out", "op":"text", "value":"must not print"}
            ]),
            "out",
        );
        assert_console_native_matches(&source, Some("signed 64-bit integer overflow"));
    }
    let source = console(
        json!([
            {"id":"chunk", "op":"text", "value":"x".repeat(16384)},
            {"id":"too_large", "op":"concat", "values":["chunk","chunk","chunk","chunk","chunk"]},
            {"id":"max", "op":"int", "value":i64::MAX},
            {"id":"one", "op":"int", "value":1},
            {"id":"unused", "op":"add", "left":"max", "right":"one"},
            {"id":"out", "op":"text", "value":"must not print"}
        ]),
        "out",
    );
    assert_console_native_matches(&source, Some("computed Text exceeds 65536 bytes"));
}

#[test]
fn both_profiles_preserve_text_output_and_json_size_limits() {
    let output_too_large = console(
        json!([
            {"id":"chunk", "op":"text", "value":"x".repeat(16384)},
            {"id":"out", "op":"concat", "values":["chunk","chunk","chunk","chunk"]}
        ]),
        "out",
    );
    assert_console_native_matches(
        &output_too_large,
        Some("console output including newline exceeds 65536 bytes"),
    );
    // {"x":"..."} adds eight bytes to a 65528-byte Text value.
    for (tail, error) in [("", None), ("x", Some("computed Json exceeds 65536 bytes"))] {
        let source = console(
            json!([
                {"id":"chunk", "op":"text", "value":"x".repeat(16382)},
                {"id":"tail", "op":"text", "value":tail},
                {"id":"text", "op":"concat", "values":["chunk","chunk","chunk","chunk","tail"]},
                {"id":"unused", "op":"object", "fields":{"x":"text"}},
                {"id":"out", "op":"text", "value":"okay"}
            ]),
            "out",
        );
        assert_console_native_matches(&source, error);
    }
    let escaped_too_large = console(
        json!([
            {"id":"chunk", "op":"text", "value":"\0".repeat(1000)},
            {"id":"text", "op":"concat", "values":["chunk","chunk","chunk","chunk","chunk","chunk","chunk","chunk","chunk","chunk","chunk"]},
            {"id":"unused", "op":"object", "fields":{"x":"text"}},
            {"id":"out", "op":"text", "value":"must not print"}
        ]),
        "out",
    );
    assert_console_native_matches(
        &escaped_too_large,
        Some("computed Json exceeds 65536 bytes"),
    );
}
