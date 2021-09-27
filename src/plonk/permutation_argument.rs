use std::collections::HashMap;
use std::fmt::Debug;

use rayon::prelude::*;

use crate::field::field_types::Field;
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::PartitionWitness;
use crate::polynomial::polynomial::PolynomialValues;

/// Node in the Disjoint Set Forest.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ForestNode<V: Field> {
    pub parent: usize,
    pub size: usize,
    pub value: Option<V>,
}

/// Disjoint Set Forest data-structure following https://en.wikipedia.org/wiki/Disjoint-set_data_structure.
impl<F: Field> PartitionWitness<F> {
    pub fn new(
        num_wires: usize,
        num_routed_wires: usize,
        degree: usize,
        num_virtual_targets: usize,
    ) -> Self {
        Self {
            forest: Vec::with_capacity(degree * num_wires + num_virtual_targets),
            num_wires,
            num_routed_wires,
            degree,
        }
    }

    /// Add a new partition with a single member.
    pub fn add(&mut self, t: Target) {
        let index = self.forest.len();
        debug_assert_eq!(self.target_index(t), index);
        self.forest.push(ForestNode {
            parent: index,
            size: 1,
            value: None,
        });
    }

    /// Path compression method, see https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives.
    pub fn find(&mut self, x_index: usize) -> usize {
        let x = self.forest[x_index];
        if x.parent != x_index {
            let root_index = self.find(x.parent);
            self.forest[x_index].parent = root_index;
            root_index
        } else {
            x_index
        }
    }

    /// Merge two sets.
    pub fn merge(&mut self, tx: Target, ty: Target) {
        let x_index = self.find(self.target_index(tx));
        let y_index = self.find(self.target_index(ty));

        if x_index == y_index {
            return;
        }

        let mut x = self.forest[x_index];
        let mut y = self.forest[y_index];

        if x.size >= y.size {
            y.parent = x_index;
            x.size += y.size;
        } else {
            x.parent = y_index;
            y.size += x.size;
        }

        self.forest[x_index] = x;
        self.forest[y_index] = y;
    }

    /// Compress all paths. After calling this, every `parent` value will point to the node's
    /// representative.
    pub(crate) fn compress_paths(&mut self) {
        for i in 0..self.forest.len() {
            self.find(i);
        }
    }

    pub fn wire_partition(&mut self) -> WirePartition {
        let mut partition = HashMap::<_, Vec<_>>::new();

        // Here we keep just the Wire targets, filtering out everything else.
        for gate in 0..self.degree {
            for input in 0..self.num_routed_wires {
                let w = Wire { gate, input };
                let t = Target::Wire(w);
                let x = self.forest[self.target_index(t)];
                partition.entry(x.parent).or_default().push(w);
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
        for input in 0..num_routed_wires {
            for gate in 0..degree {
                let wire = Wire { gate, input };
                let neighbor = neighbors[&wire];
                sigma.push(neighbor.input * degree + neighbor.gate);
            }
        }
        sigma
    }
}
