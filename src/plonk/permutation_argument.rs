use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use rayon::prelude::*;

use crate::field::field_types::Field;
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::PartitionWitness;
use crate::polynomial::polynomial::PolynomialValues;

/// Node in the Disjoint Set Forest.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ForestNode<T: Debug + Copy + Eq + PartialEq, V: Field> {
    pub t: T,
    pub parent: usize,
    pub size: usize,
    pub index: usize,
    pub value: Option<V>,
}

/// Disjoint Set Forest data-structure following https://en.wikipedia.org/wiki/Disjoint-set_data_structure.
impl<F: Field> PartitionWitness<F> {
    pub fn new(num_wires: usize, num_routed_wires: usize, degree: usize) -> Self {
        Self {
            nodes: vec![],
            num_wires,
            num_routed_wires,
            degree,
        }
    }

    /// Add a new partition with a single member.
    pub fn add(&mut self, t: Target) {
        let index = self.nodes.len();
        debug_assert_eq!(self.target_index(t), index);
        self.nodes.push(ForestNode {
            t,
            parent: index,
            size: 1,
            index,
            value: None,
        });
    }

    /// Path compression method, see https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives.
    pub fn find(&mut self, x: ForestNode<Target, F>) -> ForestNode<Target, F> {
        if x.parent != x.index {
            let root = self.find(self.nodes[x.parent]);
            self.nodes[x.index].parent = root.index;
            root
        } else {
            x
        }
    }

    /// Merge two sets.
    pub fn merge(&mut self, tx: Target, ty: Target) {
        let mut x = self.nodes[self.target_index(tx)];
        let mut y = self.nodes[self.target_index(ty)];

        x = self.find(x);
        y = self.find(y);

        if x == y {
            return;
        }

        if x.size >= y.size {
            y.parent = x.index;
            x.size += y.size;
        } else {
            x.parent = y.index;
            y.size += x.size;
        }

        self.nodes[x.index] = x;
        self.nodes[y.index] = y;
    }
}
impl<F: Field> PartitionWitness<F> {
    pub fn wire_partition(mut self) -> (WirePartitions, Self) {
        let mut partition = HashMap::<_, Vec<_>>::new();
        for gate in 0..self.degree {
            for input in 0..self.num_routed_wires {
                let w = Wire { gate, input };
                let t = Target::Wire(w);
                let x = self.nodes[self.target_index(t)];
                partition.entry(self.find(x).t).or_default().push(w);
            }
        }
        // I'm not 100% sure this loop is needed, but I'm afraid removing it might lead to subtle bugs.
        for index in 0..self.nodes.len() - self.degree * self.num_wires {
            let t = Target::VirtualTarget { index };
            let x = self.nodes[self.target_index(t)];
            self.find(x);
        }

        // Here we keep just the Wire targets, filtering out everything else.
        let partition = partition.into_values().collect::<Vec<_>>();

        (WirePartitions { partition }, self)
    }
}

pub struct WirePartitions {
    partition: Vec<Vec<Wire>>,
}

impl WirePartitions {
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
