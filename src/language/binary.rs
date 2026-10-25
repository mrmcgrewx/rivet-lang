//! RVB binary source: version-dispatched, bounded direct decoding into typed IR.
//!
//! There is no intermediate JSON document or textual assembly input. See
//! docs/BINARY-FORMAT.md for the canonical source encoding.

use super::*;

const MAGIC: &[u8; 4] = b"RVB\0";
const HEADER_BYTES: usize = 16;

/// Readable inspection metadata; the canonical source remains the original bytes.
#[derive(Clone, Debug, Serialize)]
pub struct BinaryInspection {
    pub ir: String,
    pub instructions: Vec<BinaryInstructionInfo>,
    pub entry: EntryInfo,
    pub entry_offset: usize,
    pub source_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct BinaryInstructionInfo {
    pub index: usize,
    pub opcode: u8,
    pub type_name: String,
    pub offset: usize,
    pub payload_offset: usize,
    pub payload_length: usize,
    /// Exact text, decimal signed i64, or `true`/`false`; absent for nonliterals.
    pub literal: Option<String>,
}

/// Validate RVB1/RVB2 source bytes and construct typed IR directly.
/// Reports the first structural/type error with its absolute byte offset.
/// Computed values are not evaluated during decoding.
pub fn compile_binary(source: &[u8]) -> Result<CompiledProgram, Vec<Diagnostic>> {
    decode(source)
        .map(|(program, _, _)| program)
        .map_err(|error| vec![error])
}

/// Decode the same source as `compile_binary`, returning a human inspection view.
pub fn inspect_binary(source: &[u8]) -> Result<BinaryInspection, Vec<Diagnostic>> {
    let (program, instructions, entry_offset) = decode(source).map_err(|error| vec![error])?;
    Ok(BinaryInspection {
        ir: program.disassemble(),
        entry: program.entry_info(),
        instructions,
        entry_offset,
        source_bytes: source.len(),
    })
}

pub(super) fn error(offset: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        path: format!("byte 0x{offset:04x}"),
        message: message.into(),
    }
}

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    base: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8], base: usize) -> Self {
        Self {
            bytes,
            cursor: 0,
            base,
        }
    }

    pub(super) fn position(&self) -> usize {
        self.base + self.cursor
    }

    pub(super) fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    pub(super) fn take(&mut self, length: usize, context: &str) -> Result<&'a [u8], Diagnostic> {
        if length > self.remaining() {
            return Err(error(
                self.position(),
                format!(
                    "truncated {context}: need {length} bytes, have {}",
                    self.remaining()
                ),
            ));
        }
        let start = self.cursor;
        self.cursor += length;
        Ok(&self.bytes[start..self.cursor])
    }

    pub(super) fn u8(&mut self, context: &str) -> Result<u8, Diagnostic> {
        Ok(self.take(1, context)?[0])
    }

    pub(super) fn u16(&mut self, context: &str) -> Result<u16, Diagnostic> {
        let bytes = self.take(2, context)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(super) fn u32(&mut self, context: &str) -> Result<u32, Diagnostic> {
        let bytes = self.take(4, context)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn utf8(&mut self, length: usize, context: &str) -> Result<String, Diagnostic> {
        let offset = self.position();
        let bytes = self.take(length, context)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|cause| {
                error(
                    offset + cause.valid_up_to(),
                    format!("invalid UTF-8 in {context}"),
                )
            })
    }

    pub(super) fn finish(&self, context: &str) -> Result<(), Diagnostic> {
        if self.remaining() != 0 {
            Err(error(
                self.position(),
                format!(
                    "unexpected trailing bytes in {context}: {} bytes",
                    self.remaining()
                ),
            ))
        } else {
            Ok(())
        }
    }
}

fn operand(
    reader: &mut Reader<'_>,
    instructions: &[NamedInstruction],
    expected: Option<Type>,
) -> Result<ValueRegister, Diagnostic> {
    let offset = reader.position();
    let index = usize::from(reader.u16("register reference")?);
    let instruction = instructions.get(index).ok_or_else(|| {
        error(
            offset,
            format!("register %{index} is not defined; references must name an earlier register"),
        )
    })?;
    let register = instruction.operation.result_register(index);
    if let Some(expected) = expected
        && register.value_type() != expected
    {
        return Err(error(
            offset,
            format!(
                "register %{index}: expected {expected}, found {}",
                register.value_type()
            ),
        ));
    }
    Ok(register)
}

