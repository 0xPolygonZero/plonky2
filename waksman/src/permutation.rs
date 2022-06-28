use std::collections::BTreeMap;
use std::marker::PhantomData;

use plonky2::field::{extension::Extendable, types::Field};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::bimap::bimap_from_lists;
use crate::gates::switch::SwitchGate;

/// Assert that two lists of expressions evaluate to permutations of one another.
pub fn assert_permutation_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: Vec<Vec<Target>>,
    b: Vec<Vec<Target>>,
) {
    assert_eq!(
        a.len(),
        b.len(),
        "Permutation must have same number of inputs and outputs"
    );
    assert_eq!(a[0].len(), b[0].len(), "Chunk size must be the same");

    let chunk_size = a[0].len();

    match a.len() {
        // Two empty lists are permutations of one another, trivially.
        0 => (),
        // Two singleton lists are permutations of one another as long as their items are equal.
        1 => {
            for e in 0..chunk_size {
                builder.connect(a[0][e], b[0][e])
            }
        }
        2 => assert_permutation_2x2_circuit(
            builder,
            a[0].clone(),
            a[1].clone(),
            b[0].clone(),
            b[1].clone(),
        ),
        // For larger lists, we recursively use two smaller permutation networks.
        _ => assert_permutation_helper_circuit(builder, a, b),
    }
}

/// Assert that [a1, a2] is a permutation of [b1, b2].
fn assert_permutation_2x2_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a1: Vec<Target>,
    a2: Vec<Target>,
    b1: Vec<Target>,
    b2: Vec<Target>,
) {
    assert!(
        a1.len() == a2.len() && a2.len() == b1.len() && b1.len() == b2.len(),
        "Chunk size must be the same"
    );

    let chunk_size = a1.len();

    let (_switch, gate_out1, gate_out2) = create_switch_circuit(builder, a1, a2);
    for e in 0..chunk_size {
        builder.connect(b1[e], gate_out1[e]);
        builder.connect(b2[e], gate_out2[e]);
    }
}

/// Given two input wire chunks, add a new switch to the circuit (by adding one copy to a switch
/// gate). Returns the wire for the switch boolean, and the two output wire chunks.
fn create_switch_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a1: Vec<Target>,
    a2: Vec<Target>,
) -> (Target, Vec<Target>, Vec<Target>) {
    assert_eq!(a1.len(), a2.len(), "Chunk size must be the same");

    let chunk_size = a1.len();

    let gate = SwitchGate::new_from_config(&builder.config, chunk_size);
    let params = vec![F::from_canonical_usize(chunk_size)];
    let (row, next_copy) = builder.find_slot(gate, &params, &[]);

    let mut c = Vec::new();
    let mut d = Vec::new();
    for e in 0..chunk_size {
        builder.connect(
            a1[e],
            Target::wire(row, gate.wire_first_input(next_copy, e)),
        );
        builder.connect(
            a2[e],
            Target::wire(row, gate.wire_second_input(next_copy, e)),
        );
        c.push(Target::wire(row, gate.wire_first_output(next_copy, e)));
        d.push(Target::wire(row, gate.wire_second_output(next_copy, e)));
    }

    let switch = Target::wire(row, gate.wire_switch_bool(next_copy));

    (switch, c, d)
}

fn assert_permutation_helper_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: Vec<Vec<Target>>,
    b: Vec<Vec<Target>>,
) {
    assert_eq!(
        a.len(),
        b.len(),
        "Permutation must have same number of inputs and outputs"
    );
    assert_eq!(a[0].len(), b[0].len(), "Chunk size must be the same");

    let n = a.len();
    let even = n % 2 == 0;

    let mut child_1_a = Vec::new();
    let mut child_1_b = Vec::new();
    let mut child_2_a = Vec::new();
    let mut child_2_b = Vec::new();

    // See Figure 8 in the AS-Waksman paper.
    let a_num_switches = n / 2;
    let b_num_switches = if even {
        a_num_switches - 1
    } else {
        a_num_switches
    };

    let mut a_switches = Vec::new();
    let mut b_switches = Vec::new();
    for i in 0..a_num_switches {
        let (switch, out_1, out_2) =
            create_switch_circuit(builder, a[i * 2].clone(), a[i * 2 + 1].clone());
        a_switches.push(switch);
        child_1_a.push(out_1);
        child_2_a.push(out_2);
    }
    for i in 0..b_num_switches {
        let (switch, out_1, out_2) =
            create_switch_circuit(builder, b[i * 2].clone(), b[i * 2 + 1].clone());
        b_switches.push(switch);
        child_1_b.push(out_1);
        child_2_b.push(out_2);
    }

    // See Figure 8 in the AS-Waksman paper.
    if even {
        child_1_b.push(b[n - 2].clone());
        child_2_b.push(b[n - 1].clone());
    } else {
        child_2_a.push(a[n - 1].clone());
        child_2_b.push(b[n - 1].clone());
    }

    assert_permutation_circuit(builder, child_1_a, child_1_b);
    assert_permutation_circuit(builder, child_2_a, child_2_b);

    builder.add_simple_generator(PermutationGenerator::<F> {
        a,
        b,
        a_switches,
        b_switches,
        _phantom: PhantomData,
    });
}

