"""Encode the Two Sum program as RVB2 records, without executing it.

The generated program keeps earlier values and their indices in an IntMap.
Each iteration looks up target - value before inserting the current index.
"""

from pathlib import Path
import struct


INT, BOOL, ARRAY, MAP = 2, 3, 5, 6
NUMS, TARGET, SEEN, INDEX, ONE, MISSING, LENGTH = range(7)
VALUE, COMPLEMENT, PREVIOUS, IS_MISSING, IN_BOUNDS, RESULT = range(7, 13)
register_types = [ARRAY, INT, MAP, INT, INT, INT, INT,
                  INT, INT, INT, BOOL, BOOL, ARRAY]

records = []
labels = {}


def label(name):
    labels[name] = len(records)


def emit(opcode, result_type, *fields):
    records.append((opcode, result_type, fields))


def literal(destination, value):
    emit(0x02, INT, destination, struct.pack("<q", value))


emit(0x31, ARRAY, NUMS)
emit(0x30, INT, TARGET)
emit(0x40, MAP, SEEN)
literal(INDEX, 0)
literal(ONE, 1)
literal(MISSING, -1)
emit(0x32, INT, LENGTH, NUMS)

label("loop")
emit(0x15, BOOL, IN_BOUNDS, INDEX, LENGTH)
emit(0x51, 0, IN_BOUNDS, "inspect", "exhausted")

label("inspect")
emit(0x33, INT, VALUE, NUMS, INDEX)
emit(0x13, INT, COMPLEMENT, TARGET, VALUE)
emit(0x41, INT, PREVIOUS, SEEN, COMPLEMENT, MISSING)
emit(0x14, BOOL, IS_MISSING, PREVIOUS, MISSING)
emit(0x51, 0, IS_MISSING, "insert", "found")

label("insert")
emit(0x42, 0, SEEN, VALUE, INDEX)
emit(0x11, INT, INDEX, INDEX, ONE)
emit(0x50, 0, "loop")

label("found")
emit(0x34, ARRAY, RESULT, 2, PREVIOUS, INDEX)
emit(0x52, 0, RESULT)

# Domain-valid inputs always return from "found". Keep exhaustion well formed.
label("exhausted")
emit(0x34, ARRAY, RESULT, 0)
emit(0x52, 0, RESULT)

body = struct.pack("<H", len(register_types)) + bytes(register_types)
for opcode, result_type, fields in records:
    payload = b""
    for field in fields:
        if isinstance(field, bytes):
            payload += field
        else:
            numeric = labels[field] if isinstance(field, str) else field
            payload += struct.pack("<H", numeric)
    body += struct.pack("<BBH", opcode, result_type, len(payload)) + payload
body += struct.pack("<H", 0)
header = struct.pack("<4sHHHBBI", b"RVB\0", 2, 0, len(records), 3, 0, len(body))
Path(__file__).with_name("solution.rvb").write_bytes(header + body)
