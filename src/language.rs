//! Rivet binary source validation, typed register lowering, and bounded interpretation.

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};

mod binary;
mod native_codegen;

pub use binary::{BinaryInspection, BinaryInstructionInfo, compile_binary, inspect_binary};

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_INSTRUCTIONS: usize = 256;
const MAX_NAME_BYTES: usize = 128;
const MAX_LITERAL_BYTES: usize = 16 * 1024;
const MAX_VALUE_BYTES: usize = 64 * 1024;
const MAX_ROUTES: usize = 32;
const MAX_ROUTE_PATH_BYTES: usize = 2048;

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum Execution {
    Console { stdout: String },
    Http { port: u16, routes: Vec<Route> },
}

#[derive(Clone, Debug)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub body: Value,
}

/// Entry metadata available without evaluating the compiled program.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EntryInfo {
    Console,
    Http { port: u16, routes: Vec<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    Text,
    Int,
    Bool,
    Json,
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Text => "Text",
            Self::Int => "Int",
            Self::Bool => "Bool",
            Self::Json => "Json",
        })
    }
}

// Distinct register types make operand requirements part of the executable IR.
#[derive(Clone, Copy, Debug)]
struct TextRegister(usize);
#[derive(Clone, Copy, Debug)]
struct IntRegister(usize);
#[derive(Clone, Copy, Debug)]
struct BoolRegister(usize);
#[derive(Clone, Copy, Debug)]
struct JsonRegister(usize);

#[derive(Clone, Copy, Debug)]
enum ValueRegister {
    Text(TextRegister),
    Int(IntRegister),
    Bool(BoolRegister),
    Json(JsonRegister),
}

impl ValueRegister {
    fn index(self) -> usize {
        match self {
            Self::Text(register) => register.0,
            Self::Int(register) => register.0,
            Self::Bool(register) => register.0,
            Self::Json(register) => register.0,
        }
    }

    fn value_type(self) -> Type {
        match self {
            Self::Text(_) => Type::Text,
            Self::Int(_) => Type::Int,
            Self::Bool(_) => Type::Bool,
            Self::Json(_) => Type::Json,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ScalarRegister {
    Text(TextRegister),
    Int(IntRegister),
    Bool(BoolRegister),
}

impl ScalarRegister {
    fn index(self) -> usize {
        match self {
            Self::Text(register) => register.0,
            Self::Int(register) => register.0,
            Self::Bool(register) => register.0,
        }
    }
}

#[derive(Clone, Debug)]
enum Instruction {
    Text(String),
    Int(i64),
    Bool(bool),
    Concat(Vec<TextRegister>),
    Add(IntRegister, IntRegister),
    ToText(ScalarRegister),
    Object(Vec<(String, ValueRegister)>),
}

impl Instruction {
    fn result_register(&self, index: usize) -> ValueRegister {
        match self {
            Self::Text(_) | Self::Concat(_) | Self::ToText(_) => {
                ValueRegister::Text(TextRegister(index))
            }
            Self::Int(_) | Self::Add(..) => ValueRegister::Int(IntRegister(index)),
            Self::Bool(_) => ValueRegister::Bool(BoolRegister(index)),
            Self::Object(_) => ValueRegister::Json(JsonRegister(index)),
        }
    }
}

#[derive(Clone, Debug)]
struct NamedInstruction {
    id: String,
    operation: Instruction,
}

#[derive(Clone, Debug)]
struct CompiledRoute {
    path: String,
    status: u16,
    body: JsonRegister,
}

#[derive(Clone, Debug)]
enum Entry {
    Console(TextRegister),
    Http {
        port: u16,
        routes: Vec<CompiledRoute>,
    },
}

#[derive(Clone, Debug)]
pub struct CompiledProgram {
    name: String,
    instructions: Vec<NamedInstruction>,
    entry: Entry,
}

/// Legacy JSON parser retained for historical semantic tests and migration utilities.
/// Canonical `.rvb` source is decoded directly by [`compile_binary`], without this parser.
pub fn compile(source: &str) -> Result<CompiledProgram, Vec<Diagnostic>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![Diagnostic {
            path: "$".into(),
            message: format!("source exceeds {MAX_SOURCE_BYTES} UTF-8 bytes"),
        }]);
    }
    let value: Value = serde_json::from_str(source).map_err(|error| {
        vec![Diagnostic {
            path: "$".into(),
            message: format!("invalid JSON: {error}"),
        }]
    })?;
    let mut compiler = Compiler::default();
    let Some(root) = compiler.object(&value, "$") else {
        return Err(compiler.diagnostics);
    };
    compiler.check_fields(root, &["version", "name", "instructions", "entry"], "$");
    if let Some(version) = compiler.required(root, "version", "$")
        && version.as_u64() != Some(1)
    {
        compiler.error("$.version", "version must be the integer 1");
    }
    let name = compiler.string(root, "name", "$");
    if let Some(name) = &name
        && (name.is_empty() || name.len() > MAX_NAME_BYTES)
    {
        compiler.error("$.name", "name must contain 1 to 128 UTF-8 bytes");
    }
    if let Some(values) = compiler.array(root, "instructions", "$") {
        if values.len() > MAX_INSTRUCTIONS {
            compiler.error("$.instructions", "at most 256 instructions are allowed");
        }
        for (index, value) in values.iter().take(MAX_INSTRUCTIONS).enumerate() {
            compiler.instruction(value, index);
        }
    }
    let entry = compiler
        .required(root, "entry", "$")
        .and_then(|value| compiler.entry(value));
    if !compiler.diagnostics.is_empty() {
        return Err(compiler.diagnostics);
    }
    match (name, entry) {
        (Some(name), Some(entry)) => Ok(CompiledProgram {
            name,
            instructions: compiler.instructions,
            entry,
        }),
        _ => Err(vec![Diagnostic {
            path: "$".into(),
            message: "program is missing a valid name or entry".into(),
        }]),
    }
}

