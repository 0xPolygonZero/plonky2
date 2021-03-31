use std::sync::Arc;
use std::thread;
use std::time::Instant;

use env_logger::Env;
use rayon::prelude::*;

use field::crandall_field::CrandallField;
use field::fft;

use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CircuitConfig;
use crate::field::field::Field;
use crate::gates::constant::ConstantGate;
use crate::gates::gmimc::GMiMCGate;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::witness::PartialWitness;

mod circuit_builder;
mod circuit_data;
mod constraint_polynomial;
mod field;
mod fri;
mod gadgets;
mod gates;
mod generator;
mod gmimc;
mod hash;
mod partition;
mod plonk_common;
mod polynomial;
mod proof;
mod prover;
mod recursive_verifier;
mod rescue;
mod target;
mod util;
mod verifier;
mod wire;
mod witness;

// 112 wire polys, 3 Z polys, 4 parts of quotient poly.
const PROVER_POLYS: usize = 113 + 3 + 4;

fn main() {
    // Set the default log filter. This can be overridden using the `RUST_LOG` environment variable,
    // e.g. `RUST_LOG=debug`.
    // We default to debug for now, since there aren't many logs anyway, but we should probably
    // change this to info or warn later.
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let overall_start = Instant::now();

    bench_prove::<CrandallField>();

    // bench_fft();
    println!();
    // bench_gmimc::<CrandallField>();

    let overall_duration = overall_start.elapsed();
    println!("Overall time: {:?}", overall_duration);

    // field_search()
}

fn bench_prove<F: Field>() {
    let mut gmimc_constants = [F::ZERO; GMIMC_ROUNDS];
    for i in 0..GMIMC_ROUNDS {
        gmimc_constants[i] = F::from_canonical_u64(GMIMC_CONSTANTS[i]);
    }
    let gmimc_gate = GMiMCGate::<F, GMIMC_ROUNDS>::with_constants(
        Arc::new(gmimc_constants));

    let config = CircuitConfig {
        num_wires: 120,
        num_routed_wires: 12,
        security_bits: 128,
        rate_bits: 3,
        num_checks: 3,
    };

    let mut builder = CircuitBuilder::<F>::new(config);

    for _ in 0..5000 {
        builder.add_gate_no_constants(gmimc_gate.clone());
    }

    builder.add_gate(ConstantGate::get(), vec![F::NEG_ONE]);

    // for _ in 0..(40 * 5) {
    //     builder.add_gate(
    //         FriConsistencyGate::new(2, 3, 13),
    //         vec![F::primitive_root_of_unity(13)]);
    // }

    let prover = builder.build_prover();
    let inputs = PartialWitness::new();
    prover.prove(inputs);
}

const GMIMC_ROUNDS: usize = 101;
const GMIMC_CONSTANTS: [u64; GMIMC_ROUNDS] = [11875528958976719239, 6107683892976199900, 7756999550758271958, 14819109722912164804, 9716579428412441110, 13627117528901194436, 16260683900833506663, 5942251937084147420, 3340009544523273897, 5103423085715007461, 17051583366444092101, 11122892258227244197, 16564300648907092407, 978667924592675864, 17676416205210517593, 1938246372790494499, 8857737698008340728, 1616088456497468086, 15961521580811621978, 17427220057097673602, 14693961562064090188, 694121596646283736, 554241305747273747, 5783347729647881086, 14933083198980931734, 2600898787591841337, 9178797321043036456, 18068112389665928586, 14493389459750307626, 1650694762687203587, 12538946551586403559, 10144328970401184255, 4215161528137084719, 17559540991336287827, 1632269449854444901, 986434918028205468, 14921385763379308253, 4345141219277982730, 2645897826751167170, 9815223670029373528, 7687983869685434132, 13956100321958014639, 519639453142393369, 15617837024229225911, 1557446238053329052, 8130006133842942201, 864716631341688017, 2860289738131495304, 16723700803638270299, 8363528906277648001, 13196016034228493087, 2514677332206134618, 15626342185220554936, 466271571343554681, 17490024028988898434, 6454235936129380878, 15187752952940298536, 18043495619660620405, 17118101079533798167, 13420382916440963101, 535472393366793763, 1071152303676936161, 6351382326603870931, 12029593435043638097, 9983185196487342247, 414304527840226604, 1578977347398530191, 13594880016528059526, 13219707576179925776, 6596253305527634647, 17708788597914990288, 7005038999589109658, 10171979740390484633, 1791376803510914239, 2405996319967739434, 12383033218117026776, 17648019043455213923, 6600216741450137683, 5359884112225925883, 1501497388400572107, 11860887439428904719, 64080876483307031, 11909038931518362287, 14166132102057826906, 14172584203466994499, 593515702472765471, 3423583343794830614, 10041710997716717966, 13434212189787960052, 9943803922749087030, 3216887087479209126, 17385898166602921353, 617799950397934255, 9245115057096506938, 13290383521064450731, 10193883853810413351, 14648839921475785656, 14635698366607946133, 9134302981480720532, 10045888297267997632, 10752096344939765738];

fn bench_gmimc<F: Field>() {
    const THREADS: usize = 12;
    const LDE_BITS: i32 = 3;
    const W: usize = 13;
    let hashes_per_poly = 1 << (13 + LDE_BITS);
    let threads = (0..THREADS).map(|_i| {
        thread::spawn(move || {
            let mut x = [F::ZERO; W];
            for i in 0..W {
                x[i] = F::from_canonical_u64((i as u64) * 123456 + 789);
            }

            let hashes_per_thread = hashes_per_poly * PROVER_POLYS / THREADS;
            let start = Instant::now();
            for _ in 0..hashes_per_thread {
                x = gmimc::gmimc_permute_array::<_, W, GMIMC_ROUNDS>(x, GMIMC_CONSTANTS);
            }
            let duration = start.elapsed();
            println!("took {:?}", duration);
            println!("avg {:?}us", duration.as_secs_f64() * 1e6 / (hashes_per_thread as f64));
            println!("result {:?}", x);
        })
    }).collect::<Vec<_>>();

    for t in threads {
        t.join().expect("oops");
    }
}

fn bench_fft() {
    let degree = 1 << 13;
    let lde_bits = 3;
    let lde_size = degree << lde_bits;
    println!("{} << {} = {}", degree, lde_bits, lde_size);

    let start = Instant::now();
    (0usize..PROVER_POLYS).into_par_iter().for_each(|i| {
        let mut coeffs = vec![CrandallField::ZERO; lde_size];
        for j in 0usize..lde_size {
            coeffs[j] = CrandallField((i * j) as u64);
        }

        let start = Instant::now();
        let result = fft::fft(PolynomialCoeffs { coeffs });
        let duration = start.elapsed();
        println!("FFT took {:?}", duration);
        println!("FFT result: {:?}", result.values[0]);
    });
    println!("FFT overall took {:?}", start.elapsed());
}