fn route<F: Field>(
    a_values: Vec<Vec<F>>,
    b_values: Vec<Vec<F>>,
    a_switches: Vec<Target>,
    b_switches: Vec<Target>,
    witness: &PartitionWitness<F>,
    out_buffer: &mut GeneratedValues<F>,
) {
    assert_eq!(a_values.len(), b_values.len());
    let n = a_values.len();
    let even = n % 2 == 0;

    // We use a bimap to match indices of values in a to indices of the same values in b.
    // This means that given a wire on one side, we can easily find the matching wire on the other side.
    let ab_map = bimap_from_lists(a_values, b_values);

    let switches = [a_switches, b_switches];

    // We keep track of the new wires we've routed (after routing some wires, we need to check `witness`
    // and `newly_set` instead of just `witness`.
    let mut newly_set = [vec![false; n], vec![false; n]];

    // Given a side and an index, returns the index in the other side that corresponds to the same value.
    let ab_map_by_side = |side: usize, index: usize| -> usize {
        *match side {
            0 => ab_map.get_by_left(&index),
            1 => ab_map.get_by_right(&index),
            _ => panic!("Expected side to be 0 or 1"),
        }
        .unwrap()
    };

    // We maintain two maps for wires which have been routed to a particular subnetwork on one side
    // of the network (left or right) but not the other. The keys are wire indices, and the values
    // are subnetwork indices.
    let mut partial_routes = [BTreeMap::new(), BTreeMap::new()];

    // After we route a wire on one side, we find the corresponding wire on the other side and check
    // if it still needs to be routed. If so, we add it to partial_routes.
    let enqueue_other_side = |partial_routes: &mut [BTreeMap<usize, bool>],
                              witness: &PartitionWitness<F>,
                              newly_set: &mut [Vec<bool>],
                              side: usize,
                              this_i: usize,
                              subnet: bool| {
        let other_side = 1 - side;
        let other_i = ab_map_by_side(side, this_i);
        let other_switch_i = other_i / 2;

        if other_switch_i >= switches[other_side].len() {
            // The other wire doesn't go through a switch, so there's no routing to be done.
            // This happens in the case of the very last wire.
            return;
        }

        if witness.contains(switches[other_side][other_switch_i])
            || newly_set[other_side][other_switch_i]
        {
            // The other switch has already been routed.
            return;
        }

        let other_i_sibling = 4 * other_switch_i + 1 - other_i;
        if let Some(&sibling_subnet) = partial_routes[other_side].get(&other_i_sibling) {
            // The other switch's sibling is already pending routing.
            assert_ne!(subnet, sibling_subnet);
        } else {
            let opt_old_subnet = partial_routes[other_side].insert(other_i, subnet);
            if let Some(old_subnet) = opt_old_subnet {
                assert_eq!(subnet, old_subnet, "Routing conflict (should never happen)");
            }
        }
    };

    // See Figure 8 in the AS-Waksman paper.
    if even {
        enqueue_other_side(
            &mut partial_routes,
            witness,
            &mut newly_set,
            1,
            n - 2,
            false,
        );
        enqueue_other_side(&mut partial_routes, witness, &mut newly_set, 1, n - 1, true);
    } else {
        enqueue_other_side(&mut partial_routes, witness, &mut newly_set, 0, n - 1, true);
        enqueue_other_side(&mut partial_routes, witness, &mut newly_set, 1, n - 1, true);
    }

    let route_switch = |partial_routes: &mut [BTreeMap<usize, bool>],
                        witness: &PartitionWitness<F>,
                        out_buffer: &mut GeneratedValues<F>,
                        newly_set: &mut [Vec<bool>],
                        side: usize,
                        switch_index: usize,
                        swap: bool| {
        // First, we actually set the switch configuration.
        out_buffer.set_target(switches[side][switch_index], F::from_bool(swap));
        newly_set[side][switch_index] = true;

        // Then, we enqueue the two corresponding wires on the other side of the network, to ensure
        // that they get routed in the next step.
        let this_i_1 = switch_index * 2;
        let this_i_2 = this_i_1 + 1;
        enqueue_other_side(partial_routes, witness, newly_set, side, this_i_1, swap);
        enqueue_other_side(partial_routes, witness, newly_set, side, this_i_2, !swap);
    };

    // If {a,b}_only_routes is empty, then we can route any switch next. For efficiency, we will
    // simply do top-down scans (one on the left side, one on the right side) for switches which
    // have not yet been routed. These variables represent the positions of those two scans.
    let mut scan_index = [0, 0];

    // Until both scans complete, we alternate back and worth between the left and right switch
    // layers. We process any partially routed wires for that side, or if there aren't any, we route
    // the next switch in our scan.
    while scan_index[0] < switches[0].len() || scan_index[1] < switches[1].len() {
        for side in 0..=1 {
            if !partial_routes[side].is_empty() {
                for (this_i, subnet) in partial_routes[side].clone().into_iter() {
                    let this_first_switch_input = this_i % 2 == 0;
                    let swap = this_first_switch_input == subnet;
                    let this_switch_i = this_i / 2;
                    route_switch(
                        &mut partial_routes,
                        witness,
                        out_buffer,
                        &mut newly_set,
                        side,
                        this_switch_i,
                        swap,
                    );
                }
                partial_routes[side].clear();
            } else {
                // We can route any switch next. Continue our scan for pending switches.
                while scan_index[side] < switches[side].len()
                    && (witness.contains(switches[side][scan_index[side]])
                        || newly_set[side][scan_index[side]])
                {
                    scan_index[side] += 1;
                }
                if scan_index[side] < switches[side].len() {
                    // Either switch configuration would work; we arbitrarily choose to not swap.
                    route_switch(
                        &mut partial_routes,
                        witness,
                        out_buffer,
                        &mut newly_set,
                        side,
                        scan_index[side],
                        false,
                    );
                    scan_index[side] += 1;
                }
            }
        }
    }
}

