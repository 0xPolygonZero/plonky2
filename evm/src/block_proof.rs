use std::collections::HashMap;

use ethereum_types::H256;
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::util::timing::TimingTree;

use crate::all_stark::AllStark;
use crate::arithmetic::arithmetic_stark::ArithmeticStark;
use crate::config::StarkConfig;
use crate::cpu::cpu_stark::CpuStark;
use crate::fixed_recursive_verifier::AllRecursiveCircuits;
use crate::generation::{GenerationInputs, TrieInputs};
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeStark;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::proof::{BlockMetadata, PublicValues, TrieRoots};
use crate::stark::Stark;

/// A proof of an EVM block.
#[derive(Debug, Clone)]
pub struct BlockProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    block_proof: ProofWithPublicInputs<F, C, D>,
    public_values: PublicValues,
}

/// Inputs necessary to generate a proof of a transaction.
#[derive(Debug, Clone)]
pub struct TxnInput {
    pub signed_txn: Vec<u8>,
    pub tries: TrieInputs,
    pub contract_code: HashMap<H256, Vec<u8>>,
}

impl<F, C, const D: usize> AllRecursiveCircuits<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
    [(); ArithmeticStark::<F, D>::COLUMNS]:,
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakSpongeStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
{
    pub fn prove_evm_block(
        &self,
        txns: Vec<TxnInput>,
        block_metadata: BlockMetadata,
        public_values: PublicValues,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
        timing: &mut TimingTree,
    ) -> anyhow::Result<BlockProof<F, C, D>> {
        let to_gen_inps = |txn: TxnInput| -> GenerationInputs {
            GenerationInputs {
                signed_txns: vec![txn.signed_txn],
                tries: txn.tries,
                contract_code: txn.contract_code,
                block_metadata: block_metadata.clone(),
                addresses: vec![],
            }
        };
        let txn_proofs = txns
            .into_iter() // TODO: Parallelize?
            .map(|txn| self.prove_root(all_stark, config, to_gen_inps(txn), timing))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let PublicValues {
            mut trie_roots_before,
            trie_roots_after,
            block_metadata,
        } = public_values.clone();

        let mut agg_proof = None;
        for (i, (proof, pv)) in txn_proofs.into_iter().enumerate() {
            assert_eq!(pv.trie_roots_before, trie_roots_before); // TODO: Do this in circuit.
            assert_eq!(pv.block_metadata, block_metadata); // TODO: Do this in circuit.
            trie_roots_before = pv.trie_roots_after;
            agg_proof = if i == 0 {
                Some(self.prove_aggregation(false, &proof, false, &proof)?)
            } else {
                Some(self.prove_aggregation(false, &proof, true, &agg_proof.unwrap())?)
            };
        }
        assert_eq!(trie_roots_before, trie_roots_after); // TODO: Do this in circuit.

        let agg_proof = agg_proof.expect("Empty block?"); // TODO: Should empty blocks be allowed?

        Ok(BlockProof {
            block_proof: agg_proof,
            public_values,
        })
    }

    pub fn verify_evm_block(&self, block_proof: &BlockProof<F, C, D>) -> anyhow::Result<()> {
        // TODO: Add public values checks
        self.verify_aggregation(&block_proof.block_proof)
    }
}
