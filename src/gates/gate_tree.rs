use log::info;

use crate::field::extension_field::Extendable;
use crate::gates::gate::GateRef;

/// A binary tree where leaves hold some type `T` and other nodes are empty.
#[derive(Debug, Clone)]
pub enum Tree<T> {
    Leaf(T),
    Bifurcation(Option<Box<Tree<T>>>, Option<Box<Tree<T>>>),
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::Bifurcation(None, None)
    }
}

impl<T: Clone> Tree<T> {
    /// Traverse a tree using a depth-first traversal and collect data and position for each leaf.
    /// A leaf's position is represented by its left/right path, where `false` means left and `true` means right.
    pub fn traversal(&self) -> Vec<(T, Vec<bool>)> {
        let mut res = Vec::new();
        let prefix = [];
        self.traverse(&prefix, &mut res);
        res
    }

    /// Utility function to traverse the tree.
    fn traverse(&self, prefix: &[bool], current: &mut Vec<(T, Vec<bool>)>) {
        match &self {
            // If node is a leaf, collect the data and position.
            Tree::Leaf(t) => {
                current.push((t.clone(), prefix.to_vec()));
            }
            // Otherwise, traverse the left subtree and then the right subtree.
            Tree::Bifurcation(left, right) => {
                if let Some(l) = left {
                    let mut left_prefix = prefix.to_vec();
                    left_prefix.push(false);
                    l.traverse(&left_prefix, current);
                }
                if let Some(r) = right {
                    let mut right_prefix = prefix.to_vec();
                    right_prefix.push(true);
                    r.traverse(&right_prefix, current);
                }
            }
        }
    }
}

impl<F: Extendable<D>, const D: usize> Tree<GateRef<F, D>> {
    /// Construct a binary tree of gates using the following greedy algorithm:
    /// We want a tree where the maximum `M` of
    /// `F(gate) = gate.degree() + gate.num_constants() + tree.depth(gate)`
    /// over all gates is minimized. Such a tree is constructed by iterating over possible values of `M`
    /// (from 1 to 99, then we give up) and then looking for a tree with this value of `M`
    /// using `Self::find_tree`. This latter function greedily adds gates at the depth where
    /// `F(gate)=M` to ensure no space is wasted. We return the first tree found in this manner,
    /// i.e., the one with minimal `M` value.
    pub fn from_gates(mut gates: Vec<GateRef<F, D>>) -> Self {
        let timer = std::time::Instant::now();
        gates.sort_unstable_by_key(|g| (-(g.0.degree() as isize), -(g.0.num_constants() as isize)));

        for max_degree_bits in 1..10 {
            let max_degree = 1 << max_degree_bits;
            for max_wires in 1..100 {
                if let Some(mut tree) = Self::find_tree(&gates, max_degree, max_wires) {
                    tree.shorten();
                    info!(
                        "Found tree with max degree {} in {}s.",
                        max_degree,
                        timer.elapsed().as_secs_f32()
                    );
                    return tree;
                }
            }
        }

        panic!("Can't find a tree.")
    }

    /// Greedily add gates wherever possible. Returns `None` if this fails.
    fn find_tree(gates: &[GateRef<F, D>], max_degree: usize, max_wires: usize) -> Option<Self> {
        let mut tree = Tree::default();

        for g in gates {
            tree.try_add_gate(g, max_degree, max_wires)?;
        }
        Some(tree)
    }

    /// Try to add a gate in the tree. Returns `None` if this fails.
    fn try_add_gate(
        &mut self,
        g: &GateRef<F, D>,
        max_degree: usize,
        max_wires: usize,
    ) -> Option<()> {
        let depth = max_degree
            .checked_sub(g.0.degree())?
            .min(max_wires.checked_sub(g.0.num_constants())?);
        self.try_add_gate_at_depth(g, depth)
    }

