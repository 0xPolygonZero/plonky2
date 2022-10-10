global mpt_insert_extension:
    // stack: node_type, node_payload_ptr, insert_len, insert_key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, insert_len, insert_key, value_ptr, retdest
    PANIC // TODO
