#!/usr/bin/env python3
"""Author the committed RVB1/RVB2 source bytes directly. No JSON/text source compiler.

The output .rvb files are the program source; this optional reproducibility tool
writes opcodes, type tags, register operands, and payload bytes per BINARY-FORMAT.
"""
from pathlib import Path
from struct import pack

ROOT = Path(__file__).resolve().parents[1]


def record(opcode, result_type, payload):
    return pack('<BBH', opcode, result_type, len(payload)) + payload


def text(value):
    return record(0x01, 1, value.encode('utf-8'))


def integer(value):
    return record(0x02, 2, pack('<q', value))


def obj(*fields):
    payload = pack('<H', len(fields))
    for key, register in sorted(fields):
        key = key.encode('utf-8')
        payload += pack('<H', len(key)) + key + pack('<H', register)
    return record(0x20, 4, payload)


def program(records, entry_tag, entry):
    body = b''.join(records) + entry
    return b'RVB\0' + pack('<HHHBBI', 1, 0, len(records), entry_tag, 0, len(body)) + body


def http(port, path, status, body_register):
    path = path.encode('ascii')
    return pack('<HHH', port, 1, len(path)) + path + pack('<HH', status, body_register)


def two_sum():
    # RVB2 registers: nums, target, seen, index, length, one, missing,
    # in_bounds, value, complement, previous_index, absent, answer.
    types = bytes([5, 2, 6, 2, 2, 2, 2, 3, 2, 2, 2, 3, 5])

    def op(code, result, *operands):
        return record(code, result, pack('<' + 'H' * len(operands), *operands))

    def const(destination, value):
        return record(0x02, 2, pack('<Hq', destination, value))

    # Program control flow and all algorithm operations are these binary records.
    # The helper only packs bytes; it never solves a test case or reads input.
    records = [
        op(0x31, 5, 0),             # 00 nums = read_array()
        op(0x30, 2, 1),             # 01 target = read_int()
        op(0x40, 6, 2),             # 02 seen = map_new()
        const(3, 0),                # 03 index = 0
        op(0x32, 2, 4, 0),          # 04 length = array_len(nums)
        const(5, 1),                # 05 one = 1
        const(6, -1),               # 06 missing = -1
        op(0x15, 3, 7, 3, 4),       # 07 in_bounds = index < length
        op(0x51, 0, 7, 9, 19),      # 08 branch to loop body or no-pair result
        op(0x33, 2, 8, 0, 3),       # 09 value = nums[index]
        op(0x13, 2, 9, 1, 8),       # 10 complement = target - value
        op(0x41, 2, 10, 2, 9, 6),   # 11 previous_index = seen.get(complement, -1)
        op(0x14, 3, 11, 10, 6),     # 12 absent = previous_index == -1
        op(0x51, 0, 11, 14, 17),    # 13 branch to insertion or found pair
        op(0x42, 0, 2, 8, 3),       # 14 seen[value] = index (after lookup)
        op(0x11, 2, 3, 3, 5),       # 15 index = index + 1
        op(0x50, 0, 7),             # 16 repeat loop
        op(0x34, 5, 12, 2, 10, 3),  # 17 answer = [previous_index, index]
        op(0x52, 0, 12),            # 18 return answer
        op(0x34, 5, 12, 0),         # 19 answer = [] (outside problem guarantee)
        op(0x52, 0, 12),            # 20 return answer
    ]
    body = pack('<H', len(types)) + types + b''.join(records) + pack('<H', 0)
    return b'RVB\0' + pack('<HHHBBI', 2, 0, len(records), 3, 0, len(body)) + body


