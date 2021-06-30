use std::collections::HashMap;

use rayon::prelude::*;

use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialValues;
use crate::target::Target;
use crate::wire::Wire;

#[derive(Debug, Clone)]
pub struct TargetPartitions {
    partitions: Vec<Vec<Target>>,
    indices: HashMap<Target, usize>,
}

impl Default for TargetPartitions {
    fn default() -> Self {
        TargetPartitions::new()
    }
}

impl TargetPartitions {
    pub fn new() -> Self {
        Self {
            partitions: Vec::new(),
            indices: HashMap::new(),
        }
    }

    pub fn get_partition(&self, target: Target) -> &[Target] {
        &self.partitions[self.indices[&target]]
    }

    /// Add a new partition with a single member.
    pub fn add_partition(&mut self, target: Target) {
        let index = self.partitions.len();
        self.partitions.push(vec![target]);
        self.indices.insert(target, index);
    }

    /// Merge the two partitions containing the two given targets. Does nothing if the targets are
    /// already members of the same partition.
    pub fn merge(&mut self, a: Target, b: Target) {
        let a_index = self.indices[&a];
        let b_index = self.indices[&b];
        if a_index != b_index {
            // Merge a's partition into b's partition, leaving a's partition empty.
            // We have to clone because Rust's borrow checker doesn't know that
            // self.partitions[b_index] and self.partitions[b_index] are disjoint.
            let mut a_partition = self.partitions[a_index].clone();
            let b_partition = &mut self.partitions[b_index];
            for a_sibling in &a_partition {
                *self.indices.get_mut(a_sibling).unwrap() = b_index;
            }
            b_partition.append(&mut a_partition);
        }
    }

    pub fn to_wire_partitions(&self) -> WirePartitions {
        // Here we just drop all CircuitInputs, leaving all GateInputs.
        let mut partitions = Vec::new();
        let mut indices = HashMap::new();

        for old_partition in &self.partitions {
            let mut new_partition = Vec::new();
            for target in old_partition {
                if let Target::Wire(w) = *target {
                    new_partition.push(w);
                }
            }
            partitions.push(new_partition);
        }

        for (&target, &index) in &self.indices {
            if let Target::Wire(gi) = target {
                indices.insert(gi, index);
            }
        }

        WirePartitions {
            partitions,
            indices,
        }
    }
}

pub struct WirePartitions {
    partitions: Vec<Vec<Wire>>,
    indices: HashMap<Wire, usize>,
}

impl WirePartitions {
    /// Find a wire's "neighbor" in the context of Plonk's "extended copy constraints" check. In
    /// other words, find the next wire in the given wire's partition. If the given wire is last in
    /// its partition, this will loop around. If the given wire has a partition all to itself, it
    /// is considered its own neighbor.
    fn get_neighbor(&self, wire: Wire) -> Wire {
        let partition = &self.partitions[self.indices[&wire]];
        let n = partition.len();
        for i in 0..n {
            if partition[i] == wire {
                let neighbor_index = (i + 1) % n;
                return partition[neighbor_index];
            }
        }
        panic!("Wire not found in the expected partition")
    }

    pub(crate) fn get_sigma_polys<F: Field>(
        &self,
        degree_log: usize,
        k_is: &[F],
        subgroup: &[F],
    ) -> Vec<PolynomialValues<F>> {
        let degree = 1 << degree_log;
        let sigma = self.get_sigma_map(degree);

        sigma
            .chunks(degree)
            .map(|chunk| {
                let values = chunk
                    .par_iter()
                    .map(|&x| k_is[x / degree] * subgroup[x % degree])
                    .collect::<Vec<_>>();
                PolynomialValues::new(values)
            })
            .collect()
    }

    /// Generates sigma in the context of Plonk, which is a map from `[kn]` to `[kn]`, where `k` is
    /// the number of routed wires and `n` is the number of gates.
    fn get_sigma_map(&self, degree: usize) -> Vec<usize> {
        debug_assert_eq!(self.indices.len() % degree, 0);
        let num_routed_wires = self.indices.len() / degree;

        let mut sigma = Vec::new();
        for input in 0..num_routed_wires {
            for gate in 0..degree {
                let wire = Wire { gate, input };
                let neighbor = self.get_neighbor(wire);
                sigma.push(neighbor.input * degree + neighbor.gate);
            }
        }
        sigma
    }
}
