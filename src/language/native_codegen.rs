//! Direct native lowering of the validated, typed register graph.
//!
//! This module never evaluates the program. Potentially failing operations stay
//! in source order, including instructions whose result is otherwise unused.

use super::{CompiledProgram, Entry, Instruction, ScalarRegister, ValueRegister};
use std::fmt::Write;

const DECLARATIONS: &str = r#"declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare ptr @rv_text(ptr, i64)
declare ptr @rv_concat(ptr, i64)
declare ptr @rv_i64_text(i64)
declare ptr @rv_bool_text(i8 zeroext)
declare ptr @rv_json_text(ptr)
declare ptr @rv_json_i64(i64)
declare ptr @rv_json_bool(i8 zeroext)
declare ptr @rv_object(ptr, ptr, ptr, i64)
declare void @rv_print(ptr)
declare void @rv_fail(ptr, i64) noreturn
declare void @rv_add_route(ptr, i64, i32, ptr)
declare i32 @rv_serve(i32)
declare void @rv_cleanup()
"#;

pub(super) fn emit(program: &CompiledProgram) -> String {
    let mut emitter = Emitter::default();
    emitter.body.push_str("define i32 @main() {\nentry:\n");
    for (index, instruction) in program.instructions.iter().enumerate() {
        // IDs are validated ASCII identifiers. User text is only emitted as
        // escaped byte constants, never as LLVM syntax or comments.
        writeln!(emitter.body, "  ; %{index}: {}", instruction.id).unwrap();
        let operand = emitter.instruction(index, &instruction.id, &instruction.operation);
        emitter.operands.push(operand);
    }
    match &program.entry {
        Entry::Compute(_) => unreachable!("compute lowering has its own emitter"),
        Entry::Console(register) => {
            writeln!(
                emitter.body,
                "  call void @rv_print(ptr {})",
                emitter.operands[register.0]
            )
            .unwrap();
            emitter
                .body
                .push_str("  call void @rv_cleanup()\n  ret i32 0\n}\n");
        }
        Entry::Http { port, routes } => {
            for route in routes {
                let path = emitter.bytes(route.path.as_bytes());
                writeln!(
                    emitter.body,
                    "  call void @rv_add_route(ptr {path}, i64 {}, i32 {}, ptr {})",
                    route.path.len(),
                    route.status,
                    emitter.operands[route.body.0]
                )
                .unwrap();
            }
            writeln!(emitter.body, "  %exit = call i32 @rv_serve(i32 {port})").unwrap();
            emitter
                .body
                .push_str("  call void @rv_cleanup()\n  ret i32 %exit\n}\n");
        }
    }
    format!(
        "; Rivet native LLVM IR v1; generated from validated typed registers.\n\
         source_filename = \"rivet-program\"\n\n{}\n{DECLARATIONS}\n{}",
        emitter.constants, emitter.body
    )
}

#[derive(Default)]
struct Emitter {
    constants: String,
    body: String,
    operands: Vec<String>,
    next_constant: usize,
}

impl Emitter {
    /// UTF-8 is represented as bytes with an explicit semantic length. The
    /// additional terminator is never part of the value, including for strings
    /// that already contain NUL bytes. Hex-escape every byte to make injection
    /// of LLVM syntax impossible and to keep output deterministic.
    fn bytes(&mut self, value: &[u8]) -> String {
        let name = self.constant_name();
        write!(
            self.constants,
            "{name} = private unnamed_addr constant [{} x i8] c\"",
            value.len() + 1
        )
        .unwrap();
        for byte in value.iter().chain(std::iter::once(&0)) {
            write!(self.constants, "\\{byte:02X}").unwrap();
        }
        self.constants.push_str("\", align 1\n");
        name
    }

    fn constant_name(&mut self) -> String {
        let name = format!("@c{}", self.next_constant);
        self.next_constant += 1;
        name
    }

    fn pointer_array(&mut self, name: &str, values: &[String]) -> String {
        if values.is_empty() {
            return "null".into();
        }
        let count = values.len();
        writeln!(self.body, "  %{name} = alloca [{count} x ptr]").unwrap();
        for (index, value) in values.iter().enumerate() {
            writeln!(
                self.body,
                "  %{name}.{index} = getelementptr inbounds [{count} x ptr], ptr %{name}, i64 0, i64 {index}\n  store ptr {value}, ptr %{name}.{index}"
            )
            .unwrap();
        }
        format!("%{name}")
    }