fn decode_instruction(
    reader: &mut Reader<'_>,
    instructions: &[NamedInstruction],
) -> Result<(NamedInstruction, BinaryInstructionInfo), Diagnostic> {
    let offset = reader.position();
    let opcode = reader.u8("instruction opcode")?;
    let expected_tag = match opcode {
        0x01 | 0x10 | 0x12 => 1,
        0x02 | 0x11 => 2,
        0x03 => 3,
        0x20 => 4,
        _ => return Err(error(offset, format!("unknown opcode 0x{opcode:02x}"))),
    };
    let type_tag = reader.u8("instruction result type")?;
    if type_tag != expected_tag {
        return Err(error(
            offset + 1,
            format!(
                "opcode 0x{opcode:02x} requires result type tag {expected_tag}, found {type_tag}"
            ),
        ));
    }
    let payload_length = usize::from(reader.u16("instruction payload length")?);
    let payload_offset = reader.position();
    let mut payload = Reader::new(
        reader.take(payload_length, "instruction payload")?,
        payload_offset,
    );
    let operation = match opcode {
        0x01 => {
            if payload_length > MAX_LITERAL_BYTES {
                return Err(error(
                    payload_offset,
                    format!("Text literal exceeds {MAX_LITERAL_BYTES} bytes"),
                ));
            }
            Instruction::Text(payload.utf8(payload_length, "Text literal")?)
        }
        0x02 => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(payload.take(8, "Int literal")?);
            Instruction::Int(i64::from_le_bytes(bytes))
        }
        0x03 => match payload.u8("Bool literal")? {
            0 => Instruction::Bool(false),
            1 => Instruction::Bool(true),
            value => {
                return Err(error(
                    payload_offset,
                    format!("Bool literal must be 0 or 1, found {value}"),
                ));
            }
        },
        0x10 => {
            let count = usize::from(payload.u16("concat operand count")?);
            if count > payload.remaining() / 2 {
                return Err(error(
                    payload.position(),
                    "concat operand count exceeds payload length",
                ));
            }
            let mut operands = Vec::with_capacity(count);
            for _ in 0..count {
                let register = operand(&mut payload, instructions, Some(Type::Text))?;
                operands.push(TextRegister(register.index()));
            }
            Instruction::Concat(operands)
        }
        0x11 => {
            let left = operand(&mut payload, instructions, Some(Type::Int))?;
            let right = operand(&mut payload, instructions, Some(Type::Int))?;
            Instruction::Add(IntRegister(left.index()), IntRegister(right.index()))
        }
        0x12 => {
            let offset = payload.position();
            let register = operand(&mut payload, instructions, None)?;
            let scalar = match register {
                ValueRegister::Text(register) => ScalarRegister::Text(register),
                ValueRegister::Int(register) => ScalarRegister::Int(register),
                ValueRegister::Bool(register) => ScalarRegister::Bool(register),
                ValueRegister::Json(_) => {
                    return Err(error(
                        offset,
                        "to_text expects Text, Int, or Bool; found Json",
                    ));
                }
            };
            Instruction::ToText(scalar)
        }
        0x20 => {
            let count = usize::from(payload.u16("object field count")?);
            if count > payload.remaining() / 4 {
                return Err(error(
                    payload.position(),
                    "object field count exceeds payload length",
                ));
            }
            let mut fields: Vec<(String, ValueRegister)> = Vec::with_capacity(count);
            for _ in 0..count {
                let length = usize::from(payload.u16("object key length")?);
                let key_offset = payload.position();
                let key = payload.utf8(length, "object key")?;
                if fields
                    .last()
                    .is_some_and(|(previous, _)| previous.as_bytes() >= key.as_bytes())
                {
                    return Err(error(
                        key_offset,
                        "object keys must be unique and strictly ascending in UTF-8 byte order",
                    ));
                }
                let register = operand(&mut payload, instructions, None)?;
                fields.push((key, register));
            }
            Instruction::Object(fields)
        }
        _ => unreachable!("opcode was checked before reading the payload"),
    };
    payload.finish("instruction payload")?;
    let index = instructions.len();
    let literal = match &operation {
        Instruction::Text(value) => Some(value.clone()),
        Instruction::Int(value) => Some(value.to_string()),
        Instruction::Bool(value) => Some(value.to_string()),
        _ => None,
    };
    let info = BinaryInstructionInfo {
        index,
        opcode,
        type_name: operation.result_register(index).value_type().to_string(),
        offset,
        payload_offset,
        payload_length,
        literal,
    };
    Ok((
        NamedInstruction {
            id: format!("r{index}"),
            operation,
        },
        info,
    ))
}

