#!/usr/bin/env python3
"""Author the committed RVB1 source bytes directly. No JSON/text source compiler.

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
    for name, source in [('hello', hello), ('server', server), ('computed', computed)]:
        path = ROOT / 'examples' / f'{name}.rvb'
        path.write_bytes(source)
        print(f'{path.relative_to(ROOT)}: {len(source)} binary source bytes')


if __name__ == '__main__':
    main()