    fn instruction(&mut self, index: usize, id: &str, instruction: &Instruction) -> String {
        let result = format!("%r{index}");
        match instruction {
            Instruction::Text(value) => {
                let bytes = self.bytes(value.as_bytes());
                writeln!(
                    self.body,
                    "  {result} = call ptr @rv_text(ptr {bytes}, i64 {})",
                    value.len()
                )
                .unwrap();
            }
            Instruction::Int(value) => return value.to_string(),
            Instruction::Bool(value) => return value.to_string(),
            Instruction::Concat(registers) => {
                let values = registers
                    .iter()
                    .map(|register| self.operands[register.0].clone())
                    .collect::<Vec<_>>();
                let array = self.pointer_array(&format!("concat{index}"), &values);
                writeln!(
                    self.body,
                    "  {result} = call ptr @rv_concat(ptr {array}, i64 {})",
                    registers.len()
                )
                .unwrap();
            }
            Instruction::Add(left, right) => {
                let message =
                    format!("instruction {id:?} (%{index}): signed 64-bit integer overflow");
                let message_bytes = self.bytes(message.as_bytes());
                writeln!(
                    self.body,
                    "  %sum{index} = call {{ i64, i1 }} @llvm.sadd.with.overflow.i64(i64 {}, i64 {})\n  {result} = extractvalue {{ i64, i1 }} %sum{index}, 0\n  %overflow{index} = extractvalue {{ i64, i1 }} %sum{index}, 1\n  br i1 %overflow{index}, label %failed{index}, label %next{index}\nfailed{index}:\n  call void @rv_fail(ptr {message_bytes}, i64 {})\n  unreachable\nnext{index}:",
                    self.operands[left.0], self.operands[right.0], message.len()
                )
                .unwrap();
            }
            Instruction::ToText(register) => {
                let (function, argument) = match register {
                    // An immutable Text alias has identical observable semantics
                    // to the reference interpreter's copy; it cannot overflow.
                    ScalarRegister::Text(register) => return self.operands[register.0].clone(),
                    ScalarRegister::Int(register) => {
                        ("rv_i64_text", format!("i64 {}", self.operands[register.0]))
                    }
                    ScalarRegister::Bool(register) => {
                        writeln!(
                            self.body,
                            "  %bool{index} = zext i1 {} to i8",
                            self.operands[register.0]
                        )
                        .unwrap();
                        ("rv_bool_text", format!("i8 zeroext %bool{index}"))
                    }
                };
                writeln!(self.body, "  {result} = call ptr @{function}({argument})").unwrap();
            }
            Instruction::Object(fields) => {
                let mut keys = Vec::with_capacity(fields.len());
                let mut values = Vec::with_capacity(fields.len());
                for (field_index, (key, register)) in fields.iter().enumerate() {
                    keys.push(self.bytes(key.as_bytes()));
                    let value = format!("%json{index}.{field_index}");
                    let (function, argument) = match register {
                        ValueRegister::Text(register) => {
                            ("rv_json_text", format!("ptr {}", self.operands[register.0]))
                        }
                        ValueRegister::Int(register) => {
                            ("rv_json_i64", format!("i64 {}", self.operands[register.0]))
                        }
                        ValueRegister::Bool(register) => {
                            writeln!(
                                self.body,
                                "  %bool{index}.{field_index} = zext i1 {} to i8",
                                self.operands[register.0]
                            )
                            .unwrap();
                            (
                                "rv_json_bool",
                                format!("i8 zeroext %bool{index}.{field_index}"),
                            )
                        }
                        ValueRegister::Json(register) => {
                            values.push(self.operands[register.0].clone());
                            continue;
                        }
                    };
                    writeln!(self.body, "  {value} = call ptr @{function}({argument})").unwrap();
                    values.push(value);
                }
                let (key_array, length_array) = if fields.is_empty() {
                    ("null".into(), "null".into())
                } else {
                    let key_array = self.constant_name();
                    let length_array = self.constant_name();
                    let count = fields.len();
                    writeln!(
                        self.constants,
                        "{key_array} = private constant [{count} x ptr] [{}]",
                        keys.iter()
                            .map(|key| format!("ptr {key}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .unwrap();
                    writeln!(
                        self.constants,
                        "{length_array} = private constant [{count} x i64] [{}]",
                        fields
                            .iter()
                            .map(|(key, _)| format!("i64 {}", key.len()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .unwrap();
                    (key_array, length_array)
                };
                let value_array = self.pointer_array(&format!("object{index}"), &values);
                writeln!(
                    self.body,
                    "  {result} = call ptr @rv_object(ptr {key_array}, ptr {length_array}, ptr {value_array}, i64 {})",
                    fields.len()
                )
                .unwrap();
            }
        }
        result
    }
}
