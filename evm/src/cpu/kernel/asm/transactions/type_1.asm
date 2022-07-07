// Type 1 transactions, introduced by EIP 2930, have the format
// 0x01 || rlp([chain_id, nonce, gas_price, gas_limit, to, value, data, access_list, y_parity, r, s])
global process_type_1_txn:
    JUMPDEST
    // stack: (empty)
    PUSH return_from_parsing
    PUSH SEGMENT_TXN_DATA
    PUSH 0 // starting address is 0
    // stack: addr, segment, return_from_parsing
    PUSH read_rlp
    JUMP

return_from_parsing:
    JUMPDEST
    // stack: (empty)
    PANIC // TODO: Unfinished
