# plonky2

plonky2 is an implementation of recursive arguments based on Plonk and FRI. It uses FRI to check systems of polynomial constraints, similar to the DEEP-ALI method described in the [DEEP-FRI](https://arxiv.org/abs/1903.12243) paper. It is the successor of [plonky](https://github.com/mir-protocol/plonky), which was based on Plonk and Halo.

plonky2 is largely focused on recursion performance. We use custom gates to mitigate the bottlenecks of FRI verification, such as hashing and interpolation. We also encode witness data in a ~64 bit field, so field operations take just a few cycles. To achieve 128-bit security, we repeat certain checks, and run certain parts of the argument in an extension field.


## Running

To run the recursion benchmark,

```sh
RUSTFLAGS=-Ctarget-cpu=native cargo run --release
```


## Disclaimer

This code has not been thoroughly reviewed or tested, and should not be used in any production systems.


## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
