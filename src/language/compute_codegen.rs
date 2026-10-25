//! Direct LLVM lowering for RVB2. Each source PC becomes native control flow;
//! no encoded program, runtime instruction dispatcher or algorithm helper exists.
use super::compute::{ComputeProgram, Op, RegisterType};
use std::fmt::Write;

const DECLARATIONS: &str = r#"declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare i64 @rv2_read_int()
declare ptr @rv2_read_array()
declare i64 @rv2_array_len(ptr)
declare i64 @rv2_array_get(ptr, i64)
declare ptr @rv2_array_make(ptr, i64)
declare ptr @rv2_map_new()
declare i64 @rv2_map_get(ptr, i64, i64)
declare void @rv2_map_put(ptr, i64, i64)
declare void @rv2_tick()
declare void @rv2_return(ptr)
declare void @rv_fail(ptr, i64) noreturn
declare void @rv_cleanup()
"#;
fn llvm_type(kind: RegisterType) -> &'static str {
    match kind {
        RegisterType::Int => "i64",
        RegisterType::Bool => "i1",
        RegisterType::Array | RegisterType::Map => "ptr",
    }
}
struct Emitter<'a> {
    program: &'a ComputeProgram,
    body: String,
    next: usize,
}
impl Emitter<'_> {
    fn temp(&mut self) -> String {
        let result = format!("%t{}", self.next);
        self.next += 1;
        result
    }
    fn load(&mut self, register: usize) -> String {
        let result = self.temp();
        writeln!(
            self.body,
            "  {result} = load {}, ptr %r{register}",
            llvm_type(self.program.registers[register])
        )
        .unwrap();
        result
    }
    fn store(&mut self, register: usize, value: &str) {
        writeln!(
            self.body,
            "  store {} {value}, ptr %r{register}",
            llvm_type(self.program.registers[register])
        )
        .unwrap();
    }
    fn call(&mut self, destination: usize, function: &str, arguments: &str) {
        let value = self.temp();
        writeln!(
            self.body,
            "  {value} = call {} @{function}({arguments})",
            llvm_type(self.program.registers[destination])
        )
        .unwrap();
        self.store(destination, &value);
    }
    fn operation(&mut self, pc: usize, operation: &Op) {
        writeln!(self.body, "p{pc}:\n  call void @rv2_tick()").unwrap();
        match operation {
            Op::Int(d, value) => self.store(*d, &value.to_string()),
            Op::Bool(d, value) => self.store(*d, if *value { "true" } else { "false" }),
            Op::Add(d, a, b) | Op::Sub(d, a, b) => {
                let a = self.load(*a);
                let b = self.load(*b);
                let pair = self.temp();
                let value = self.temp();
                let overflow = self.temp();
                let intrinsic = if matches!(operation, Op::Add(..)) {
                    "sadd"
                } else {
                    "ssub"
                };
                writeln!(self.body, "  {pair} = call {{ i64, i1 }} @llvm.{intrinsic}.with.overflow.i64(i64 {a}, i64 {b})\n  {value} = extractvalue {{ i64, i1 }} {pair}, 0\n  {overflow} = extractvalue {{ i64, i1 }} {pair}, 1\n  br i1 {overflow}, label %overflow, label %ok{pc}\nok{pc}:").unwrap();
                self.store(*d, &value);
            }
            Op::Equal(d, a, b) | Op::Less(d, a, b) => {
                let a = self.load(*a);
                let b = self.load(*b);
                let result = self.temp();
                let comparison = if matches!(operation, Op::Equal(..)) {
                    "eq"
                } else {
                    "slt"
                };
                writeln!(self.body, "  {result} = icmp {comparison} i64 {a}, {b}").unwrap();
                self.store(*d, &result);
            }
            Op::ReadInt(d) => self.call(*d, "rv2_read_int", ""),
            Op::ReadArray(d) => self.call(*d, "rv2_read_array", ""),
            Op::Map(d) => self.call(*d, "rv2_map_new", ""),
            Op::Length(d, array) => {
                let array = self.load(*array);
                self.call(*d, "rv2_array_len", &format!("ptr {array}"));
            }
            Op::Get(d, array, index) => {
                let array = self.load(*array);
                let index = self.load(*index);
                self.call(*d, "rv2_array_get", &format!("ptr {array}, i64 {index}"));
            }
            Op::Array(d, registers) => {
                let length = registers.len();
                for (index, register) in registers.iter().enumerate() {
                    let value = self.load(*register);
                    let slot = self.temp();
                    writeln!(self.body, "  {slot} = getelementptr [{length} x i64], ptr %array{pc}, i64 0, i64 {index}\n  store i64 {value}, ptr {slot}").unwrap();
                }
                let pointer = if length == 0 {
                    "null".into()
                } else {
                    format!("%array{pc}")
                };
                self.call(
                    *d,
                    "rv2_array_make",
                    &format!("ptr {pointer}, i64 {length}"),
                );
            }
            Op::Lookup(d, map, key, fallback) => {
                let map = self.load(*map);
                let key = self.load(*key);
                let fallback = self.load(*fallback);
                self.call(
                    *d,
                    "rv2_map_get",
                    &format!("ptr {map}, i64 {key}, i64 {fallback}"),
                );
            }
            Op::Put(map, key, value) => {
                let map = self.load(*map);
                let key = self.load(*key);
                let value = self.load(*value);
                writeln!(
                    self.body,
                    "  call void @rv2_map_put(ptr {map}, i64 {key}, i64 {value})"
                )
                .unwrap();
            }
            Op::Jump(target) => {
                writeln!(self.body, "  br label %p{target}").unwrap();
                return;
            }
            Op::Branch(condition, yes, no) => {
                let condition = self.load(*condition);
                writeln!(
                    self.body,
                    "  br i1 {condition}, label %p{yes}, label %p{no}"
                )
                .unwrap();
                return;
            }
            Op::Return(array) => {
                let array = self.load(*array);
                writeln!(
                    self.body,
                    "  call void @rv2_return(ptr {array})\n  call void @rv_cleanup()\n  ret i32 0"
                )
                .unwrap();
                return;
            }
        }
        if pc + 1 < self.program.operations.len() {
            writeln!(self.body, "  br label %p{}", pc + 1).unwrap();
        } else {
            // Validation permits a structurally valid unreachable tail. Reachable
            // fallthrough is rejected; no out-of-range block is emitted.
            self.body.push_str("  unreachable\n");
        }
    }
}

