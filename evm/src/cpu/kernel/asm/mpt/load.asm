// TODO: Receipt trie leaves are variable-length, so we need to be careful not
// to permit buffer over-reads.

// Load all partial trie data from prover inputs.
global load_all_mpts:
    // stack: retdest
    // First set @GLOBAL_METADATA_TRIE_DATA_SIZE = 1.
    // We don't want it to start at 0, as we use 0 as a null pointer.
    PUSH 1
    %set_trie_data_size

    %load_mpt_and_return_root_ptr %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    %load_mpt_and_return_root_ptr %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    %load_mpt_and_return_root_ptr %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)

    PROVER_INPUT(mpt)
    // stack: num_storage_tries, retdest
    DUP1 %mstore_global_metadata(@GLOBAL_METADATA_NUM_STORAGE_TRIES)
    // stack: num_storage_tries, retdest
    PUSH 0 // i = 0
    // stack: i, num_storage_tries, retdest
storage_trie_loop:
    DUP2 DUP2 EQ
    // stack: i == num_storage_tries, i, num_storage_tries, retdest
    %jumpi(storage_trie_loop_end)
    // stack: i, num_storage_tries, retdest
    PROVER_INPUT(mpt)
    // stack: storage_trie_addr, i, num_storage_tries, retdest
    DUP2
    // stack: i, storage_trie_addr, i, num_storage_tries, retdest
    %mstore_kernel(@SEGMENT_STORAGE_TRIE_ADDRS)
    // stack: i, num_storage_tries, retdest
    %load_mpt_and_return_root_ptr
    // stack: root_ptr, i, num_storage_tries, retdest
    DUP2
    // stack: i, root_ptr, i, num_storage_tries, retdest
    %mstore_kernel(@SEGMENT_STORAGE_TRIE_PTRS)
    // stack: i, num_storage_tries, retdest
    %jump(storage_trie_loop)
storage_trie_loop_end:
    // stack: i, num_storage_tries, retdest
    %pop2
    // stack: retdest
    JUMP

// Load an MPT from prover inputs.
// Pre stack: retdest
// Post stack: (empty)
load_mpt:
    // stack: retdest
    PROVER_INPUT(mpt)
    // stack: node_type, retdest
    DUP1 %append_to_trie_data
    // stack: node_type, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(load_mpt_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(load_mpt_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(load_mpt_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(load_mpt_leaf)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(load_mpt_digest)
    PANIC // Invalid node type

load_mpt_empty:
    // stack: node_type, retdest
    POP
    // stack: retdest
    JUMP

load_mpt_branch:
    // stack: node_type, retdest
    POP
    // stack: retdest
    // Save the offset of our 16 child pointers so we can write them later.
    // Then advance out current trie pointer beyond them, so we can load the
    // value and have it placed after our child pointers.
    %get_trie_data_size
    // stack: ptr_children, retdest
    DUP1 %add_const(16)
    // stack: ptr_leaf, ptr_children, retdest
    %set_trie_data_size
    // stack: ptr_children, retdest
    %load_leaf_value

    // Load the 16 children.
    %rep 16
        %load_mpt_and_return_root_ptr
        // stack: child_ptr, ptr_next_child, retdest
        DUP2
        // stack: ptr_next_child, child_ptr, ptr_next_child, retdest
        %mstore_trie_data
        // stack: ptr_next_child, retdest
        %increment
        // stack: ptr_next_child, retdest
    %endrep

    // stack: ptr_next_child, retdest
    POP
    JUMP

load_mpt_extension:
    // stack: node_type, retdest
    POP
    // stack: retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: retdest

    // Let i be the current trie data size. We still need to expand this node by
    // one element, appending our child pointer. Thus our child node will start
    // at i + 1. So we will set our child pointer to i + 1.
    %get_trie_data_size
    %increment
    %append_to_trie_data
    // stack: retdest

    %load_mpt
    // stack: retdest
    JUMP

load_mpt_leaf:
    // stack: node_type, retdest
    POP
    // stack: retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: retdest
    %load_leaf_value
    // stack: retdest
    JUMP

load_mpt_digest:
    // stack: node_type, retdest
    POP
    // stack: retdest
    PROVER_INPUT(mpt) // read digest
    %append_to_trie_data
    // stack: retdest
    JUMP

// Convenience macro to call load_mpt and return where we left off.
%macro load_mpt
    PUSH %%after
    %jump(load_mpt)
%%after:
%endmacro

%macro load_mpt_and_return_root_ptr
    // stack: (empty)
    %get_trie_data_size
    // stack: ptr
    %load_mpt
    // stack: ptr
%endmacro

// Load a leaf from prover input, and append it to trie data.
%macro load_leaf_value
    // stack: (empty)
    PROVER_INPUT(mpt)
    // stack: leaf_len
    DUP1 %append_to_trie_data
    // stack: leaf_len
%%loop:
    DUP1 ISZERO
    // stack: leaf_len == 0, leaf_len
    %jumpi(%%finish)
    // stack: leaf_len
    PROVER_INPUT(mpt)
    // stack: leaf_part, leaf_len
    %append_to_trie_data
    // stack: leaf_len
    %decrement
    // stack: leaf_len'
    %jump(%%loop)
%%finish:
    POP
    // stack: (empty)
%endmacro
