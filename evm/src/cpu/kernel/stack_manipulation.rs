use std::cmp::Ordering;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BinaryHeap, HashMap};

use itertools::Itertools;

use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::cpu::kernel::ast::{Item, Literal, PushTarget, StackReplacement};
use crate::cpu::kernel::stack_manipulation::StackOp::Pop;
use crate::memory;

pub(crate) fn expand_stack_manipulation(body: Vec<Item>) -> Vec<Item> {
    let mut expanded = vec![];
    for item in body {
        if let Item::StackManipulation(names, replacements) = item {
            expanded.extend(expand(names, replacements));
        } else {
            expanded.push(item);
        }
    }
    expanded
}

fn expand(names: Vec<String>, replacements: Vec<StackReplacement>) -> Vec<Item> {
    let mut src = names.into_iter().map(StackItem::NamedItem).collect_vec();

    let unique_literals = replacements
        .iter()
        .filter_map(|item| match item {
            StackReplacement::Literal(n) => Some(n.clone()),
            _ => None,
        })
        .unique()
        .collect_vec();

    let mut dst = replacements
        .into_iter()
        .map(|item| match item {
            StackReplacement::NamedItem(name) => StackItem::NamedItem(name),
            StackReplacement::Literal(n) => StackItem::Literal(n),
            StackReplacement::MacroVar(_) | StackReplacement::Constant(_) => {
                panic!("Should have been expanded already: {:?}", item)
            }
        })
        .collect_vec();

    // %stack uses our convention where the top item is written on the left side.
    // `shortest_path` expects the opposite, so we reverse src and dst.
    src.reverse();
    dst.reverse();

    let path = shortest_path(src, dst, unique_literals);
    path.into_iter().map(StackOp::into_item).collect()
}

/// Finds the lowest-cost sequence of `StackOp`s that transforms `src` to `dst`.
/// Uses a variant of Dijkstra's algorithm.
fn shortest_path(
    src: Vec<StackItem>,
    dst: Vec<StackItem>,
    unique_literals: Vec<Literal>,
) -> Vec<StackOp> {
    // Nodes to visit, starting with the lowest-cost node.
    let mut queue = BinaryHeap::new();
    queue.push(Node {
        stack: src.clone(),
        cost: 0,
    });

    // For each node, stores `(best_cost, Option<(parent, op)>)`.
    let mut node_info = HashMap::<Vec<StackItem>, (u32, Option<(Vec<StackItem>, StackOp)>)>::new();
    node_info.insert(src.clone(), (0, None));

    while let Some(node) = queue.pop() {
        if node.stack == dst {
            // The destination is now the lowest-cost node, so we must have found the best path.
            let mut path = vec![];
            let mut stack = &node.stack;
            // Rewind back to src, recording a list of operations which will be backwards.
            while let Some((parent, op)) = &node_info[stack].1 {
                stack = parent;
                path.push(op.clone());
            }
            assert_eq!(stack, &src);
            path.reverse();
            return path;
        }

        let (best_cost, _) = node_info[&node.stack];
        if best_cost < node.cost {
            // Since we can't efficiently remove nodes from the heap, it can contain duplicates.
            // In this case, we've already visited this stack state with a lower cost.
            continue;
        }

        for op in next_ops(&node.stack, &dst, &unique_literals) {
            let neighbor = match op.apply_to(node.stack.clone()) {
                Some(n) => n,
                None => continue,
            };

            let cost = node.cost + op.cost();
            let entry = node_info.entry(neighbor.clone());
            if let Occupied(e) = &entry && e.get().0 <= cost {
                // We already found a better or equal path.
                continue;
            }

            let neighbor_info = (cost, Some((node.stack.clone(), op.clone())));
            match entry {
                Occupied(mut e) => {
                    e.insert(neighbor_info);
                }
                Vacant(e) => {
                    e.insert(neighbor_info);
                }
            }

            queue.push(Node {
                stack: neighbor,
                cost,
            });
        }
    }

    panic!("No path found from {:?} to {:?}", src, dst)
}

