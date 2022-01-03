# Plonky2

Plonky2 is an implementation of recursive arguments based on Plonk and FRI. It uses FRI to check systems of polynomial constraints, similar to the DEEP-ALI method described in the [DEEP-FRI](https://arxiv.org/abs/1903.12243) paper. It is the successor of [plonky](https://github.com/mir-protocol/plonky), which was based on Plonk and Halo.

Plonky2 is largely focused on recursion performance. We use custom gates to mitigate the bottlenecks of FRI verification, such as hashing and interpolation. We also encode witness data in a ~64 bit field, so field operations take just a few cycles. To achieve 128-bit security, we repeat certain checks, and run certain parts of the argument in an extension field.


## Running

To see recursion performance, one can run this test, which generates a chain of three recursion proofs:

```sh
RUST_LOG=debug RUSTFLAGS=-Ctarget-cpu=native cargo test --release test_recursive_recursive_verifier -- --ignored
```


## Copyright

Plonky2 was developed by Polygon Zero (formerly Mir). While we plan to adopt an open source license, we haven't selected one yet, so all rights are reserved for the time being. Please reach out to us if you have thoughts on licensing.


## Disclaimer

This code has not been thoroughly reviewed or tested, and should not be used in any production systems.

