// Load all partial trie data from prover inputs.
global mpt_load_all:
    // First set GLOBAL_METADATA_TRIE_DATA_SIZE = 1.
    // We don't want it to start at 0, as we use 0 as a null pointer.
    PUSH 1
    %mstore(@GLOBAL_METADATA_TRIE_DATA_SIZE)

    TODO

mpt_load_state:
    PROVER_INPUT(mpt::state)
    TODO
