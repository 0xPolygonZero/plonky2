# Plonky2

Plonky2 is a SNARK implementation based on techniques from PLONK and FRI. It is the successor of [Plonky](https://github.com/mir-protocol/plonky), which was based on PLONK and Halo.

Plonky2 is built for speed, and features a highly efficient recursive circuit. On a Macbook Pro, recursive proofs can be generated in about 170 ms.


## Documentation

For more details about the Plonky2 argument system, see this [writeup](plonky2.pdf).


## Building

Plonky2 requires a recent nightly toolchain, although we plan to transition to stable in the future.

To use a nightly toolchain for Plonky2 by default, you can run
```
rustup override set nightly
```
in the Plonky2 directory.


## Running

To see recursion performance, one can run this test, which generates a chain of three recursion proofs:

```sh
RUST_LOG=debug RUSTFLAGS=-Ctarget-cpu=native cargo test --release test_recursive_recursive_verifier -- --ignored
```


## Jemalloc

By default, Plonky2 uses the [Jemalloc](http://jemalloc.net) memory allocator due to its superior performance. Currently, it changes the default allocator of any binary to which it is linked. You can disable this behavior by removing the corresponding lines in [`plonky2/src/lib.rs`](https://github.com/mir-protocol/plonky2/blob/main/plonky2/src/lib.rs).

Jemalloc is known to cause crashes when a binary compiled for x86 is run on an Apple silicon-based Mac under [Rosetta 2](https://support.apple.com/en-us/HT211861). If you are experiencing crashes on your Apple silicon Mac, run `rustc --print target-libdir`. The output should contain `aarch64-apple-darwin`. If the output contains `x86_64-apple-darwin`, then you are running the Rust toolchain for x86; we recommend switching to the native ARM version.


## Copyright

Plonky2 was developed by Polygon Zero (formerly Mir). While we plan to adopt an open source license, we haven't selected one yet, so all rights are reserved for the time being. Please reach out to us if you have thoughts on licensing.


## Disclaimer

This code has not yet been audited, and should not be used in any production systems.

