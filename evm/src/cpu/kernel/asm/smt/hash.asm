%macro smt_hash_state
    PUSH %%after %jump(smt_hash_state)
%%after:
%endmacro

global smt_hash_state:
    // stack: retdest
    PUSH 0 %mstore_kernel_general(13371337) // is_storage flag. TODO: Use a constant instead.
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
global smt_hash:
    // stack: ptr, retdest
    DUP1
    %mload_trie_data
    // stack: node, node_ptr, retdest
    DUP1 %eq_const(@SMT_NODE_HASH) %jumpi(smt_hash_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL) %jumpi(smt_hash_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF) %jumpi(smt_hash_leaf)
    PANIC

global smt_hash_hash:
    // stack: node, node_ptr, retdest
    POP
    // stack: node_ptr, retdest
    %increment
    // stack: node_ptr+1, retdest
    %mload_trie_data
    // stack: hash, retdest
    SWAP1 JUMP

global smt_hash_internal:
    // stack: node, node_ptr, retdest
    POP
    // stack: node_ptr, retdest
    %increment
    // stack: node_ptr+1, retdest
    DUP1
    %mload_trie_data
    %stack (left_child_ptr, node_ptr_plus_1, retdest) -> (left_child_ptr, smt_hash_internal_after_left, node_ptr_plus_1, retdest)
    %jump(smt_hash)
global smt_hash_internal_after_left:
    // stack: left_hash, node_ptr+1, retdest
    SWAP1 %increment
    // stack: node_ptr+2, left_hash, retdest
    %mload_trie_data
    %stack (right_child_ptr, left_hash, retdest) -> (right_child_ptr, smt_hash_internal_after_right, left_hash, retdest)
    %jump(smt_hash)
global smt_hash_internal_after_right:
    // stack: right_hash, left_hash, retdest
    %stack (right_hash) -> (0, @SEGMENT_KERNEL_GENERAL, 33, right_hash, 32)
    %mstore_unpacking POP
    %stack (left_hash) -> (0, @SEGMENT_KERNEL_GENERAL, 1, left_hash, 32)
    %mstore_unpacking POP
    // stack: retdest
    PUSH 1 %mstore_kernel_general(0)
    %stack () -> (0, @SEGMENT_KERNEL_GENERAL, 0, 65)
    KECCAK_GENERAL
    // stack: hash, retdest
    SWAP1 JUMP

global smt_hash_leaf:
    // stack: node, node_ptr, retdest
    POP
    // stack: node_ptr, retdest
    %increment
    // stack: node_ptr+1, retdest
    %mload_trie_data
    // stack: payload_ptr, retdest
    %mload_kernel_general(13371337)
    // stack: is_value, payload_ptr, retdest
    %jumpi(smt_hash_leaf_value)
global smt_hash_leaf_account:
    // stack: payload_ptr, retdest
    DUP1 %mload_trie_data
    // stack: key, payload_ptr, retdest
    SWAP1 %increment
    // stack: payload_ptr+1, key, retdest
    DUP1 %mload_trie_data
    // stack: nonce, payload_ptr+1, key, retdest
    SWAP1
    // stack: payload_ptr+1, nonce, key, retdest
    %increment
    // stack: payload_ptr+2, nonce, key, retdest
    DUP1 %mload_trie_data
    // stack: balance, payload_ptr+2, nonce, key, retdest
    SWAP1
    // stack: payload_ptr+2, balance, nonce, key, retdest
    %increment
    // stack: payload_ptr+3, balance, nonce, key, retdest
    DUP1 %mload_trie_data
    // stack: storage_root, payload_ptr+3, balance, nonce, key, retdest
    PUSH 1 %mstore_kernel_general(13371337)
    %stack (storage_root) -> (storage_root, smt_hash_leaf_account_after_storage)
    %jump(smt_hash)
global smt_hash_leaf_account_after_storage:
    PUSH 0 %mstore_kernel_general(13371337)
    // stack: storage_root_hash, payload_ptr+3, balance, nonce, key, retdest
    SWAP1
    // stack: payload_ptr+3, storage_root_hash, balance, nonce, key, retdest
    %increment
    // stack: payload_ptr+4, storage_root_hash, balance, nonce, key, retdest
    %mload_trie_data
    // stack: code_hash, storage_root_hash, balance, nonce, key, retdest

    // 0    | 1----32 | 33----40 | 41----72 | 73-------104 | 105----136
    // flag |   key   |  nonce   | balance  | storage_root | code_hash

    // 0----7 | 8----39 | 40--------71 | 72----103
    // nonce  | balance | storage_root | code_hash

    %stack (code_hash) -> (0, @SEGMENT_KERNEL_GENERAL, 72, code_hash, 32)
    %mstore_unpacking POP

    %stack (storage_root) -> (0, @SEGMENT_KERNEL_GENERAL, 40, storage_root, 32)
    %mstore_unpacking POP

    %stack (balance) -> (0, @SEGMENT_KERNEL_GENERAL, 8, balance, 32)
    %mstore_unpacking POP

    %stack (nonce) -> (0, @SEGMENT_KERNEL_GENERAL, 0, nonce)
    %mstore_unpacking_u64_LE

    // stack: key, retdest
    %stack () -> (0, @SEGMENT_KERNEL_GENERAL, 0, 104)
    KECCAK_GENERAL
    // stack: hash, key, retdest

    // flag
    PUSH 0 %mstore_kernel_general(0)

    %stack (hash) -> (0, @SEGMENT_KERNEL_GENERAL, 33, hash, 32)
    %mstore_unpacking POP

    %stack (key) -> (0, @SEGMENT_KERNEL_GENERAL, 1, key, 32)
    %mstore_unpacking POP

    %stack () -> (0, @SEGMENT_KERNEL_GENERAL, 0, 65)
    KECCAK_GENERAL

    SWAP1 JUMP

global smt_hash_leaf_value:
    // stack: payload_ptr, retdest
    DUP1 %mload_trie_data
    // stack: key, payload_ptr, retdest
    SWAP1
    // stack: payload_ptr, key, retdest
    %increment
    // stack: payload_ptr+1, key, retdest
    %mload_trie_data
    // stack: value, key, retdest
    PUSH 0 %mstore_kernel_general(0)
    %stack (value) -> (0, @SEGMENT_KERNEL_GENERAL, 33, value, 32)
    %mstore_unpacking POP
    // stack: key, retdest
    %stack (key) -> (0, @SEGMENT_KERNEL_GENERAL, 1, key, 32)
    %mstore_unpacking POP
    // stack: retdest
    %stack () -> (0, @SEGMENT_KERNEL_GENERAL, 0, 65)
    KECCAK_GENERAL
    // stack: hash, retdest
    SWAP1 JUMP