def add_two_numbers():
    # Real linked-node handles: left, right, reversed digits, result, and next.
    # Other registers hold carry/constants/lengths/comparisons/current digits.
    types = bytes([7, 7, 7, 7, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 7, 2])
    records, labels, fixups = [], {}, []

    def mark(name):
        labels[name] = len(records)

    def op(code, result, *operands):
        payload = bytearray()
        for operand in operands:
            if isinstance(operand, str):
                fixups.append((len(records), len(payload), operand))
                operand = 0
            payload += pack('<H', operand)
        records.append((code, result, payload))

    def const(destination, value):
        records.append((0x02, 2, bytearray(pack('<Hq', destination, value))))

    op(0x65, 7, 0)  # Read two generic count-prefixed linked lists.
    op(0x65, 7, 1)
    op(0x60, 7, 2)
    op(0x60, 7, 3)
    for destination, value in [(4, 0), (5, 0), (6, 10), (7, 1), (8, -1), (9, 101)]:
        const(destination, value)
    op(0x66, 2, 10, 0)
    op(0x66, 2, 11, 1)
    for length in [10, 11]:
        op(0x15, 3, 12, 5, length)  # 0 < length < 101.
        op(0x54, 0, 12)
        op(0x15, 3, 12, length, 9)
        op(0x54, 0, 12)

    mark('loop')
    op(0x62, 3, 12, 0)
    op(0x51, 0, 12, 'test_right', 'digit')
    mark('test_right')
    op(0x62, 3, 12, 1)
    op(0x51, 0, 12, 'test_carry', 'digit')
    mark('test_carry')
    op(0x14, 3, 12, 4, 5)
    op(0x51, 0, 12, 'reverse', 'digit')
    mark('digit')
    op(0x16, 2, 13, 4)  # sum = carry.
    for name, cursor, length in [('left', 0, 10), ('right', 1, 11)]:
        op(0x62, 3, 12, cursor)
        op(0x51, 0, 12, f'{name}_done', f'{name}_value')
        mark(f'{name}_value')
        op(0x63, 2, 16, cursor)
        op(0x15, 3, 12, 8, 16)  # Assert digit >= 0 and < 10 in Rivet.
        op(0x54, 0, 12)
        op(0x15, 3, 12, 16, 6)
        op(0x54, 0, 12)
        op(0x64, 7, 15, cursor)
        op(0x62, 3, 12, 15)
        op(0x51, 0, 12, f'{name}_last', f'{name}_advance')
        mark(f'{name}_last')
        op(0x15, 3, 12, 7, length)
        op(0x51, 0, 12, f'{name}_nonzero', f'{name}_advance')
        mark(f'{name}_nonzero')
        op(0x15, 3, 12, 5, 16)  # Multi-node input's final digit cannot be zero.
        op(0x54, 0, 12)
        mark(f'{name}_advance')
        op(0x11, 2, 13, 13, 16)
        op(0x16, 7, cursor, 15)
        mark(f'{name}_done')
    op(0x18, 2, 14, 13, 6)  # digit = sum % 10.
    op(0x17, 2, 4, 13, 6)   # carry = sum / 10.
    op(0x61, 7, 2, 14, 2)   # Prepend to temporary result.
    op(0x50, 0, 'loop')

    # Reverse the temporary immutable chain using Rivet node operations.
    mark('reverse')
    op(0x62, 3, 12, 2)
    op(0x51, 0, 12, 'return', 'reverse_node')
    mark('reverse_node')
    op(0x63, 2, 16, 2)
    op(0x61, 7, 3, 16, 3)
    op(0x64, 7, 2, 2)
    op(0x50, 0, 'reverse')
    mark('return')
    op(0x53, 0, 3)

    # Resolve writer-local labels to numeric PCs; no names enter the source file.
    for index, offset, label in fixups:
        records[index][2][offset:offset + 2] = pack('<H', labels[label])
    body = pack('<H', len(types)) + types
    body += b''.join(record(code, result, payload) for code, result, payload in records)
    body += pack('<H', 0)
    return b'RVB\0' + pack('<HHHBBI', 2, 0, len(records), 3, 0, len(body)) + body


