use std::collections::BTreeMap;
use std::convert::TryInto;
use std::marker::PhantomData;

use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::switch::SwitchGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::PartialWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::bimap::bimap_from_lists;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Assert that two lists of expressions evaluate to permutations of one another.
    pub fn assert_permutation<const CHUNK_SIZE: usize>(
        &mut self,
        a: Vec<[Target; CHUNK_SIZE]>,
        b: Vec<[Target; CHUNK_SIZE]>,
    ) {
        assert_eq!(
            a.len(),
            b.len(),
            "Permutation must have same number of inputs and outputs"
        );
        assert_eq!(a[0].len(), b[0].len(), "Chunk sizes must be the same");

        match a.len() {
            // Two empty lists are permutations of one another, trivially.
            0 => (),
            // Two singleton lists are permutations of one another as long as their items are equal.
            1 => {
                for e in 0..CHUNK_SIZE {
                    self.assert_equal(a[0][e], b[0][e])
                }
            }
            2 => self.assert_permutation_2x2(a[0], a[1], b[0], b[1]),
            // For larger lists, we recursively use two smaller permutation networks.
            //_ => self.assert_permutation_recursive(a, b)
            _ => self.assert_permutation_recursive(a, b),
        }
    }

    /// Assert that [a1, a2] is a permutation of [b1, b2].
    fn assert_permutation_2x2<const CHUNK_SIZE: usize>(
        &mut self,
        a1: [Target; CHUNK_SIZE],
        a2: [Target; CHUNK_SIZE],
        b1: [Target; CHUNK_SIZE],
        b2: [Target; CHUNK_SIZE],
    ) {
        let (switch, gate_c, gate_d) = self.create_switch(a1, a2);
        for e in 0..CHUNK_SIZE {
            self.route(b1[e], gate_c[e]);
            self.route(b2[e], gate_d[e]);
        }

        self.add_generator(TwoByTwoPermutationGenerator::<F, CHUNK_SIZE> {
            a1,
            a2,
            b1,
            b2,
            switch,
            _phantom: PhantomData,
        });
    }

    fn create_switch<const CHUNK_SIZE: usize>(
        &mut self,
        a1: [Target; CHUNK_SIZE],
        a2: [Target; CHUNK_SIZE],
    ) -> (Target, [Target; CHUNK_SIZE], [Target; CHUNK_SIZE]) {
        if self.current_switch_gates.len() < CHUNK_SIZE {
            self.current_switch_gates
                .extend(vec![None; CHUNK_SIZE - self.current_switch_gates.len()]);
        }

        let (gate_index, mut next_copy) = match self.current_switch_gates[CHUNK_SIZE - 1] {
            None => {
                let gate = SwitchGate::<F, D, CHUNK_SIZE>::new_from_config(self.config.clone());
                let gate_index = self.add_gate(gate.clone(), vec![]);
                (gate_index, 0)
            }
            Some((idx, next_copy)) => (idx, next_copy),
        };

        let num_copies =
            SwitchGate::<F, D, CHUNK_SIZE>::max_num_copies(self.config.num_routed_wires);

        let mut c = Vec::new();
        let mut d = Vec::new();
        for e in 0..CHUNK_SIZE {
            self.route(
                a1[e],
                Target::wire(
                    gate_index,
                    SwitchGate::<F, D, CHUNK_SIZE>::wire_first_input(next_copy, e),
                ),
            );
            self.route(
                a2[e],
                Target::wire(
                    gate_index,
                    SwitchGate::<F, D, CHUNK_SIZE>::wire_second_input(next_copy, e),
                ),
            );
            c.push(Target::wire(
                gate_index,
                SwitchGate::<F, D, CHUNK_SIZE>::wire_first_output(next_copy, e),
            ));
            d.push(Target::wire(
                gate_index,
                SwitchGate::<F, D, CHUNK_SIZE>::wire_second_output(next_copy, e),
            ));
        }

        let switch = Target::wire(
            gate_index,
            SwitchGate::<F, D, CHUNK_SIZE>::wire_switch_bool(num_copies, next_copy),
        );

        let c_arr: [Target; CHUNK_SIZE] = c.try_into().unwrap();
        let d_arr: [Target; CHUNK_SIZE] = d.try_into().unwrap();

        next_copy += 1;
        if next_copy == num_copies {
            let new_gate = SwitchGate::<F, D, CHUNK_SIZE>::new_from_config(self.config.clone());
            let new_gate_index = self.add_gate(new_gate.clone(), vec![]);
            self.current_switch_gates[CHUNK_SIZE - 1] = Some((new_gate_index, 0));
        } else {
            self.current_switch_gates[CHUNK_SIZE - 1] = Some((gate_index, next_copy));
        }

        (switch, c_arr, d_arr)
    }

    fn assert_permutation_recursive<const CHUNK_SIZE: usize>(
        &mut self,
        a: Vec<[Target; CHUNK_SIZE]>,
        b: Vec<[Target; CHUNK_SIZE]>,
    ) {
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
            let (switch, out_1, out_2) = self.create_switch(a[i * 2], a[i * 2 + 1]);
            a_switches.push(switch);
            child_1_a.push(out_1);
            child_2_a.push(out_2);
        }
        for i in 0..b_num_switches {
            let (switch, out_1, out_2) = self.create_switch(b[i * 2], b[i * 2 + 1]);
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

        self.assert_permutation(child_1_a, child_1_b);
        self.assert_permutation(child_2_a, child_2_b);

        self.add_generator(PermutationGenerator::<F, CHUNK_SIZE> {
            a,
            b,
            a_switches,
            b_switches,
            _phantom: PhantomData,
        });
    }
}

