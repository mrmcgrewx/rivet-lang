//! RVB2 computation IR, direct byte decoder, CFG validation and reference semantics.
//! The instruction stream is source; input data is read only during execution.
use super::{
    CompiledProgram, Diagnostic, Entry, Execution, MAX_INSTRUCTIONS, MAX_SOURCE_BYTES,
    binary::{BinaryInstructionInfo, Decoded, Reader, error},
};
use std::{collections::HashMap, fmt::Write, rc::Rc};

const MAX_REGISTERS: usize = 256;
const MAX_ARRAY: usize = 10_000;
const MAX_INPUT: usize = 262_144;
const MAX_STEPS: usize = 1_000_000;
const MAX_ALLOCATION: usize = 64 * 1024 * 1024;
const MAP_ALLOCATION: usize = 32768 * 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RegisterType {
    Int,
    Bool,
    Array,
    Map,
}
impl RegisterType {
    fn tag(self) -> u8 {
        match self {
            Self::Int => 2,
            Self::Bool => 3,
            Self::Array => 5,
            Self::Map => 6,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::Bool => "Bool",
            Self::Array => "IntArray",
            Self::Map => "IntMap",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Op {
    Int(usize, i64),
    Bool(usize, bool),
    Add(usize, usize, usize),
    Sub(usize, usize, usize),
    Equal(usize, usize, usize),
    Less(usize, usize, usize),
    ReadInt(usize),
    ReadArray(usize),
    Length(usize, usize),
    Get(usize, usize, usize),
    Array(usize, Vec<usize>),
    Map(usize),
    Lookup(usize, usize, usize, usize),
    Put(usize, usize, usize),
    Jump(usize),
    Branch(usize, usize, usize),
    Return(usize),
}
impl Op {
    fn destination(&self) -> Option<usize> {
        match self {
            Self::Int(d, _)
            | Self::Bool(d, _)
            | Self::Add(d, ..)
            | Self::Sub(d, ..)
            | Self::Equal(d, ..)
            | Self::Less(d, ..)
            | Self::ReadInt(d)
            | Self::ReadArray(d)
            | Self::Length(d, _)
            | Self::Get(d, ..)
            | Self::Array(d, _)
            | Self::Map(d)
            | Self::Lookup(d, ..) => Some(*d),
            Self::Put(..) | Self::Jump(_) | Self::Branch(..) | Self::Return(_) => None,
        }
    }
    fn reads(&self) -> Vec<usize> {
        match self {
            Self::Int(..)
            | Self::Bool(..)
            | Self::ReadInt(_)
            | Self::ReadArray(_)
            | Self::Map(_)
            | Self::Jump(_) => vec![],
            Self::Add(_, a, b)
            | Self::Sub(_, a, b)
            | Self::Equal(_, a, b)
            | Self::Less(_, a, b)
            | Self::Get(_, a, b) => vec![*a, *b],
            Self::Length(_, a) | Self::Return(a) | Self::Branch(a, ..) => vec![*a],
            Self::Array(_, values) => values.clone(),
            Self::Lookup(_, a, b, c) | Self::Put(a, b, c) => vec![*a, *b, *c],
        }
    }
    fn successors(&self, pc: usize) -> Vec<usize> {
        match self {
            Self::Return(_) => vec![],
            Self::Jump(target) => vec![*target],
            Self::Branch(_, yes, no) => vec![*yes, *no],
            _ => vec![pc + 1],
        }
    }
    fn record(&self) -> (u8, u8, Vec<u8>) {
        let mut payload = Vec::new();
        let (opcode, tag) = match self {
            Self::Int(d, value) => {
                put_u16(&mut payload, *d);
                payload.extend_from_slice(&value.to_le_bytes());
                (0x02, 2)
            }
            Self::Bool(d, value) => {
                put_u16(&mut payload, *d);
                payload.push(u8::from(*value));
                (0x03, 3)
            }
            Self::Add(d, a, b)
            | Self::Sub(d, a, b)
            | Self::Equal(d, a, b)
            | Self::Less(d, a, b)
            | Self::Get(d, a, b) => {
                put_refs(&mut payload, &[*d, *a, *b]);
                match self {
                    Self::Add(..) => (0x11, 2),
                    Self::Sub(..) => (0x13, 2),
                    Self::Equal(..) => (0x14, 3),
                    Self::Less(..) => (0x15, 3),
                    _ => (0x33, 2),
                }
            }
            Self::ReadInt(d) | Self::ReadArray(d) | Self::Map(d) => {
                put_u16(&mut payload, *d);
                match self {
                    Self::ReadInt(_) => (0x30, 2),
                    Self::ReadArray(_) => (0x31, 5),
                    _ => (0x40, 6),
                }
            }
            Self::Length(d, a) => {
                put_refs(&mut payload, &[*d, *a]);
                (0x32, 2)
            }
            Self::Array(d, values) => {
                put_refs(&mut payload, &[*d, values.len()]);
                put_refs(&mut payload, values);
                (0x34, 5)
            }
            Self::Lookup(d, map, key, fallback) => {
                put_refs(&mut payload, &[*d, *map, *key, *fallback]);
                (0x41, 2)
            }
            Self::Put(map, key, value) => {
                put_refs(&mut payload, &[*map, *key, *value]);
                (0x42, 0)
            }
            Self::Jump(target) => {
                put_u16(&mut payload, *target);
                (0x50, 0)
            }
            Self::Branch(condition, yes, no) => {
                put_refs(&mut payload, &[*condition, *yes, *no]);
                (0x51, 0)
            }
            Self::Return(array) => {
                put_u16(&mut payload, *array);
                (0x52, 0)
            }
        };
        (opcode, tag, payload)
    }
}

#[derive(Clone, Debug)]
pub(super) struct ComputeProgram {
    pub(super) registers: Vec<RegisterType>,
    pub(super) operations: Vec<Op>,
}

fn register(
    reader: &mut Reader<'_>,
    types: &[RegisterType],
    expected: RegisterType,
) -> Result<usize, Diagnostic> {
    let offset = reader.position();
    let register = usize::from(reader.u16("register reference")?);
    let actual = types.get(register).ok_or_else(|| {
        error(
            offset,
            format!("register %{register} is outside the declared register bank"),
        )
    })?;
    if *actual != expected {
        return Err(error(
            offset,
            format!(
                "register %{register}: expected {}, found {}",
                expected.name(),
                actual.name()
            ),
        ));
    }
    Ok(register)
}
fn target(reader: &mut Reader<'_>, count: usize) -> Result<usize, Diagnostic> {
    let offset = reader.position();
    let pc = usize::from(reader.u16("branch target")?);
    if pc >= count {
        return Err(error(
            offset,
            format!("branch target @{pc} is outside the instruction stream"),
        ));
    }
    Ok(pc)
}

fn decode_op(
    reader: &mut Reader<'_>,
    types: &[RegisterType],
    count: usize,
    index: usize,
) -> Result<(Op, BinaryInstructionInfo), Diagnostic> {
    use RegisterType::{Array, Bool, Int, Map};
    let offset = reader.position();
    let opcode = reader.u8("instruction opcode")?;
    let expected = match opcode {
        0x02 | 0x11 | 0x13 | 0x30 | 0x32 | 0x33 | 0x41 => Some(Int),
        0x03 | 0x14 | 0x15 => Some(Bool),
        0x31 | 0x34 => Some(Array),
        0x40 => Some(Map),
        0x42 | 0x50 | 0x51 | 0x52 => None,
        _ => return Err(error(offset, format!("unknown RVB2 opcode 0x{opcode:02x}"))),
    };
    let tag = reader.u8("result type tag")?;
    let expected_tag = expected.map_or(0, RegisterType::tag);
    if tag != expected_tag {
        return Err(error(
            offset + 1,
            format!("opcode 0x{opcode:02x} requires result type tag {expected_tag}, found {tag}"),
        ));
    }
    let payload_length = usize::from(reader.u16("instruction payload length")?);
    let payload_offset = reader.position();
    let mut p = Reader::new(
        reader.take(payload_length, "instruction payload")?,
        payload_offset,
    );
    let destination = match expected {
        Some(t) => Some(register(&mut p, types, t)?),
        None => None,
    };
    let d = destination.unwrap_or(0);
    let operation = match opcode {
        0x02 => {
            let bytes: [u8; 8] = p
                .take(8, "Int literal")?
                .try_into()
                .expect("length checked");
            Op::Int(d, i64::from_le_bytes(bytes))
        }
        0x03 => {
            let value = p.u8("Bool literal")?;
            if value > 1 {
                return Err(error(p.position() - 1, "Bool literal must be 0 or 1"));
            }
            Op::Bool(d, value == 1)
        }
        0x11 | 0x13 | 0x14 | 0x15 => {
            let a = register(&mut p, types, Int)?;
            let b = register(&mut p, types, Int)?;
            match opcode {
                0x11 => Op::Add(d, a, b),
                0x13 => Op::Sub(d, a, b),
                0x14 => Op::Equal(d, a, b),
                _ => Op::Less(d, a, b),
            }
        }
        0x30 => Op::ReadInt(d),
        0x31 => Op::ReadArray(d),
        0x32 => Op::Length(d, register(&mut p, types, Array)?),
        0x33 => Op::Get(
            d,
            register(&mut p, types, Array)?,
            register(&mut p, types, Int)?,
        ),
        0x34 => {
            let length_offset = p.position();
            let length = usize::from(p.u16("array element count")?);
            if length > MAX_ARRAY {
                return Err(error(length_offset, "array element count exceeds 10000"));
            }
            if length > p.remaining() / 2 {
                return Err(error(p.position(), "truncated array register list"));
            }
            let mut values = Vec::with_capacity(length);
            for _ in 0..length {
                values.push(register(&mut p, types, Int)?);
            }
            Op::Array(d, values)
        }
        0x40 => Op::Map(d),
        0x41 => Op::Lookup(
            d,
            register(&mut p, types, Map)?,
            register(&mut p, types, Int)?,
            register(&mut p, types, Int)?,
        ),
        0x42 => Op::Put(
            register(&mut p, types, Map)?,
            register(&mut p, types, Int)?,
            register(&mut p, types, Int)?,
        ),
        0x50 => Op::Jump(target(&mut p, count)?),
        0x51 => Op::Branch(
            register(&mut p, types, Bool)?,
            target(&mut p, count)?,
            target(&mut p, count)?,
        ),
        0x52 => Op::Return(register(&mut p, types, Array)?),
        _ => unreachable!(),
    };
    p.finish("RVB2 instruction payload")?;
    Ok((
        operation,
        BinaryInstructionInfo {
            index,
            opcode,
            type_name: expected.map_or("Void", RegisterType::name).into(),
            offset,
            payload_offset,
            payload_length,
            literal: None,
        },
    ))
}

pub(super) fn decode(source: &[u8]) -> Result<Decoded, Diagnostic> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(0, "binary source exceeds 65536 bytes"));
    }
    let mut reader = Reader::new(source, 0);
    if reader.take(4, "RVB magic")? != b"RVB\0" {
        return Err(error(0, "expected RVB binary source magic 52 56 42 00"));
    }
    if reader.u16("format version")? != 2 {
        return Err(error(4, "expected RVB2 version 2"));
    }
    if reader.u16("format flags")? != 0 {
        return Err(error(6, "RVB2 flags must be zero"));
    }
    let count = usize::from(reader.u16("instruction count")?);
    if !(1..=MAX_INSTRUCTIONS).contains(&count) {
        return Err(error(8, "RVB2 requires 1 to 256 instructions"));
    }
    if reader.u8("entry tag")? != 3 {
        return Err(error(10, "binary version 2 entry tag must be 3 (compute)"));
    }
    if reader.u8("reserved header byte")? != 0 {
        return Err(error(11, "reserved header byte must be zero"));
    }
    let body_length = reader.u32("body length")? as usize;
    if body_length != reader.remaining() {
        return Err(error(
            12,
            "declared body length does not match remaining bytes",
        ));
    }
    let register_count = usize::from(reader.u16("register count")?);
    if !(1..=MAX_REGISTERS).contains(&register_count) {
        return Err(error(16, "RVB2 requires 1 to 256 registers"));
    }
    let mut registers = Vec::with_capacity(register_count);
    for _ in 0..register_count {
        let offset = reader.position();
        registers.push(match reader.u8("register type")? {
            2 => RegisterType::Int,
            3 => RegisterType::Bool,
            5 => RegisterType::Array,
            6 => RegisterType::Map,
            value => {
                return Err(error(
                    offset,
                    format!("unknown RVB2 register type tag {value}"),
                ));
            }
        });
    }
    let mut operations = Vec::with_capacity(count);
    let mut infos = Vec::with_capacity(count);
    for index in 0..count {
        let (op, info) = decode_op(&mut reader, &registers, count, index)?;
        operations.push(op);
        infos.push(info);
    }
    let entry_offset = reader.position();
    if reader.u16("compute entry PC")? != 0 {
        return Err(error(entry_offset, "RVB2 compute entry PC must be zero"));
    }
    reader.finish("binary source")?;
    let program = ComputeProgram {
        registers,
        operations,
    };
    program.validate_flow(&infos)?;
    Ok((
        CompiledProgram {
            name: "Binary compute program".into(),
            instructions: vec![],
            entry: Entry::Compute(program),
        },
        infos,
        entry_offset,
    ))
}

