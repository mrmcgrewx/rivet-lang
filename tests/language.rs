use rivet::language::{Execution, compile};
use serde_json::{Value, json};

const HELLO: &str = include_str!("fixtures/legacy/hello.rivet.json");
const SERVER: &str = include_str!("fixtures/legacy/server.rivet.json");
const COMPUTED: &str = include_str!("fixtures/legacy/computed.rivet.json");

fn console_program(instructions: Value, output: &str) -> Value {
    json!({
        "version": 1,
        "name": "test",
        "instructions": instructions,
        "entry": {"kind": "console", "value": output}
    })
}

fn compile_value(value: &Value) -> rivet::language::CompiledProgram {
    compile(&value.to_string())
        .unwrap_or_else(|errors| panic!("unexpected diagnostics: {errors:#?}"))
}

fn rejects(value: &Value, path: &str, message: &str) {
    let errors = compile(&value.to_string()).expect_err("invalid program unexpectedly compiled");
    assert!(
        errors
            .iter()
            .any(|error| error.path == path && error.message.contains(message)),
        "expected {path}: {message}, got {errors:#?}"
    );
}

fn stdout(program: &rivet::language::CompiledProgram) -> String {
    match program.execute().unwrap() {
        Execution::Console { stdout } => stdout,
        other => panic!("expected console output, got {other:?}"),
    }
}

#[test]
fn hello_example_and_edits_execute_through_registers() {
    let program = compile(HELLO).unwrap();
    assert_eq!(program.name(), "Hello world");
    assert_eq!(program.instruction_count(), 3);
    assert_eq!(stdout(&program), "Hello, world!\n");
    assert_eq!(stdout(&program.clone()), "Hello, world!\n");
    let mut edited: Value = serde_json::from_str(HELLO).unwrap();
    edited["instructions"][1]["value"] = json!("Rivet!");
    assert_eq!(stdout(&compile_value(&edited)), "Hello, Rivet!\n");
}

#[test]
fn server_example_yields_program_defined_route_and_body() {
    match compile(SERVER).unwrap().execute().unwrap() {
        Execution::Http { port, routes } => {
            assert_eq!(port, 8787);
            assert_eq!(routes.len(), 1);
            assert_eq!(routes[0].method, "GET");
            assert_eq!(routes[0].path, "/hello");
            assert_eq!(routes[0].status, 200);
            assert_eq!(routes[0].body, json!({"message":"hello world"}));
        }
        other => panic!("expected HTTP execution, got {other:?}"),
    }
}

#[test]
fn computed_example_preserves_numbers_booleans_and_nested_json() {
    match compile(COMPUTED).unwrap().execute().unwrap() {
        Execution::Http { port, routes } => {
            assert_eq!(port, 0);
            assert_eq!(routes[0].path, "/computed");
            assert_eq!(
                routes[0].body,
                json!({
                    "message": "The answer is 42",
                    "details": {"answer": 42, "ready": true}
                })
            );
        }
        other => panic!("expected HTTP execution, got {other:?}"),
    }
}

#[test]
fn typed_ir_is_deterministic_and_contains_indexed_operands() {
    let program = compile(HELLO).unwrap();
    assert_eq!(
        program.disassemble(),
        concat!(
            "rivet-ir v1 \"Hello world\"\n",
            "%0: Text = text \"Hello, \"    ; greeting\n",
            "%1: Text = text \"world!\"    ; audience\n",
            "%2: Text = concat [%0, %1]    ; message\n",
            "entry console %2\n",
        )
    );
    assert_eq!(program.disassemble(), compile(HELLO).unwrap().disassemble());
    let computed = compile(COMPUTED).unwrap().disassemble();
    assert!(computed.contains("%2: Int = add %0, %1"));
    assert!(computed.contains("%3: Text = to_text %2"));
    assert!(computed.contains("%7: Json = object {\"answer\": %2, \"ready\": %6}"));
}

