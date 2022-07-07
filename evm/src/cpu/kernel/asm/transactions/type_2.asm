// Type 2 transactions, introduced by EIP 1559, have the format
// 0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list, y_parity, r, s])
global process_type_2_txn:
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