def median_two_sorted_arrays():
    # Array handles 0..2, integer search state 3..22/24..25, Bool 23/31,
    # Float results 26..30. Labels exist only in this byte-writing tool.
    types = bytes([5, 5, 5] + [2] * 20 + [3, 2, 2] + [8] * 5 + [3])
    records, labels, fixups = [], {}, []

    def mark(name):
        labels[name] = len(records)

    def op(code, result, *operands):
        payload = bytearray()
        for operand in operands:
            if isinstance(operand, str):
                fixups.append((len(records), len(payload), operand))
                operand = 0
            payload += pack('<H', operand)
        records.append((code, result, payload))

    def const(destination, value):
        records.append((0x02, 2, bytearray(pack('<Hq', destination, value))))

    op(0x31, 5, 0)
    op(0x31, 5, 1)
    op(0x32, 2, 3, 0)
    op(0x32, 2, 4, 1)
    for destination, value in [(6, 0), (7, 1), (8, 2), (9, 1001),
                               (20, -(2**63)), (21, 2**63 - 1)]:
        const(destination, value)
    records.append((0x04, 8, bytearray(pack('<Hd', 29, 2.0))))
    records.append((0x03, 3, bytearray(pack('<HB', 31, 0))))
    for length in [3, 4]:
        op(0x15, 3, 23, length, 9)
        op(0x54, 0, 23)
    op(0x11, 2, 14, 3, 4)
    op(0x15, 3, 23, 6, 14)
    op(0x54, 0, 23)
    op(0x15, 3, 23, 4, 3)
    op(0x51, 0, 23, 'swap', 'setup')
    mark('swap')
    for destination, source in [(2, 0), (0, 1), (1, 2)]:
        op(0x16, 5, destination, source)
    for destination, source in [(5, 3), (3, 4), (4, 5)]:
        op(0x16, 2, destination, source)
    mark('setup')
    op(0x16, 2, 10, 6)
    op(0x16, 2, 11, 3)
    op(0x11, 2, 15, 14, 7)
    op(0x17, 2, 15, 15, 8)
    mark('search')
    op(0x15, 3, 23, 11, 10)
    op(0x51, 0, 23, 'invalid', 'partition')
    mark('partition')
    op(0x11, 2, 12, 10, 11)
    op(0x17, 2, 12, 12, 8)
    op(0x13, 2, 13, 15, 12)
    for destination, source in [(16, 20), (17, 21), (18, 20), (19, 21)]:
        op(0x16, 2, destination, source)
    # Guard all four edge loads; empty arrays and endpoint cuts never index out.
    for label, array, cut, edge, bound, left in [
        ('al', 0, 12, 16, 6, True), ('ar', 0, 12, 17, 3, False),
        ('bl', 1, 13, 18, 6, True), ('br', 1, 13, 19, 4, False),
    ]:
        op(0x14, 3, 23, cut, bound)
        op(0x51, 0, 23, label + '_done', label + '_load')
        mark(label + '_load')
        if left:
            op(0x13, 2, 22, cut, 7)
            op(0x33, 2, edge, array, 22)
        else:
            op(0x33, 2, edge, array, cut)
        mark(label + '_done')
    op(0x15, 3, 23, 19, 16)
    op(0x51, 0, 23, 'move_left', 'check_right')
    mark('move_left')
    op(0x13, 2, 11, 12, 7)
    op(0x50, 0, 'search')
    mark('check_right')
    op(0x15, 3, 23, 17, 18)
    op(0x51, 0, 23, 'move_right', 'found')
    mark('move_right')
    op(0x11, 2, 10, 12, 7)
    op(0x50, 0, 'search')
    mark('found')
    op(0x16, 2, 24, 16)
    op(0x15, 3, 23, 16, 18)
    op(0x51, 0, 23, 'left_b', 'parity')
    mark('left_b')
    op(0x16, 2, 24, 18)
    mark('parity')
    op(0x19, 8, 26, 24)
    op(0x18, 2, 22, 14, 8)
    op(0x14, 3, 23, 22, 6)
    op(0x51, 0, 23, 'even', 'odd')
    mark('odd')
    op(0x55, 0, 26)
    mark('even')
    op(0x16, 2, 25, 17)
    op(0x15, 3, 23, 19, 17)
    op(0x51, 0, 23, 'right_b', 'mean')
    mark('right_b')
    op(0x16, 2, 25, 19)
    mark('mean')
    op(0x19, 8, 27, 25)
    op(0x1a, 8, 28, 26, 27)
    op(0x1b, 8, 30, 28, 29)
    op(0x55, 0, 30)
    mark('invalid')
    op(0x54, 0, 31)
    op(0x50, 0, 'invalid')  # Satisfy structural CFG even after failing assertion.

    for index, offset, label in fixups:
        records[index][2][offset:offset + 2] = pack('<H', labels[label])
    body = pack('<H', len(types)) + types
    body += b''.join(record(code, result, payload) for code, result, payload in records)
    body += pack('<H', 0)
    return b'RVB\0' + pack('<HHHBBI', 2, 0, len(records), 3, 0, len(body)) + body


def main():
    # Hand-authored source bytes: three records (text, text, concat), console r2.
    hello = bytes.fromhex(
        '52 56 42 00 01 00 00 00 03 00 01 00 21 00 00 00 '
        '01 01 07 00 48 65 6c 6c 6f 2c 20 '
        '01 01 06 00 77 6f 72 6c 64 21 '
        '10 01 06 00 02 00 00 00 01 00 '
        '02 00'
    )
    server = program([text('hello world'), obj(('message', 0))], 2, http(8787, '/hello', 200, 1))
    computed = program([
        integer(40), integer(2), record(0x11, 2, pack('<HH', 0, 1)),
        record(0x12, 1, pack('<H', 2)), text('The answer is '),
        record(0x10, 1, pack('<HHH', 2, 4, 3)), record(0x03, 3, b'\x01'),
        obj(('answer', 2), ('ready', 6)), obj(('details', 7), ('message', 5)),
    ], 2, http(0, '/computed', 200, 8))
    for name, source in [('hello', hello), ('server', server), ('computed', computed),
                         ('two-sum', two_sum()), ('add-two-numbers', add_two_numbers()),
                         ('median-two-sorted-arrays', median_two_sorted_arrays())]:
        path = ROOT / 'examples' / f'{name}.rvb'
        path.write_bytes(source)
        print(f'{path.relative_to(ROOT)}: {len(source)} binary source bytes')


if __name__ == '__main__':
    main()
