use std::collections::HashMap;
use std::iter::FromIterator;

use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::GateRef;

#[derive(Debug, Clone)]
enum Node<T> {
    Terminus(T),
    Bifurcation,
}

#[derive(Debug, Clone)]
pub struct Tree<T> {
    node: Node<T>,
    left: Option<Box<Tree<T>>>,
    right: Option<Box<Tree<T>>>,
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self {
            node: Node::Bifurcation,
            left: None,
            right: None,
        }
    }
}

impl<T: Clone> Tree<T> {
    pub fn preorder_traversal(&self) -> Vec<(T, Vec<bool>)> {
        let mut res = Vec::new();
        let prefix = [];
        self.traverse(&prefix, &mut res);
        res
    }

    fn traverse(&self, prefix: &[bool], current: &mut Vec<(T, Vec<bool>)>) {
        if let Node::Terminus(t) = &self.node {
            current.push((t.clone(), prefix.to_vec()));
        } else {
            if let Some(l) = &self.left {
                let mut left_prefix = prefix.to_vec();
                left_prefix.push(false);
                l.traverse(&left_prefix, current);
            }
            if let Some(r) = &self.right {
                let mut right_prefix = prefix.to_vec();
                right_prefix.push(true);
                r.traverse(&right_prefix, current);
            }
        }
    }
}

#[derive(Clone)]
pub struct GatePrefixes<F: Extendable<D>, const D: usize> {
    pub prefixes: HashMap<GateRef<F, D>, Vec<bool>>,
}

impl<F: Extendable<D>, const D: usize> From<Tree<GateRef<F, D>>> for GatePrefixes<F, D> {
    fn from(tree: Tree<GateRef<F, D>>) -> Self {
        GatePrefixes {
            prefixes: HashMap::from_iter(tree.preorder_traversal()),
        }
    }
}

impl<F: Extendable<D>, const D: usize> Tree<GateRef<F, D>> {
    pub fn from_gates(mut gates: Vec<GateRef<F, D>>) -> Self {
        let timer = std::time::Instant::now();
        gates.sort_unstable_by_key(|g| -((g.0.degree() + g.0.num_constants()) as isize));

        for max_degree in 1..100 {
            if let Some(mut tree) = Self::find_tree(&gates, max_degree) {
                tree.prune();
                println!(
                    "Found tree with max degree {} in {}s.",
                    max_degree,
                    timer.elapsed().as_secs_f32()
                );
                return tree;
            }
        }

        panic!("Can't find a tree.")
    }

    fn find_tree(gates: &[GateRef<F, D>], max_degree: usize) -> Option<Self> {
        let mut tree = Tree::default();

        for g in gates {
            tree.try_add_gate(g, max_degree)?;
        }
        Some(tree)
    }

    fn try_add_gate(&mut self, g: &GateRef<F, D>, max_degree: usize) -> Option<()> {
        let depth = max_degree.checked_sub(g.0.num_constants() + g.0.degree())?;
        self.try_add_gate_at_depth(g, depth).is_err().then(|| ())
    }

    fn try_add_gate_at_depth(&mut self, g: &GateRef<F, D>, depth: usize) -> Result<(), GateAdded> {
        if depth == 0 {
            return if let Node::Bifurcation = self.node {
                self.node = Node::Terminus(g.clone());
                Err(GateAdded)
            } else {
                Ok(())
            };
        }

        if let Node::Terminus(_) = self.node {
            return Ok(());
        }

        if let Some(left) = &mut self.left {
            left.try_add_gate_at_depth(g, depth - 1)?;
        } else {
            let mut left = Tree::default();
            if left.try_add_gate_at_depth(g, depth - 1).is_err() {
                self.left = Some(Box::new(left));
                return Err(GateAdded);
            }
        }
        if let Some(right) = &mut self.right {
            right.try_add_gate_at_depth(g, depth - 1)?;
        } else {
            let mut right = Tree::default();
            if right.try_add_gate_at_depth(g, depth - 1).is_err() {
                self.right = Some(Box::new(right));
                return Err(GateAdded);
            }
        }

        Ok(())
    }

    fn prune(&mut self) {
        if let (Some(left), None) = (&self.left, &self.right) {
            debug_assert!(matches!(self.node, Node::Bifurcation));
            let mut new = *left.clone();
            new.prune();
            *self = new;
        }
        if let Some(left) = &mut self.left {
            left.prune();
        }
        if let Some(right) = &mut self.right {
            right.prune();
        }
    }
}

struct GateAdded;