    /// Try to add a gate in the tree at a specified depth. Returns `None` if this fails.
    fn try_add_gate_at_depth(&mut self, g: &GateRef<F, D>, depth: usize) -> Option<()> {
        // If depth is 0, we have to insert the gate here.
        if depth == 0 {
            return if let Tree::Bifurcation(None, None) = self {
                // Insert the gate as a new leaf.
                *self = Tree::Leaf(g.clone());
                Some(())
            } else {
                // A leaf is already here.
                None
            };
        }

        // A leaf is already here so we cannot go deeper.
        if let Tree::Leaf(_) = self {
            return None;
        }

        if let Tree::Bifurcation(left, right) = self {
            if let Some(left) = left {
                // Try to add the gate to the left if there's already a left subtree.
                if left.try_add_gate_at_depth(g, depth - 1).is_some() {
                    return Some(());
                }
            } else {
                // Add a new left subtree and try to add the gate to it.
                let mut new_left = Tree::default();
                if new_left.try_add_gate_at_depth(g, depth - 1).is_some() {
                    *left = Some(Box::new(new_left));
                    return Some(());
                }
            }
            if let Some(right) = right {
                // Try to add the gate to the right if there's already a right subtree.
                if right.try_add_gate_at_depth(g, depth - 1).is_some() {
                    return Some(());
                }
            } else {
                // Add a new right subtree and try to add the gate to it.
                let mut new_right = Tree::default();
                if new_right.try_add_gate_at_depth(g, depth - 1).is_some() {
                    *right = Some(Box::new(new_right));
                    return Some(());
                }
            }
        }

        None
    }

    /// `Self::find_tree` returns a tree where each gate has `F(gate)=M` (see `Self::from_gates` comment).
    /// This can produce subtrees with more nodes than necessary. This function removes useless nodes,
    /// i.e., nodes that have a left but no right subtree.
    fn shorten(&mut self) {
        if let Tree::Bifurcation(left, right) = self {
            if let (Some(left), None) = (left, right) {
                // If the node has a left but no right subtree, set the node to its (shortened) left subtree.
                let mut new = *left.clone();
                new.shorten();
                *self = new;
            }
        }
        if let Tree::Bifurcation(left, right) = self {
            if let Some(left) = left {
                // Shorten the left subtree if there is one.
                left.shorten();
            }
            if let Some(right) = right {
                // Shorten the right subtree if there is one.
                right.shorten();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic::ArithmeticGate;
    use crate::gates::base_sum::BaseSumGate;
    use crate::gates::constant::ConstantGate;
    use crate::gates::gmimc::GMiMCGate;
    use crate::gates::interpolation::InterpolationGate;
    use crate::gates::mul_extension::MulExtensionGate;
    use crate::gates::noop::NoopGate;
    use crate::hash::GMIMC_ROUNDS;

    #[test]
    fn test_prefix_generation() {
        env_logger::init();
        type F = CrandallField;
        const D: usize = 4;

        let gates = vec![
            NoopGate::get::<F, D>(),
            ConstantGate::get(),
            ArithmeticGate::new(),
            BaseSumGate::<4>::new(4),
            GMiMCGate::<F, D, GMIMC_ROUNDS>::with_automatic_constants(),
            InterpolationGate::new(4),
            MulExtensionGate::new(),
        ];
        let len = gates.len();

        let tree = Tree::from_gates(gates.clone());
        let mut gates_with_prefix = tree.traversal();
        for (g, p) in &gates_with_prefix {
            println!("{} {:?}", &g.0.id()[..20.min(g.0.id().len())], p);
            println!(
                "{} {}",
                g.0.degree() + p.len(),
                g.0.num_constants() + p.len()
            );
        }

        assert_eq!(
            gates_with_prefix.len(),
            gates.len(),
            "The tree has too much or too little gates."
        );
        assert!(
            gates.iter().all(|g| gates_with_prefix
                .iter()
                .map(|(gg, _)| gg)
                .find(|gg| *gg == g)
                .is_some()),
            "Some gates are not in the tree."
        );
        assert!(
            gates_with_prefix
                .iter()
                .all(|(g, p)| g.0.degree() + g.0.num_constants() + p.len() <= 8),
            "Total degree is larger than 8."
        );

        gates_with_prefix.sort_unstable_by_key(|(g, p)| p.len());
        for i in 0..gates_with_prefix.len() {
            for j in i + 1..gates_with_prefix.len() {
                assert_ne!(
                    &gates_with_prefix[i].1,
                    &gates_with_prefix[j].1[0..gates_with_prefix[i].1.len()],
                    "Some gates share the same prefix"
                );
            }
        }
    }
}
