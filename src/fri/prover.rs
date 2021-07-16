use crate::field::extension_field::{flatten, unflatten, Extendable};
use crate::field::field::Field;
use crate::field::interpolation::{barycentric_weights, interpolate};
use crate::fri::FriConfig;
use crate::hash::hash_n_to_1;
use crate::merkle_tree::MerkleTree;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::{FriInitialTreeProof, FriProof, FriQueryRound, FriQueryStep, Hash};
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

/// Builds a FRI proof.
pub fn fri_proof<F: Field + Extendable<D>, const D: usize>(
    initial_merkle_trees: &[&MerkleTree<F>],
    // Coefficients of the polynomial on which the LDT is performed. Only the first `1/rate` coefficients are non-zero.
    lde_polynomial_coeffs: &PolynomialCoeffs<F::Extension>,
    // Evaluation of the polynomial on the large domain.
    lde_polynomial_values: &PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> FriProof<F, D> {
    let n = lde_polynomial_values.values.len();
    assert_eq!(lde_polynomial_coeffs.coeffs.len(), n);

    // Commit phase
    let (trees, final_coeffs) = fri_committed_trees(
        lde_polynomial_coeffs,
        lde_polynomial_values,
        challenger,
        config,
    );

    // PoW phase
    let current_hash = challenger.get_hash();
    let pow_witness = fri_proof_of_work(current_hash, config);

    // Query phase
    let query_round_proofs =
        fri_prover_query_rounds(initial_merkle_trees, &trees, challenger, n, config);

    FriProof {
        commit_phase_merkle_roots: trees.iter().map(|t| t.root).collect(),
        query_round_proofs,
        final_poly: final_coeffs,
        pow_witness,
    }
}

fn fri_committed_trees<F: Field + Extendable<D>, const D: usize>(
    polynomial_coeffs: &PolynomialCoeffs<F::Extension>,
    polynomial_values: &PolynomialValues<F::Extension>,
    challenger: &mut Challenger<F>,
    config: &FriConfig,
) -> (Vec<MerkleTree<F>>, PolynomialCoeffs<F::Extension>) {
    let mut values = polynomial_values.clone();

    let mut trees = Vec::new();

    let mut domain =
        F::coset_two_adic_subgroup(log2_strict(polynomial_values.len()), F::coset_shift());
    let num_reductions = config.reduction_arity_bits.len();
    for i in 0..num_reductions {
        let arity = 1 << config.reduction_arity_bits[i];

        reverse_index_bits_in_place(&mut values.values);
        let tree = MerkleTree::new(
            values
                .values
                .chunks(arity)
                .map(|chunk: &[F::Extension]| flatten(chunk))
                .collect(),
            false,
        );
        reverse_index_bits_in_place(&mut values.values);

        challenger.observe_hash(&tree.root);
        trees.push(tree);

        let beta = challenger.get_extension_challenge();
        let n = values.len();
        debug_assert_eq!(n % arity, 0);
        let roots_of_unity = F::Extension::two_adic_subgroup(config.reduction_arity_bits[i]);
        values = values
            .values
            .iter()
            .take(n / arity)
            .zip(&domain)
            .enumerate()
            .map(|(i, (&v, &x))| {
                let points = (0..arity)
                    .map(|j| {
                        (
                            roots_of_unity[j] * x.into(),
                            values.values[i + j * n / arity],
                        )
                    })
                    .collect::<Vec<_>>();
                let weights = barycentric_weights(&points);
                interpolate(&points, beta, &weights)
            })
            .collect::<Vec<_>>()
            .into();

        domain = domain
            .into_iter()
            .take(n / arity)
            .map(|x| x.exp(arity as u64))
            .collect();
    }

    let coeffs = values.coset_ifft(
        F::coset_shift()
            .exp_power_of_2(config.reduction_arity_bits.iter().copied().sum())
            .into(),
    );

    challenger.observe_extension_elements(&coeffs.coeffs);
    (trees, coeffs)
}

fn fri_proof_of_work<F: Field>(current_hash: Hash<F>, config: &FriConfig) -> F {
    (0u64..)
        .find(|&i| {
            hash_n_to_1(
                current_hash
                    .elements
                    .iter()
                    .copied()
                    .chain(Some(F::from_canonical_u64(i)))
                    .collect(),
                false,
            )
            .to_canonical_u64()
            .leading_zeros()
                >= config.proof_of_work_bits + F::ORDER.leading_zeros()
        })
        .map(F::from_canonical_u64)
        .expect("Proof of work failed.")
}

fn fri_prover_query_rounds<F: Field + Extendable<D>, const D: usize>(
    initial_merkle_trees: &[&MerkleTree<F>],
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> Vec<FriQueryRound<F, D>> {
    (0..config.num_query_rounds)
        .map(|_| fri_prover_query_round(initial_merkle_trees, trees, challenger, n, config))
        .collect()
}

fn fri_prover_query_round<F: Field + Extendable<D>, const D: usize>(
    initial_merkle_trees: &[&MerkleTree<F>],
    trees: &[MerkleTree<F>],
    challenger: &mut Challenger<F>,
    n: usize,
    config: &FriConfig,
) -> FriQueryRound<F, D> {
    let mut query_steps = Vec::new();
    let x = challenger.get_challenge();
    let mut x_index = x.to_canonical_u64() as usize % n;
    let initial_proof = initial_merkle_trees
        .iter()
        .map(|t| (t.get(x_index).to_vec(), t.prove(x_index)))
        .collect::<Vec<_>>();
    for (i, tree) in trees.iter().enumerate() {
        let arity_bits = config.reduction_arity_bits[i];
        let arity = 1 << arity_bits;
        let mut evals = unflatten(tree.get(x_index >> arity_bits));
        evals.remove(x_index & (arity - 1));
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
