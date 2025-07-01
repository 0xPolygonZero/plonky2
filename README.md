# Plonky2 & more
[![Discord](https://img.shields.io/discord/743511677072572486?logo=discord)](https://discord.gg/QZKRUpqCJ6)

This repository was originally for Plonky2, a SNARK implementation based on techniques from PLONK and FRI. It has since expanded to include tools such as Starky, a highly performant STARK implementation.

## ⚠️ Plonky2 Deprecation Notice

Plonky2 is being deprecated and will no longer receive updates or support.

Please consider using **[Plonky3](https://github.com/Plonky3/Plonky3)** instead, Polygon's next-generation ZK proving system.

## Documentation

For more details about the Plonky2 argument system, see this [writeup](plonky2/plonky2.pdf).

Polymer Labs has written up a helpful tutorial [here](https://polymerlabs.medium.com/a-tutorial-on-writing-zk-proofs-with-plonky2-part-i-be5812f6b798)!


## Examples

A good starting point for how to use Plonky2 for simple applications is the included examples:

* [`factorial`](plonky2/examples/factorial.rs): Proving knowledge of 100!
* [`fibonacci`](plonky2/examples/fibonacci.rs): Proving knowledge of the hundredth Fibonacci number
* [`range_check`](plonky2/examples/range_check.rs): Proving that a field element is in a given range
* [`square_root`](plonky2/examples/square_root.rs): Proving knowledge of the square root of a given field element

To run an example, use

```sh
cargo run --example <example_name>
```


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

Plonky2 prefers the [Jemalloc](http://jemalloc.net) memory allocator due to its superior performance. To use it, include `jemallocator = "0.5.0"` in your `Cargo.toml` and add the following lines
to your `main.rs`:

```rust
use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
```

Jemalloc is known to cause crashes when a binary compiled for x86 is run on an Apple silicon-based Mac under [Rosetta 2](https://support.apple.com/en-us/HT211861). If you are experiencing crashes on your Apple silicon Mac, run `rustc --print target-libdir`. The output should contain `aarch64-apple-darwin`. If the output contains `x86_64-apple-darwin`, then you are running the Rust toolchain for x86; we recommend switching to the native ARM version.

## Documentation

Generate documentation locally:

```sh
cargo doc --no-deps --open
```

## Contributing guidelines

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## Licenses

All crates of this monorepo are licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## Security

This code has been audited prior to the `v1.0.0` release. The audits reports and findings are available in the [audits](./audits/) folder of this repository.
An audited codebase isn't necessarily free of bugs and security exploits, hence we recommend care when using `plonky2` in production settings.

If you find a security issue in the codebase, please refer to our [Security guidelines](./SECURITY.md) for private disclosure.

While Plonky2 is configurable, its defaults generally target 100 bits of security. The default FRI configuration targets 100 bits of *conjectured* security based on the conjecture in [ethSTARK](https://eprint.iacr.org/2021/582).

Plonky2's default hash function is Poseidon, configured with 8 full rounds, 22 partial rounds, a width of 12 field elements (each ~64 bits), and an S-box of `x^7`. [BBLP22](https://tosc.iacr.org/index.php/ToSC/article/view/9850) suggests that this configuration may have around 95 bits of security, falling a bit short of our 100 bit target.


## Links

- [Polygon Zero's zkEVM](https://github.com/0xPolygonZero/zk_evm), an efficient Type 1 zkEVM built on top of Starky and plonky2
- [System Zero](https://github.com/0xPolygonZero/system-zero), a zkVM built on top of Starky
- [Waksman](https://github.com/0xPolygonZero/plonky2-waksman), Plonky2 gadgets for permutation checking using Waksman networks
- [Insertion](https://github.com/0xPolygonZero/plonky2-insertion), Plonky2 gadgets for insertion into a list
- [u32](https://github.com/0xPolygonZero/plonky2-u32), Plonky2 gadgets for u32 arithmetic
- [ECDSA](https://github.com/0xPolygonZero/plonky2-ecdsa), Plonky2 gadgets for the ECDSA algorithm
