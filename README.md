# Plonky2 & more

This repository was originally for Plonky2, a SNARK implementation based on techniques from PLONK and FRI. It has since expanded to include tools such as Starky, a highly performant STARK implementation.


## Documentation

For more details about the Plonky2 argument system, see this [writeup](plonky2/plonky2.pdf).


## Building

Plonky2 requires a recent nightly toolchain, although we plan to transition to stable in the future.

To use a nightly toolchain for Plonky2 by default, you can run
```
rustup override set nightly
```
in the Plonky2 directory.


## Running

To see recursion performance, one can run this bench, which generates a chain of three recursion proofs:

```sh
RUSTFLAGS=-Ctarget-cpu=native cargo run --release --example bench_recursion -- -vv
```

## Jemalloc

Plonky2 prefers the [Jemalloc](http://jemalloc.net) memory allocator due to its superior performance. To use it, include `jemallocator = "0.5.0"` in`Cargo.toml`and add the following lines
to your `main.rs`:

```rust
use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
```

Jemalloc is known to cause crashes when a binary compiled for x86 is run on an Apple silicon-based Mac under [Rosetta 2](https://support.apple.com/en-us/HT211861). If you are experiencing crashes on your Apple silicon Mac, run `rustc --print target-libdir`. The output should contain `aarch64-apple-darwin`. If the output contains `x86_64-apple-darwin`, then you are running the Rust toolchain for x86; we recommend switching to the native ARM version.


## Licenses

As this is a monorepo, see the individual crates within for license information.


## Security

This code has not yet been audited, and should not be used in any production systems.

While Plonky2 is configurable, its defaults generally target 100 bits of security. The default FRI configuration targets 100 bits of *conjectured* security based on the conjecture in [ethSTARK](https://eprint.iacr.org/2021/582).

Plonky2's default hash function is Poseidon, configured with 8 full rounds, 22 partial rounds, a width of 12 field elements (each ~64 bits), and an S-box of `x^7`. [BBLP22](https://tosc.iacr.org/index.php/ToSC/article/view/9850) suggests that this configuration may have around 95 bits of security, falling a bit short of our 100 bit target.

