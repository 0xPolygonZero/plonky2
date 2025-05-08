// #[cfg(not(feature = "std"))]
// use alloc::vec;

use itertools::Itertools;
use plonky2_maybe_rayon::*;
use serde::{Deserialize, Serialize};
use crate::timed;
use crate::util::{
    timing::TimingTree,
    reducing::ReducingFactor, 
    reverse_bits, 
    reverse_index_bits_in_place, 
    log2_strict
};

use crate::field::{
    extension::{unflatten, Extendable, FieldExtension}, 
    polynomial::PolynomialCoeffs, 
    types::Field
};
use crate::hash::{
    hash_types::RichField, 
    merkle_proofs::MerkleProof,
    merkle_tree::{MerkleCap, MerkleTree},
};
use crate::iop::challenger::Challenger;
use crate::plonk::config::{GenericConfig, Hasher};
use crate::fri::oracle::PolynomialBatch;
use crate::fri::{structure::{FriBatchInfo, FriInstanceInfo}, FriParams};
use crate::boil::QN;
use std::time::Instant;

pub static mut IVCDEBUG_OB_PROV:bool = true;

// Describes the Accumulator
// Think of the Accumulator as a vector of polynomials of the form [F(X) - F(x_i) / (X - x_i)] 
// We can access these polynomials virtually via the polynomial F 
// and pairs (x_i, F(x_i))
//
#[derive(Debug)]
pub struct Acc<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    // Merklte Tree which represents F 
    pub merkle_tree: MerkleTree<F, H>,
    // the vector of coefficients of F
    pub polynomial_coeffs: PolynomialCoeffs<F::Extension>,
    // out-of-domain random point
    pub ood_sample: F::Extension,
    // claimed evaluation of F at ood point
    pub ood_answer: F::Extension, 
    // QN of in-domain random points
    pub ind_samples: Vec<F>,
    // evaluations of F at each of the in-domain random points
    pub ind_answers: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> Acc<F, H, D> {
    pub fn evaluate(
        &self,
        point: F::Extension,
        ood_quotient: bool,
        x_index: usize,
        log_n: usize
    ) {
        if ood_quotient == false {
            let subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
                * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
            let quotient = self.polynomial_coeffs
                    .divide_by_linear(F::Extension::from_basefield(subgroup_x));
            println!(".......... quotient/({}) -> {}", x_index, quotient.eval(point));
            let sv = 
                (self.polynomial_coeffs.eval(point) 
                - self.polynomial_coeffs.eval(F::Extension::from_basefield(subgroup_x)))
                / (point - F::Extension::from_basefield(subgroup_x));
            println!(".......... quotient/({}) -> {}", x_index, sv);
        } else {
            let quotient = self.polynomial_coeffs
                    .divide_by_linear(self.ood_sample);
            println!(".......... quotient/(ood) -> {}", quotient.eval(point));
            let sv = 
                (self.polynomial_coeffs.eval(point) 
                - self.polynomial_coeffs.eval(self.ood_sample))
                / (point - self.ood_sample);
            println!(".......... quotient/(ood) -> {}", sv);
        }
    }
}

// Succinct description of the Accumulator for the Verifier
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct AccInfo<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    // Caps of the Merkle Tree 
    pub merkle_cap: MerkleCap<F, H>,
    // same as in the struct Acc
    pub ood_sample: F::Extension,
    pub ood_answer: F::Extension, 
    pub ind_samples: Vec<F>,
    pub ind_answers: Vec<F::Extension>,
}

// Describes the accumulation proof
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct AccProof<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    // Merkle caps for a new accumulator
    pub merkle_cap: MerkleCap<F, H>,
    // claimed eval of a new F poly for ood random point
    pub ood_answer: F::Extension,
    // claimed evals of a new F poly for each the QN in-domain random points
    pub ind_answers: Vec<F::Extension>,
    // Merkle proof for each of the QN in-domain random points
    pub qproofs: Vec<BoilQueryProof<F, H, D>>,
}

