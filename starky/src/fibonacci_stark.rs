use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with state `[x0, x1]` using the state transition
/// `x0 <- x1, x1 <- x0 + x1`.
#[derive(Copy, Clone)]
struct FibonacciStark<F: RichField + Extendable<D>, const D: usize> {
    num_rows: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> FibonacciStark<F, D> {
    // The first public input is `x0`.
    const PI_INDEX_X0: usize = 0;
    // The second public input is `x1`.
    const PI_INDEX_X1: usize = 1;
    // The third public input is the second element of the last row, which should be equal to the
    // `num_rows`-th Fibonacci number.
    const PI_INDEX_RES: usize = 2;

    fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, x1` as inital state values.
    fn generate_trace(&self, x0: F, x1: F) -> Vec<[F; Self::COLUMNS]> {
        (0..self.num_rows)
            .scan([x0, x1], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[1];
                acc[1] = tmp[0] + tmp[1];
                Some(tmp)
            })
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
    const COLUMNS: usize = 2;
    const PUBLIC_INPUTS: usize = 3;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Check public inputs.
        yield_constr
            .constraint_first_row(vars.local_values[0] - vars.public_inputs[Self::PI_INDEX_X0]);
        yield_constr
            .constraint_first_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_X1]);
        yield_constr
            .constraint_last_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_RES]);

        // x0 <- x1
        yield_constr.constraint(vars.next_values[0] - vars.local_values[1]);
        // x1 <- x0 + x1
        yield_constr.constraint(vars.next_values[1] - vars.local_values[0] - vars.local_values[1]);
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Check public inputs.
        let pis_constraints = [
            builder.sub_extension(vars.local_values[0], vars.public_inputs[Self::PI_INDEX_X0]),
            builder.sub_extension(vars.local_values[1], vars.public_inputs[Self::PI_INDEX_X1]),
            builder.sub_extension(vars.local_values[1], vars.public_inputs[Self::PI_INDEX_RES]),
        ];
        yield_constr.constraint_first_row(builder, pis_constraints[0]);
        yield_constr.constraint_first_row(builder, pis_constraints[1]);
        yield_constr.constraint_last_row(builder, pis_constraints[2]);

        // x0 <- x1
        let first_col_constraint = builder.sub_extension(vars.next_values[0], vars.local_values[1]);
        yield_constr.constraint(builder, first_col_constraint);
        // x1 <- x0 + x1
        let second_col_constraint = {
            let tmp = builder.sub_extension(vars.next_values[1], vars.local_values[0]);
            builder.sub_extension(tmp, vars.local_values[1])
        };
        yield_constr.constraint(builder, second_col_constraint);
    }

    fn constraint_degree(&self) -> usize {
        2
    }
}

// #[cfg(test)]
// mod tests {
//     use anyhow::Result;
//     use plonky2::field::extension_field::Extendable;
//     use plonky2::field::field_types::Field;
//     use plonky2::hash::hash_types::RichField;
//     use plonky2::iop::witness::PartialWitness;
//     use plonky2::plonk::circuit_builder::CircuitBuilder;
//     use plonky2::plonk::circuit_data::CommonCircuitData;
//     use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
//     use plonky2::plonk::proof::ProofWithPublicInputs;
//     use plonky2::util::timing::TimingTree;
//
//     use crate::config::StarkConfig;
//     use crate::fibonacci_stark::FibonacciStark;
//     use crate::proof::StarkProofWithPublicInputs;
//     use crate::prover::prove;
//     use crate::recursive_verifier::add_virtual_stark_proof_with_pis;
//     use crate::stark_testing::test_stark_low_degree;
//     use crate::verifier::verify;
//
//     fn fibonacci<F: Field>(n: usize, x0: F, x1: F) -> F {
//         (0..n).fold((x0, x1), |x, _| (x.1, x.0 + x.1)).1
//     }
//
//     #[test]
//     fn test_fibonacci_stark() -> Result<()> {
//         const D: usize = 2;
//         type C = PoseidonGoldilocksConfig;
//         type F = <C as GenericConfig<D>>::F;
//         type S = FibonacciStark<F, D>;
//
//         let config = StarkConfig::standard_fast_config();
//         let num_rows = 1 << 5;
//         let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];
//         let stark = S::new(num_rows);
//         let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
//         let proof = prove::<F, C, S, D>(
//             stark,
//             &config,
//             trace,
//             public_inputs,
//             &mut TimingTree::default(),
//         )?;
//
//         verify(stark, proof, &config)
//     }
//
//     #[test]
//     fn test_fibonacci_stark_degree() -> Result<()> {
//         const D: usize = 2;
//         type C = PoseidonGoldilocksConfig;
//         type F = <C as GenericConfig<D>>::F;
//         type S = FibonacciStark<F, D>;
//
//         let config = StarkConfig::standard_fast_config();
//         let num_rows = 1 << 5;
//         let stark = S::new(num_rows);
//         test_stark_low_degree(stark)
//     }
//
//     #[test]
//     fn test_recursive_stark_verifier() -> Result<()> {
//         init_logger();
//         const D: usize = 2;
//         type C = PoseidonGoldilocksConfig;
//         type F = <C as GenericConfig<D>>::F;
//         type S = FibonacciStark<F, D>;
//
//         let config = StarkConfig::standard_fast_config();
//         let num_rows = 1 << 5;
//         let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];
//         let stark = S::new(num_rows);
//         let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
//         let proof = prove::<F, C, S, D>(
//             stark,
//             &config,
//             trace,
//             public_inputs,
//             &mut TimingTree::default(),
//         )?;
//
//         let (proof, _vd, cd) =
//             recursive_proof::<F, C, C, D>(proof, vd, cd, &config, None, true, true)?;
//         test_serialization(&proof, &cd)?;
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_recursive_recursive_verifier() -> Result<()> {
//         init_logger();
//         const D: usize = 2;
//         type C = PoseidonGoldilocksConfig;
//         type F = <C as GenericConfig<D>>::F;
//
//         let config = CircuitConfig::standard_recursion_config();
//
//         // Start with a degree 2^14 proof
//         let (proof, vd, cd) = dummy_proof::<F, C, D>(&config, 16_000)?;
//         assert_eq!(cd.degree_bits, 14);
//
//         // Shrink it to 2^13.
//         let (proof, vd, cd) =
//             recursive_proof::<F, C, C, D>(proof, vd, cd, &config, Some(13), false, false)?;
//         assert_eq!(cd.degree_bits, 13);
//
//         // Shrink it to 2^12.
//         let (proof, _vd, cd) =
//             recursive_proof::<F, C, C, D>(proof, vd, cd, &config, None, true, true)?;
//         assert_eq!(cd.degree_bits, 12);
//
//         test_serialization(&proof, &cd)?;
//
//         Ok(())
//     }
//
//     /// Creates a chain of recursive proofs where the last proof is made as small as reasonably
//     /// possible, using a high rate, high PoW bits, etc.
//     #[test]
//     #[ignore]
//     fn test_size_optimized_recursion() -> Result<()> {
//         init_logger();
//         const D: usize = 2;
//         type C = PoseidonGoldilocksConfig;
//         type KC = KeccakGoldilocksConfig;
//         type F = <C as GenericConfig<D>>::F;
//
//         let standard_config = CircuitConfig::standard_recursion_config();
//
//         // An initial dummy proof.
//         let (proof, vd, cd) = dummy_proof::<F, C, D>(&standard_config, 4_000)?;
//         assert_eq!(cd.degree_bits, 12);
//
//         // A standard recursive proof.
//         let (proof, vd, cd) = recursive_proof(proof, vd, cd, &standard_config, None, false, false)?;
//         assert_eq!(cd.degree_bits, 12);
//
//         // A high-rate recursive proof, designed to be verifiable with fewer routed wires.
//         let high_rate_config = CircuitConfig {
//             fri_config: FriConfig {
//                 rate_bits: 7,
//                 proof_of_work_bits: 16,
//                 num_query_rounds: 12,
//                 ..standard_config.fri_config.clone()
//             },
//             ..standard_config
//         };
//         let (proof, vd, cd) =
//             recursive_proof::<F, C, C, D>(proof, vd, cd, &high_rate_config, None, true, true)?;
//         assert_eq!(cd.degree_bits, 12);
//
//         // A final proof, optimized for size.
//         let final_config = CircuitConfig {
//             num_routed_wires: 37,
//             fri_config: FriConfig {
//                 rate_bits: 8,
//                 cap_height: 0,
//                 proof_of_work_bits: 20,
//                 reduction_strategy: FriReductionStrategy::MinSize(None),
//                 num_query_rounds: 10,
//             },
//             ..high_rate_config
//         };
//         let (proof, _vd, cd) =
//             recursive_proof::<F, KC, C, D>(proof, vd, cd, &final_config, None, true, true)?;
//         assert_eq!(cd.degree_bits, 12, "final proof too large");
//
//         test_serialization(&proof, &cd)?;
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_recursive_verifier_multi_hash() -> Result<()> {
//         init_logger();
//         const D: usize = 2;
//         type PC = PoseidonGoldilocksConfig;
//         type KC = KeccakGoldilocksConfig;
//         type F = <PC as GenericConfig<D>>::F;
//
//         let config = CircuitConfig::standard_recursion_config();
//         let (proof, vd, cd) = dummy_proof::<F, PC, D>(&config, 4_000)?;
//
//         let (proof, vd, cd) =
//             recursive_proof::<F, PC, PC, D>(proof, vd, cd, &config, None, false, false)?;
//         test_serialization(&proof, &cd)?;
//
//         let (proof, _vd, cd) =
//             recursive_proof::<F, KC, PC, D>(proof, vd, cd, &config, None, false, false)?;
//         test_serialization(&proof, &cd)?;
//
//         Ok(())
//     }
//
//     /// Creates a dummy proof which should have roughly `num_dummy_gates` gates.
//     fn dummy_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
//         config: &CircuitConfig,
//         num_dummy_gates: u64,
//     ) -> Result<(
//         ProofWithPublicInputs<F, C, D>,
//         VerifierOnlyCircuitData<C, D>,
//         CommonCircuitData<F, C, D>,
//     )> {
//         let mut builder = CircuitBuilder::<F, D>::new(config.clone());
//         for _ in 0..num_dummy_gates {
//             builder.add_gate(NoopGate, vec![]);
//         }
//
//         let data = builder.build::<C>();
//         let inputs = PartialWitness::new();
//         let proof = data.prove(inputs)?;
//         data.verify(proof.clone())?;
//
//         Ok((proof, data.verifier_only, data.common))
//     }
//
//     fn recursive_proof<
//         F: RichField + Extendable<D>,
//         C: GenericConfig<D, F = F>,
//         InnerC: GenericConfig<D, F = F>,
//         const D: usize,
//     >(
//         inner_proof: StarkProofWithPublicInputs<F, InnerC, D>,
//         config: &StarkConfig,
//         print_gate_counts: bool,
//         print_timing: bool,
//     ) -> Result<(
//         ProofWithPublicInputs<F, C, D>,
//         VerifierOnlyCircuitData<C, D>,
//         CommonCircuitData<F, C, D>,
//     )>
//     where
//         InnerC::Hasher: AlgebraicHasher<F>,
//     {
//         let mut builder = CircuitBuilder::<F, D>::new(config.clone());
//         let mut pw = PartialWitness::new();
//         let degree_bits = inner_proof.proof.recover_degree_bits(config);
//         let pt = add_virtual_stark_proof_with_pis(&mut builder, stark, config, degree_bits);
//         pw.set_proof_with_pis_target(&pt, &inner_proof);
//
//         let inner_data = VerifierCircuitTarget {
//             constants_sigmas_cap: builder.add_virtual_cap(inner_cd.config.fri_config.cap_height),
//         };
//         pw.set_cap_target(
//             &inner_data.constants_sigmas_cap,
//             &inner_vd.constants_sigmas_cap,
//         );
//
//         builder.verify_proof(pt, &inner_data, &inner_cd);
//
//         if print_gate_counts {
//             builder.print_gate_counts(0);
//         }
//
//         if let Some(min_degree_bits) = min_degree_bits {
//             // We don't want to pad all the way up to 2^min_degree_bits, as the builder will add a
//             // few special gates afterward. So just pad to 2^(min_degree_bits - 1) + 1. Then the
//             // builder will pad to the next power of two, 2^min_degree_bits.
//             let min_gates = (1 << (min_degree_bits - 1)) + 1;
//             for _ in builder.num_gates()..min_gates {
//                 builder.add_gate(NoopGate, vec![]);
//             }
//         }
//
//         let data = builder.build::<C>();
//
//         let mut timing = TimingTree::new("prove", Level::Debug);
//         let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;
//         if print_timing {
//             timing.print();
//         }
//
//         data.verify(proof.clone())?;
//
//         Ok((proof, data.verifier_only, data.common))
//     }
//
//     /// Test serialization and print some size info.
//     fn test_serialization<
//         F: RichField + Extendable<D>,
//         C: GenericConfig<D, F = F>,
//         const D: usize,
//     >(
//         proof: &ProofWithPublicInputs<F, C, D>,
//         cd: &CommonCircuitData<F, C, D>,
//     ) -> Result<()> {
//         let proof_bytes = proof.to_bytes()?;
//         info!("Proof length: {} bytes", proof_bytes.len());
//         let proof_from_bytes = ProofWithPublicInputs::from_bytes(proof_bytes, cd)?;
//         assert_eq!(proof, &proof_from_bytes);
//
//         let now = std::time::Instant::now();
//         let compressed_proof = proof.clone().compress(cd)?;
//         let decompressed_compressed_proof = compressed_proof.clone().decompress(cd)?;
//         info!("{:.4}s to compress proof", now.elapsed().as_secs_f64());
//         assert_eq!(proof, &decompressed_compressed_proof);
//
//         let compressed_proof_bytes = compressed_proof.to_bytes()?;
//         info!(
//             "Compressed proof length: {} bytes",
//             compressed_proof_bytes.len()
//         );
//         let compressed_proof_from_bytes =
//             CompressedProofWithPublicInputs::from_bytes(compressed_proof_bytes, cd)?;
//         assert_eq!(compressed_proof, compressed_proof_from_bytes);
//
//         Ok(())
//     }
//
//     fn init_logger() {
//         let _ = env_logger::builder().format_timestamp(None).try_init();
//     }
// }