fn route<F: Field, const CHUNK_SIZE: usize>(
    a_values: Vec<[F; CHUNK_SIZE]>,
    b_values: Vec<[F; CHUNK_SIZE]>,
    a_switches: Vec<Target>,
    b_switches: Vec<Target>,
    witness: &PartialWitness<F>,
    out_buffer: &mut GeneratedValues<F>,
) {
    assert_eq!(a_values.len(), b_values.len());
    let n = a_values.len();
    let even = n % 2 == 0;
    // Bimap: maps indices of values in a to indices of the same values in b
    let ab_map = bimap_from_lists(a_values, b_values);
    let switches = [a_switches, b_switches];

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
                              witness: &PartialWitness<F>,
                              _out_buffer: &mut GeneratedValues<F>,
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

        if witness.contains(switches[other_side][other_switch_i]) {
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
        enqueue_other_side(&mut partial_routes, witness, out_buffer, 1, n - 2, false);
        enqueue_other_side(&mut partial_routes, witness, out_buffer, 1, n - 1, true);
    } else {
        enqueue_other_side(&mut partial_routes, witness, out_buffer, 0, n - 1, true);
        enqueue_other_side(&mut partial_routes, witness, out_buffer, 1, n - 1, true);
    }

    let route_switch = |partial_routes: &mut [BTreeMap<usize, bool>],
                        witness: &PartialWitness<F>,
                        out_buffer: &mut GeneratedValues<F>,
                        side: usize,
                        switch_index: usize,
                        swap: bool| {
        // First, we actually set the switch configuration.
        out_buffer.set_target(switches[side][switch_index], F::from_bool(swap));

        // Then, we enqueue the two corresponding wires on the other side of the network, to ensure
        // that they get routed in the next step.
        let this_i_1 = switch_index * 2;
        let this_i_2 = this_i_1 + 1;
        enqueue_other_side(partial_routes, witness, out_buffer, side, this_i_1, swap);
        enqueue_other_side(partial_routes, witness, out_buffer, side, this_i_2, !swap);
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
                        side,
                        this_switch_i,
                        swap,
                    );
                }
                partial_routes[side].clear();
            } else {
                // We can route any switch next. Continue our scan for pending switches.
                while scan_index[side] < switches[side].len()
                    && witness.contains(switches[side][scan_index[side]])
                {
                    scan_index[side] += 1;
                }
                if scan_index[side] < switches[side].len() {
                    // Either switch configuration would work; we arbitrarily choose to not swap.
                    route_switch(
                        &mut partial_routes,
                        witness,
                        out_buffer,
                        side,
                        scan_index[side],
                        false,
                    );
                }
            }
        }
    }
}

