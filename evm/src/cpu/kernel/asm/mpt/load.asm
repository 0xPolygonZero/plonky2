// TODO: Receipt trie leaves are variable-length, so we need to be careful not
// to permit buffer over-reads.

// Load all partial trie data from prover inputs.
global load_all_mpts:
    // stack: retdest
    // First set @GLOBAL_METADATA_TRIE_DATA_SIZE = 1.
    // We don't want it to start at 0, as we use 0 as a null pointer.
    PUSH 1
    %set_trie_data_size

    %load_mpt %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    %load_mpt %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    %load_mpt %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)

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
    %load_mpt
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
// Post stack: node_ptr
load_mpt:
    // stack: retdest
    PROVER_INPUT(mpt)
    // stack: node_type, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(load_mpt_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(load_mpt_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(load_mpt_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(load_mpt_leaf)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(load_mpt_digest)
    PANIC // Invalid node type

load_mpt_empty:
    // TRIE_DATA[0] = 0, and an empty node has type 0, so we can simply return the null pointer.
    %stack (node_type, retdest) -> (retdest, 0)
    JUMP

load_mpt_branch:
    // stack: node_type, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, retdest
    // Save the offset of our 16 child pointers so we can write them later.
    // Then advance our current trie pointer beyond them, so we can load the
    // value and have it placed after our child pointers.
    %get_trie_data_size
    // stack: children_ptr, node_ptr, retdest
    DUP1 %add_const(17) // Skip over 16 children plus the value pointer
    // stack: value_ptr, children_ptr, node_ptr, retdest
    %set_trie_data_size
    // stack: children_ptr, node_ptr, retdest
    %load_value
    SWAP1
    // stack: children_ptr, value_ptr, node_ptr, retdest

    // Load the 16 children.
    %rep 16
        %load_mpt
        // stack: child_ptr, next_child_ptr_ptr, value_ptr, node_ptr, retdest
        DUP2
        // stack: next_child_ptr_ptr, child_ptr, next_child_ptr_ptr, value_ptr, node_ptr, retdest
        %mstore_trie_data
        // stack: next_child_ptr_ptr, value_ptr, node_ptr, retdest
        %increment
        // stack: next_child_ptr_ptr, value_ptr, node_ptr, retdest
    %endrep

    // stack: value_ptr_ptr, value_ptr, node_ptr, retdest
    %mstore_trie_data
    // stack: node_ptr, retdest
    SWAP1
    JUMP

load_mpt_extension:
    // stack: node_type, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: node_ptr, retdest

    %get_trie_data_size
    // stack: child_ptr_ptr, node_ptr, retdest
    // Increment trie_data_size, to leave room for child_ptr_ptr, before we load our child.
    DUP1 %increment %set_trie_data_size
    // stack: child_ptr_ptr, node_ptr, retdest

    %load_mpt
    // stack: child_ptr, child_ptr_ptr, node_ptr, retdest
    SWAP1
    %mstore_trie_data
    // stack: node_ptr, retdest
    SWAP1
    JUMP

load_mpt_leaf:
    // stack: node_type, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: node_ptr, retdest
    // We save value_ptr_ptr = get_trie_data_size, then increment trie_data_size
    // to skip over the slot for value_ptr. We will write value_ptr after the
    // load_value call.
    %get_trie_data_size
    // stack: value_ptr_ptr, node_ptr, retdest
    DUP1 %increment %set_trie_data_size
    // stack: value_ptr_ptr, node_ptr, retdest
    %load_value
    // stack: value_ptr, value_ptr_ptr, node_ptr, retdest
    SWAP1 %mstore_trie_data
    // stack: node_ptr, retdest
    SWAP1
    JUMP

load_mpt_digest:
    // stack: node_type, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, retdest
    PROVER_INPUT(mpt) // read digest
    %append_to_trie_data
    // stack: node_ptr, retdest
    SWAP1
    JUMP

// Convenience macro to call load_mpt and return where we left off.
%macro load_mpt
    PUSH %%after
    %jump(load_mpt)
%%after:
%endmacro

// Load a value from prover input, append it to trie data, and return a pointer to it.
// Return null if the value is empty.
%macro load_value
    // stack: (empty)
    PROVER_INPUT(mpt)
    // stack: value_len
    DUP1 %jumpi(%%has_value)
    %stack (value_len) -> (0)
    %jump(%%end)
%%has_value:
    // stack: value_len
    %get_trie_data_size
    // stack: value_ptr, value_len
    SWAP1
    // stack: value_len, value_ptr
%%loop:
    DUP1 ISZERO
    // stack: value_len == 0, value_len, value_ptr
    %jumpi(%%finish_loop)
    // stack: value_len, value_ptr
    PROVER_INPUT(mpt)
    // stack: value_part, value_len, value_ptr
    %append_to_trie_data
    // stack: value_len, value_ptr
    %decrement
    // stack: value_len', value_ptr
    %jump(%%loop)
%%finish_loop:
    // stack: value_len, value_ptr
    POP
    // stack: value_ptr
%%end:
%endmacro
