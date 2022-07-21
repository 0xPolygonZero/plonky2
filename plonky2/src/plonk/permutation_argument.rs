use std::collections::HashMap;

use maybe_rayon::*;
use plonky2_field::polynomial::PolynomialValues;
use plonky2_field::types::Field;

use crate::iop::target::Target;
use crate::iop::wire::Wire;

/// Disjoint Set Forest data-structure following https://en.wikipedia.org/wiki/Disjoint-set_data_structure.
pub struct Forest {
    /// A map of parent pointers, stored as indices.
    pub(crate) parents: Vec<usize>,

    num_wires: usize,
    num_routed_wires: usize,
    degree: usize,
}

impl Forest {
    pub fn new(
        num_wires: usize,
        num_routed_wires: usize,
        degree: usize,
        num_virtual_targets: usize,
    ) -> Self {
        let capacity = num_wires * degree + num_virtual_targets;
        Self {
            parents: Vec::with_capacity(capacity),
            num_wires,
            num_routed_wires,
            degree,
        }
    }

    pub(crate) fn target_index(&self, target: Target) -> usize {
        target.index(self.num_wires, self.degree)
    }

    /// Add a new partition with a single member.
    pub fn add(&mut self, t: Target) {
        let index = self.parents.len();
        debug_assert_eq!(self.target_index(t), index);
        self.parents.push(index);
    }

    /// Path compression method, see https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives.
    pub fn find(&mut self, mut x_index: usize) -> usize {
        // Note: We avoid recursion here since the chains can be long, causing stack overflows.

        // First, find the representative of the set containing `x_index`.
        let mut representative = x_index;
        while self.parents[representative] != representative {
            representative = self.parents[representative];
        }

        // Then, update each node in this chain to point directly to the representative.
        while self.parents[x_index] != x_index {
            let old_parent = self.parents[x_index];
            self.parents[x_index] = representative;
            x_index = old_parent;
        }

        representative
    }

    /// Merge two sets.
    pub fn merge(&mut self, tx: Target, ty: Target) {
        let x_index = self.find(self.target_index(tx));
        let y_index = self.find(self.target_index(ty));

        if x_index == y_index {
            return;
        }

        self.parents[y_index] = x_index;
    }

    /// Compress all paths. After calling this, every `parent` value will point to the node's
    /// representative.
    pub(crate) fn compress_paths(&mut self) {
        for i in 0..self.parents.len() {
            self.find(i);
        }
    }

    /// Assumes `compress_paths` has already been called.
    pub fn wire_partition(&mut self) -> WirePartition {
        let mut partition = HashMap::<_, Vec<_>>::new();

        // Here we keep just the Wire targets, filtering out everything else.
        for row in 0..self.degree {
            for column in 0..self.num_routed_wires {
                let w = Wire { row, column };
                let t = Target::Wire(w);
                let x_parent = self.parents[self.target_index(t)];
                partition.entry(x_parent).or_default().push(w);
            }
        }

        let partition = partition.into_values().collect();
        WirePartition { partition }
    }
}

pub struct WirePartition {
    partition: Vec<Vec<Wire>>,
}

impl WirePartition {
    pub(crate) fn get_sigma_polys<F: Field>(
        &self,
        degree_log: usize,
        k_is: &[F],
        subgroup: &[F],
    ) -> Vec<PolynomialValues<F>> {
        let degree = 1 << degree_log;
        let sigma = self.get_sigma_map(degree, k_is.len());

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
    fn get_sigma_map(&self, degree: usize, num_routed_wires: usize) -> Vec<usize> {
        // Find a wire's "neighbor" in the context of Plonk's "extended copy constraints" check. In
        // other words, find the next wire in the given wire's partition. If the given wire is last in
        // its partition, this will loop around. If the given wire has a partition all to itself, it
        // is considered its own neighbor.
        let mut neighbors = HashMap::new();
        for subset in &self.partition {
            for n in 0..subset.len() {
                neighbors.insert(subset[n], subset[(n + 1) % subset.len()]);
            }
        }

        let mut sigma = Vec::new();
        for column in 0..num_routed_wires {
            for row in 0..degree {
                let wire = Wire { row, column };
                let neighbor = neighbors[&wire];
                sigma.push(neighbor.column * degree + neighbor.row);
            }
        }
        sigma
    }
}
