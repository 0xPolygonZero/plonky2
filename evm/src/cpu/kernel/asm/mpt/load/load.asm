// Load all partial trie data from prover inputs.
global load_all_mpts:
    // stack: retdest
    // First set @GLOBAL_METADATA_TRIE_DATA_SIZE = 1.
    // We don't want it to start at 0, as we use 0 as a null pointer.
    PUSH 1
    %set_trie_data_size

    %load_mpt(mpt_load_state_trie_value)   %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    %load_mpt(mpt_load_txn_trie_value)     %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    %load_mpt(mpt_load_receipt_trie_value) %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)

    // stack: retdest
    JUMP

// Load an MPT from prover inputs.
// Pre stack: load_value, retdest
// Post stack: node_ptr
global load_mpt:
    // stack: load_value, retdest
    PROVER_INPUT(mpt)
    // stack: node_type, load_value, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(load_mpt_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(load_mpt_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(load_mpt_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(load_mpt_leaf)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(load_mpt_digest)
    PANIC // Invalid node type

load_mpt_empty:
    // TRIE_DATA[0] = 0, and an empty node has type 0, so we can simply return the null pointer.
    %stack (node_type, load_value, retdest) -> (retdest, 0)
    JUMP

load_mpt_branch:
    // stack: node_type, load_value, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, load_value, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, load_value, retdest
    // Save the offset of our 16 child pointers so we can write them later.
    // Then advance our current trie pointer beyond them, so we can load the
    // value and have it placed after our child pointers.
    %get_trie_data_size
    // stack: children_ptr, node_ptr, load_value, retdest
    DUP1 %add_const(17) // Skip over 16 children plus the value pointer
    // stack: end_of_branch_ptr, children_ptr, node_ptr, load_value, retdest
    DUP1 %set_trie_data_size
    // Now the top of the stack points to where the branch node will end and the
    // value will begin, if there is a value. But we need to ask the prover if a
    // value is present, and point to null if not.
    // stack: end_of_branch_ptr, children_ptr, node_ptr, load_value, retdest
    PROVER_INPUT(mpt)
    // stack: is_value_present, end_of_branch_ptr, children_ptr, node_ptr, load_value, retdest
    %jumpi(load_mpt_branch_value_present)
    // There is no value present, so value_ptr = null.
    %stack (end_of_branch_ptr) -> (0)
    // stack: value_ptr, children_ptr, node_ptr, load_value, retdest
    %jump(load_mpt_branch_after_load_value)
load_mpt_branch_value_present:
    // stack: value_ptr, children_ptr, node_ptr, load_value, retdest
    PUSH load_mpt_branch_after_load_value
    DUP5 // load_value
    JUMP
load_mpt_branch_after_load_value:
    // stack: value_ptr, children_ptr, node_ptr, load_value, retdest
    SWAP1
    // stack: children_ptr, value_ptr, node_ptr, load_value, retdest

    // Load the 16 children.
    %rep 16
        DUP4 // load_value
        %load_mpt
        // stack: child_ptr, next_child_ptr_ptr, value_ptr, node_ptr, load_value, retdest
        DUP2
        // stack: next_child_ptr_ptr, child_ptr, next_child_ptr_ptr, value_ptr, node_ptr, load_value, retdest
        %mstore_trie_data
        // stack: next_child_ptr_ptr, value_ptr, node_ptr, load_value, retdest
        %increment
        // stack: next_child_ptr_ptr, value_ptr, node_ptr, load_value, retdest
    %endrep

    // stack: value_ptr_ptr, value_ptr, node_ptr, load_value, retdest
    %mstore_trie_data
    %stack (node_ptr, load_value, retdest) -> (retdest, node_ptr)
    JUMP

load_mpt_extension:
    // stack: node_type, load_value, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, load_value, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, load_value, retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: node_ptr, load_value, retdest

    %get_trie_data_size
    // stack: child_ptr_ptr, node_ptr, load_value, retdest
    // Increment trie_data_size, to leave room for child_ptr_ptr, before we load our child.
    DUP1 %increment %set_trie_data_size
    %stack (child_ptr_ptr, node_ptr, load_value, retdest)
        -> (load_value, load_mpt_extension_after_load_mpt,
            child_ptr_ptr, retdest, node_ptr)
    %jump(load_mpt)
load_mpt_extension_after_load_mpt:
    // stack: child_ptr, child_ptr_ptr, retdest, node_ptr
    SWAP1 %mstore_trie_data
    // stack: retdest, node_ptr
    JUMP

load_mpt_leaf:
    // stack: node_type, load_value, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, load_value, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, load_value, retdest
    PROVER_INPUT(mpt) // read num_nibbles
    %append_to_trie_data
    PROVER_INPUT(mpt) // read packed_nibbles
    %append_to_trie_data
    // stack: node_ptr, load_value, retdest
    // We save value_ptr_ptr = get_trie_data_size, then increment trie_data_size
    // to skip over the slot for value_ptr_ptr. We will write to value_ptr_ptr
    // after the load_value call.
    %get_trie_data_size
    // stack: value_ptr_ptr, node_ptr, load_value, retdest
    DUP1 %increment
    // stack: value_ptr, value_ptr_ptr, node_ptr, load_value, retdest
    DUP1 %set_trie_data_size
    // stack: value_ptr, value_ptr_ptr, node_ptr, load_value, retdest
    %stack (value_ptr, value_ptr_ptr, node_ptr, load_value, retdest)
        -> (load_value, load_mpt_leaf_after_load_value,
            value_ptr_ptr, value_ptr, retdest, node_ptr)
    JUMP
load_mpt_leaf_after_load_value:
    // stack: value_ptr_ptr, value_ptr, retdest, node_ptr
    %mstore_trie_data
    // stack: retdest, node_ptr
    JUMP

load_mpt_digest:
    // stack: node_type, load_value, retdest
    %get_trie_data_size
    // stack: node_ptr, node_type, load_value, retdest
    SWAP1 %append_to_trie_data
    // stack: node_ptr, load_value, retdest
    PROVER_INPUT(mpt) // read digest
    %append_to_trie_data
    %stack (node_ptr, load_value, retdest) -> (retdest, node_ptr)
    JUMP

// Convenience macro to call load_mpt and return where we left off.
// Pre stack: load_value
// Post stack: node_ptr
%macro load_mpt
    %stack (load_value) -> (load_value, %%after)
    %jump(load_mpt)
%%after:
%endmacro

// Convenience macro to call load_mpt and return where we left off.
// Pre stack: (empty)
// Post stack: node_ptr
%macro load_mpt(load_value)
    PUSH %%after
    PUSH $load_value
    %jump(load_mpt)
%%after:
%endmacro
