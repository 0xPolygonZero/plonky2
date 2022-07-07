// Type 0 transactions, aka legacy transaction, have the format
// rlp([nonce, gas_price, gas_limit, to, value, data, v, r, s])
// The field v was originally encoded as
//     27 + y_parity
// but as of EIP 155 it can also be encoded as
//     35 + 2 * CHAIN_ID + y_parity
global process_type_0_txn:
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