pub(super) fn emit(program: &ComputeProgram) -> String {
    let mut emitter = Emitter {
        program,
        body: String::from("define i32 @main() {\nentry:\n"),
        next: 0,
    };
    for (register, kind) in program.registers.iter().enumerate() {
        writeln!(emitter.body, "  %r{register} = alloca {}", llvm_type(*kind)).unwrap();
    }
    for (pc, operation) in program.operations.iter().enumerate() {
        if let Op::Array(_, values) = operation
            && !values.is_empty()
        {
            // Scratch is allocated once even when the source record is in a loop.
            writeln!(
                emitter.body,
                "  %array{pc} = alloca [{} x i64]",
                values.len()
            )
            .unwrap();
        }
    }
    emitter.body.push_str("  br label %p0\n");
    for (pc, operation) in program.operations.iter().enumerate() {
        emitter.operation(pc, operation);
    }
    let message = "signed 64-bit integer overflow";
    writeln!(
        emitter.body,
        "overflow:\n  call void @rv_fail(ptr @overflow_message, i64 {})\n  unreachable\n}}",
        message.len()
    )
    .unwrap();
    format!(
        "; Rivet native LLVM IR v2; direct control flow from validated binary source.\nsource_filename = \"rivet-compute-program\"\n@overflow_message = private unnamed_addr constant [{} x i8] c\"{message}\"\n\n{DECLARATIONS}\n{}",
        message.len(),
        emitter.body
    )
}
