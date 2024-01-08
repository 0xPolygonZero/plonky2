use core::mem::{self, MaybeUninit};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::anyhow;
use eth_trie_utils::partial_trie::{HashedPartialTrie, Node, PartialTrie};
use hashbrown::HashMap;
use itertools::{zip_eq, Itertools};
use plonky2::field::extension::Extendable;
use plonky2::fri::FriParams;
use plonky2::gates::constant::ConstantGate;
use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::RecursiveChallenger;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, VerifierCircuitData, VerifierCircuitTarget,
};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::recursion::cyclic_recursion::check_cyclic_proof_verifier_data;
use plonky2::recursion::dummy_circuit::cyclic_base_proof;
use plonky2::util::serialization::gate_serialization::default;
use plonky2::util::serialization::{
    Buffer, GateSerializer, IoResult, Read, WitnessGeneratorSerializer, Write,
};
use plonky2::util::timing::TimingTree;
use plonky2_util::log2_ceil;

use crate::all_stark::{all_cross_table_lookups, AllStark, Table, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cross_table_lookup::{
    get_grand_product_challenge_set_target, verify_cross_table_lookups_circuit, CrossTableLookup,
    GrandProductChallengeSet,
};
use crate::generation::GenerationInputs;
use crate::get_challenges::observe_public_values_target;
use crate::proof::{
    AllProof, BlockHashesTarget, BlockMetadataTarget, ExtraBlockData, ExtraBlockDataTarget,
    PublicValues, PublicValuesTarget, StarkProofWithMetadata, TrieRoots, TrieRootsTarget,
};
use crate::prover::{check_abort_signal, prove};
use crate::recursive_verifier::{
    add_common_recursion_gates, add_virtual_public_values, get_memory_extra_looking_sum_circuit,
    recursive_stark_circuit, set_public_value_targets, PlonkWrapperCircuit, PublicInputs,
    StarkWrapperCircuit,
};
use crate::stark::Stark;
use crate::util::h256_limbs;

/// The recursion threshold. We end a chain of recursive proofs once we reach this size.
const THRESHOLD_DEGREE_BITS: usize = 13;

/// Contains all recursive circuits used in the system. For each STARK and each initial
/// `degree_bits`, this contains a chain of recursive circuits for shrinking that STARK from
/// `degree_bits` to a constant `THRESHOLD_DEGREE_BITS`. It also contains a special root circuit
/// for combining each STARK's shrunk wrapper proof into a single proof.
#[derive(Eq, PartialEq, Debug)]
pub struct AllRecursiveCircuits<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    /// The EVM root circuit, which aggregates the (shrunk) per-table recursive proofs.
    pub root: RootCircuitData<F, C, D>,
    /// The aggregation circuit, which verifies two proofs that can either be root or
    /// aggregation proofs.
    pub aggregation: AggregationCircuitData<F, C, D>,
    /// The block circuit, which verifies an aggregation root proof and an optional previous block proof.
    pub block: BlockCircuitData<F, C, D>,
    /// Holds chains of circuits for each table and for each initial `degree_bits`.
    pub by_table: [RecursiveCircuitsForTable<F, C, D>; NUM_TABLES],
}

/// Data for the EVM root circuit, which is used to combine each STARK's shrunk wrapper proof
/// into a single proof.
#[derive(Eq, PartialEq, Debug)]
pub struct RootCircuitData<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub circuit: CircuitData<F, C, D>,
    proof_with_pis: [ProofWithPublicInputsTarget<D>; NUM_TABLES],
    /// For each table, various inner circuits may be used depending on the initial table size.
    /// This target holds the index of the circuit (within `final_circuits()`) that was used.
    index_verifier_data: [Target; NUM_TABLES],
    /// Public inputs containing public values.
    public_values: PublicValuesTarget,
    /// Public inputs used for cyclic verification. These aren't actually used for EVM root
    /// proofs; the circuit has them just to match the structure of aggregation proofs.
    cyclic_vk: VerifierCircuitTarget,
}

impl<F, C, const D: usize> RootCircuitData<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    fn to_buffer(
        &self,
        buffer: &mut Vec<u8>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        buffer.write_circuit_data(&self.circuit, gate_serializer, generator_serializer)?;
        for proof in &self.proof_with_pis {
            buffer.write_target_proof_with_public_inputs(proof)?;
        }
        for index in self.index_verifier_data {
            buffer.write_target(index)?;
        }
        self.public_values.to_buffer(buffer)?;
        buffer.write_target_verifier_circuit(&self.cyclic_vk)?;
        Ok(())
    }

    fn from_buffer(
        buffer: &mut Buffer,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let circuit = buffer.read_circuit_data(gate_serializer, generator_serializer)?;
        let mut proof_with_pis = Vec::with_capacity(NUM_TABLES);
        for _ in 0..NUM_TABLES {
            proof_with_pis.push(buffer.read_target_proof_with_public_inputs()?);
        }
        let mut index_verifier_data = Vec::with_capacity(NUM_TABLES);
        for _ in 0..NUM_TABLES {
            index_verifier_data.push(buffer.read_target()?);
        }
        let public_values = PublicValuesTarget::from_buffer(buffer)?;
        let cyclic_vk = buffer.read_target_verifier_circuit()?;

        Ok(Self {
            circuit,
            proof_with_pis: proof_with_pis.try_into().unwrap(),
            index_verifier_data: index_verifier_data.try_into().unwrap(),
            public_values,
            cyclic_vk,
        })
    }
}

/// Data for the aggregation circuit, which is used to compress two proofs into one. Each inner
/// proof can be either an EVM root proof or another aggregation proof.
#[derive(Eq, PartialEq, Debug)]
pub struct AggregationCircuitData<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub circuit: CircuitData<F, C, D>,
    lhs: AggregationChildTarget<D>,
    rhs: AggregationChildTarget<D>,
    public_values: PublicValuesTarget,
    cyclic_vk: VerifierCircuitTarget,
}

impl<F, C, const D: usize> AggregationCircuitData<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    fn to_buffer(
        &self,
        buffer: &mut Vec<u8>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        buffer.write_circuit_data(&self.circuit, gate_serializer, generator_serializer)?;
        buffer.write_target_verifier_circuit(&self.cyclic_vk)?;
        self.public_values.to_buffer(buffer)?;
        self.lhs.to_buffer(buffer)?;
        self.rhs.to_buffer(buffer)?;
        Ok(())
    }

    fn from_buffer(
        buffer: &mut Buffer,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let circuit = buffer.read_circuit_data(gate_serializer, generator_serializer)?;
        let cyclic_vk = buffer.read_target_verifier_circuit()?;
        let public_values = PublicValuesTarget::from_buffer(buffer)?;
        let lhs = AggregationChildTarget::from_buffer(buffer)?;
        let rhs = AggregationChildTarget::from_buffer(buffer)?;
        Ok(Self {
            circuit,
            lhs,
            rhs,
            public_values,
            cyclic_vk,
        })
    }
}

#[derive(Eq, PartialEq, Debug)]
struct AggregationChildTarget<const D: usize> {
    is_agg: BoolTarget,
    agg_proof: ProofWithPublicInputsTarget<D>,
    evm_proof: ProofWithPublicInputsTarget<D>,
}

