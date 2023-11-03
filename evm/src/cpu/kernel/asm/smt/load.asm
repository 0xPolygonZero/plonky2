%macro load_state_smt
    PUSH %%after %jump(load_state_smt)
%%after:
%endmacro

// Simply copy the serialized state SMT to `TrieData`.
// First entry is the length of the serialized data.
global load_state_smt:
    // stack: retdest
    PROVER_INPUT(smt::state)
    // stack: len, retdest
    %get_trie_data_size
    // stack: i, len, retdest
    DUP2 %mstore_global_metadata(@GLOBAL_METADATA_TRIE_DATA_SIZE)
    // stack: i, len, retdest
    DUP1 %add_const(2) // First two entries are [0,0] for an empty hash node.
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: i, len, retdest
    %stack (i, len) -> (len, i, i)
    ADD SWAP1
    // stack: i, len, retdest
loop:
    // stack: i, len, retdest
    DUP2 DUP2 EQ %jumpi(loop_end)
    // stack: i, len, retdest
    PROVER_INPUT(smt::state)
    DUP2
    // stack: i, x, i, len, retdest
    %mstore_trie_data
    // stack: i, len, retdest
    %increment
    %jump(loop)
loop_end:
    // stack: i, len, retdest
    %pop2 JUMP