#[test]
fn signed_integer_and_scalar_text_semantics() {
    let value = console_program(
        json!([
            {"id":"min","op":"int","value":i64::MIN},
            {"id":"max","op":"int","value":i64::MAX},
            {"id":"sum","op":"add","left":"min","right":"max"},
            {"id":"number","op":"to_text","value":"sum"},
            {"id":"flag","op":"bool","value":false},
            {"id":"flag_text","op":"to_text","value":"flag"},
            {"id":"space","op":"text","value":" "},
            {"id":"space_copy","op":"to_text","value":"space"},
            {"id":"out","op":"concat","values":["number","space_copy","flag_text"]}
        ]),
        "out",
    );
    assert_eq!(stdout(&compile_value(&value)), "-1 false\n");
}

#[test]
fn empty_concat_and_empty_http_routes_are_valid() {
    let value = console_program(json!([{"id":"empty","op":"concat","values":[]}]), "empty");
    assert_eq!(stdout(&compile_value(&value)), "\n");
    let value = json!({"version":1,"name":"empty","instructions":[],"entry":{"kind":"http","port":0,"routes":[]}});
    assert!(
        matches!(compile_value(&value).execute().unwrap(), Execution::Http { port: 0, routes } if routes.is_empty())
    );
}

#[test]
fn syntax_errors_report_line_and_column() {
    let diagnostics = compile("{\n  \"version\": 1,\n}").unwrap_err();
    assert_eq!(diagnostics[0].path, "$");
    assert!(diagnostics[0].message.contains("line 3 column 1"));
    assert!(compile("{}").is_err());
    assert!(compile("[]").is_err());
    assert!(compile("null").is_err());
}

#[test]
fn rejects_unknown_fields_at_each_schema_level() {
    for (pointer, expected_path) in [
        ("/typo", "$.typo"),
        ("/instructions/0/typo", "$.instructions[0].typo"),
        ("/entry/typo", "$.entry.typo"),
        ("/entry/routes/0/typo", "$.entry.routes[0].typo"),
    ] {
        let mut value: Value = serde_json::from_str(SERVER).unwrap();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(key.into(), json!(1));
        rejects(&value, expected_path, "unknown field");
    }
    let mut value: Value = serde_json::from_str(HELLO).unwrap();
    value["entry"]["port"] = json!(1234);
    rejects(&value, "$.entry.port", "unknown field");
}

#[test]
fn rejects_missing_and_wrongly_typed_fields() {
    let original: Value = serde_json::from_str(HELLO).unwrap();
    for field in ["version", "name", "instructions", "entry"] {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(field);
        rejects(&value, &format!("$.{field}"), "required field is missing");
    }
    for version in [json!(0), json!(2), json!(1.0), json!("1"), Value::Null] {
        let mut value = original.clone();
        value["version"] = version;
        rejects(&value, "$.version", "integer 1");
    }
    let mut value = original.clone();
    value["instructions"][0]["value"] = json!(99);
    rejects(&value, "$.instructions[0].value", "expected a string");
    value["instructions"] = json!({});
    rejects(&value, "$.instructions", "expected an array");
}

#[test]
fn rejects_invalid_identifiers_and_duplicates() {
    for id in [
        "",
        "1number",
        "two words",
        "hyphen-name",
        "é",
        &"x".repeat(65),
    ] {
        let value = console_program(json!([{"id":id,"op":"text","value":"ok"}]), id);
        rejects(&value, "$.instructions[0].id", "ID must be");
    }
    let id = "x".repeat(64);
    let value = console_program(json!([{"id":id,"op":"text","value":"ok"}]), &id);
    assert_eq!(stdout(&compile_value(&value)), "ok\n");
    let value = console_program(
        json!([
            {"id":"same","op":"text","value":"first"},
            {"id":"same","op":"text","value":"second"}
        ]),
        "same",
    );
    rejects(&value, "$.instructions[1].id", "duplicate instruction ID");
}