#[derive(Debug)]
struct PermutationGenerator<F: Field> {
    a: Vec<Vec<Target>>,
    b: Vec<Vec<Target>>,
    a_switches: Vec<Target>,
    b_switches: Vec<Target>,
    _phantom: PhantomData<F>,
}

impl<F: Field> SimpleGenerator<F> for PermutationGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        self.a.iter().chain(&self.b).flatten().cloned().collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let a_values = self
            .a
            .iter()
            .map(|chunk| chunk.iter().map(|wire| witness.get_target(*wire)).collect())
            .collect();
        let b_values = self
            .b
            .iter()
            .map(|chunk| chunk.iter().map(|wire| witness.get_target(*wire)).collect())
            .collect();
        route(
            a_values,
            b_values,
            self.a_switches.clone(),
            self.b_switches.clone(),
            witness,
            out_buffer,
        );
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::types::Field;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rand::{seq::SliceRandom, thread_rng, Rng};

    use super::*;

    fn test_permutation_good(size: usize) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let lst: Vec<F> = (0..size * 2).map(F::from_canonical_usize).collect();
        let a: Vec<Vec<Target>> = lst[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();
        let mut b = a.clone();
        b.shuffle(&mut thread_rng());

        assert_permutation_circuit(&mut builder, a, b);

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        data.verify(proof)
    }

    fn test_permutation_duplicates(size: usize) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let mut rng = thread_rng();
        let lst: Vec<F> = (0..size * 2)
            .map(|_| F::from_canonical_usize(rng.gen_range(0..2usize)))
            .collect();
        let a: Vec<Vec<Target>> = lst[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();

        let mut b = a.clone();
        b.shuffle(&mut thread_rng());

        assert_permutation_circuit(&mut builder, a, b);

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        data.verify(proof)
    }

    fn test_permutation_bad(size: usize) -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let lst1: Vec<F> = F::rand_vec(size * 2);
        let lst2: Vec<F> = F::rand_vec(size * 2);
        let a: Vec<Vec<Target>> = lst1[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();
        let b: Vec<Vec<Target>> = lst2[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();

        assert_permutation_circuit(&mut builder, a, b);

        let data = builder.build::<C>();
        data.prove(pw)?;

        Ok(())
    }

    #[test]
    fn test_permutations_duplicates() -> Result<()> {
        for n in 2..9 {
            test_permutation_duplicates(n)?;
        }

        Ok(())
    }

    #[test]
    fn test_permutations_good() -> Result<()> {
        for n in 2..9 {
            test_permutation_good(n)?;
        }

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_small() {
        let size = 2;

        test_permutation_bad(size).unwrap()
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_medium() {
        let size = 6;

        test_permutation_bad(size).unwrap()
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_large() {
        let size = 10;

        test_permutation_bad(size).unwrap()
    }
}