struct TwoByTwoPermutationGenerator<F: Field, const CHUNK_SIZE: usize> {
    a1: [Target; CHUNK_SIZE],
    a2: [Target; CHUNK_SIZE],
    b1: [Target; CHUNK_SIZE],
    b2: [Target; CHUNK_SIZE],
    switch: Target,
    _phantom: PhantomData<F>,
}

impl<F: Field, const CHUNK_SIZE: usize> SimpleGenerator<F>
    for TwoByTwoPermutationGenerator<F, CHUNK_SIZE>
{
    fn dependencies(&self) -> Vec<Target> {
        [self.a1, self.a2, self.b1, self.b2]
            .to_vec()
            .iter()
            .map(|arr| arr.to_vec())
            .flatten()
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let a1_values: Vec<_> = self
            .a1
            .to_vec()
            .iter()
            .map(|x| witness.get_target(*x))
            .collect();
        let a2_values: Vec<_> = self
            .a2
            .to_vec()
            .iter()
            .map(|x| witness.get_target(*x))
            .collect();
        let b1_values: Vec<_> = self
            .b1
            .to_vec()
            .iter()
            .map(|x| witness.get_target(*x))
            .collect();
        let b2_values: Vec<_> = self
            .b2
            .to_vec()
            .iter()
            .map(|x| witness.get_target(*x))
            .collect();

        let no_switch = a1_values.iter().zip(b1_values.iter()).all(|(a, b)| a == b)
            && a2_values.iter().zip(b2_values.iter()).all(|(a, b)| a == b);
        let switch = a1_values.iter().zip(b2_values.iter()).all(|(a, b)| a == b)
            && a2_values.iter().zip(b1_values.iter()).all(|(a, b)| a == b);

        if no_switch {
            out_buffer.set_target(self.switch, F::ZERO);
        } else if switch {
            out_buffer.set_target(self.switch, F::ONE);
        } else {
            panic!("No permutation");
        }
    }
}

struct PermutationGenerator<F: Field, const CHUNK_SIZE: usize> {
    a: Vec<[Target; CHUNK_SIZE]>,
    b: Vec<[Target; CHUNK_SIZE]>,
    a_switches: Vec<Target>,
    b_switches: Vec<Target>,
    _phantom: PhantomData<F>,
}

impl<F: Field, const CHUNK_SIZE: usize> SimpleGenerator<F> for PermutationGenerator<F, CHUNK_SIZE> {
    fn dependencies(&self) -> Vec<Target> {
        self.a
            .iter()
            .map(|arr| arr.to_vec())
            .flatten()
            .chain(self.b.iter().map(|arr| arr.to_vec()).flatten())
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let wire_chunk_to_vals = |wire: [Target; CHUNK_SIZE]| {
            let mut vals = [F::ZERO; CHUNK_SIZE];
            for e in 0..CHUNK_SIZE {
                vals[e] = witness.get_target(wire[e]);
            }
            vals
        };

        let a_values = self
            .a
            .iter()
            .map(|chunk| wire_chunk_to_vals(*chunk))
            .collect();
        let b_values = self
            .b
            .iter()
            .map(|chunk| wire_chunk_to_vals(*chunk))
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

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn route_2x2() -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let one = F::ONE;
        let two = F::from_canonical_usize(2);
        let seven = F::from_canonical_usize(7);
        let eight = F::from_canonical_usize(8);

        let one_two = [builder.constant(one), builder.constant(two)];
        let seven_eight = [builder.constant(seven), builder.constant(eight)];

        let a = vec![one_two, seven_eight];
        let b = vec![seven_eight, one_two];

        builder.assert_permutation(a, b);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    /*fn test_permutation(size: usize) -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let vec = FF::rand_vec(len);
        let v: Vec<_> = vec.iter().map(|x| builder.constant_extension(*x)).collect();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let elem = builder.constant_extension(vec[i]);
            builder.random_access(it, elem, v.clone());
        }

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }*/
}
