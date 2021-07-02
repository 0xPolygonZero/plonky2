use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use rayon::prelude::*;

use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialValues;
use crate::target::Target;
use crate::wire::Wire;

/// Node in the Disjoint Set Forest.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ForestNode<T: Debug + Copy + Eq + PartialEq> {
    t: T,
    parent: usize,
    size: usize,
    index: usize,
}

/// Disjoint Set Forest data-structure following https://en.wikipedia.org/wiki/Disjoint-set_data_structure.
#[derive(Debug, Clone)]
pub struct TargetPartition<T: Debug + Copy + Eq + PartialEq + Hash, F: Fn(T) -> usize> {
    forest: Vec<ForestNode<T>>,
    /// Function to compute a node's index in the forest.
    indices: F,
}

impl<T: Debug + Copy + Eq + PartialEq + Hash, F: Fn(T) -> usize> TargetPartition<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            forest: Vec::new(),
            indices: f,
        }
    }
    /// Add a new partition with a single member.
    pub fn add(&mut self, t: T) {
        let index = self.forest.len();
        debug_assert_eq!((self.indices)(t), index);
        self.forest.push(ForestNode {
            t,
            parent: index,
            size: 1,
            index,
        });
    }

    /// Path halving method, see https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives.
    pub fn find(&mut self, mut x: ForestNode<T>) -> ForestNode<T> {
        while x.parent != x.index {
            let grandparent = self.forest[x.parent].parent;
            x.parent = grandparent;
            x = self.forest[grandparent];
        }
        x
    }

    /// Merge two sets.
    pub fn merge(&mut self, tx: T, ty: T) {
        let index_x = (self.indices)(tx);
        let index_y = (self.indices)(ty);
        let mut x = self.forest[index_x];
        let mut y = self.forest[index_y];

        x = self.forest[x.parent];
        y = self.forest[y.parent];

        if x == y {
            return;
        }

        if x.size < y.size {
            std::mem::swap(&mut x, &mut y);
        }

        y.parent = x.index;
        x.size += y.size;

        self.forest[index_x] = x;
        self.forest[index_y] = y;
    }
}
impl<F: Fn(Target) -> usize> TargetPartition<Target, F> {
    pub fn wire_partitions(&mut self) -> WirePartitions {
        let mut partition = HashMap::<_, Vec<_>>::new();
        let nodes = self.forest.clone();
        for x in nodes {
            let v = partition.entry(self.find(x).t).or_default();
            v.push(x.t);
        }

        let mut indices = HashMap::new();
        // // Here we keep just the Wire targets, filtering out everything else.
        let partition = partition
            .into_values()
            .map(|v| {
                v.into_iter()
                    .filter_map(|t| match t {
                        Target::Wire(w) => Some(w),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        partition.iter().enumerate().for_each(|(i, v)| {
            v.iter().for_each(|t| {
                indices.insert(*t, i);
            });
        });

        WirePartitions { partition, indices }
    }
}

pub struct WirePartitions {
    partition: Vec<Vec<Wire>>,
    indices: HashMap<Wire, usize>,
}

impl WirePartitions {
    /// Find a wire's "neighbor" in the context of Plonk's "extended copy constraints" check. In
    /// other words, find the next wire in the given wire's partition. If the given wire is last in
    /// its partition, this will loop around. If the given wire has a partition all to itself, it
    /// is considered its own neighbor.
    fn get_neighbor(&self, wire: Wire) -> Wire {
        let partition = &self.partition[self.indices[&wire]];
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