// Describes proof for a single in-domain random point
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct BoilQueryProof<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    // Merkle proofs for evaluations of FRI batches (4 of them)
    pub base_evals_proofs: Vec<(Vec<F>, MerkleProof<F, H>)>,
    // Merkle proofs for evaluations of input accumulators (in case of IVC, there is only one acc)
    pub ext_evals_proofs: Vec<(F::Extension, MerkleProof<F, H>)>,
}

pub fn prove_accumulation<F, C, const D: usize>(
    accs: &[&Acc<F, C::Hasher, D>],
    fri_instance: &FriInstanceInfo<F, D>,
    fri_oracles: &[&PolynomialBatch<F,C,D>],
    fri_params: &FriParams,
    challenger: &mut Challenger<F, C::Hasher>,
    timing: &mut TimingTree,
) -> (Acc<F, C::Hasher, D>, AccProof<F, C::Hasher, D>) 
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{

    println!("! boil:: prove_accumulation()");

    // get new challenge
    let alpha = challenger.get_extension_challenge::<D>();
    let alpha_copy = alpha.clone();
    let mut alpha = ReducingFactor::new(alpha);

    // commit phase
    let mut lincom_poly = PolynomialCoeffs::empty();
    let n = fri_params.lde_size(); 
    let log_n = log2_strict(n);

    if unsafe { IVCDEBUG_OB_PROV } {
        println!("***\n DEBUG INFO:\n***");
        let now = Instant::now();
        println!("||| Elapsed (...): {}", now.elapsed().as_secs_f64());

        println!("... polys degree = {}, lde deggree = {}", fri_params.degree_bits, fri_params.lde_bits());
        println!("... #fri_batches to accumulate = {}", fri_instance.batches.len());
        fri_instance.batches.iter().for_each(|x| {
            println!("    point: {}, polys in batch: {}", x.point, x.polynomials.len());
        });
        println!{"... #accumulators to accumulate = {}", accs.len()};
        accs.iter().for_each(|x| {
            println!("... acc degree = {}", x.polynomial_coeffs.len());
        });
        println!("alpha = {}", alpha_copy);
    }

    // combine all the polynomials from commited batches with alpha
    for FriBatchInfo { point, polynomials } in &fri_instance.batches {
        let polys_coeff = polynomials.iter().map(|fri_poly| {
            &fri_oracles[fri_poly.oracle_index].polynomials[fri_poly.polynomial_index]
        });
        let composition_poly = timed!(
            timing,
            &format!("reduce batch of {} polynomials", polynomials.len()),
            alpha.reduce_polys_base(polys_coeff)
        );
        let mut quotient = composition_poly.divide_by_linear(*point);
        quotient.coeffs.push(F::Extension::ZERO); // pad back to power of two
        alpha.shift_poly(&mut lincom_poly);
        lincom_poly += quotient;
    }

    let mut debug_acc_polys = vec![]; 
    // combine all the polynomials from accumulators with alpha
    for acc in accs {
        assert!(acc.ind_samples.len() == QN); // all the accumulators must have the same shape
        let mut acc_polys = acc.ind_samples.par_iter().enumerate() 
            .map(|(_i, sample)| {
                let x_index = sample.to_canonical_u64() as usize % n;
                let subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
                    * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
                let mut cur = acc.polynomial_coeffs
                    .divide_by_linear(F::Extension::from_basefield(subgroup_x));
                cur.coeffs.push(F::Extension::ZERO);
                cur
            })
            .collect::<Vec<_>>();
        let mut ood_quotient = acc.polynomial_coeffs.divide_by_linear(acc.ood_sample);
        ood_quotient.coeffs.push(F::Extension::ZERO);
        acc_polys.push(ood_quotient);
        if unsafe { IVCDEBUG_OB_PROV } {
            debug_acc_polys = acc_polys.clone();
        }
        let acc_polys_reduced = alpha.reduce_polys2(acc_polys);
        alpha.shift_poly(&mut lincom_poly);
        lincom_poly += acc_polys_reduced;
    }

    let lde_lincom_poly = lincom_poly.lde(fri_params.config.rate_bits);
    let mut lde_final_values = timed!(
        timing,
        &format!("perform final FFT {}", lde_lincom_poly.len()),
        lde_lincom_poly.coset_fft(F::coset_shift().into())
    );


    reverse_index_bits_in_place(&mut lde_final_values.values);
    let leaves = lde_final_values.values
        .iter()
        .map(|&x| x.to_basefield_array().to_vec() )
        .collect();
    let tree = MerkleTree::<F, C::Hasher>::new(leaves, fri_params.config.cap_height);
    

    // Query phase
    challenger.observe_cap(&tree.cap);

    let ood_point = challenger.get_extension_challenge::<D>();
    let ood_eval = lincom_poly.eval(ood_point);
    let ind_points  = challenger.get_n_challenges(QN).into_iter().collect_vec();
    let ind_evals = ind_points
        .par_iter()
        .map(|rand| {
            let x_index = rand.to_canonical_u64() as usize % n;
            // let subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
            //     * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
            // let ev = lincom_poly.eval(
            //     F::Extension::from_basefield(subgroup_x)
            // );
            // ev
            lde_final_values.values[x_index]
        })
        .collect::<Vec<F::Extension>>();

    if unsafe { IVCDEBUG_OB_PROV } {
        for rand in &ind_points {
            let y_index = rand.to_canonical_u64() as usize % n;
            let subgroup_y = F::MULTIPLICATIVE_GROUP_GENERATOR
                * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(y_index, log_n) as u64);
            println!("\n*** chal = {} OR {}", y_index, subgroup_y);
            for acc in accs {
                for i in 0..QN {
                    let p = acc.ind_samples[i].to_canonical_u64() as usize % n; 
                    acc.evaluate(F::Extension::from_basefield(subgroup_y), false, p, log_n);
                    println!("... poly value = {}", debug_acc_polys[i].eval(F::Extension::from_basefield(subgroup_y)));
                }
                acc.evaluate(F::Extension::from_basefield(subgroup_y), true, 0, log_n);
                println!("... poly value = {}", debug_acc_polys[QN].eval(F::Extension::from_basefield(subgroup_y)));
            }
        }
    }

    let qproofs = ind_points
        .par_iter()
        .map(|rand| {
            let x_index = rand.to_canonical_u64() as usize % n;
            let base_evals_proofs = fri_oracles 
                .iter()
                .map(|&t| (t.merkle_tree.get(x_index).to_vec(), t.merkle_tree.prove(x_index)))
                .collect::<Vec<_>>();
            let ext_evals_proofs = accs
                .iter()
                .map(|&t| (unflatten(t.merkle_tree.get(x_index))[0], t.merkle_tree.prove(x_index)))
                .collect::<Vec<_>>();
            BoilQueryProof {
                base_evals_proofs,
                ext_evals_proofs,
            }
        })
        .collect::<Vec<BoilQueryProof<F, C::Hasher, D>>>();
   
    let ccap = tree.cap.clone();
    let new_acc = Acc {
        merkle_tree: tree,
        polynomial_coeffs: lincom_poly,
        ood_sample: ood_point,
        ood_answer: ood_eval, 
        ind_samples: ind_points.clone(),
        ind_answers: ind_evals.clone(),
    };
    let acc_proof = AccProof {
        merkle_cap: ccap,
        ood_answer: ood_eval, 
        ind_answers: ind_evals.clone(),
        qproofs,
    };


    // let indic = ind_points
    //     .iter()
    //     .map(|rand| {
    //         let x_index = rand.to_canonical_u64() as usize % n;
    //         x_index
    //     })
    //     .collect::<Vec<usize>>();
    // let sss: FriChallenges<F, D> = FriChallenges {
    //     fri_alpha: myalpha,
    //     fri_betas: Vec::new(),
    //     fri_pow_response: F::ZERO,
    //     fri_query_indices: indic, 
    // };

    (new_acc, acc_proof)
}