fn decode_entry(
    reader: &mut Reader<'_>,
    tag: u8,
    instructions: &[NamedInstruction],
) -> Result<Entry, Diagnostic> {
    if tag == 1 {
        let register = operand(reader, instructions, Some(Type::Text))?;
        return Ok(Entry::Console(TextRegister(register.index())));
    }
    let port = reader.u16("HTTP port")?;
    let count_offset = reader.position();
    let count = usize::from(reader.u16("HTTP route count")?);
    if count > MAX_ROUTES {
        return Err(error(
            count_offset,
            format!("at most {MAX_ROUTES} HTTP routes are allowed"),
        ));
    }
    let mut routes = Vec::with_capacity(count);
    let mut paths = HashSet::new();
    for _ in 0..count {
        let path_length = usize::from(reader.u16("route path length")?);
        let path_offset = reader.position();
        if path_length > MAX_ROUTE_PATH_BYTES {
            return Err(error(
                path_offset,
                format!("route path exceeds {MAX_ROUTE_PATH_BYTES} bytes"),
            ));
        }
        let path = reader.utf8(path_length, "route path")?;
        if !path.starts_with('/')
            || !path.is_ascii()
            || path.bytes().any(|byte| {
                byte.is_ascii_whitespace() || byte.is_ascii_control() || matches!(byte, b'?' | b'#')
            })
        {
            return Err(error(
                path_offset,
                "route path must start with / and contain only ASCII without whitespace, controls, query, or fragment",
            ));
        }
        if !paths.insert(path.clone()) {
            return Err(error(path_offset, "duplicate route path"));
        }
        let status_offset = reader.position();
        let status = reader.u16("route status")?;
        if !(200..=299).contains(&status) || matches!(status, 204 | 205) {
            return Err(error(
                status_offset,
                "status must be 200 to 299 except bodyless statuses 204 and 205",
            ));
        }
        let body = operand(reader, instructions, Some(Type::Json))?;
        routes.push(CompiledRoute {
            path,
            status,
            body: JsonRegister(body.index()),
        });
    }
    Ok(Entry::Http { port, routes })
}

pub(super) type Decoded = (CompiledProgram, Vec<BinaryInstructionInfo>, usize);

fn decode(source: &[u8]) -> Result<Decoded, Diagnostic> {
    if source.get(4..6) == Some(&[2, 0]) {
        return super::compute::decode(source);
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(
            0,
            format!("binary source exceeds {MAX_SOURCE_BYTES} bytes"),
        ));
    }
    let mut reader = Reader::new(source, 0);
    if reader.take(4, "RVB1 magic")? != MAGIC {
        return Err(error(
            0,
            "expected RVB1 binary source (magic 52 56 42 00); textual source is not accepted",
        ));
    }
    if reader.u16("format version")? != 1 {
        return Err(error(4, "unsupported binary source version; expected 1"));
    }
    if reader.u16("format flags")? != 0 {
        return Err(error(6, "RVB1 flags must be zero"));
    }
    let instruction_count = usize::from(reader.u16("instruction count")?);
    if instruction_count > MAX_INSTRUCTIONS {
        return Err(error(
            8,
            format!("at most {MAX_INSTRUCTIONS} instructions are allowed"),
        ));
    }
    let entry_tag = reader.u8("entry tag")?;
    if !matches!(entry_tag, 1 | 2) {
        return Err(error(10, "entry tag must be 1 (console) or 2 (HTTP)"));
    }
    if reader.u8("reserved header byte")? != 0 {
        return Err(error(11, "reserved header byte must be zero"));
    }
    let body_length = reader.u32("body length")? as usize;
    if body_length != reader.remaining() {
        return Err(error(
            12,
            format!(
                "declared body length {body_length} does not match {} remaining bytes",
                reader.remaining()
            ),
        ));
    }
    let mut instructions = Vec::with_capacity(instruction_count);
    let mut infos = Vec::with_capacity(instruction_count);
    for _ in 0..instruction_count {
        let (instruction, info) = decode_instruction(&mut reader, &instructions)?;
        instructions.push(instruction);
        infos.push(info);
    }
    let entry_offset = reader.position();
    let entry = decode_entry(&mut reader, entry_tag, &instructions)?;
    reader.finish("binary source")?;
    let name = if entry_tag == 1 {
        "Binary console program"
    } else {
        "Binary HTTP program"
    }
    .to_owned();
    Ok((
        CompiledProgram {
            name,
            instructions,
            entry,
        },
        infos,
        entry_offset,
    ))
}

