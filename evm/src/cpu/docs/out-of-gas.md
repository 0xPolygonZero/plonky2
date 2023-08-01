# Out of Gas Errors

The CPU table has a `gas` register that keeps track of the gas used by the transaction so far.

The crucial invariant in our out-of-gas checking method is that at any point in the program's execution, we have not used more gas than we have available; that is `gas` is at most the gas allocation for the transaction (which is stored separately by the kernel). We assume that the gas allocation will never be 2^32 or more, so if `gas` does not fit in one limb, then we've run out of gas.

When a native instruction (one that is not a syscall) is executed, a constraint ensures that the `gas` register is increased by the correct amount. This is not automatic for syscalls; the syscall handler itself must calculate and charge the appropriate amount.

If everything goes smoothly and we have not run out of gas, `gas` should be no more than the gas allowance at the point that we `STOP`, `REVERT`, stack overflow, or whatever. Indeed, because we assume that the gas overflow handler is invoked _as soon as_ we've run out of gas, all these termination methods must verify that `gas` <= allowance, and `PANIC` if this is not the case. This is also true for the out-of-gas handler, which should check that (a) we have not yet run out of gas and (b) we are about to run out of gas, `PANIC`king if either of those does not hold.

When we do run out of gas, however, this event must be handled. Syscalls are responsible for checking that their execution would not cause the transaction to run out of gas. If the syscall detects that it would need to charge more gas than available, it must abort the transaction by jumping to `exc_out_of_gas`, which in turn verifies that the out-of-gas hasn't _already_ occured.

Native instructions do this differently. If the prover notices that execution of the instruction would cause an out-of-gas error, it must jump to the appropriate handler instead of executing the instruction. (The handler contains special code that `PANIC`s if the prover invoked it incorrectly.)

## Overflow

We must be careful to ensure that `gas` does not overflow to prevent denial of service attacks.

Note that a syscall cannot be the instruction that causes an overflow. This is because every syscall is required to verify that its execution does not cause us to exceed the gas limit. Upon entry into a syscall, a constraint verifies that `gas` < 2^32. Some syscalls may have to be careful to ensure that the gas check is performed correctly (for example, that overflow modulo 2^256 does not occur). So we can assume that upon entry and exit out of a syscall, `gas` < 2^32.

Similarly, native instructions alone cannot cause wraparound. The most expensive instruction, `JUMPI`, costs 10 gas. Even if we were to execute 2^32 consecutive `JUMPI` instructions, the maximum length of a trace, we are nowhere close to consuming 2^64 - 2^32 + 1 (= Golilocks prime) gas.

The final scenario we must tackle is an expensive syscall followed by many expensive native instructions. Upon exit from a syscall, `gas` < 2^32. Again, even if that syscall is followed by 2^32 native instructions of cost 10, we do not see wraparound modulo Goldilocks.
