# generic_recursion

**Version:** 0.1.0


`generic_recursion` is a crate that allows to easily aggregate an unlimited amount of plonky2 proofs,
generated with a circuit belong to a specific set of circuits, in a single recursive proof,
which can be verified with the same verifier data independently from the number of proofs being
aggregated.

The main component of the crate is the `AggregationScheme` data structure, which implements the
`RecursiveCircuit` trait. This data structure already provides all the methods necessary to
aggregate an unlimited number of plonky2 proofs generated with a set of circuits specified as
input by the user, which will be henceforth referred to as _base_circuits_.

All the base circuits in the set specified by the user are required to employ the same format
for their public inputs, and the `AggregationScheme` needs to know such a format.
To specify the public input format of the base circuits, and the information about them which
are needed by the `AggregationScheme` in order to compute the public inputs of the aggregated
proof from the public inputs of the proofs to be aggregated, this crate introduces the
`PublicInputAggregation` trait. The crate already provides implementations of this trait
for several public input formats, which can be found in the `shared_state` module.

## Tests

Tests can be run with:
```sh
cargo test --release
```
## Usage

An `AggregationScheme` can be instantiated with the method `build_circuit`, which requires the
user to provide the set of circuits that define which proofs can be aggregated with the
instantiated `AggregationScheme`. Refer to section [How To Specify the Set of Circuits](#how-to-specify-the-set-of-circuits)
to learn how to specify such set of circuits.

Once an `AggregationScheme` is instantiated, the user can start providing proofs to be
aggregated, which must be generated with a circuit belonging to the set specified when
instantiating the `AggregationScheme`.
Before being aggregated, each proof must be preprocessed by invoking the
`prepare_base_proof_for_aggregation` method, which yields a `PreparedProof`;
the methods of `AggregationScheme` that recursively aggregate proofs accept as input only
`PreparedProof`s.
Once a proof is converted to a `PreparedProof`, the user can add it to the set of proofs to be
aggregated with the `add_proofs_for_aggregation` method; the final aggregated proof is
then computed by invoking the `aggregate_proofs_with` method, where the user can also provide
other `PreparedProof`s to be aggregated that have not been previously added to the set of
proofs to be aggregated.

For a real example on how to use the `AggregationScheme`, users can refer to the integration
test `test_recursive_aggregation` found in `tests/integration.rs`

### How To Specify the Set of Circuits
To specify the set of circuits that define which proofs can be aggregated the user must
implement the `BaseCircuitInfo` trait for the data structure representing each circuit. The
main purpose of such trait is binding to each circuit the format of the public inputs, by
specifying the implementation of the `PublicInputAggregation` trait corresponding to such
format:

```rust
pub trait BaseCircuitInfo<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    type PIScheme: PublicInputAggregation;

    fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D>;
}
```

The constraints that all the base circuits in the set employed to construct the
`AggregationScheme` must share the same public input format is imposed by the fact that all
the circuits provided as input to the `build_circuit` method must implement `BaseCircuitInfo`
trait specifying the same implementation of `PublicInputAggregation` as their `PIScheme`.

#### Example

For example, suppose that a user wants to aggregate proofs generated from a set of circuits
with 2 base circuits, represented by data-structures `BaseCircuit1` and `BaseCircuit2`, which
employ the format for their public input specified by `SimpleStatePublicInput` (which is one
of the implementations of `PublicInputAggregation` provided by this crate). To instantiate an
`AggregationScheme` for such set of circuits, the user should do as follows:
1. Implement `BaseCircuitInfo` trait for both the circuits, specifying `SimpleStatePublicInput`
as their `PIScheme`:
```rust
impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
        BaseCircuitInfo<F, C, D> for BaseCircuit1
    {
        type PIScheme = SimpleStatePublicInput;

        fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
            // custom implementation
        }
   }

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
BaseCircuitInfo<F, C, D> for BaseCircuit2<F, C, D>
{
    type PIScheme = SimpleStatePublicInput;

    fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
        // custom implementation
    }
}
```
2. Given 2 instances of the `BaseCircuit1` and `BaseCircuit2` data-structures, called
`base_circuit_1` and `base_circuit_2`, respectively, build the set of circuits and instantiate
the `AggregationScheme` with the `build_circuit` method as follows:

```rust
let circuit_set = vec![prepare_base_circuit_for_circuit_set(base_circuit_1),
    prepare_base_circuit_for_circuit_set(base_circuit_2)];

        let aggregation_scheme =
            AggregationScheme::build_circuit(
                circuit_set.into_iter(),
            )?;
```


## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions
