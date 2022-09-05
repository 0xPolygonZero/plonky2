use itertools::Itertools;
use maybe_rayon::*;
use plonky2_field::extension::{flatten, unflatten, Extendable};
use plonky2_field::polynomial::{PolynomialCoeffs, PolynomialValues};
use plonky2_util::reverse_index_bits_in_place;

use crate::fri::proof::{FriInitialTreeProof, FriProof, FriQueryRound, FriQueryStep};
use crate::fri::{FriConfig, FriParams};
use crate::hash::hash_types::{HashOut, RichField};
use crate::hash::merkle_tree::MerkleTree;
use crate::iop::challenger::Challenger;
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::plonk_common::reduce_with_powers;
use crate::timed;
use crate::util::timing::TimingTree;

/// Builds a FRI proof.
pub fn fri_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    initial_merkle_trees: &[&MerkleTree<F, C::Hasher>],
    // Coefficients of the polynomial on which the LDT is performed. Only the first `1/rate` coefficients are non-zero.
    lde_polynomial_coeffs: PolynomialCoeffs<F::Extension>,
    // Evaluation of the polynomial on the large domain.
    lde_polynomial_values: PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F, C::Hasher>,
    fri_params: &FriParams,
    timing: &mut TimingTree,
) -> FriProof<F, C::Hasher, D>
where
    [(); C::Hasher::HASH_SIZE]:,
{
    let n = lde_polynomial_values.len();
    assert_eq!(lde_polynomial_coeffs.len(), n);

    // Commit phase
    let (trees, final_coeffs) = timed!(
        timing,
        "fold codewords in the commitment phase",
        fri_committed_trees::<F, C, D>(
            lde_polynomial_coeffs,
            lde_polynomial_values,
            challenger,
            fri_params,
        )
    );

    // PoW phase
    let current_hash = challenger.get_hash();
    let pow_witness = timed!(
        timing,
        "find proof-of-work witness",
        fri_proof_of_work::<F, C, D>(current_hash, &fri_params.config)
    );

    // Query phase
    let query_round_proofs =
        fri_prover_query_rounds::<F, C, D>(initial_merkle_trees, &trees, challenger, n, fri_params);

    FriProof {
        commit_phase_merkle_caps: trees.iter().map(|t| t.cap.clone()).collect(),
        query_round_proofs,
        final_poly: final_coeffs,
        pow_witness,
    }
}

fn fri_committed_trees<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    mut coeffs: PolynomialCoeffs<F::Extension>,
    mut values: PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F, C::Hasher>,
    fri_params: &FriParams,
) -> (
    Vec<MerkleTree<F, C::Hasher>>,
    PolynomialCoeffs<F::Extension>,
)
where
    [(); C::Hasher::HASH_SIZE]:,
{
    let mut trees = Vec::new();

    let mut shift = F::MULTIPLICATIVE_GROUP_GENERATOR;
    for arity_bits in &fri_params.reduction_arity_bits {
        let arity = 1 << arity_bits;

        reverse_index_bits_in_place(&mut values.values);
        let chunked_values = values
            .values
            .par_chunks(arity)
            .map(|chunk: &[F::Extension]| flatten(chunk))
            .collect();
        let tree = MerkleTree::<F, C::Hasher>::new(chunked_values, fri_params.config.cap_height);

        challenger.observe_cap(&tree.cap);
        trees.push(tree);

        let beta = challenger.get_extension_challenge::<D>();
        // P(x) = sum_{i<r} x^i * P_i(x^r) becomes sum_{i<r} beta^i * P_i(x).
        coeffs = PolynomialCoeffs::new(
            coeffs
                .coeffs
                .par_chunks_exact(arity)
                .map(|chunk| reduce_with_powers(chunk, beta))
                .collect::<Vec<_>>(),
        );
        shift = shift.exp_u64(arity as u64);
        values = coeffs.coset_fft(shift.into())
    }

    // The coefficients being removed here should always be zero.
    coeffs
        .coeffs
        .truncate(coeffs.len() >> fri_params.config.rate_bits);

    challenger.observe_extension_elements(&coeffs.coeffs);
    (trees, coeffs)
}

fn fri_proof_of_work<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    current_hash: HashOut<F>,
    config: &FriConfig,
) -> F {
    (0..=F::NEG_ONE.to_canonical_u64())
        .into_par_iter()
        .find_any(|&i| {
            C::InnerHasher::hash_no_pad(
                &current_hash
                    .elements
                    .iter()
                    .copied()
                    .chain(Some(F::from_canonical_u64(i)))
                    .collect_vec(),
            )
            .elements[0]
                .to_canonical_u64()
                .leading_zeros()
                >= config.proof_of_work_bits + (64 - F::order().bits()) as u32
        })
        .map(F::from_canonical_u64)
        .expect("Proof of work failed. This is highly unlikely!")
}

fn fri_prover_query_rounds<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    initial_merkle_trees: &[&MerkleTree<F, C::Hasher>],
    trees: &[MerkleTree<F, C::Hasher>],
    challenger: &mut Challenger<F, C::Hasher>,
    n: usize,
    fri_params: &FriParams,
) -> Vec<FriQueryRound<F, C::Hasher, D>> {
    challenger
        .get_n_challenges(fri_params.config.num_query_rounds)
        .into_par_iter()
        .map(|rand| {
            let x_index = rand.to_canonical_u64() as usize % n;
            fri_prover_query_round::<F, C, D>(initial_merkle_trees, trees, x_index, fri_params)
        })
        .collect()
}

fn fri_prover_query_round<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    initial_merkle_trees: &[&MerkleTree<F, C::Hasher>],
    trees: &[MerkleTree<F, C::Hasher>],
    mut x_index: usize,
    fri_params: &FriParams,
) -> FriQueryRound<F, C::Hasher, D> {
    let mut query_steps = Vec::new();
    let initial_proof = initial_merkle_trees
        .iter()
        .map(|t| (t.get(x_index).to_vec(), t.prove(x_index)))
        .collect::<Vec<_>>();
    for (i, tree) in trees.iter().enumerate() {
        let arity_bits = fri_params.reduction_arity_bits[i];
        let evals = unflatten(tree.get(x_index >> arity_bits));
        let merkle_proof = tree.prove(x_index >> arity_bits);

        query_steps.push(FriQueryStep {
            evals,
            merkle_proof,
        });

        x_index >>= arity_bits;
    }
    FriQueryRound {
        initial_trees_proof: FriInitialTreeProof {
            evals_proofs: initial_proof,
        },
        steps: query_steps,
    }
}