impl<const D: usize> AggregationChildTarget<D> {
    fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_bool(self.is_agg)?;
        buffer.write_target_proof_with_public_inputs(&self.agg_proof)?;
        buffer.write_target_proof_with_public_inputs(&self.evm_proof)?;
        Ok(())
    }

    fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let is_agg = buffer.read_target_bool()?;
        let agg_proof = buffer.read_target_proof_with_public_inputs()?;
        let evm_proof = buffer.read_target_proof_with_public_inputs()?;
        Ok(Self {
            is_agg,
            agg_proof,
            evm_proof,
        })
    }

    fn public_values<F: RichField + Extendable<D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> PublicValuesTarget {
        let agg_pv = PublicValuesTarget::from_public_inputs(&self.agg_proof.public_inputs);
        let evm_pv = PublicValuesTarget::from_public_inputs(&self.evm_proof.public_inputs);
        PublicValuesTarget::select(builder, self.is_agg, agg_pv, evm_pv)
    }
}

/// Data for the block circuit, which is used to generate a final block proof,
/// and compress it with an optional parent proof if present.
#[derive(Eq, PartialEq, Debug)]
pub struct BlockCircuitData<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub circuit: CircuitData<F, C, D>,
    has_parent_block: BoolTarget,
    parent_block_proof: ProofWithPublicInputsTarget<D>,
    agg_root_proof: ProofWithPublicInputsTarget<D>,
    public_values: PublicValuesTarget,
    cyclic_vk: VerifierCircuitTarget,
}

impl<F, C, const D: usize> BlockCircuitData<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    fn to_buffer(
        &self,
        buffer: &mut Vec<u8>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        buffer.write_circuit_data(&self.circuit, gate_serializer, generator_serializer)?;
        buffer.write_target_bool(self.has_parent_block)?;
        buffer.write_target_proof_with_public_inputs(&self.parent_block_proof)?;
        buffer.write_target_proof_with_public_inputs(&self.agg_root_proof)?;
        self.public_values.to_buffer(buffer)?;
        buffer.write_target_verifier_circuit(&self.cyclic_vk)?;
        Ok(())
    }

    fn from_buffer(
        buffer: &mut Buffer,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let circuit = buffer.read_circuit_data(gate_serializer, generator_serializer)?;
        let has_parent_block = buffer.read_target_bool()?;
        let parent_block_proof = buffer.read_target_proof_with_public_inputs()?;
        let agg_root_proof = buffer.read_target_proof_with_public_inputs()?;
        let public_values = PublicValuesTarget::from_buffer(buffer)?;
        let cyclic_vk = buffer.read_target_verifier_circuit()?;
        Ok(Self {
            circuit,
            has_parent_block,
            parent_block_proof,
            agg_root_proof,
            public_values,
            cyclic_vk,
        })
    }
}