/// A node in the priority queue used by Dijkstra's algorithm.
#[derive(Eq, PartialEq)]
struct Node {
    stack: Vec<StackItem>,
    cost: u32,
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // We want a min-heap rather than the default max-heap, so this is the opposite of the
        // natural ordering of costs.
        other.cost.cmp(&self.cost)
    }
}

/// Like `StackReplacement`, but without constants or macro vars, since those were expanded already.
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
enum StackItem {
    NamedItem(String),
    Literal(Literal),
}

#[derive(Clone, Debug)]
enum StackOp {
    Push(Literal),
    Pop,
    Dup(u8),
    Swap(u8),
}

/// A set of candidate operations to consider for the next step in the path from `src` to `dst`.
fn next_ops(src: &[StackItem], dst: &[StackItem], unique_literals: &[Literal]) -> Vec<StackOp> {
    if let Some(top) = src.last() && !dst.contains(top) {
        // If the top of src doesn't appear in dst, don't bother with anything other than a POP.
        return vec![StackOp::Pop]
    }

    let mut ops = vec![StackOp::Pop];

    ops.extend(
        unique_literals
            .iter()
            // Only consider pushing this literal if we need more occurrences of it, otherwise swaps
            // will be a better way to rearrange the existing occurrences as needed.
            .filter(|lit| {
                let item = StackItem::Literal((*lit).clone());
                let src_count = src.iter().filter(|x| **x == item).count();
                let dst_count = dst.iter().filter(|x| **x == item).count();
                src_count < dst_count
            })
            .cloned()
            .map(StackOp::Push),
    );

    let src_len = src.len() as u8;

    ops.extend(
        (1..=src_len)
            // Only consider duplicating this item if we need more occurrences of it, otherwise swaps
            // will be a better way to rearrange the existing occurrences as needed.
            .filter(|i| {
                let item = &src[src.len() - *i as usize];
                let src_count = src.iter().filter(|x| *x == item).count();
                let dst_count = dst.iter().filter(|x| *x == item).count();
                src_count < dst_count
            })
            .map(StackOp::Dup),
    );

    ops.extend((1..src_len).map(StackOp::Swap));

    ops
}

impl StackOp {
    fn cost(&self) -> u32 {
        let (cpu_rows, memory_rows) = match self {
            StackOp::Push(n) => {
                let bytes = n.to_trimmed_be_bytes().len() as u32;
                // This is just a rough estimate; we can update it after implementing PUSH.
                (bytes, bytes)
            }
            // A POP takes one cycle, and doesn't involve memory, it just decrements a pointer.
            Pop => (1, 0),
            // A DUP takes one cycle, and a read and a write.
            StackOp::Dup(_) => (1, 2),
            // A SWAP takes one cycle with four memory ops, to read both values then write to them.
            StackOp::Swap(_) => (1, 4),
        };

        let cpu_cost = cpu_rows * NUM_CPU_COLUMNS as u32;
        let memory_cost = memory_rows * memory::columns::NUM_COLUMNS as u32;
        cpu_cost + memory_cost
    }

    /// Returns an updated stack after this operation is performed, or `None` if this operation
    /// would not be valid on the given stack.
    fn apply_to(&self, mut stack: Vec<StackItem>) -> Option<Vec<StackItem>> {
        let len = stack.len();
        match self {
            StackOp::Push(n) => {
                stack.push(StackItem::Literal(n.clone()));
            }
            Pop => {
                stack.pop()?;
            }
            StackOp::Dup(n) => {
                let idx = len.checked_sub(*n as usize)?;
                stack.push(stack[idx].clone());
            }
            StackOp::Swap(n) => {
                let from = len.checked_sub(1)?;
                let to = len.checked_sub(*n as usize + 1)?;
                stack.swap(from, to);
            }
        }
        Some(stack)
    }

    fn into_item(self) -> Item {
        match self {
            StackOp::Push(n) => Item::Push(PushTarget::Literal(n)),
            Pop => Item::StandardOp("POP".into()),
            StackOp::Dup(n) => Item::StandardOp(format!("DUP{}", n)),
            StackOp::Swap(n) => Item::StandardOp(format!("SWAP{}", n)),
        }
    }
}