fn write_u16(output: &mut Vec<u8>, value: usize) {
    // The private IR only admits bounded, validated source programs. All fields
    // represented as u16 are bounded by the source limit or instruction count.
    output.extend_from_slice(
        &u16::try_from(value)
            .expect("validated RVB1 field fits u16")
            .to_le_bytes(),
    );
}

impl CompiledProgram {
    /// Encode the validated typed program as canonical source bytes in its original version.
    /// Debug names are not part of the binary format and are intentionally omitted.
    pub fn encode_binary(&self) -> Vec<u8> {
        if let Entry::Compute(program) = &self.entry {
            return program.encode();
        }
        let mut body = Vec::new();
        for instruction in &self.instructions {
            let mut payload = Vec::new();
            let (opcode, tag) = match &instruction.operation {
                Instruction::Text(value) => {
                    payload.extend_from_slice(value.as_bytes());
                    (0x01, 1)
                }
                Instruction::Int(value) => {
                    payload.extend_from_slice(&value.to_le_bytes());
                    (0x02, 2)
                }
                Instruction::Bool(value) => {
                    payload.push(u8::from(*value));
                    (0x03, 3)
                }
                Instruction::Concat(registers) => {
                    write_u16(&mut payload, registers.len());
                    for register in registers {
                        write_u16(&mut payload, register.0);
                    }
                    (0x10, 1)
                }
                Instruction::Add(left, right) => {
                    write_u16(&mut payload, left.0);
                    write_u16(&mut payload, right.0);
                    (0x11, 2)
                }
                Instruction::ToText(register) => {
                    write_u16(&mut payload, register.index());
                    (0x12, 1)
                }
                Instruction::Object(fields) => {
                    write_u16(&mut payload, fields.len());
                    for (key, register) in fields {
                        write_u16(&mut payload, key.len());
                        payload.extend_from_slice(key.as_bytes());
                        write_u16(&mut payload, register.index());
                    }
                    (0x20, 4)
                }
            };
            body.extend_from_slice(&[opcode, tag]);
            write_u16(&mut body, payload.len());
            body.extend_from_slice(&payload);
        }
        let entry_tag = match &self.entry {
            Entry::Compute(_) => unreachable!("compute encoding dispatched above"),
            Entry::Console(register) => {
                write_u16(&mut body, register.0);
                1
            }
            Entry::Http { port, routes } => {
                body.extend_from_slice(&port.to_le_bytes());
                write_u16(&mut body, routes.len());
                for route in routes {
                    write_u16(&mut body, route.path.len());
                    body.extend_from_slice(route.path.as_bytes());
                    body.extend_from_slice(&route.status.to_le_bytes());
                    write_u16(&mut body, route.body.0);
                }
                2
            }
        };
        let mut output = Vec::with_capacity(HEADER_BYTES + body.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&1u16.to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        write_u16(&mut output, self.instructions.len());
        output.extend_from_slice(&[entry_tag, 0]);
        output.extend_from_slice(&(body.len() as u32).to_le_bytes());
        output.extend_from_slice(&body);
        output
    }
}
