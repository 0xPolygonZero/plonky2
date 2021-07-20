use std::collections::HashMap;
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

    /// Path compression method, see https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives.
    pub fn find(&mut self, x: ForestNode<T>) -> ForestNode<T> {
        if x.parent != x.index {
            let root = self.find(self.forest[x.parent]);
            self.forest[x.index].parent = root.index;
            root
        } else {
            x
        }
    }

    /// Merge two sets.
    pub fn merge(&mut self, tx: T, ty: T) {
        let mut x = self.forest[(self.indices)(tx)];
        let mut y = self.forest[(self.indices)(ty)];

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

        self.forest[x.index] = x;
        self.forest[y.index] = y;
    }
}
impl<F: Fn(Target) -> usize> TargetPartition<Target, F> {
    pub fn wire_partition(&mut self) -> WirePartitions {
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

        WirePartitions { partition }
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
