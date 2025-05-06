use anyhow::ensure;

use plonky2_field::extension::{flatten, Extendable, FieldExtension};

use crate::util::{reducing::ReducingFactor, reverse_bits, log2_strict};
use crate::hash::{hash_types::RichField, merkle_proofs::verify_merkle_proof_to_cap, merkle_tree::MerkleCap};

use crate::plonk::config::GenericConfig;
use crate::fri::{proof::FriChallenges, 
                FriParams,
                verifier::PrecomputedReducedOpenings,
                structure::{FriBatchInfo, FriInstanceInfo, FriOpenings}}; 
use crate::boil::{boil_prover::{AccProof, AccInfo, BoilQueryProof}, QN};

pub static mut IVCDEBUG_OB_VER:bool = false; 

pub fn validate_acc_proof_shape<F, C, const D: usize>(
    proof: &AccProof<F, C::Hasher, D>,
    instances: &[FriInstanceInfo<F, D>],
    params: &FriParams,
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let AccProof { merkle_cap, ood_answer: _ood, ind_answers, qproofs: query_proofs } = proof;
    let cap_height = params.config.cap_height;

    ensure!(params.hiding == false);
    ensure!(merkle_cap.height() == cap_height);
    ensure!(ind_answers.len() == QN);
    
    for round_proof in query_proofs {
        let BoilQueryProof { base_evals_proofs, ext_evals_proofs } = round_proof;

        let oracle_count = base_evals_proofs.len();
        let mut leaf_len = vec![0; oracle_count];
        for inst in instances {
            ensure!(oracle_count == inst.oracles.len());
            for (i, oracle) in inst.oracles.iter().enumerate() {
                leaf_len[i] += oracle.num_polys;
            }
        }
        for (i, (leaf, merkle_proof)) in base_evals_proofs.iter().enumerate() {
            ensure!(leaf.len() == leaf_len[i]);
            ensure!(merkle_proof.len() + cap_height == params.lde_bits());
        }
        for (_, (_leaf, merkle_proof)) in ext_evals_proofs.iter().enumerate() {
            // ensure!(leaf.len() == D);
            ensure!(merkle_proof.len() + cap_height == params.lde_bits());
        }
    }
    Ok(())
}


pub fn verify_acc_proof<F, C, const D: usize>(
    proof: &AccProof<F, C::Hasher, D>,
    challenges: &FriChallenges<F, D>,
    accs: &[&AccInfo<F, C::Hasher, D>],
    fri_instance: &FriInstanceInfo<F, D>,
    fri_openings: &FriOpenings<F, D>,
    fri_initial_merkle_caps: &[MerkleCap<F, C::Hasher>],
    fri_params: &FriParams,
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    println!("! boil:: verify_acc_proof()");

    validate_acc_proof_shape::<F, C, D>(proof, &[fri_instance.clone()], fri_params)?;

    let ind_points = challenges.fri_query_indices.clone();
    let n = fri_params.lde_size();
    let log_n = log2_strict(n);
    let mut alpha = ReducingFactor::new(challenges.fri_alpha);
    let reduced_openings =
        PrecomputedReducedOpenings::from_os_and_alpha(fri_openings, challenges.fri_alpha);

    if unsafe { IVCDEBUG_OB_VER } {
        println!("***\n DEBUG INFO \n***");
        println!("...polys degree = {}, lde deggree = {}", fri_params.degree_bits, fri_params.lde_bits());
        println!("...#fri_batches to accumulate = {}", fri_instance.batches.len());
        fri_instance.batches.iter().for_each(|x| {
            println!("    point: {}, polys in batch: {}", x.point, x.polynomials.len());
        });
        println!{"...#accumulators to accumulate = {}", accs.len()};
        println!("alpha = {}", challenges.fri_alpha);
    }

    for ((i, &x_index), query_proof) in ind_points.iter().enumerate().zip(&proof.qproofs) {
        let subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
            * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
    
        let mut sum = F::Extension::from_basefield(F::ZERO);
        for (batch, reduced_openings) in fri_instance.batches
            .iter()
            .zip(&reduced_openings.reduced_openings_at_point)
        {
            let FriBatchInfo { point, polynomials } = batch;
            let evals = polynomials
                .iter()
                .map(|p| {
                    let oracle_evals = &query_proof.base_evals_proofs[p.oracle_index].0;
                    let v1:F = oracle_evals[p.polynomial_index];
                    v1
                })
                .map(F::Extension::from_basefield);
            let reduced_evals = alpha.reduce(evals);
            let numerator = reduced_evals - *reduced_openings;
            let denominator = F::Extension::from_basefield(subgroup_x) - *point;
            sum = alpha.shift(sum);
            sum += numerator / denominator;
        }

        if unsafe { IVCDEBUG_OB_VER } {
            println!("\n*** chal #{}/{} -> x_index={} = {}", i + 1, QN, x_index, subgroup_x);
        }
        for (acc_info, oracle_eval) in accs.iter().zip(&query_proof.ext_evals_proofs) {
            let ev = oracle_eval.0;
            let mut vals = vec![];
            acc_info.ind_samples.iter() 
                .zip(&acc_info.ind_answers)
                .for_each(|(a_point, a_opening)| {
                    // ev - *opening 
                    let y_index = a_point.to_canonical_u64() as usize % n;
                    let subgroup_y = F::MULTIPLICATIVE_GROUP_GENERATOR
                        * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(y_index, log_n) as u64);
                    let isv = (ev - *a_opening) / (F::Extension::from_basefield(subgroup_x) - F::Extension::from_basefield(subgroup_y)); 
                    if unsafe { IVCDEBUG_OB_VER } {
                        println!(".......... quotient/({}) -> {}", y_index, isv);
                    }
                    vals.push(isv);
                });
            let osv = (ev - acc_info.ood_answer) / (F::Extension::from_basefield(subgroup_x) - acc_info.ood_sample); 
            vals.push(osv);
            let sum2 = alpha.reduce(vals.iter());
            sum = alpha.shift(sum);
            sum += sum2;
            if unsafe { IVCDEBUG_OB_VER } {
                println!(".......... quotient/(ood) -> {}", osv);
            }
        }
        if unsafe { IVCDEBUG_OB_VER } {
            println!("*** claimed eval = {:?}", proof.ind_answers[i]);
            println!("*** computed eval = {:?}", sum);
        }

        ensure!(proof.ind_answers[i] == sum);
    }

    let AccProof { merkle_cap: _, ood_answer: _, ind_answers: _, qproofs } = proof;
    for (&x_index, query_round) in challenges
        .fri_query_indices
        .iter()
        .zip(qproofs) {
            let BoilQueryProof { base_evals_proofs, ext_evals_proofs} = query_round;
            if unsafe { IVCDEBUG_OB_VER } {
                println!("... checking next base_evals merkle proofs");
            }
            for ((evals, merkle_proof), cap) in base_evals_proofs.iter().zip(fri_initial_merkle_caps) {
                verify_merkle_proof_to_cap::<F, C::Hasher>(evals.clone(), x_index, cap, merkle_proof)?;
            }
            if unsafe { IVCDEBUG_OB_VER } {
                println!("... checking next ext_evals merkle proofs");
            }
            for ((evals, merkle_proof), acc) in ext_evals_proofs.iter().zip(accs) {
                let ev_v: Vec<F> = flatten(&[evals.clone()]);
                verify_merkle_proof_to_cap::<F, C::Hasher>(ev_v, x_index, &acc.merkle_cap, merkle_proof)?;
            }
    }

    Ok(())
}

