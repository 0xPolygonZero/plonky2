// Reads RLP data from the prover input, and writes a more friendly encoding of
// that data to the given memory segment.

// In this friendly representation, a list is encoded with one memory cell for
// its length, followed by one cell per item in the list. If an item is a
// bytestring, it's directly encoded as a u256 in memory. If an item is a
// sublist, we instead encode an address in memory where that sublist's
// encoding can be found.

// This representation doesn't let us distinguish bytestrings from lists, but
// for our purposes we always know which type to expect, e.g. an account is
// always represented as a list of four bytestrings. It also doesn't let us
// encode bytestrings with >256 bytes, but those don't come up in our setting.

global read_rlp:
    JUMPDEST
    // stack: addr, segment, retdest
    %jump_if_input_le(0x7f, prefix_00_to_7f)
    %jump_if_input_le(0xb7, prefix_80_to_b7)
    %jump_if_input_le(0xbf, prefix_b8_to_bf)
    %jump_if_input_le(0xf7, prefix_c0_to_f7)
    %jump prefix_f8_to_ff

// Peak at the next input. If it's within the given bound, jump to the given
// destination.
%macro jump_if_input_le(bound, dest)
    // stack: ...
    PUSH $bound
    // stack: $bound, ...
    PEAK_INPUT
    // stack: first_byte, $bound, ...
    LE
    // stack: first_byte <= $bound, ...
    PUSH $dest
    JUMPI
%endmacro

// For a single byte whose value is in the [0x00, 0x7f] range, that byte is
// its own RLP encoding.
prefix_00_to_7f:
    JUMPDEST
    // stack: addr, segment, retdest
    INPUT
    // stack: first_byte, addr, segment, retdest
    SWAP2
    // stack: segment, addr, first_byte, retdest
    MSTORE_GENERAL
    // stack: retdest
    JUMP

// If a string is 0-55 bytes long, the RLP encoding consists of a single byte
// with value 0x80 plus the length of the string followed by the string. The
// range of the first byte is thus [0x80, 0xb7].
prefix_80_to_b7:
    JUMPDEST
    // stack: addr, segment, retdest
    PUSH prefix_80_to_b7_continue
    // stack: prefix_80_to_b7_continue, addr, segment, retdest
    INPUT
    // stack: 0x80 + len, prefix_80_to_b7_continue, addr, segment, retdest
    %sub_const(0x80)
    // stack: len, prefix_80_to_b7_continue, addr, segment, retdest
    %jump read_int_given_len
prefix_80_to_b7_continue:
    JUMPDEST
    // stack: int, addr, segment, retdest
    DUP1
    // stack: int, int, addr, segment, retdest
    %assert_gt_const(0x7f) // If <= 0x7f, it should have been a single byte.
    SWAP2
    // stack: segment, addr, int, retdest
    MSTORE_GENERAL
    // stack: retdest
    JUMP

// If a string is more than 55 bytes long, the RLP encoding consists of a single
// byte with value 0xb7 plus the length in bytes of the length of the string in
// binary form, followed by the length of the string, followed by the string.
// The range of the first byte is thus [0xb8, 0xbf].
prefix_b8_to_bf:
    JUMPDEST
    // stack: addr, segment, retdest
    // We don't support strings >32 bytes. They shouldn't arise in our setting.
    PANIC

// If the total payload of a list is 0-55 bytes long, the RLP encoding consists
// of a single byte with value 0xc0 plus the length of the list followed by the
// concatenation of the RLP encodings of the items. The range of the first byte
// is thus [0xc0, 0xf7].
prefix_c0_to_f7:
    JUMPDEST
    // stack: addr, segment, retdest
    INPUT
    // stack: 0xc0 + len, addr, segment, retdest
    %sub_const(0xc0)
    // stack: len, addr, segment, retdest
    %jump read_list_given_len

// If the total payload of a list is more than 55 bytes long, the RLP encoding
// consists of a single byte with value 0xf7 plus the length in bytes of the
// length of the payload in binary form, followed by the length of the payload,
// followed by the concatenation of the RLP encodings of the items. The range of
// the first byte is thus [0xf8, 0xff].
prefix_f8_to_ff:
    JUMPDEST
    // stack: addr, segment, retdest
    INPUT
    // stack: 0xf7 + len_of_len, addr, segment, retdest
    %sub_const(0xf7)
    // stack: len_of_len, addr, segment, retdest
    PUSH prefix_f8_to_ff_continue
    SWAP1
    // stack: len_of_len, prefix_f8_to_ff_continue, addr, segment, retdest
    %jump read_int_given_len
prefix_f8_to_ff_continue:
    JUMPDEST
    // stack: len, addr, segment, retdest
    DUP1
    %assert_gt(55) // A list < 55 bytes should have used the shorter encoding.
    // stack: len, addr, segment, retdest
    %jump read_list_given_len

// Given the length of an integer in bytes, reads that integer from the prover
// input tape, and returns it on the stack.
read_int_given_len:
    JUMPDEST
    // stack: len, retdest
    PUSH 0 // We start with an accumulator of 0.
    // stack: acc, len, retdest
read_int_given_len_loop:
    JUMPDEST
    // stack: acc, len, retdest
    DUP2
    ISZERO
    // stack: len == 0, acc, len, retdest
    %jumpi read_int_given_len_finish
    // stack: acc, len, retdest
    // Update our accumulator: acc' = 256 * acc + next_byte
    %mul_const(256)
    INPUT
    ADD
    // stack: acc', len, retdest
    // Update our length: len' = len - 1
    SWAP1
    %sub_const(1)
    SWAP1
    // stack: acc', len', retdest
    %jump read_int_given_len_loop
read_int_given_len_finish:
    JUMPDEST
    // stack: acc, len, retdest
    SWAP1
    // stack: len, acc, retdest
    POP
    // stack: acc, retdest
    SWAP1
    // stack: retdest, acc
    JUMP

// Given the length of a RLP list, reads that list from the prover input tape,
// and writes it to the given memory location.
read_list_given_len:
    JUMPDEST
    // stack: len, addr, segment, retdest
    // First, we store the length by copying (segment, addr, len) to the top of
    // the stack and executing MSTORE_GENERAL.
    DUP3
    DUP3
    DUP3
    // stack: segment, addr, len, len, addr, segment, retdest
    MSTORE_GENERAL
    // stack: len, addr, segment, retdest
    // Next, we increment addr.
    SWAP1
    %add_const(1)
    SWAP1
    // stack: len, addr, segment, retdest
read_list_given_len_loop:
    JUMPDEST
    // stack: len, addr, segment, retdest
    DUP1
    ISZERO
    // stack: len == 0, len, addr, segment, retdest
    %jumpi read_list_given_len_finish
    // stack: len, addr, segment, retdest
    // Decrement len.
    %sub_const(1)
    // Read the next item in the list.
    PUSH read_list_given_len_loop
    // stack: read_list_given_len_loop, len, addr, segment, retdest
    DUP4
    // stack: segment, read_list_given_len_loop, len, addr, segment, retdest
    DUP4
    // stack: addr, segment, read_list_given_len_loop, len, addr, segment, retdest
    %jump read_rlp
read_list_given_len_finish:
    JUMPDEST
    // stack: len, addr, segment, retdest
    %pop3
    // stack: retdest
    JUMP