#[derive(Default)]
struct Compiler {
    diagnostics: Vec<Diagnostic>,
    instructions: Vec<NamedInstruction>,
    symbols: HashMap<String, ValueRegister>,
    seen_ids: HashSet<String>,
}

impl Compiler {
    fn error(&mut self, path: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            path: path.into(),
            message: message.into(),
        });
    }

    fn object<'a>(&mut self, value: &'a Value, path: &str) -> Option<&'a Map<String, Value>> {
        value.as_object().or_else(|| {
            self.error(path, "expected an object");
            None
        })
    }

    fn check_fields(&mut self, object: &Map<String, Value>, allowed: &[&str], path: &str) {
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                self.error(&property_path(path, key), "unknown field");
            }
        }
    }

    fn required<'a>(
        &mut self,
        object: &'a Map<String, Value>,
        key: &str,
        path: &str,
    ) -> Option<&'a Value> {
        object.get(key).or_else(|| {
            self.error(&property_path(path, key), "required field is missing");
            None
        })
    }

    fn string(&mut self, object: &Map<String, Value>, key: &str, path: &str) -> Option<String> {
        self.required(object, key, path).and_then(|value| {
            value.as_str().map(str::to_owned).or_else(|| {
                self.error(&property_path(path, key), "expected a string");
                None
            })
        })
    }

    fn array<'a>(
        &mut self,
        object: &'a Map<String, Value>,
        key: &str,
        path: &str,
    ) -> Option<&'a Vec<Value>> {
        self.required(object, key, path).and_then(|value| {
            value.as_array().or_else(|| {
                self.error(&property_path(path, key), "expected an array");
                None
            })
        })
    }

    fn unsigned_16(&mut self, object: &Map<String, Value>, key: &str, path: &str) -> Option<u16> {
        self.required(object, key, path).and_then(|value| {
            value
                .as_u64()
                .and_then(|number| u16::try_from(number).ok())
                .or_else(|| {
                    self.error(
                        &property_path(path, key),
                        "expected an integer from 0 to 65535",
                    );
                    None
                })
        })
    }

    fn reference(&mut self, value: &Value, path: &str) -> Option<ValueRegister> {
        let Some(id) = value.as_str() else {
            self.error(path, "expected an instruction ID string");
            return None;
        };
        self.symbols.get(id).copied().or_else(|| {
            self.error(
                path,
                format!("unknown or forward reference {id:?}; operands must name an earlier valid instruction"),
            );
            None
        })
    }

    fn operand(
        &mut self,
        object: &Map<String, Value>,
        key: &str,
        path: &str,
    ) -> Option<ValueRegister> {
        self.required(object, key, path)
            .and_then(|value| self.reference(value, &property_path(path, key)))
    }

    fn expect_type(&mut self, register: ValueRegister, expected: Type, path: &str) -> bool {
        if register.value_type() != expected {
            self.error(
                path,
                format!("expected {expected}, found {}", register.value_type()),
            );
            false
        } else {
            true
        }
    }

    fn instruction(&mut self, value: &Value, index: usize) {
        let path = format!("$.instructions[{index}]");
        let start_errors = self.diagnostics.len();
        let Some(object) = self.object(value, &path) else {
            return;
        };
        let id = self.string(object, "id", &path);
        if let Some(id) = &id {
            if !valid_id(id) {
                self.error(
                    &property_path(&path, "id"),
                    "ID must be 1 to 64 ASCII characters: letter or underscore first, then letters, digits, or underscores",
                );
            } else if !self.seen_ids.insert(id.clone()) {
                self.error(
                    &property_path(&path, "id"),
                    format!("duplicate instruction ID {id:?}"),
                );
            }
        }
        let Some(op) = self.string(object, "op", &path) else {
            return;
        };
        let allowed: &[&str] = match op.as_str() {
            "text" | "int" | "bool" | "to_text" => &["id", "op", "value"],
            "concat" => &["id", "op", "values"],
            "add" => &["id", "op", "left", "right"],
            "object" => &["id", "op", "fields"],
            _ => {
                self.error(
                    &property_path(&path, "op"),
                    format!("unknown operation {op:?}"),
                );
                return;
            }
        };
        self.check_fields(object, allowed, &path);
        let operation = match op.as_str() {
            "text" => self.string(object, "value", &path).map(|text| {
                if text.len() > MAX_LITERAL_BYTES {
                    self.error(
                        &property_path(&path, "value"),
                        "text literal exceeds 16384 UTF-8 bytes",
                    );
                }
                Instruction::Text(text)
            }),
            "int" => self.required(object, "value", &path).and_then(|value| {
                value.as_i64().map(Instruction::Int).or_else(|| {
                    self.error(
                        &property_path(&path, "value"),
                        "expected a signed 64-bit integer",
                    );
                    None
                })
            }),
            "bool" => self.required(object, "value", &path).and_then(|value| {
                value.as_bool().map(Instruction::Bool).or_else(|| {
                    self.error(&property_path(&path, "value"), "expected a boolean");
                    None
                })
            }),
            "concat" => self.array(object, "values", &path).map(|values| {
                let mut registers = Vec::with_capacity(values.len());
                for (operand_index, value) in values.iter().enumerate() {
                    let operand_path = format!("{path}.values[{operand_index}]");
                    if let Some(register) = self.reference(value, &operand_path)
                        && self.expect_type(register, Type::Text, &operand_path)
                    {
                        registers.push(TextRegister(register.index()));
                    }
                }
                Instruction::Concat(registers)
            }),
            "add" => {
                let left = self.operand(object, "left", &path);
                let right = self.operand(object, "right", &path);
                if let Some(register) = left {
                    self.expect_type(register, Type::Int, &property_path(&path, "left"));
                }
                if let Some(register) = right {
                    self.expect_type(register, Type::Int, &property_path(&path, "right"));
                }
                left.zip(right).map(|(left, right)| {
                    Instruction::Add(IntRegister(left.index()), IntRegister(right.index()))
                })
            }
            "to_text" => self.operand(object, "value", &path).and_then(|register| {
                let scalar = match register {
                    ValueRegister::Text(register) => ScalarRegister::Text(register),
                    ValueRegister::Int(register) => ScalarRegister::Int(register),
                    ValueRegister::Bool(register) => ScalarRegister::Bool(register),
                    ValueRegister::Json(_) => {
                        self.error(
                            &property_path(&path, "value"),
                            "expected Text, Int, or Bool, found Json",
                        );
                        return None;
                    }
                };
                Some(Instruction::ToText(scalar))
            }),
            "object" => self.required(object, "fields", &path).and_then(|value| {
                let fields_path = property_path(&path, "fields");
                let fields = self.object(value, &fields_path)?;
                let mut registers = Vec::with_capacity(fields.len());
                for (key, value) in fields {
                    if let Some(register) = self.reference(value, &property_path(&fields_path, key))
                    {
                        registers.push((key.clone(), register));
                    }
                }
                // Stable IR and output even if serde_json's map implementation changes.
                registers.sort_by(|left, right| left.0.cmp(&right.0));
                Some(Instruction::Object(registers))
            }),
            _ => unreachable!("operation was checked above"),
        };
        if self.diagnostics.len() == start_errors
            && let (Some(id), Some(operation)) = (id, operation)
        {
            self.symbols.insert(
                id.clone(),
                operation.result_register(self.instructions.len()),
            );
            self.instructions.push(NamedInstruction { id, operation });
        }
    }

    fn entry(&mut self, value: &Value) -> Option<Entry> {
        let path = "$.entry";
        let object = self.object(value, path)?;
        let kind = self.string(object, "kind", path)?;
        match kind.as_str() {
            "console" => {
                self.check_fields(object, &["kind", "value"], path);
                let register = self.operand(object, "value", path)?;
                self.expect_type(register, Type::Text, "$.entry.value");
                Some(Entry::Console(TextRegister(register.index())))
            }
            "http" => {
                self.check_fields(object, &["kind", "port", "routes"], path);
                let port = self.unsigned_16(object, "port", path);
                let values = self.array(object, "routes", path)?;
                if values.len() > MAX_ROUTES {
                    self.error("$.entry.routes", "at most 32 routes are allowed");
                }
                let mut paths = HashSet::new();
                let mut routes = Vec::new();
                for (index, value) in values.iter().take(MAX_ROUTES).enumerate() {
                    if let Some(route) = self.route(value, index) {
                        if !paths.insert(route.path.clone()) {
                            self.error(
                                &format!("$.entry.routes[{index}].path"),
                                format!("duplicate route path {:?}", route.path),
                            );
                        }
                        routes.push(route);
                    }
                }
                port.map(|port| Entry::Http { port, routes })
            }
            _ => {
                self.error(
                    "$.entry.kind",
                    format!("unknown entry kind {kind:?}; expected console or http"),
                );
                None
            }
        }
    }

    fn route(&mut self, value: &Value, index: usize) -> Option<CompiledRoute> {
        let path = format!("$.entry.routes[{index}]");
        let object = self.object(value, &path)?;
        self.check_fields(object, &["method", "path", "status", "body"], &path);
        if let Some(method) = self.string(object, "method", &path)
            && method != "GET"
        {
            self.error(
                &property_path(&path, "method"),
                "only GET routes are supported",
            );
        }
        let route_path = self.string(object, "path", &path);
        if let Some(route_path) = &route_path
            && route_path.len() > MAX_ROUTE_PATH_BYTES
        {
            self.error(
                &property_path(&path, "path"),
                format!("route path exceeds {MAX_ROUTE_PATH_BYTES} bytes"),
            );
        }
        if let Some(route_path) = &route_path
            && (!route_path.starts_with('/')
                || !route_path.is_ascii()
                || route_path.bytes().any(|byte| {
                    byte.is_ascii_whitespace()
                        || byte.is_ascii_control()
                        || matches!(byte, b'?' | b'#')
                }))
        {
            self.error(
                &property_path(&path, "path"),
                "route path must start with / and contain only ASCII without whitespace, controls, query, or fragment",
            );
        }
        let status = self.unsigned_16(object, "status", &path);
        if let Some(status) = status
            && (!(200..=299).contains(&status) || matches!(status, 204 | 205))
        {
            self.error(
                &property_path(&path, "status"),
                "status must be 200 to 299 except bodyless statuses 204 and 205",
            );
        }
        let body = self.operand(object, "body", &path);
        if let Some(body) = body {
            self.expect_type(body, Type::Json, &property_path(&path, "body"));
        }
        Some(CompiledRoute {
            path: route_path?,
            status: status?,
            body: JsonRegister(body?.index()),
        })
    }
}