fn put_u16(output: &mut Vec<u8>, value: usize) {
    output.extend_from_slice(
        &u16::try_from(value)
            .expect("validated RVB2 u16 field")
            .to_le_bytes(),
    );
}
fn put_refs(output: &mut Vec<u8>, values: &[usize]) {
    for value in values {
        put_u16(output, *value);
    }
}

impl ComputeProgram {
    pub(super) fn instruction_count(&self) -> usize {
        self.operations.len()
    }

    /// A forward must-analysis: only definitions present on every reachable incoming
    /// path are available. Entry always has the additional empty initial path.
    fn validate_flow(&self, infos: &[BinaryInstructionInfo]) -> Result<(), Diagnostic> {
        let count = self.operations.len();
        let mut reachable = vec![false; count];
        let mut work = vec![0];
        let mut predecessors = vec![Vec::new(); count];
        while let Some(pc) = work.pop() {
            if reachable[pc] {
                continue;
            }
            reachable[pc] = true;
            for next in self.operations[pc].successors(pc) {
                if next >= count {
                    return Err(error(
                        infos[pc].offset,
                        format!(
                            "reachable instruction @{pc} falls through past the instruction stream"
                        ),
                    ));
                }
                predecessors[next].push(pc);
                work.push(next);
            }
        }
        let width = self.registers.len();
        let mut inputs = vec![vec![true; width]; count];
        let mut outputs = inputs.clone();
        loop {
            let mut changed = false;
            for pc in 0..count {
                if !reachable[pc] {
                    continue;
                }
                let mut available = if pc == 0 {
                    vec![false; width]
                } else {
                    vec![true; width]
                };
                for previous in &predecessors[pc] {
                    for (present, incoming) in available.iter_mut().zip(&outputs[*previous]) {
                        *present &= incoming;
                    }
                }
                let mut after = available.clone();
                if let Some(destination) = self.operations[pc].destination() {
                    after[destination] = true;
                }
                if available != inputs[pc] || after != outputs[pc] {
                    inputs[pc] = available;
                    outputs[pc] = after;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for (pc, op) in self.operations.iter().enumerate() {
            if !reachable[pc] {
                continue;
            }
            for register in op.reads() {
                if !inputs[pc][register] {
                    return Err(error(
                        infos[pc].offset,
                        format!(
                            "instruction @{pc} reads register %{register} before it is definitely initialized on every reachable path"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        let mut body = Vec::new();
        put_u16(&mut body, self.registers.len());
        body.extend(self.registers.iter().map(|t| t.tag()));
        for operation in &self.operations {
            let (opcode, tag, payload) = operation.record();
            body.extend_from_slice(&[opcode, tag]);
            put_u16(&mut body, payload.len());
            body.extend(payload);
        }
        put_u16(&mut body, 0);
        let mut bytes = b"RVB\0\x02\0\0\0".to_vec();
        put_u16(&mut bytes, self.operations.len());
        bytes.extend_from_slice(&[3, 0]);
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend(body);
        bytes
    }

    pub(super) fn disassemble(&self) -> String {
        let mut text = String::from("rivet-ir v2 \"Binary compute program\"\n");
        for (row, types) in self.registers.chunks(4).enumerate() {
            let declarations = types
                .iter()
                .enumerate()
                .map(|(column, kind)| format!("%{}: {}", row * 4 + column, kind.name()))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(text, "registers {declarations}").unwrap();
        }
        text.push_str("entry compute @0\n");
        for (pc, op) in self.operations.iter().enumerate() {
            write!(text, "@{pc}: ").unwrap();
            match op {
                Op::Int(d, n) => write!(text, "%{d} = int {n}"),
                Op::Bool(d, b) => write!(text, "%{d} = bool {b}"),
                Op::Add(d, a, b) => write!(text, "%{d} = add %{a}, %{b}"),
                Op::Sub(d, a, b) => write!(text, "%{d} = sub %{a}, %{b}"),
                Op::Equal(d, a, b) => write!(text, "%{d} = eq %{a}, %{b}"),
                Op::Less(d, a, b) => write!(text, "%{d} = lt %{a}, %{b}"),
                Op::ReadInt(d) => write!(text, "%{d} = read_int"),
                Op::ReadArray(d) => write!(text, "%{d} = read_array"),
                Op::Length(d, a) => write!(text, "%{d} = array_len %{a}"),
                Op::Get(d, a, i) => write!(text, "%{d} = array_get %{a}, %{i}"),
                Op::Array(d, values) => write!(
                    text,
                    "%{d} = array_make [{}]",
                    values
                        .iter()
                        .map(|r| format!("%{r}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Op::Map(d) => write!(text, "%{d} = map_new"),
                Op::Lookup(d, m, k, f) => write!(text, "%{d} = map_get %{m}, %{k}, %{f}"),
                Op::Put(m, k, v) => write!(text, "map_put %{m}, %{k}, %{v}"),
                Op::Jump(t) => write!(text, "jump @{t}"),
                Op::Branch(c, yes, no) => write!(text, "branch %{c}, @{yes}, @{no}"),
                Op::Return(a) => write!(text, "return %{a}"),
            }
            .unwrap();
            text.push('\n');
        }
        text
    }

    pub(super) fn execute(&self, input: &str) -> Result<Execution, String> {
        if input.len() > MAX_INPUT {
            return Err("runtime input exceeds 262144 bytes".into());
        }
        let mut input = Input {
            bytes: input.as_bytes(),
            cursor: 0,
        };
        let mut registers = vec![Value::Empty; self.registers.len()];
        let mut arrays = Vec::new();
        let mut maps: Vec<HashMap<i64, i64>> = Vec::new();
        let mut allocated = 0;
        let mut pc = 0;
        for _ in 0..MAX_STEPS {
            let op = &self.operations[pc];
            let result: Result<Option<Execution>, String> = (|| {
                match op {
                    Op::Int(d, value) => registers[*d] = Value::Int(*value),
                    Op::Bool(d, value) => registers[*d] = Value::Bool(*value),
                    Op::Add(d, a, b) | Op::Sub(d, a, b) => {
                        let a = registers[*a].int();
                        let b = registers[*b].int();
                        let value = if matches!(op, Op::Add(..)) {
                            a.checked_add(b)
                        } else {
                            a.checked_sub(b)
                        };
                        registers[*d] = Value::Int(value.ok_or("signed 64-bit integer overflow")?);
                    }
                    Op::Equal(d, a, b) => {
                        registers[*d] = Value::Bool(registers[*a].int() == registers[*b].int())
                    }
                    Op::Less(d, a, b) => {
                        registers[*d] = Value::Bool(registers[*a].int() < registers[*b].int())
                    }
                    Op::ReadInt(d) => registers[*d] = Value::Int(input.integer()?),
                    Op::ReadArray(d) => {
                        let length = input.integer()?;
                        if !(0..=MAX_ARRAY as i64).contains(&length) {
                            return Err("array length must be between 0 and 10000".into());
                        }
                        charge(&mut allocated, length as usize * 8)?;
                        let mut values = Vec::with_capacity(length as usize);
                        for _ in 0..length {
                            values.push(input.integer()?);
                        }
                        let array = Rc::new(values);
                        arrays.push(Rc::clone(&array));
                        registers[*d] = Value::Array(array);
                    }
                    Op::Length(d, a) => {
                        registers[*d] = Value::Int(registers[*a].array().len() as i64)
                    }
                    Op::Get(d, a, i) => {
                        let index = registers[*i].int();
                        let value = usize::try_from(index)
                            .ok()
                            .and_then(|i| registers[*a].array().get(i))
                            .copied()
                            .ok_or("array index out of bounds")?;
                        registers[*d] = Value::Int(value);
                    }
                    Op::Array(d, values) => {
                        charge(&mut allocated, values.len() * 8)?;
                        let array = Rc::new(values.iter().map(|r| registers[*r].int()).collect());
                        arrays.push(Rc::clone(&array));
                        registers[*d] = Value::Array(array);
                    }
                    Op::Map(d) => {
                        charge(&mut allocated, MAP_ALLOCATION)?;
                        registers[*d] = Value::Map(maps.len());
                        maps.push(HashMap::new());
                    }
                    Op::Lookup(d, m, k, f) => {
                        let value = maps[registers[*m].map()]
                            .get(&registers[*k].int())
                            .copied()
                            .unwrap_or(registers[*f].int());
                        registers[*d] = Value::Int(value);
                    }
                    Op::Put(m, k, v) => {
                        let map = &mut maps[registers[*m].map()];
                        let key = registers[*k].int();
                        if map.len() >= MAX_ARRAY && !map.contains_key(&key) {
                            return Err("map exceeds 10000 unique keys".into());
                        }
                        map.insert(key, registers[*v].int());
                    }
                    Op::Jump(target) => {
                        pc = *target;
                        return Ok(None);
                    }
                    Op::Branch(condition, yes, no) => {
                        pc = if registers[*condition].boolean() {
                            *yes
                        } else {
                            *no
                        };
                        return Ok(None);
                    }
                    Op::Return(a) => {
                        input.finish()?;
                        let mut stdout = String::from("[");
                        for (index, value) in registers[*a].array().iter().enumerate() {
                            if index != 0 {
                                stdout.push(',');
                            }
                            write!(stdout, "{value}").unwrap();
                        }
                        stdout.push_str("]\n");
                        return Ok(Some(Execution::Console { stdout }));
                    }
                }
                pc += 1;
                Ok(None)
            })();
            match result.map_err(|message| format!("instruction @{pc}: {message}"))? {
                Some(result) => return Ok(result),
                None => continue,
            }
        }
        Err("execution exceeded 1000000 instructions".into())
    }
}

#[derive(Clone)]
enum Value {
    Empty,
    Int(i64),
    Bool(bool),
    Array(Rc<Vec<i64>>),
    Map(usize),
}
impl Value {
    fn int(&self) -> i64 {
        if let Self::Int(value) = self {
            *value
        } else {
            unreachable!("validated initialized Int")
        }
    }
    fn boolean(&self) -> bool {
        if let Self::Bool(value) = self {
            *value
        } else {
            unreachable!("validated initialized Bool")
        }
    }
    fn array(&self) -> &Vec<i64> {
        if let Self::Array(value) = self {
            value
        } else {
            unreachable!("validated initialized IntArray")
        }
    }
    fn map(&self) -> usize {
        if let Self::Map(value) = self {
            *value
        } else {
            unreachable!("validated initialized IntMap")
        }
    }
}
fn charge(used: &mut usize, amount: usize) -> Result<(), String> {
    if amount > MAX_ALLOCATION - *used {
        return Err("compute allocation budget exceeds 67108864 bytes".into());
    }
    *used += amount;
    Ok(())
}
struct Input<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl Input<'_> {
    fn space(byte: u8) -> bool {
        matches!(byte, b' ' | b'\t'..=b'\r')
    }
    fn skip(&mut self) {
        while self.cursor < self.bytes.len() && Self::space(self.bytes[self.cursor]) {
            self.cursor += 1;
        }
    }
    fn integer(&mut self) -> Result<i64, String> {
        self.skip();
        let start = self.cursor;
        while self.cursor < self.bytes.len() && !Self::space(self.bytes[self.cursor]) {
            self.cursor += 1;
        }
        let token = &self.bytes[start..self.cursor];
        if token.is_empty() {
            return Err("runtime input is missing an integer".into());
        }
        let digits = if token[0] == b'-' { &token[1..] } else { token };
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return Err("runtime input contains an invalid integer".into());
        }
        std::str::from_utf8(token)
            .expect("ASCII digits validated")
            .parse()
            .map_err(|_| "runtime input integer is outside signed 64-bit range".into())
    }
    fn finish(&mut self) -> Result<(), String> {
        self.skip();
        if self.cursor != self.bytes.len() {
            return Err("runtime input has unconsumed data".into());
        }
        Ok(())
    }
}
