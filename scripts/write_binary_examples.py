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
                         ('two-sum', two_sum())]:
        path = ROOT / 'examples' / f'{name}.rvb'
        path.write_bytes(source)
        print(f'{path.relative_to(ROOT)}: {len(source)} binary source bytes')


if __name__ == '__main__':
    main()