fn valid_id(id: &str) -> bool {
    let mut bytes = id.bytes();
    (1..=64).contains(&id.len())
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn property_path(parent: &str, key: &str) -> String {
    if valid_id(key) {
        format!("{parent}.{key}")
    } else {
        format!("{parent}[{}]", quoted(key))
    }
}

fn quoted(value: &str) -> String {
    // Serializing a Rust string into an owned JSON string cannot fail.
    serde_json::to_string(value).expect("strings are serializable")
}

#[derive(Debug)]
enum RuntimeValue {
    Text(String),
    Int(i64),
    Bool(bool),
    Json(Value),
}

impl RuntimeValue {
    fn to_json(&self) -> Value {
        match self {
            Self::Text(value) => Value::String(value.clone()),
            Self::Int(value) => Value::from(*value),
            Self::Bool(value) => Value::Bool(*value),
            Self::Json(value) => value.clone(),
        }
    }
}

fn text_at(values: &[RuntimeValue], register: TextRegister) -> Result<&str, String> {
    match values.get(register.0) {
        Some(RuntimeValue::Text(value)) => Ok(value),
        _ => Err("internal IR invariant failed: expected initialized Text register".into()),
    }
}

fn int_at(values: &[RuntimeValue], register: IntRegister) -> Result<i64, String> {
    match values.get(register.0) {
        Some(RuntimeValue::Int(value)) => Ok(*value),
        _ => Err("internal IR invariant failed: expected initialized Int register".into()),
    }
}

fn json_at(values: &[RuntimeValue], register: JsonRegister) -> Result<&Value, String> {
    match values.get(register.0) {
        Some(RuntimeValue::Json(value)) => Ok(value),
        _ => Err("internal IR invariant failed: expected initialized Json register".into()),
    }
}

fn checked_size(bytes: usize, context: &str) -> Result<usize, String> {
    if bytes > MAX_VALUE_BYTES {
        Err(format!(
            "{context} exceeds {MAX_VALUE_BYTES} bytes ({bytes} bytes required)"
        ))
    } else {
        Ok(bytes)
    }
}

fn evaluate(operation: &Instruction, values: &[RuntimeValue]) -> Result<RuntimeValue, String> {
    match operation {
        Instruction::Text(value) => Ok(RuntimeValue::Text(value.clone())),
        Instruction::Int(value) => Ok(RuntimeValue::Int(*value)),
        Instruction::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        Instruction::Concat(registers) => {
            let mut size = 0;
            for register in registers {
                size = checked_size(size + text_at(values, *register)?.len(), "computed Text")?;
            }
            let mut result = String::with_capacity(size);
            for register in registers {
                result.push_str(text_at(values, *register)?);
            }
            Ok(RuntimeValue::Text(result))
        }
        Instruction::Add(left, right) => {
            let left = int_at(values, *left)?;
            let right = int_at(values, *right)?;
            left.checked_add(right)
                .map(RuntimeValue::Int)
                .ok_or_else(|| format!("signed 64-bit integer overflow: {left} + {right}"))
        }
        Instruction::ToText(register) => {
            let result = match register {
                ScalarRegister::Text(register) => text_at(values, *register)?.to_owned(),
                ScalarRegister::Int(register) => int_at(values, *register)?.to_string(),
                ScalarRegister::Bool(register) => match values.get(register.0) {
                    Some(RuntimeValue::Bool(value)) => value.to_string(),
                    _ => {
                        return Err(
                            "internal IR invariant failed: expected initialized Bool register"
                                .into(),
                        );
                    }
                },
            };
            checked_size(result.len(), "computed Text")?;
            Ok(RuntimeValue::Text(result))
        }
        Instruction::Object(fields) => {
            let mut result = Map::new();
            let mut size = 2; // Opening and closing braces, including for an empty object.
            for (key, register) in fields {
                let value = values
                    .get(register.index())
                    .ok_or("internal IR invariant failed: expected initialized value register")?
                    .to_json();
                let encoded = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
                size = checked_size(
                    size + usize::from(!result.is_empty()) + quoted(key).len() + 1 + encoded.len(),
                    "computed Json",
                )?;
                // Check before retaining the next cloned subtree, bounding intermediate growth.
                result.insert(key.clone(), value);
            }
            Ok(RuntimeValue::Json(Value::Object(result)))
        }
    }
}

impl CompiledProgram {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Lower the validated typed registers to LLVM IR for native compilation.
    pub fn emit_llvm(&self) -> String {
        native_codegen::emit(self)
    }

    pub fn entry_info(&self) -> EntryInfo {
        match &self.entry {
            Entry::Console(_) => EntryInfo::Console,
            Entry::Http { port, routes } => EntryInfo::Http {
                port: *port,
                routes: routes.iter().map(|route| route.path.clone()).collect(),
            },
        }
    }

    /// A stable, human-readable dump of the indexed typed executable representation.
    pub fn disassemble(&self) -> String {
        let mut output = format!("rivet-ir v1 {}\n", quoted(&self.name));
        for (index, instruction) in self.instructions.iter().enumerate() {
            let value_type = instruction.operation.result_register(index).value_type();
            write!(output, "%{index}: {value_type} = ").unwrap();
            match &instruction.operation {
                Instruction::Text(value) => write!(output, "text {}", quoted(value)).unwrap(),
                Instruction::Int(value) => write!(output, "int {value}").unwrap(),
                Instruction::Bool(value) => write!(output, "bool {value}").unwrap(),
                Instruction::Concat(registers) => {
                    let operands = registers
                        .iter()
                        .map(|register| format!("%{}", register.0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(output, "concat [{operands}]").unwrap();
                }
                Instruction::Add(left, right) => {
                    write!(output, "add %{}, %{}", left.0, right.0).unwrap()
                }
                Instruction::ToText(register) => {
                    write!(output, "to_text %{}", register.index()).unwrap()
                }
                Instruction::Object(fields) => {
                    let fields = fields
                        .iter()
                        .map(|(key, register)| format!("{}: %{}", quoted(key), register.index()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(output, "object {{{fields}}}").unwrap();
                }
            }
            writeln!(output, "    ; {}", instruction.id).unwrap();
        }
        match &self.entry {
            Entry::Console(register) => writeln!(output, "entry console %{}", register.0).unwrap(),
            Entry::Http { port, routes } => {
                writeln!(output, "entry http 127.0.0.1:{port}").unwrap();
                for route in routes {
                    writeln!(
                        output,
                        "  GET {} -> {} json %{}",
                        quoted(&route.path),
                        route.status,
                        route.body.0
                    )
                    .unwrap();
                }
            }
        }
        output
    }

    /// Evaluate every instruction once, yielding console output or an HTTP service definition.
    /// Binding sockets and emitting output are runtime responsibilities, outside the language core.
    pub fn execute(&self) -> Result<Execution, String> {
        let mut values = Vec::with_capacity(self.instructions.len());
        for (index, instruction) in self.instructions.iter().enumerate() {
            let value = evaluate(&instruction.operation, &values)
                .map_err(|error| format!("instruction {:?} (%{index}): {error}", instruction.id))?;
            values.push(value);
        }
        match &self.entry {
            Entry::Console(register) => {
                let text = text_at(&values, *register)?;
                checked_size(text.len() + 1, "console output including newline")?;
                let mut stdout = String::with_capacity(text.len() + 1);
                stdout.push_str(text);
                stdout.push('\n');
                Ok(Execution::Console { stdout })
            }
            Entry::Http { port, routes } => {
                let routes = routes
                    .iter()
                    .map(|route| {
                        Ok(Route {
                            method: "GET".into(),
                            path: route.path.clone(),
                            status: route.status,
                            body: json_at(&values, route.body)?.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Execution::Http {
                    port: *port,
                    routes,
                })
            }
        }
    }
}
