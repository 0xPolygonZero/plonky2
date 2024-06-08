# Plonky2

Plonky2 is a SNARK implementation based on techniques from PLONK and FRI. It is the successor of [Plonky](https://github.com/0xPolygonZero/plonky), which was based on PLONK and Halo.

Plonky2 is built for speed, and features a highly efficient recursive circuit. On a Macbook Pro, recursive proofs can be generated in about 170 ms.


## Note on `1.0.0` versions

Starting from `v1.0.0`, the `plonky2_field` and `plonky2` crates use a *different* generator for the two-adic subgroup of the Goldilocks field.
As such, any proof generated with a previous `plonky2` version would be *unverifiable* with newer versions of this crate starting from `v1.0.0`.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