#[test]
fn rejects_unknown_operations_and_entry_kinds() {
    let value = console_program(json!([{"id":"out","op":"eval","value":"alert(1)"}]), "out");
    rejects(&value, "$.instructions[0].op", "unknown operation");
    let mut value: Value = serde_json::from_str(HELLO).unwrap();
    value["entry"]["kind"] = json!("shell");
    rejects(&value, "$.entry.kind", "unknown entry kind");
}

#[test]
fn rejects_missing_forward_self_and_non_string_operands() {
    for reference in [json!("missing"), json!("later"), json!("out"), json!(12)] {
        let value = console_program(
            json!([
                {"id":"out","op":"concat","values":[reference]},
                {"id":"later","op":"text","value":"late"}
            ]),
            "out",
        );
        let expected = if reference.is_string() {
            "unknown or forward reference"
        } else {
            "expected an instruction ID"
        };
        rejects(&value, "$.instructions[0].values[0]", expected);
    }
}

#[test]
fn rejects_operand_and_entry_type_mismatches() {
    for (instruction, path, message) in [
        (
            json!({"id":"out","op":"concat","values":["integer"]}),
            "$.instructions[3].values[0]",
            "expected Text, found Int",
        ),
        (
            json!({"id":"out","op":"add","left":"text","right":"integer"}),
            "$.instructions[3].left",
            "expected Int, found Text",
        ),
        (
            json!({"id":"out","op":"to_text","value":"object"}),
            "$.instructions[3].value",
            "found Json",
        ),
    ] {
        let value = console_program(
            json!([
                {"id":"text","op":"text","value":"value"},
                {"id":"integer","op":"int","value":1},
                {"id":"object","op":"object","fields":{}},
                instruction
            ]),
            "text",
        );
        rejects(&value, path, message);
    }
    let value = console_program(json!([{"id":"integer","op":"int","value":1}]), "integer");
    rejects(&value, "$.entry.value", "expected Text, found Int");
    let mut value: Value = serde_json::from_str(SERVER).unwrap();
    value["entry"]["routes"][0]["body"] = json!("message");
    rejects(
        &value,
        "$.entry.routes[0].body",
        "expected Json, found Text",
    );
}

#[test]
fn rejects_non_integer_out_of_range_and_wrong_literal_types() {
    for literal in [
        json!(1.5),
        json!(1.0),
        json!(u64::MAX),
        json!(true),
        json!("1"),
    ] {
        let value = console_program(
            json!([
                {"id":"n","op":"int","value":literal},
                {"id":"out","op":"text","value":"ok"}
            ]),
            "out",
        );
        rejects(&value, "$.instructions[0].value", "signed 64-bit integer");
    }
    let value = console_program(
        json!([
            {"id":"b","op":"bool","value":"true"},
            {"id":"out","op":"text","value":"ok"}
        ]),
        "out",
    );
    rejects(&value, "$.instructions[0].value", "expected a boolean");
}

