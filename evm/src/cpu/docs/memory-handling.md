# Memory structure
The CPU interacts with the EVM memory via memory channels. At each CPU row, a memory channel can execute a write, a read, or be disabled. A full memory channel is composed of:
- 1 `used` flag. If it's set to `true`, a memory operation is executed in this channel at this row. If it's set to `false`, no operation is done but its columns might be reused for other purposes.
- 1 `is_read` flag. It indicates if a memory operation is a read or a write.
- 3 address columns. A memory address is made of three parts: `context`, `segment` and `virtual`.
- 8 value columns. EVM words are 256 bits long, and they are broken down in 8 32-bit limbs.
The last memory channel is `current_row.partial_channel`: it doesn't have its own column values and shares them with the first memory channel `current_row.mem_channels[0]`. This allows use to save eight columns.

# Top of the stack
The majority of memory operations involve the stack. The stack is a segment in memory, and stack operations (popping or pushing) use the memory channels. Every CPU instruction performs between 0 and 4 pops, and may push at most once. However, for efficiency purposes, we hold the top of the stack in `current_row.mem_channels[0]`, only writing it in memory if necessary.

## Motivation
See https://github.com/0xPolygonZero/plonky2/issues/1149.

## Top reading and writing
When a CPU instruction modifies the stack, it must update the top of the stack accordingly. There are three cases.

- **The instruction pops and pushes:** The new top of the stack is stored in `next_row.mem_channels[0]`; it may be computed by the instruction, or it could be read from memory. In either case, the instruction is responsible for setting `next_row.mem_channels[0]`'s flags and address columns correctly. After use, the previous top of the stack is discarded and doesn't need to be written in memory.

- **The instruction pushes, but doesn't pop:** The new top of the stack is stored in `next_row.mem_channels[0]`; it may be computed by the instruction, or it could be read from memory. In either case, the instruction is responsible for setting `next_row.mem_channels[0]`'s flags and address columns correctly. If the stack wasn't empty (`current_row.stack_len > 0`), the current top of the stack is stored with a memory read in `current_row.partial_channel`, which shares its values with `current_row.mem_channels[0]` (which holds the current top of the stack). If the stack was empty, `current_row.partial_channel` is disabled.

- **The instruction pops, but doesn't push:** After use, the current top of the stack is discarded and doesn't need to be written in memory. If the stack isn't now empty (`current_row.stack_len > num_pops`), the new top of the stack is set in `next_row.mem_channels[0]` with a memory read from the stack segment. If the stack is now empty, `next_row.mem_channels[0]` is disabled.

In the last two cases, there is an edge case if `current_row.stack_len` is equal to a `special_len`. For a strictly pushing instruction, this happens if the stack is empty, and `special_len = 0`. For a strictly popping instruction, this happens if the next stack is empty, i.e. that all remaining elements are popped, and `special_len = num_pops`. Note that we do not need to check for values below `num_pops`, since this would be a stack underflow exception which is handled separately.
The edge case is detected with the compound flag `1 - not_special_len * stack_inv_aux`, where `not_special_len = current_row - special_len` and `stack_inv_aux` is constrained to be the modular inverse of `is_not_special_len` if it's non-zero, or `0` otherwise. The flag is `1` if `stack_len` is equal to `special_len`, and `0` otherwise.

This logic can be found in code in the `eval_packed_one` function of `stack.rs`, which multiplies all of the constraints with a degree 1 filter passed as argument.

## Operation flag merging
To reduce the total number of columns, many operation flags are merged together (e.g. DUP and SWAP) and are distinguished with the binary decomposition of their opcodes. The filter for a merged operation is now of degree 2: for example, `is_swap = current_row.op.dup_swap * current_row.opcode_bits[4]` since the 4th bit is set to 1 for a SWAP and 0 for a DUP. If the two instructions have different stack behaviors, this can be a problem: `eval_packed_one`'s degrees are already of degree 3 and it can't support degree 2 filters.

When this happens, stack constraints are defined manually in the operation's dedicated file (e.g. `dup_swap.rs`). Implementation details vary case-by-case and can be found in the files.
