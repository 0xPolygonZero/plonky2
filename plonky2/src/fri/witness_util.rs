use anyhow::Result;
use itertools::Itertools;

use crate::field::extension::Extendable;
use crate::fri::proof::{FriProof, FriProofTarget};
use crate::hash::hash_types::{HashOut, RichField};
use crate::hash::merkle_proofs::MerkleProof;
use crate::iop::witness::WitnessWrite;
use crate::plonk::config::AlgebraicHasher;

/// Set the targets in a `FriProofTarget` to their corresponding values in a `FriProof`.
pub fn set_fri_proof_target<F, W, H, const D: usize>(
    witness: &mut W,
    fri_proof_target: &FriProofTarget<D>,
    fri_proof: &FriProof<F, H, D>,
) -> Result<()>
where
    F: RichField + Extendable<D>,
    W: WitnessWrite<F> + ?Sized,
    H: AlgebraicHasher<F>,
{
    witness.set_target(fri_proof_target.pow_witness, fri_proof.pow_witness)?;

    assert_eq!(
        fri_proof_target.final_poly.0.len(),
        fri_proof.final_poly.coeffs.len(),
        "final poly"
    );
    for (&t, &x) in fri_proof_target
        .final_poly
        .0
        .iter()
        .zip_eq(&fri_proof.final_poly.coeffs)
    {
        witness.set_extension_target(t, x)?;
    }

    assert_eq!(
        fri_proof_target.commit_phase_merkle_caps.len(),
        fri_proof.commit_phase_merkle_caps.len(),
        "merkle caps"
    );
    for (t, x) in fri_proof_target
        .commit_phase_merkle_caps
        .iter()
        .zip_eq(&fri_proof.commit_phase_merkle_caps)
    {
        witness.set_cap_target(t, x)?;
    }

    assert_eq!(
        fri_proof_target.query_round_proofs.len(),
        fri_proof.query_round_proofs.len(),
        "query rounds"
    );
    for (qt, q) in fri_proof_target
        .query_round_proofs
        .iter()
        .zip_eq(&fri_proof.query_round_proofs)
    {
        assert!(
            (qt.initial_trees_proof.evals_proofs.len() == q.initial_trees_proof.evals_proofs.len())
                || (qt.initial_trees_proof.evals_proofs.len()
                    == q.initial_trees_proof.evals_proofs.len() + 1),
            "initial trees proof"
        );

        let cur_evals = if qt.initial_trees_proof.evals_proofs.len()
            == q.initial_trees_proof.evals_proofs.len() + 1
        {
            let mut tmp = q.initial_trees_proof.evals_proofs.clone();
            let l = qt.initial_trees_proof.evals_proofs
                [qt.initial_trees_proof.evals_proofs.len() - 1]
                .1
                .siblings
                .len();
            let dummy_proof = MerkleProof {
                siblings: vec![HashOut::default(); l],
            };
            tmp.push((vec![], dummy_proof));
            tmp
        } else {
            q.initial_trees_proof.evals_proofs.clone()
        };
        for (at, a) in qt
            .initial_trees_proof
            .evals_proofs
            .iter()
            .zip_eq(&cur_evals)
        {
            assert_eq!(at.0.len(), a.0.len(), "at");
            for (&t, &x) in at.0.iter().zip_eq(&a.0) {
                witness.set_target(t, x)?;
            }
            assert_eq!(at.1.siblings.len(), a.1.siblings.len(), "siblings");
            for (&t, &x) in at.1.siblings.iter().zip_eq(&a.1.siblings) {
                witness.set_hash_target(t, x)?;
            }
        }

        assert_eq!(qt.steps.len(), q.steps.len(), "steps");
        for (st, s) in qt.steps.iter().zip_eq(&q.steps) {
            assert_eq!(st.evals.len(), s.evals.len(), "evals");
            for (&t, &x) in st.evals.iter().zip_eq(&s.evals) {
                witness.set_extension_target(t, x)?;
            }

            assert_eq!(
                st.merkle_proof.siblings.len(),
                s.merkle_proof.siblings.len(),
                "merkle siblings"
            );
            for (&t, &x) in st
                .merkle_proof
                .siblings
                .iter()
                .zip_eq(&s.merkle_proof.siblings)
            {
                witness.set_hash_target(t, x)?;
            }
        }
    }

    Ok(())
}
