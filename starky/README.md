# Starky

Starky is a FRI-based STARK implementation.

It is built for speed, features highly efficient recursive verification through `plonky2` circuits and gadgets, and is
being used as backend proving system for the Polygon Zero Type-1 zkEVM.

## Note on Zero-Knowledgeness

While STARKs can be made Zero-Knowledge, the primary purpose of `starky` is to provide fast STARK proof generation. As such,
ZK is disabled by default on `starky`. Applications requiring their proof to be `zero-knowledge` would need to apply a
recursive wrapper on top of their STARK proof with the `zero_knowledge` parameter activated in their `CircuitConfig`.
See `plonky2` documentation for more info.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