impl<F, C, const D: usize> AllRecursiveCircuits<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
    /// Serializes all these preprocessed circuits into a sequence of bytes.
    ///
    /// # Arguments
    ///
    /// - `skip_tables`: a boolean indicating whether to serialize only the upper circuits
    /// or the entire prover state, including recursive circuits to shrink STARK proofs.
    /// - `gate_serializer`: a custom gate serializer needed to serialize recursive circuits
    /// common data.
    /// - `generator_serializer`: a custom generator serializer needed to serialize recursive
    /// circuits proving data.
    pub fn to_bytes(
        &self,
        skip_tables: bool,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Vec<u8>> {
        // TODO: would be better to initialize it dynamically based on the supported max degree.
        let mut buffer = Vec::with_capacity(1 << 34);
        self.root
            .to_buffer(&mut buffer, gate_serializer, generator_serializer)?;
        self.aggregation
            .to_buffer(&mut buffer, gate_serializer, generator_serializer)?;
        self.block
            .to_buffer(&mut buffer, gate_serializer, generator_serializer)?;
        if !skip_tables {
            for table in &self.by_table {
                table.to_buffer(&mut buffer, gate_serializer, generator_serializer)?;
            }
        }
        Ok(buffer)
    }

    /// Deserializes a sequence of bytes into an entire prover state containing all recursive circuits.
    ///
    /// # Arguments
    ///
    /// - `bytes`: a slice of bytes to deserialize this prover state from.
    /// - `skip_tables`: a boolean indicating whether to deserialize only the upper circuits
    /// or the entire prover state, including recursive circuits to shrink STARK proofs.
    /// - `gate_serializer`: a custom gate serializer needed to serialize recursive circuits
    /// common data.
    /// - `generator_serializer`: a custom generator serializer needed to serialize recursive
    /// circuits proving data.
    pub fn from_bytes(
        bytes: &[u8],
        skip_tables: bool,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let mut buffer = Buffer::new(bytes);
        let root =
            RootCircuitData::from_buffer(&mut buffer, gate_serializer, generator_serializer)?;
        let aggregation = AggregationCircuitData::from_buffer(
            &mut buffer,
            gate_serializer,
            generator_serializer,
        )?;
        let block =
            BlockCircuitData::from_buffer(&mut buffer, gate_serializer, generator_serializer)?;

        let by_table = match skip_tables {
            true => (0..NUM_TABLES)
                .map(|_| RecursiveCircuitsForTable {
                    by_stark_size: BTreeMap::default(),
                })
                .collect_vec()
                .try_into()
                .unwrap(),
            false => {
                // Tricky use of MaybeUninit to remove the need for implementing Debug
                // for all underlying types, necessary to convert a by_table Vec to an array.
                let mut by_table: [MaybeUninit<RecursiveCircuitsForTable<F, C, D>>; NUM_TABLES] =
                    unsafe { MaybeUninit::uninit().assume_init() };
                for table in &mut by_table[..] {
                    let value = RecursiveCircuitsForTable::from_buffer(
                        &mut buffer,
                        gate_serializer,
                        generator_serializer,
                    )?;
                    *table = MaybeUninit::new(value);
                }
                unsafe {
                    mem::transmute::<_, [RecursiveCircuitsForTable<F, C, D>; NUM_TABLES]>(by_table)
                }
            }
        };

        Ok(Self {
            root,
            aggregation,
            block,
            by_table,
        })
    }

    /// Preprocess all recursive circuits used by the system.
    ///
    /// # Arguments
    ///
    /// - `all_stark`: a structure defining the logic of all STARK modules and their associated
    /// cross-table lookups.
    /// - `degree_bits_ranges`: the logarithmic ranges to be supported for the recursive tables.
    /// Transactions may yield arbitrary trace lengths for each STARK module (within some bounds),
    /// unknown prior generating the witness to create a proof. Thus, for each STARK module, we
    /// construct a map from `2^{degree_bits} = length` to a chain of shrinking recursion circuits,
    /// starting from that length, for each `degree_bits` in the range specified for this STARK module.
    /// Specifying a wide enough range allows a prover to cover all possible scenarios.
    /// - `stark_config`: the configuration to be used for the STARK prover. It will usually be a fast
    /// one yielding large proofs.
    pub fn new(
        all_stark: &AllStark<F, D>,
        degree_bits_ranges: &[Range<usize>; NUM_TABLES],
        stark_config: &StarkConfig,
    ) -> Self {
        let arithmetic = RecursiveCircuitsForTable::new(
            Table::Arithmetic,
            &all_stark.arithmetic_stark,
            degree_bits_ranges[Table::Arithmetic as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let byte_packing = RecursiveCircuitsForTable::new(
            Table::BytePacking,
            &all_stark.byte_packing_stark,
            degree_bits_ranges[Table::BytePacking as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let cpu = RecursiveCircuitsForTable::new(
            Table::Cpu,
            &all_stark.cpu_stark,
            degree_bits_ranges[Table::Cpu as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let keccak = RecursiveCircuitsForTable::new(
            Table::Keccak,
            &all_stark.keccak_stark,
            degree_bits_ranges[Table::Keccak as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let keccak_sponge = RecursiveCircuitsForTable::new(
            Table::KeccakSponge,
            &all_stark.keccak_sponge_stark,
            degree_bits_ranges[Table::KeccakSponge as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let logic = RecursiveCircuitsForTable::new(
            Table::Logic,
            &all_stark.logic_stark,
            degree_bits_ranges[Table::Logic as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );
        let memory = RecursiveCircuitsForTable::new(
            Table::Memory,
            &all_stark.memory_stark,
            degree_bits_ranges[Table::Memory as usize].clone(),
            &all_stark.cross_table_lookups,
            stark_config,
        );

        let by_table = [
            arithmetic,
            byte_packing,
            cpu,
            keccak,
            keccak_sponge,
            logic,
            memory,
        ];
        let root = Self::create_root_circuit(&by_table, stark_config);
        let aggregation = Self::create_aggregation_circuit(&root);
        let block = Self::create_block_circuit(&aggregation);
        Self {
            root,
            aggregation,
            block,
            by_table,
        }
    }

    /// Outputs the `VerifierCircuitData` needed to verify any block proof
    /// generated by an honest prover.
    /// While the [`AllRecursiveCircuits`] prover state can also verify proofs, verifiers
    /// only need a fraction of the state to verify proofs. This allows much less powerful
    /// entities to behave as verifiers, by only loading the necessary data to verify block proofs.
    ///
    /// # Usage
    ///
    /// ```ignore
    /// let prover_state = AllRecursiveCircuits { ... };
    /// let verifier_state = prover_state.final_verifier_data();
    ///
    /// // Verify a provided block proof
    /// assert!(verifier_state.verify(&block_proof).is_ok());
    /// ```
    pub fn final_verifier_data(&self) -> VerifierCircuitData<F, C, D> {
        self.block.circuit.verifier_data()
    }

    fn create_root_circuit(
        by_table: &[RecursiveCircuitsForTable<F, C, D>; NUM_TABLES],
        stark_config: &StarkConfig,
    ) -> RootCircuitData<F, C, D> {
        let inner_common_data: [_; NUM_TABLES] =
            core::array::from_fn(|i| &by_table[i].final_circuits()[0].common);

        let mut builder = CircuitBuilder::new(CircuitConfig::standard_recursion_config());

        let public_values = add_virtual_public_values(&mut builder);

        let recursive_proofs =
            core::array::from_fn(|i| builder.add_virtual_proof_with_pis(inner_common_data[i]));
        let pis: [_; NUM_TABLES] = core::array::from_fn(|i| {
            PublicInputs::<Target, <C::Hasher as AlgebraicHasher<F>>::AlgebraicPermutation>::from_vec(
                &recursive_proofs[i].public_inputs,
                stark_config,
            )
        });
        let index_verifier_data = core::array::from_fn(|_i| builder.add_virtual_target());

        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(&mut builder);
        for pi in &pis {
            for h in &pi.trace_cap {
                challenger.observe_elements(h);
            }
        }

        observe_public_values_target::<F, C, D>(&mut challenger, &public_values);

        let ctl_challenges = get_grand_product_challenge_set_target(
            &mut builder,
            &mut challenger,
            stark_config.num_challenges,
        );
        // Check that the correct CTL challenges are used in every proof.
        for pi in &pis {
            for i in 0..stark_config.num_challenges {
                builder.connect(
                    ctl_challenges.challenges[i].beta,
                    pi.ctl_challenges.challenges[i].beta,
                );
                builder.connect(
                    ctl_challenges.challenges[i].gamma,
                    pi.ctl_challenges.challenges[i].gamma,
                );
            }
        }

        let state = challenger.compact(&mut builder);
        for (&before, &s) in zip_eq(state.as_ref(), pis[0].challenger_state_before.as_ref()) {
            builder.connect(before, s);
        }
        // Check that the challenger state is consistent between proofs.
        for i in 1..NUM_TABLES {
            for (&before, &after) in zip_eq(
                pis[i].challenger_state_before.as_ref(),
                pis[i - 1].challenger_state_after.as_ref(),
            ) {
                builder.connect(before, after);
            }
        }

        // Extra sums to add to the looked last value.
        // Only necessary for the Memory values.
        let mut extra_looking_sums =
            vec![vec![builder.zero(); stark_config.num_challenges]; NUM_TABLES];

        // Memory
        extra_looking_sums[Table::Memory as usize] = (0..stark_config.num_challenges)
            .map(|c| {
                get_memory_extra_looking_sum_circuit(
                    &mut builder,
                    &public_values,
                    ctl_challenges.challenges[c],
                )
            })
            .collect_vec();

        // Verify the CTL checks.
        verify_cross_table_lookups_circuit::<F, D>(
            &mut builder,
            all_cross_table_lookups(),
            pis.map(|p| p.ctl_zs_first),
            extra_looking_sums,
            stark_config,
        );

        for (i, table_circuits) in by_table.iter().enumerate() {
            let final_circuits = table_circuits.final_circuits();
            for final_circuit in &final_circuits {
                assert_eq!(
                    &final_circuit.common, inner_common_data[i],
                    "common_data mismatch"
                );
            }
            let mut possible_vks = final_circuits
                .into_iter()
                .map(|c| builder.constant_verifier_data(&c.verifier_only))
                .collect_vec();
            // random_access_verifier_data expects a vector whose length is a power of two.
            // To satisfy this, we will just add some duplicates of the first VK.
            while !possible_vks.len().is_power_of_two() {
                possible_vks.push(possible_vks[0].clone());
            }
            let inner_verifier_data =
                builder.random_access_verifier_data(index_verifier_data[i], possible_vks);

            builder.verify_proof::<C>(
                &recursive_proofs[i],
                &inner_verifier_data,
                inner_common_data[i],
            );
        }

        // We want EVM root proofs to have the exact same structure as aggregation proofs, so we add
        // public inputs for cyclic verification, even though they'll be ignored.
        let cyclic_vk = builder.add_verifier_data_public_inputs();

        builder.add_gate(
            ConstantGate::new(inner_common_data[0].config.num_constants),
            vec![],
        );

        RootCircuitData {
            circuit: builder.build::<C>(),
            proof_with_pis: recursive_proofs,
            index_verifier_data,
            public_values,
            cyclic_vk,
        }
    }

    fn create_aggregation_circuit(
        root: &RootCircuitData<F, C, D>,
    ) -> AggregationCircuitData<F, C, D> {
        let mut builder = CircuitBuilder::<F, D>::new(root.circuit.common.config.clone());
        let public_values = add_virtual_public_values(&mut builder);
        let cyclic_vk = builder.add_verifier_data_public_inputs();
        let lhs = Self::add_agg_child(&mut builder, root);
        let rhs = Self::add_agg_child(&mut builder, root);

        let lhs_public_values = lhs.public_values(&mut builder);
        let rhs_public_values = rhs.public_values(&mut builder);
        // Connect all block hash values
        BlockHashesTarget::connect(
            &mut builder,
            public_values.block_hashes,
            lhs_public_values.block_hashes,
        );
        BlockHashesTarget::connect(
            &mut builder,
            public_values.block_hashes,
            rhs_public_values.block_hashes,
        );
        // Connect all block metadata values.
        BlockMetadataTarget::connect(
            &mut builder,
            public_values.block_metadata,
            lhs_public_values.block_metadata,
        );
        BlockMetadataTarget::connect(
            &mut builder,
            public_values.block_metadata,
            rhs_public_values.block_metadata,
        );
        // Connect aggregation `trie_roots_before` with lhs `trie_roots_before`.
        TrieRootsTarget::connect(
            &mut builder,
            public_values.trie_roots_before,
            lhs_public_values.trie_roots_before,
        );
        // Connect aggregation `trie_roots_after` with rhs `trie_roots_after`.
        TrieRootsTarget::connect(
            &mut builder,
            public_values.trie_roots_after,
            rhs_public_values.trie_roots_after,
        );
        // Connect lhs `trie_roots_after` with rhs `trie_roots_before`.
        TrieRootsTarget::connect(
            &mut builder,
            lhs_public_values.trie_roots_after,
            rhs_public_values.trie_roots_before,
        );

        Self::connect_extra_public_values(
            &mut builder,
            &public_values.extra_block_data,
            &lhs_public_values.extra_block_data,
            &rhs_public_values.extra_block_data,
        );

        // Pad to match the root circuit's degree.
        while log2_ceil(builder.num_gates()) < root.circuit.common.degree_bits() {
            builder.add_gate(NoopGate, vec![]);
        }

        let circuit = builder.build::<C>();
        AggregationCircuitData {
            circuit,
            lhs,
            rhs,
            public_values,
            cyclic_vk,
        }
    }

    fn connect_extra_public_values(
        builder: &mut CircuitBuilder<F, D>,
        pvs: &ExtraBlockDataTarget,
        lhs: &ExtraBlockDataTarget,
        rhs: &ExtraBlockDataTarget,
    ) {
        // Connect checkpoint state root values.
        for (&limb0, &limb1) in pvs
            .checkpoint_state_trie_root
            .iter()
            .zip(&rhs.checkpoint_state_trie_root)
        {
            builder.connect(limb0, limb1);
        }
        for (&limb0, &limb1) in pvs
            .checkpoint_state_trie_root
            .iter()
            .zip(&lhs.checkpoint_state_trie_root)
        {
            builder.connect(limb0, limb1);
        }

        // Connect the transaction number in public values to the lhs and rhs values correctly.
        builder.connect(pvs.txn_number_before, lhs.txn_number_before);
        builder.connect(pvs.txn_number_after, rhs.txn_number_after);

        // Connect lhs `txn_number_after` with rhs `txn_number_before`.
        builder.connect(lhs.txn_number_after, rhs.txn_number_before);

        // Connect the gas used in public values to the lhs and rhs values correctly.
        builder.connect(pvs.gas_used_before, lhs.gas_used_before);
        builder.connect(pvs.gas_used_after, rhs.gas_used_after);

        // Connect lhs `gas_used_after` with rhs `gas_used_before`.
        builder.connect(lhs.gas_used_after, rhs.gas_used_before);
    }

    fn add_agg_child(
        builder: &mut CircuitBuilder<F, D>,
        root: &RootCircuitData<F, C, D>,
    ) -> AggregationChildTarget<D> {
        let common = &root.circuit.common;
        let root_vk = builder.constant_verifier_data(&root.circuit.verifier_only);
        let is_agg = builder.add_virtual_bool_target_safe();
        let agg_proof = builder.add_virtual_proof_with_pis(common);
        let evm_proof = builder.add_virtual_proof_with_pis(common);
        builder
            .conditionally_verify_cyclic_proof::<C>(
                is_agg, &agg_proof, &evm_proof, &root_vk, common,
            )
            .expect("Failed to build cyclic recursion circuit");
        AggregationChildTarget {
            is_agg,
            agg_proof,
            evm_proof,
        }
    }

    fn create_block_circuit(agg: &AggregationCircuitData<F, C, D>) -> BlockCircuitData<F, C, D> {
        // The block circuit is similar to the agg circuit; both verify two inner proofs.
        // We need to adjust a few things, but it's easier than making a new CommonCircuitData.
        let expected_common_data = CommonCircuitData {
            fri_params: FriParams {
                degree_bits: 14,
                ..agg.circuit.common.fri_params.clone()
            },
            ..agg.circuit.common.clone()
        };

        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let public_values = add_virtual_public_values(&mut builder);
        let has_parent_block = builder.add_virtual_bool_target_safe();
        let parent_block_proof = builder.add_virtual_proof_with_pis(&expected_common_data);
        let agg_root_proof = builder.add_virtual_proof_with_pis(&agg.circuit.common);

        // Connect block hashes
        Self::connect_block_hashes(&mut builder, &parent_block_proof, &agg_root_proof);

        let parent_pv = PublicValuesTarget::from_public_inputs(&parent_block_proof.public_inputs);
        let agg_pv = PublicValuesTarget::from_public_inputs(&agg_root_proof.public_inputs);

        // Connect block `trie_roots_before` with parent_pv `trie_roots_before`.
        TrieRootsTarget::connect(
            &mut builder,
            public_values.trie_roots_before,
            parent_pv.trie_roots_before,
        );
        // Connect the rest of block `public_values` with agg_pv.
        TrieRootsTarget::connect(
            &mut builder,
            public_values.trie_roots_after,
            agg_pv.trie_roots_after,
        );
        BlockMetadataTarget::connect(
            &mut builder,
            public_values.block_metadata,
            agg_pv.block_metadata,
        );
        BlockHashesTarget::connect(
            &mut builder,
            public_values.block_hashes,
            agg_pv.block_hashes,
        );
        ExtraBlockDataTarget::connect(
            &mut builder,
            public_values.extra_block_data,
            agg_pv.extra_block_data,
        );

        // Make connections between block proofs, and check initial and final block values.
        Self::connect_block_proof(&mut builder, has_parent_block, &parent_pv, &agg_pv);

        let cyclic_vk = builder.add_verifier_data_public_inputs();
        builder
            .conditionally_verify_cyclic_proof_or_dummy::<C>(
                has_parent_block,
                &parent_block_proof,
                &expected_common_data,
            )
            .expect("Failed to build cyclic recursion circuit");

        let agg_verifier_data = builder.constant_verifier_data(&agg.circuit.verifier_only);
        builder.verify_proof::<C>(&agg_root_proof, &agg_verifier_data, &agg.circuit.common);

        let circuit = builder.build::<C>();
        BlockCircuitData {
            circuit,
            has_parent_block,
            parent_block_proof,
            agg_root_proof,
            public_values,
            cyclic_vk,
        }
    }

    /// Connect the 256 block hashes between two blocks
    fn connect_block_hashes(
        builder: &mut CircuitBuilder<F, D>,
        lhs: &ProofWithPublicInputsTarget<D>,
        rhs: &ProofWithPublicInputsTarget<D>,
    ) {
        let lhs_public_values = PublicValuesTarget::from_public_inputs(&lhs.public_inputs);
        let rhs_public_values = PublicValuesTarget::from_public_inputs(&rhs.public_inputs);
        for i in 0..255 {
            for j in 0..8 {
                builder.connect(
                    lhs_public_values.block_hashes.prev_hashes[8 * (i + 1) + j],
                    rhs_public_values.block_hashes.prev_hashes[8 * i + j],
                );
            }
        }
        let expected_hash = lhs_public_values.block_hashes.cur_hash;
        let prev_block_hash = &rhs_public_values.block_hashes.prev_hashes[255 * 8..256 * 8];
        for i in 0..expected_hash.len() {
            builder.connect(expected_hash[i], prev_block_hash[i]);
        }
    }

    fn connect_block_proof(
        builder: &mut CircuitBuilder<F, D>,
        has_parent_block: BoolTarget,
        lhs: &PublicValuesTarget,
        rhs: &PublicValuesTarget,
    ) {
        // Between blocks, we only connect state tries.
        for (&limb0, limb1) in lhs
            .trie_roots_after
            .state_root
            .iter()
            .zip(rhs.trie_roots_before.state_root)
        {
            builder.connect(limb0, limb1);
        }

        // Between blocks, the checkpoint state trie remains unchanged.
        for (&limb0, limb1) in lhs
            .extra_block_data
            .checkpoint_state_trie_root
            .iter()
            .zip(rhs.extra_block_data.checkpoint_state_trie_root)
        {
            builder.connect(limb0, limb1);
        }

        // Connect block numbers.
        let one = builder.one();
        let prev_block_nb = builder.sub(rhs.block_metadata.block_number, one);
        builder.connect(lhs.block_metadata.block_number, prev_block_nb);

        // Check initial block values.
        Self::connect_initial_values_block(builder, rhs);

        // Connect intermediary values for gas_used and bloom filters to the block's final values. We only plug on the right, so there is no need to check the left-handside block.
        Self::connect_final_block_values_to_intermediary(builder, rhs);

        let has_not_parent_block = builder.sub(one, has_parent_block.target);

        // Check that the checkpoint block has the predetermined state trie root in `ExtraBlockData`.
        Self::connect_checkpoint_block(builder, rhs, has_not_parent_block);
    }

    fn connect_checkpoint_block(
        builder: &mut CircuitBuilder<F, D>,
        x: &PublicValuesTarget,
        has_not_parent_block: Target,
    ) where
        F: RichField + Extendable<D>,
    {
        for (&limb0, limb1) in x
            .trie_roots_before
            .state_root
            .iter()
            .zip(x.extra_block_data.checkpoint_state_trie_root)
        {
            let mut constr = builder.sub(limb0, limb1);
            constr = builder.mul(has_not_parent_block, constr);
            builder.assert_zero(constr);
        }
    }

    fn connect_final_block_values_to_intermediary(
        builder: &mut CircuitBuilder<F, D>,
        x: &PublicValuesTarget,
    ) where
        F: RichField + Extendable<D>,
    {
        builder.connect(
            x.block_metadata.block_gas_used,
            x.extra_block_data.gas_used_after,
        );
    }

    fn connect_initial_values_block(builder: &mut CircuitBuilder<F, D>, x: &PublicValuesTarget)
    where
        F: RichField + Extendable<D>,
    {
        // The initial number of transactions is 0.
        builder.assert_zero(x.extra_block_data.txn_number_before);
        // The initial gas used is 0.
        builder.assert_zero(x.extra_block_data.gas_used_before);

        // The transactions and receipts tries are empty at the beginning of the block.
        let initial_trie = HashedPartialTrie::from(Node::Empty).hash();

        for (i, limb) in h256_limbs::<F>(initial_trie).into_iter().enumerate() {
            let limb_target = builder.constant(limb);
            builder.connect(x.trie_roots_before.transactions_root[i], limb_target);
            builder.connect(x.trie_roots_before.receipts_root[i], limb_target);
        }
    }

    /// For a given transaction payload passed as [`GenerationInputs`], create a proof
    /// for each STARK module, then recursively shrink and combine them, eventually
    /// culminating in a transaction proof, also called root proof.
    ///
    /// # Arguments
    ///
    /// - `all_stark`: a structure defining the logic of all STARK modules and their associated
    /// cross-table lookups.
    /// - `config`: the configuration to be used for the STARK prover. It will usually be a fast
    /// one yielding large proofs.
    /// - `generation_inputs`: a transaction and auxiliary data needed to generate a proof, provided
    /// in Intermediary Representation.
    /// - `timing`: a profiler defining a scope hierarchy and the time consumed by each one.
    /// - `abort_signal`: an optional [`AtomicBool`] wrapped behind an [`Arc`], to send a kill signal
    /// early. This is only necessary in a distributed setting where a worker may be blocking the entire
    /// queue.
    ///
    /// # Outputs
    ///
    /// This method outputs a tuple of [`ProofWithPublicInputs<F, C, D>`] and its [`PublicValues`]. Only
    /// the proof with public inputs is necessary for a verifier to assert correctness of the computation,
    /// but the public values are output for the prover convenience, as these are necessary during proof
    /// aggregation.
    pub fn prove_root(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
        generation_inputs: GenerationInputs,
        timing: &mut TimingTree,
        abort_signal: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<(ProofWithPublicInputs<F, C, D>, PublicValues)> {
        let all_proof = prove::<F, C, D>(
            all_stark,
            config,
            generation_inputs,
            timing,
            abort_signal.clone(),
        )?;
        let mut root_inputs = PartialWitness::new();

        for table in 0..NUM_TABLES {
            let stark_proof = &all_proof.stark_proofs[table];
            let original_degree_bits = stark_proof.proof.recover_degree_bits(config);
            let table_circuits = &self.by_table[table];
            let shrunk_proof = table_circuits
                .by_stark_size
                .get(&original_degree_bits)
                .ok_or_else(|| {
                    anyhow!(format!(
                        "Missing preprocessed circuits for {:?} table with size {}.",
                        Table::all()[table],
                        original_degree_bits,
                    ))
                })?
                .shrink(stark_proof, &all_proof.ctl_challenges)?;
            let index_verifier_data = table_circuits
                .by_stark_size
                .keys()
                .position(|&size| size == original_degree_bits)
                .unwrap();
            root_inputs.set_target(
                self.root.index_verifier_data[table],
                F::from_canonical_usize(index_verifier_data),
            );
            root_inputs.set_proof_with_pis_target(&self.root.proof_with_pis[table], &shrunk_proof);

            check_abort_signal(abort_signal.clone())?;
        }

        root_inputs.set_verifier_data_target(
            &self.root.cyclic_vk,
            &self.aggregation.circuit.verifier_only,
        );

        set_public_value_targets(
            &mut root_inputs,
            &self.root.public_values,
            &all_proof.public_values,
        )
        .map_err(|_| {
            anyhow::Error::msg("Invalid conversion when setting public values targets.")
        })?;

        let root_proof = self.root.circuit.prove(root_inputs)?;

        Ok((root_proof, all_proof.public_values))
    }

    /// From an initial set of STARK proofs passed with their associated recursive table circuits,
    /// generate a recursive transaction proof.
    /// It is aimed at being used when preprocessed table circuits have not been loaded to memory.
    ///
    /// **Note**:
    /// The type of the `table_circuits` passed as arguments is
    /// `&[(RecursiveCircuitsForTableSize<F, C, D>, u8); NUM_TABLES]`. In particular, for each STARK
    /// proof contained within the `AllProof` object provided to this method, we need to pass a tuple
    /// of [`RecursiveCircuitsForTableSize<F, C, D>`] and a [`u8`]. The former is the recursive chain
    /// corresponding to the initial degree size of the associated STARK proof. The latter is the
    /// index of this degree in the range that was originally passed when constructing the entire prover
    /// state.
    ///
    /// # Usage
    ///
    /// ```ignore
    /// // Load a prover state without its recursive table circuits.
    /// let gate_serializer = DefaultGateSerializer;
    /// let generator_serializer = DefaultGeneratorSerializer::<C, D>::new();
    /// let initial_ranges = [16..25, 10..20, 12..25, 14..25, 9..20, 12..20, 17..30];
    /// let prover_state = AllRecursiveCircuits::<F, C, D>::new(
    ///     &all_stark,
    ///     &initial_ranges,
    ///     &config,
    /// );
    ///
    /// // Generate a proof from the provided inputs.
    /// let stark_proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing, abort_signal).unwrap();
    ///
    /// // Read the degrees of the internal STARK proofs.
    /// // Indices to be passed along the recursive tables
    /// // can be easily recovered as `initial_ranges[i]` - `degrees[i]`.
    /// let degrees = proof.degree_bits(&config);
    ///
    /// // Retrieve the corresponding recursive table circuits for each table with the corresponding degree.
    /// let table_circuits = { ... };
    ///
    /// // Finally shrink the STARK proof.
    /// let (proof, public_values) = prove_root_after_initial_stark(
    ///     &all_stark,
    ///     &config,
    ///     &stark_proof,
    ///     &table_circuits,
    ///     &mut timing,
    ///     abort_signal,
    /// ).unwrap();
    /// ```
    pub fn prove_root_after_initial_stark(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
        all_proof: AllProof<F, C, D>,
        table_circuits: &[(RecursiveCircuitsForTableSize<F, C, D>, u8); NUM_TABLES],
        timing: &mut TimingTree,
        abort_signal: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<(ProofWithPublicInputs<F, C, D>, PublicValues)> {
        let mut root_inputs = PartialWitness::new();

        for table in 0..NUM_TABLES {
            let (table_circuit, index_verifier_data) = &table_circuits[table];

            let stark_proof = &all_proof.stark_proofs[table];
            let original_degree_bits = stark_proof.proof.recover_degree_bits(config);

            let shrunk_proof = table_circuit.shrink(stark_proof, &all_proof.ctl_challenges)?;
            root_inputs.set_target(
                self.root.index_verifier_data[table],
                F::from_canonical_u8(*index_verifier_data),
            );
            root_inputs.set_proof_with_pis_target(&self.root.proof_with_pis[table], &shrunk_proof);

            check_abort_signal(abort_signal.clone())?;
        }

        root_inputs.set_verifier_data_target(
            &self.root.cyclic_vk,
            &self.aggregation.circuit.verifier_only,
        );

        set_public_value_targets(
            &mut root_inputs,
            &self.root.public_values,
            &all_proof.public_values,
        )
        .map_err(|_| {
            anyhow::Error::msg("Invalid conversion when setting public values targets.")
        })?;

        let root_proof = self.root.circuit.prove(root_inputs)?;

        Ok((root_proof, all_proof.public_values))
    }

    pub fn verify_root(&self, agg_proof: ProofWithPublicInputs<F, C, D>) -> anyhow::Result<()> {
        self.root.circuit.verify(agg_proof)
    }

    /// Create an aggregation proof, combining two contiguous proofs into a single one. The combined
    /// proofs can either be transaction (aka root) proofs, or other aggregation proofs, as long as
    /// their states are contiguous, meaning that the final state of the left child proof is the initial
    /// state of the right child proof.
    ///
    /// While regular transaction proofs can only assert validity of a single transaction, aggregation
    /// proofs can cover an arbitrary range, up to an entire block with all its transactions.
    ///
    /// # Arguments
    ///
    /// - `lhs_is_agg`: a boolean indicating whether the left child proof is an aggregation proof or
    /// a regular transaction proof.
    /// - `lhs_proof`: the left child proof.
    /// - `lhs_public_values`: the public values associated to the right child proof.
    /// - `rhs_is_agg`: a boolean indicating whether the right child proof is an aggregation proof or
    /// a regular transaction proof.
    /// - `rhs_proof`: the right child proof.
    /// - `rhs_public_values`: the public values associated to the right child proof.
    ///
    /// # Outputs
    ///
    /// This method outputs a tuple of [`ProofWithPublicInputs<F, C, D>`] and its [`PublicValues`]. Only
    /// the proof with public inputs is necessary for a verifier to assert correctness of the computation,
    /// but the public values are output for the prover convenience, as these are necessary during proof
    /// aggregation.
    pub fn prove_aggregation(
        &self,
        lhs_is_agg: bool,
        lhs_proof: &ProofWithPublicInputs<F, C, D>,
        lhs_public_values: PublicValues,
        rhs_is_agg: bool,
        rhs_proof: &ProofWithPublicInputs<F, C, D>,
        rhs_public_values: PublicValues,
    ) -> anyhow::Result<(ProofWithPublicInputs<F, C, D>, PublicValues)> {
        let mut agg_inputs = PartialWitness::new();

        agg_inputs.set_bool_target(self.aggregation.lhs.is_agg, lhs_is_agg);
        agg_inputs.set_proof_with_pis_target(&self.aggregation.lhs.agg_proof, lhs_proof);
        agg_inputs.set_proof_with_pis_target(&self.aggregation.lhs.evm_proof, lhs_proof);

        agg_inputs.set_bool_target(self.aggregation.rhs.is_agg, rhs_is_agg);
        agg_inputs.set_proof_with_pis_target(&self.aggregation.rhs.agg_proof, rhs_proof);
        agg_inputs.set_proof_with_pis_target(&self.aggregation.rhs.evm_proof, rhs_proof);

        agg_inputs.set_verifier_data_target(
            &self.aggregation.cyclic_vk,
            &self.aggregation.circuit.verifier_only,
        );

        // Aggregates both `PublicValues` from the provided proofs into a single one.
        let agg_public_values = PublicValues {
            trie_roots_before: lhs_public_values.trie_roots_before,
            trie_roots_after: rhs_public_values.trie_roots_after,
            extra_block_data: ExtraBlockData {
                checkpoint_state_trie_root: lhs_public_values
                    .extra_block_data
                    .checkpoint_state_trie_root,
                txn_number_before: lhs_public_values.extra_block_data.txn_number_before,
                txn_number_after: rhs_public_values.extra_block_data.txn_number_after,
                gas_used_before: lhs_public_values.extra_block_data.gas_used_before,
                gas_used_after: rhs_public_values.extra_block_data.gas_used_after,
            },
            block_metadata: rhs_public_values.block_metadata,
            block_hashes: rhs_public_values.block_hashes,
        };

        set_public_value_targets(
            &mut agg_inputs,
            &self.aggregation.public_values,
            &agg_public_values,
        )
        .map_err(|_| {
            anyhow::Error::msg("Invalid conversion when setting public values targets.")
        })?;

        let aggregation_proof = self.aggregation.circuit.prove(agg_inputs)?;
        Ok((aggregation_proof, agg_public_values))
    }

    pub fn verify_aggregation(
        &self,
        agg_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        self.aggregation.circuit.verify(agg_proof.clone())?;
        check_cyclic_proof_verifier_data(
            agg_proof,
            &self.aggregation.circuit.verifier_only,
            &self.aggregation.circuit.common,
        )
    }

    /// Create a final block proof, once all transactions of a given block have been combined into a
    /// single aggregation proof.
    ///
    /// Block proofs can either be generated as standalone, or combined with a previous block proof
    /// to assert validity of a range of blocks.
    ///
    /// # Arguments
    ///
    /// - `opt_parent_block_proof`: an optional parent block proof. Passing one will generate a proof of
    /// validity for both the block range covered by the previous proof and the current block.
    /// - `agg_root_proof`: the final aggregation proof containing all transactions within the current block.
    /// - `public_values`: the public values associated to the aggregation proof.
    ///
    /// # Outputs
    ///
    /// This method outputs a tuple of [`ProofWithPublicInputs<F, C, D>`] and its [`PublicValues`]. Only
    /// the proof with public inputs is necessary for a verifier to assert correctness of the computation.
    pub fn prove_block(
        &self,
        opt_parent_block_proof: Option<&ProofWithPublicInputs<F, C, D>>,
        agg_root_proof: &ProofWithPublicInputs<F, C, D>,
        public_values: PublicValues,
    ) -> anyhow::Result<(ProofWithPublicInputs<F, C, D>, PublicValues)> {
        let mut block_inputs = PartialWitness::new();

        block_inputs.set_bool_target(
            self.block.has_parent_block,
            opt_parent_block_proof.is_some(),
        );
        if let Some(parent_block_proof) = opt_parent_block_proof {
            block_inputs
                .set_proof_with_pis_target(&self.block.parent_block_proof, parent_block_proof);
        } else {
            if public_values.trie_roots_before.state_root
                != public_values.extra_block_data.checkpoint_state_trie_root
            {
                return Err(anyhow::Error::msg(format!(
                    "Inconsistent pre-state for first block {:?} with checkpoint state {:?}.",
                    public_values.trie_roots_before.state_root,
                    public_values.extra_block_data.checkpoint_state_trie_root,
                )));
            }

            // Initialize some public inputs for correct connection between the checkpoint block and the current one.
            let mut nonzero_pis = HashMap::new();

            // Initialize the checkpoint block roots before, and state root after.
            let state_trie_root_before_keys = 0..TrieRootsTarget::HASH_SIZE;
            for (key, &value) in state_trie_root_before_keys
                .zip_eq(&h256_limbs::<F>(public_values.trie_roots_before.state_root))
            {
                nonzero_pis.insert(key, value);
            }
            let txn_trie_root_before_keys =
                TrieRootsTarget::HASH_SIZE..TrieRootsTarget::HASH_SIZE * 2;
            for (key, &value) in txn_trie_root_before_keys.clone().zip_eq(&h256_limbs::<F>(
                public_values.trie_roots_before.transactions_root,
            )) {
                nonzero_pis.insert(key, value);
            }
            let receipts_trie_root_before_keys =
                TrieRootsTarget::HASH_SIZE * 2..TrieRootsTarget::HASH_SIZE * 3;
            for (key, &value) in receipts_trie_root_before_keys
                .clone()
                .zip_eq(&h256_limbs::<F>(
                    public_values.trie_roots_before.receipts_root,
                ))
            {
                nonzero_pis.insert(key, value);
            }
            let state_trie_root_after_keys =
                TrieRootsTarget::SIZE..TrieRootsTarget::SIZE + TrieRootsTarget::HASH_SIZE;
            for (key, &value) in state_trie_root_after_keys
                .zip_eq(&h256_limbs::<F>(public_values.trie_roots_before.state_root))
            {
                nonzero_pis.insert(key, value);
            }

            // Initialize the checkpoint state root extra data.
            let checkpoint_state_trie_keys =
                TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE
                    ..TrieRootsTarget::SIZE * 2
                        + BlockMetadataTarget::SIZE
                        + BlockHashesTarget::SIZE
                        + 8;
            for (key, &value) in checkpoint_state_trie_keys.zip_eq(&h256_limbs::<F>(
                public_values.extra_block_data.checkpoint_state_trie_root,
            )) {
                nonzero_pis.insert(key, value);
            }

            // Initialize checkpoint block hashes.
            // These will be all zeros the initial genesis checkpoint.
            let block_hashes_keys = TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE
                ..TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE
                    - 8;

            for i in 0..public_values.block_hashes.prev_hashes.len() - 1 {
                let targets = h256_limbs::<F>(public_values.block_hashes.prev_hashes[i]);
                for j in 0..8 {
                    nonzero_pis.insert(block_hashes_keys.start + 8 * (i + 1) + j, targets[j]);
                }
            }
            let block_hashes_current_start =
                TrieRootsTarget::SIZE * 2 + BlockMetadataTarget::SIZE + BlockHashesTarget::SIZE - 8;
            let cur_targets = h256_limbs::<F>(public_values.block_hashes.prev_hashes[255]);
            for i in 0..8 {
                nonzero_pis.insert(block_hashes_current_start + i, cur_targets[i]);
            }

            // Initialize the checkpoint block number.
            // Subtraction would result in an invalid proof for genesis, but we shouldn't try proving this block anyway.
            let block_number_key = TrieRootsTarget::SIZE * 2 + 6;
            nonzero_pis.insert(
                block_number_key,
                F::from_canonical_u64(public_values.block_metadata.block_number.low_u64() - 1),
            );

            block_inputs.set_proof_with_pis_target(
                &self.block.parent_block_proof,
                &cyclic_base_proof(
                    &self.block.circuit.common,
                    &self.block.circuit.verifier_only,
                    nonzero_pis,
                ),
            );
        }

        block_inputs.set_proof_with_pis_target(&self.block.agg_root_proof, agg_root_proof);

        block_inputs
            .set_verifier_data_target(&self.block.cyclic_vk, &self.block.circuit.verifier_only);

        // This is basically identical to this block public values, apart from the `trie_roots_before`
        // that may come from the previous proof, if any.
        let block_public_values = PublicValues {
            trie_roots_before: opt_parent_block_proof
                .map(|p| TrieRoots::from_public_inputs(&p.public_inputs[0..TrieRootsTarget::SIZE]))
                .unwrap_or(public_values.trie_roots_before),
            ..public_values
        };

        set_public_value_targets(
            &mut block_inputs,
            &self.block.public_values,
            &block_public_values,
        )
        .map_err(|_| {
            anyhow::Error::msg("Invalid conversion when setting public values targets.")
        })?;

        let block_proof = self.block.circuit.prove(block_inputs)?;
        Ok((block_proof, block_public_values))
    }

    pub fn verify_block(&self, block_proof: &ProofWithPublicInputs<F, C, D>) -> anyhow::Result<()> {
        self.block.circuit.verify(block_proof.clone())?;
        check_cyclic_proof_verifier_data(
            block_proof,
            &self.block.circuit.verifier_only,
            &self.block.circuit.common,
        )
    }
}

/// A map between initial degree sizes and their associated shrinking recursion circuits.
#[derive(Eq, PartialEq, Debug)]
pub struct RecursiveCircuitsForTable<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    /// A map from `log_2(height)` to a chain of shrinking recursion circuits starting at that
    /// height.
    pub by_stark_size: BTreeMap<usize, RecursiveCircuitsForTableSize<F, C, D>>,
}

impl<F, C, const D: usize> RecursiveCircuitsForTable<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    fn to_buffer(
        &self,
        buffer: &mut Vec<u8>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        buffer.write_usize(self.by_stark_size.len())?;
        for (&size, table) in &self.by_stark_size {
            buffer.write_usize(size)?;
            table.to_buffer(buffer, gate_serializer, generator_serializer)?;
        }
        Ok(())
    }

    fn from_buffer(
        buffer: &mut Buffer,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let length = buffer.read_usize()?;
        let mut by_stark_size = BTreeMap::new();
        for _ in 0..length {
            let key = buffer.read_usize()?;
            let table = RecursiveCircuitsForTableSize::from_buffer(
                buffer,
                gate_serializer,
                generator_serializer,
            )?;
            by_stark_size.insert(key, table);
        }
        Ok(Self { by_stark_size })
    }

    fn new<S: Stark<F, D>>(
        table: Table,
        stark: &S,
        degree_bits_range: Range<usize>,
        all_ctls: &[CrossTableLookup<F>],
        stark_config: &StarkConfig,
    ) -> Self {
        let by_stark_size = degree_bits_range
            .map(|degree_bits| {
                (
                    degree_bits,
                    RecursiveCircuitsForTableSize::new::<S>(
                        table,
                        stark,
                        degree_bits,
                        all_ctls,
                        stark_config,
                    ),
                )
            })
            .collect();
        Self { by_stark_size }
    }

    /// For each initial `degree_bits`, get the final circuit at the end of that shrinking chain.
    /// Each of these final circuits should have degree `THRESHOLD_DEGREE_BITS`.
    fn final_circuits(&self) -> Vec<&CircuitData<F, C, D>> {
        self.by_stark_size
            .values()
            .map(|chain| {
                chain
                    .shrinking_wrappers
                    .last()
                    .map(|wrapper| &wrapper.circuit)
                    .unwrap_or(&chain.initial_wrapper.circuit)
            })
            .collect()
    }
}

/// A chain of shrinking wrapper circuits, ending with a final circuit with `degree_bits`
/// `THRESHOLD_DEGREE_BITS`.
#[derive(Eq, PartialEq, Debug)]
pub struct RecursiveCircuitsForTableSize<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    initial_wrapper: StarkWrapperCircuit<F, C, D>,
    shrinking_wrappers: Vec<PlonkWrapperCircuit<F, C, D>>,
}

impl<F, C, const D: usize> RecursiveCircuitsForTableSize<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    pub fn to_buffer(
        &self,
        buffer: &mut Vec<u8>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        buffer.write_usize(self.shrinking_wrappers.len())?;
        if !self.shrinking_wrappers.is_empty() {
            buffer.write_common_circuit_data(
                &self.shrinking_wrappers[0].circuit.common,
                gate_serializer,
            )?;
        }
        for wrapper in &self.shrinking_wrappers {
            buffer.write_prover_only_circuit_data(
                &wrapper.circuit.prover_only,
                generator_serializer,
                &wrapper.circuit.common,
            )?;
            buffer.write_verifier_only_circuit_data(&wrapper.circuit.verifier_only)?;
            buffer.write_target_proof_with_public_inputs(&wrapper.proof_with_pis_target)?;
        }
        self.initial_wrapper
            .to_buffer(buffer, gate_serializer, generator_serializer)?;
        Ok(())
    }

    pub fn from_buffer(
        buffer: &mut Buffer,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<Self> {
        let length = buffer.read_usize()?;
        let mut shrinking_wrappers = Vec::with_capacity(length);
        if length != 0 {
            let common = buffer.read_common_circuit_data(gate_serializer)?;

            for _ in 0..length {
                let prover_only =
                    buffer.read_prover_only_circuit_data(generator_serializer, &common)?;
                let verifier_only = buffer.read_verifier_only_circuit_data()?;
                let proof_with_pis_target = buffer.read_target_proof_with_public_inputs()?;
                shrinking_wrappers.push(PlonkWrapperCircuit {
                    circuit: CircuitData {
                        common: common.clone(),
                        prover_only,
                        verifier_only,
                    },
                    proof_with_pis_target,
                })
            }
        };

        let initial_wrapper =
            StarkWrapperCircuit::from_buffer(buffer, gate_serializer, generator_serializer)?;

        Ok(Self {
            initial_wrapper,
            shrinking_wrappers,
        })
    }

    fn new<S: Stark<F, D>>(
        table: Table,
        stark: &S,
        degree_bits: usize,
        all_ctls: &[CrossTableLookup<F>],
        stark_config: &StarkConfig,
    ) -> Self {
        let initial_wrapper = recursive_stark_circuit(
            table,
            stark,
            degree_bits,
            all_ctls,
            stark_config,
            &shrinking_config(),
            THRESHOLD_DEGREE_BITS,
        );
        let mut shrinking_wrappers = vec![];

        // Shrinking recursion loop.
        loop {
            let last = shrinking_wrappers
                .last()
                .map(|wrapper: &PlonkWrapperCircuit<F, C, D>| &wrapper.circuit)
                .unwrap_or(&initial_wrapper.circuit);
            let last_degree_bits = last.common.degree_bits();
            assert!(last_degree_bits >= THRESHOLD_DEGREE_BITS);
            if last_degree_bits == THRESHOLD_DEGREE_BITS {
                break;
            }

            let mut builder = CircuitBuilder::new(shrinking_config());
            let proof_with_pis_target = builder.add_virtual_proof_with_pis(&last.common);
            let last_vk = builder.constant_verifier_data(&last.verifier_only);
            builder.verify_proof::<C>(&proof_with_pis_target, &last_vk, &last.common);
            builder.register_public_inputs(&proof_with_pis_target.public_inputs); // carry PIs forward
            add_common_recursion_gates(&mut builder);
            let circuit = builder.build::<C>();

            assert!(
                circuit.common.degree_bits() < last_degree_bits,
                "Couldn't shrink to expected recursion threshold of 2^{}; stalled at 2^{}",
                THRESHOLD_DEGREE_BITS,
                circuit.common.degree_bits()
            );
            shrinking_wrappers.push(PlonkWrapperCircuit {
                circuit,
                proof_with_pis_target,
            });
        }

        Self {
            initial_wrapper,
            shrinking_wrappers,
        }
    }

    pub fn shrink(
        &self,
        stark_proof_with_metadata: &StarkProofWithMetadata<F, C, D>,
        ctl_challenges: &GrandProductChallengeSet<F>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        let mut proof = self
            .initial_wrapper
            .prove(stark_proof_with_metadata, ctl_challenges)?;
        for wrapper_circuit in &self.shrinking_wrappers {
            proof = wrapper_circuit.prove(&proof)?;
        }
        Ok(proof)
    }
}

/// Our usual recursion threshold is 2^12 gates, but for these shrinking circuits, we use a few more
/// gates for a constant inner VK and for public inputs. This pushes us over the threshold to 2^13.
/// As long as we're at 2^13 gates, we might as well use a narrower witness.
fn shrinking_config() -> CircuitConfig {
    CircuitConfig {
        num_routed_wires: 40,
        ..CircuitConfig::standard_recursion_config()
    }
}
