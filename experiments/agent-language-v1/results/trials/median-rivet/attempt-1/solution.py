"""Encode the median solver as RVB2 instructions; this file never runs it."""
import struct
from pathlib import Path

# Every tuple is one numeric instruction record. Labels resolve only branch PCs.
records = []
labels = {}
types = [5, 5] + [2] * 22 + [3, 8, 8]


def mark(name):
    labels[name] = len(records)


def emit(opcode, result_tag, *fields):
    records.append((opcode, result_tag, fields))


def integer(destination, value):
    emit(0x02, 2, destination, value)


def copy(destination, source):
    emit(0x16, types[destination], destination, source)


def add(destination, left, right):
    emit(0x11, 2, destination, left, right)


def subtract(destination, left, right):
    emit(0x13, 2, destination, left, right)


def less(left, right):
    emit(0x15, 3, TEST, left, right)


def branch(yes, no):
    emit(0x51, 0, TEST, yes, no)


def jump(target):
    emit(0x50, 0, target)


# Register IDs, separate from instruction PCs.
A, B = 0, 1
M, N, TOTAL, HALF = 2, 3, 4, 5
LOW, HIGH, I, J, INDEX = 6, 7, 8, 9, 10
ALEFT, ARIGHT, BLEFT, BRIGHT = 11, 12, 13, 14
LEFT, RIGHT, SUM, PARITY = 15, 16, 17, 18
ZERO, ONE, TWO, NEGATIVE_BOUND, POSITIVE_BOUND = 19, 20, 21, 22, 23
TEST, RESULT, FLOAT_TWO = 24, 25, 26

emit(0x31, 5, A)
emit(0x31, 5, B)
emit(0x32, 2, M, A)
emit(0x32, 2, N, B)
integer(ZERO, 0)
integer(ONE, 1)
integer(TWO, 2)
integer(NEGATIVE_BOUND, -1000001)
integer(POSITIVE_BOUND, 1000001)
emit(0x04, 8, FLOAT_TWO, 2.0)
add(TOTAL, M, N)
add(HALF, TOTAL, ONE)
emit(0x17, 2, HALF, HALF, TWO)

# Search i in [max(0, HALF-N), min(M, HALF)], so j=HALF-i
# always lies in [0,N]. Neither array needs copying or swapping.
subtract(LOW, HALF, N)
less(LOW, ZERO)
branch('clamp_low', 'choose_high')
mark('clamp_low')
copy(LOW, ZERO)
mark('choose_high')
copy(HIGH, M)
less(HALF, M)
branch('clamp_high', 'partition')
mark('clamp_high')
copy(HIGH, HALF)

mark('partition')
add(SUM, LOW, HIGH)
emit(0x17, 2, I, SUM, TWO)
subtract(J, HALF, I)
copy(ALEFT, NEGATIVE_BOUND)
copy(ARIGHT, POSITIVE_BOUND)
copy(BLEFT, NEGATIVE_BOUND)
copy(BRIGHT, POSITIVE_BOUND)

# Out-of-array sides use strict domain-external sentinels. Every actual
# array access is reached only after its index bound has been established.
less(ZERO, I)
branch('load_a_left', 'check_a_right')
mark('load_a_left')
subtract(INDEX, I, ONE)
emit(0x33, 2, ALEFT, A, INDEX)
mark('check_a_right')
less(I, M)
branch('load_a_right', 'check_b_left')
mark('load_a_right')
emit(0x33, 2, ARIGHT, A, I)
mark('check_b_left')
less(ZERO, J)
branch('load_b_left', 'check_b_right')
mark('load_b_left')
subtract(INDEX, J, ONE)
emit(0x33, 2, BLEFT, B, INDEX)
mark('check_b_right')
less(J, N)
branch('load_b_right', 'compare_partitions')
mark('load_b_right')
emit(0x33, 2, BRIGHT, B, J)

mark('compare_partitions')
less(BRIGHT, ALEFT)
branch('move_left', 'check_move_right')
mark('move_left')
subtract(HIGH, I, ONE)
jump('partition')
mark('check_move_right')
less(ARIGHT, BLEFT)
branch('move_right', 'choose_left')
mark('move_right')
add(LOW, I, ONE)
jump('partition')

# A valid partition has HALF values on its left. The odd median is the
# largest left value; the even median averages it with the smallest right.
mark('choose_left')
copy(LEFT, ALEFT)
less(ALEFT, BLEFT)
branch('use_b_left', 'check_parity')
mark('use_b_left')
copy(LEFT, BLEFT)
mark('check_parity')
emit(0x18, 2, PARITY, TOTAL, TWO)
emit(0x14, 3, TEST, PARITY, ONE)
branch('odd_result', 'choose_right')
mark('odd_result')
emit(0x19, 8, RESULT, LEFT)
emit(0x55, 0, RESULT)
mark('choose_right')
copy(RIGHT, ARIGHT)
less(BRIGHT, ARIGHT)
branch('use_b_right', 'even_result')
mark('use_b_right')
copy(RIGHT, BRIGHT)
mark('even_result')
add(SUM, LEFT, RIGHT)
emit(0x19, 8, RESULT, SUM)
emit(0x1B, 8, RESULT, RESULT, FLOAT_TWO)
emit(0x55, 0, RESULT)

# Encoding only: no algorithm execution, input parsing, or validation runs.
body = bytearray(struct.pack('<H', len(types)) + bytes(types))
for opcode, result_tag, fields in records:
    if opcode == 0x02:
        payload = struct.pack('<Hq', *fields)
    elif opcode == 0x04:
        payload = struct.pack('<Hd', *fields)
    else:
        numeric_fields = [labels[value] if isinstance(value, str) else value
                          for value in fields]
        payload = struct.pack('<' + 'H' * len(numeric_fields), *numeric_fields)
    body.extend(struct.pack('<BBH', opcode, result_tag, len(payload)))
    body.extend(payload)
body.extend(struct.pack('<H', 0))
header = struct.pack('<4sHHHBBI', b'RVB\0', 2, 0, len(records), 3, 0, len(body))
Path(__file__).with_name('solution.rvb').write_bytes(header + body)