#[test]
fn compile_checks_utf8_byte_and_instruction_limits() {
    let original: Value = serde_json::from_str(HELLO).unwrap();
    for name in ["".into(), "a".repeat(129), "é".repeat(65)] {
        let mut value = original.clone();
        value["name"] = json!(name);
        rejects(&value, "$.name", "1 to 128 UTF-8 bytes");
    }
    let value = console_program(
        json!([{"id":"out","op":"text","value":"é".repeat(8192)}]),
        "out",
    );
    assert_eq!(stdout(&compile_value(&value)).len(), 16385);
    let value = console_program(
        json!([{"id":"out","op":"text","value":"é".repeat(8193)}]),
        "out",
    );
    rejects(&value, "$.instructions[0].value", "16384 UTF-8 bytes");
    let instructions: Vec<_> = (0..256)
        .map(|n| json!({"id":format!("v{n}"),"op":"text","value":"x"}))
        .collect();
    let mut value = console_program(json!(instructions), "v255");
    assert_eq!(compile_value(&value).instruction_count(), 256);
    value["instructions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"extra","op":"text","value":"x"}));
    rejects(&value, "$.instructions", "256 instructions");
    let source_at_limit = format!("{HELLO}{}", " ".repeat(65536 - HELLO.len()));
    assert!(compile(&source_at_limit).is_ok());
    let errors = compile(&(source_at_limit + " ")).unwrap_err();
    assert_eq!(errors[0].path, "$");
    assert!(errors[0].message.contains("65536 UTF-8 bytes"));
}

#[test]
fn checked_overflow_is_a_runtime_error_even_for_unused_instructions() {
    for (left, right) in [(i64::MAX, 1), (i64::MIN, -1)] {
        let value = console_program(
            json!([
                {"id":"a","op":"int","value":left},
                {"id":"b","op":"int","value":right},
                {"id":"overflow","op":"add","left":"a","right":"b"},
                {"id":"out","op":"text","value":"not printed"}
            ]),
            "out",
        );
        let error = compile_value(&value).execute().unwrap_err();
        assert!(error.contains("instruction \"overflow\" (%2)"));
        assert!(error.contains("signed 64-bit integer overflow"));
    }
}

#[test]
fn computed_text_and_final_output_have_separate_limits() {
    let mut value = console_program(
        json!([
            {"id":"chunk","op":"text","value":"a".repeat(16384)},
            {"id":"out","op":"concat","values":["chunk","chunk","chunk","chunk"]}
        ]),
        "out",
    );
    let error = compile_value(&value).execute().unwrap_err();
    assert!(error.contains("console output including newline exceeds 65536 bytes"));
    value["instructions"][1]["values"]
        .as_array_mut()
        .unwrap()
        .push(json!("chunk"));
    let error = compile_value(&value).execute().unwrap_err();
    assert!(error.contains("instruction \"out\" (%1): computed Text exceeds 65536 bytes"));
    value["instructions"][0]["value"] = json!("a".repeat(13107));
    assert_eq!(stdout(&compile_value(&value)).len(), 65536);
}

#[test]
fn computed_json_counts_escaping_keys_and_nested_objects() {
    let mut value: Value = serde_json::from_str(SERVER).unwrap();
    // The source remains below 64 KiB by constructing the 12,000 control bytes at runtime.
    value["instructions"] = json!([
        {"id":"chunk","op":"text","value":"\0".repeat(2000)},
        {"id":"message","op":"concat","values":["chunk","chunk","chunk","chunk","chunk","chunk"]},
        {"id":"response","op":"object","fields":{"message":"message"}}
    ]);
    let error = compile_value(&value).execute().unwrap_err();
    assert!(error.contains("computed Json exceeds 65536 bytes"));
    value["instructions"] = json!([
        {"id":"chunk","op":"text","value":"x".repeat(16000)},
        {"id":"inner","op":"object","fields":{"a":"chunk","b":"chunk"}},
        {"id":"response","op":"object","fields":{"a":"inner","b":"inner","c":"inner"}}
    ]);
    let error = compile_value(&value).execute().unwrap_err();
    assert!(error.contains("instruction \"response\" (%2): computed Json exceeds"));
    value["instructions"] = json!([
        {"id":"message","op":"concat","values":[]},
        {"id":"response","op":"object","fields":{"\0".repeat(11000):"message"}}
    ]);
    // Escaped keys themselves make this source exceed the separate source limit.
    assert!(
        compile(&value.to_string()).unwrap_err()[0]
            .message
            .contains("source exceeds")
    );
}

#[test]
fn serialized_json_limit_accepts_exact_boundary_and_rejects_one_extra_byte() {
    let mut value: Value = serde_json::from_str(SERVER).unwrap();
    value["instructions"] = json!([
        {"id":"chunk","op":"text","value":"x".repeat(16382)},
        {"id":"suffix","op":"text","value":""},
        {"id":"message","op":"concat","values":["chunk","chunk","chunk","chunk","suffix"]},
        {"id":"response","op":"object","fields":{"x":"message"}}
    ]);
    let Execution::Http { routes, .. } = compile_value(&value).execute().unwrap() else {
        panic!("expected HTTP result");
    };
    assert_eq!(serde_json::to_vec(&routes[0].body).unwrap().len(), 65536);
    value["instructions"][1]["value"] = json!("x");
    let error = compile_value(&value).execute().unwrap_err();
    assert!(error.contains("computed Json exceeds 65536 bytes (65537 bytes required)"));
}

#[test]
fn rejects_invalid_http_ports_paths_methods_and_statuses() {
    let original: Value = serde_json::from_str(SERVER).unwrap();
    for port in [json!(-1), json!(65536), json!(1.5), json!("8787")] {
        let mut value = original.clone();
        value["entry"]["port"] = port;
        rejects(&value, "$.entry.port", "integer from 0 to 65535");
    }
    for path in [
        "",
        "hello",
        "/hello?q=1",
        "/hello#fragment",
        "/a b",
        "/é",
        "/a\n",
        "/a\u{7f}",
    ] {
        let mut value = original.clone();
        value["entry"]["routes"][0]["path"] = json!(path);
        rejects(&value, "$.entry.routes[0].path", "route path must");
    }
    for method in ["POST", "get", "HEAD", ""] {
        let mut value = original.clone();
        value["entry"]["routes"][0]["method"] = json!(method);
        rejects(&value, "$.entry.routes[0].method", "only GET");
    }
    for status in [199, 204, 205, 300, 404] {
        let mut value = original.clone();
        value["entry"]["routes"][0]["status"] = json!(status);
        rejects(
            &value,
            "$.entry.routes[0].status",
            "status must be 200 to 299",
        );
    }
}

#[test]
fn route_path_limit_matches_transport_at_exact_byte_boundary() {
    let mut value: Value = serde_json::from_str(SERVER).unwrap();
    let boundary_path = format!("/{}", "a".repeat(2047));
    value["entry"]["routes"][0]["path"] = json!(boundary_path);
    let Execution::Http { routes, .. } = compile_value(&value).execute().unwrap() else {
        panic!("expected HTTP result");
    };
    assert_eq!(routes[0].path.len(), 2048);
    assert_eq!(routes[0].path, boundary_path);

    value["entry"]["routes"][0]["path"] = json!(format!("{boundary_path}a"));
    rejects(
        &value,
        "$.entry.routes[0].path",
        "route path exceeds 2048 bytes",
    );
}

#[test]
fn http_boundary_values_unique_routes_and_route_count() {
    let mut value: Value = serde_json::from_str(SERVER).unwrap();
    value["entry"]["port"] = json!(65535);
    value["entry"]["routes"][0]["status"] = json!(299);
    value["entry"]["routes"][0]["path"] = json!("/");
    assert!(compile_value(&value).execute().is_ok());
    let route = value["entry"]["routes"][0].clone();
    value["entry"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(route.clone());
    rejects(&value, "$.entry.routes[1].path", "duplicate route path");
    value["entry"]["routes"] = json!(
        (0..32)
            .map(|index| {
                let mut route = route.clone();
                route["path"] = json!(format!("/r{index}"));
                route
            })
            .collect::<Vec<_>>()
    );
    assert!(compile_value(&value).execute().is_ok());
    value["entry"]["routes"].as_array_mut().unwrap().push(route);
    rejects(&value, "$.entry.routes", "32 routes");
}

#[test]
fn object_field_paths_escape_arbitrary_keys_in_diagnostics() {
    let value = console_program(
        json!([
            {"id":"bad","op":"object","fields":{"a.b[0]":"missing"}},
            {"id":"out","op":"text","value":"ok"}
        ]),
        "out",
    );
    rejects(
        &value,
        "$.instructions[0].fields[\"a.b[0]\"]",
        "unknown or forward reference",
    );
}

#[test]
fn successful_execution_is_repeatable_and_independent() {
    let program = compile(COMPUTED).unwrap();
    for _ in 0..20 {
        let Execution::Http { mut routes, .. } = program.execute().unwrap() else {
            panic!("HTTP expected")
        };
        assert_eq!(routes[0].body["details"]["answer"], json!(42));
        routes[0].body["details"]["answer"] = json!(-1);
    }
}
