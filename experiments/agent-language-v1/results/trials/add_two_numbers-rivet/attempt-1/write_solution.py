"""Encode the linked-list addition program as RVB2 instruction records.

This file only writes binary source and resolves symbolic instruction labels.
The emitted Rivet program performs all input traversal and arithmetic.
"""
from pathlib import Path
import struct

A, B, REVERSED, OUTPUT, CARRY, TOTAL, DIGIT, TEN, ZERO, EMPTY = range(10)
register_types = bytes([7, 7, 7, 7, 2, 2, 2, 2, 2, 3])
records = []
labels = {}


def mark(name):
    labels[name] = len(records)


def emit(opcode, result_type, *fields):
    records.append((opcode, result_type, fields))


def integer(destination, value):
    records.append((0x02, 2, struct.pack('<Hq', destination, value)))


# Read the actual input lists and initialize the two result chains.
emit(0x65, 7, A)
emit(0x65, 7, B)
emit(0x60, 7, REVERSED)
emit(0x60, 7, OUTPUT)
integer(CARRY, 0)
integer(TEN, 10)
integer(ZERO, 0)

# Continue while either input list or the carry still contributes a digit.
mark('loop')
emit(0x62, 3, EMPTY, A)
emit(0x51, 0, EMPTY, 'check_b', 'add_digits')
mark('check_b')
emit(0x62, 3, EMPTY, B)
emit(0x51, 0, EMPTY, 'check_carry', 'add_digits')
mark('check_carry')
emit(0x14, 3, EMPTY, CARRY, ZERO)
emit(0x51, 0, EMPTY, 'reverse_loop', 'add_digits')

mark('add_digits')
emit(0x16, 2, TOTAL, CARRY)
emit(0x62, 3, EMPTY, A)
emit(0x51, 0, EMPTY, 'add_b', 'consume_a')
mark('consume_a')
emit(0x63, 2, DIGIT, A)
emit(0x11, 2, TOTAL, TOTAL, DIGIT)
emit(0x64, 7, A, A)

mark('add_b')
emit(0x62, 3, EMPTY, B)
emit(0x51, 0, EMPTY, 'produce_digit', 'consume_b')
mark('consume_b')
emit(0x63, 2, DIGIT, B)
emit(0x11, 2, TOTAL, TOTAL, DIGIT)
emit(0x64, 7, B, B)

mark('produce_digit')
emit(0x18, 2, DIGIT, TOTAL, TEN)
emit(0x17, 2, CARRY, TOTAL, TEN)
emit(0x61, 7, REVERSED, DIGIT, REVERSED)
emit(0x50, 0, 'loop')

# Cons created digits in reverse order; a second list traversal restores it.
mark('reverse_loop')
emit(0x62, 3, EMPTY, REVERSED)
emit(0x51, 0, EMPTY, 'return', 'reverse_node')
mark('reverse_node')
emit(0x63, 2, DIGIT, REVERSED)
emit(0x61, 7, OUTPUT, DIGIT, OUTPUT)
emit(0x64, 7, REVERSED, REVERSED)
emit(0x50, 0, 'reverse_loop')

mark('return')
emit(0x53, 0, OUTPUT)

body = bytearray(struct.pack('<H', len(register_types)))
body.extend(register_types)
for opcode, result_type, fields in records:
    if isinstance(fields, bytes):
        payload = fields
    else:
        resolved = [labels[field] if isinstance(field, str) else field for field in fields]
        payload = struct.pack('<' + 'H' * len(resolved), *resolved)
    body.extend(struct.pack('<BBH', opcode, result_type, len(payload)))
    body.extend(payload)
body.extend(struct.pack('<H', 0))
header = struct.pack('<4sHHHBBI', b'RVB\x00', 2, 0, len(records), 3, 0, len(body))
Path(__file__).with_name('solution.rvb').write_bytes(header + body)
