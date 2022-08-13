use std::cmp::Ordering;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{BinaryHeap, HashMap};
use std::hash::Hash;

use itertools::Itertools;

use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::cpu::kernel::assembler::BYTES_PER_OFFSET;
use crate::cpu::kernel::ast::{Item, PushTarget, StackReplacement};
use crate::cpu::kernel::stack::permutations::{get_stack_ops_for_perm, is_permutation};
use crate::cpu::kernel::stack::stack_manipulation::StackOp::Pop;
use crate::cpu::kernel::utils::u256_to_trimmed_be_bytes;
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
    let mut src = names
        .iter()
        .cloned()
        .map(StackItem::NamedItem)
        .collect_vec();

    let mut dst = replacements
        .into_iter()
        .map(|item| match item {
            StackReplacement::Identifier(name) => {
                // May be either a named item or a label. Named items have precedence.
                if names.contains(&name) {
                    StackItem::NamedItem(name)
                } else {
                    StackItem::PushTarget(PushTarget::Label(name))
                }
            }
            StackReplacement::Literal(n) => StackItem::PushTarget(PushTarget::Literal(n)),
            StackReplacement::MacroLabel(_)
            | StackReplacement::MacroVar(_)
            | StackReplacement::Constant(_) => {
                panic!("Should have been expanded already: {:?}", item)
            }
        })
        .collect_vec();

    // %stack uses our convention where the top item is written on the left side.
    // `shortest_path` expects the opposite, so we reverse src and dst.
    src.reverse();
    dst.reverse();

    let unique_push_targets = dst
        .iter()
        .filter_map(|item| match item {
            StackItem::PushTarget(target) => Some(target.clone()),
            _ => None,
        })
        .unique()
        .collect_vec();

    let path = shortest_path(src, dst, unique_push_targets);
    path.into_iter().map(StackOp::into_item).collect()
}

/// Finds the lowest-cost sequence of `StackOp`s that transforms `src` to `dst`.
/// Uses a variant of Dijkstra's algorithm.
fn shortest_path(
    src: Vec<StackItem>,
    dst: Vec<StackItem>,
    unique_push_targets: Vec<PushTarget>,
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

        for op in next_ops(&node.stack, &dst, &unique_push_targets) {
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
pub(crate) enum StackItem {
    NamedItem(String),
    PushTarget(PushTarget),
}

#[derive(Clone, Debug)]
pub(crate) enum StackOp {
    Push(PushTarget),
    Pop,
    Dup(u8),
    Swap(u8),
}

/// A set of candidate operations to consider for the next step in the path from `src` to `dst`.
fn next_ops(
    src: &[StackItem],
    dst: &[StackItem],
    unique_push_targets: &[PushTarget],
) -> Vec<StackOp> {
    if let Some(top) = src.last() && !dst.contains(top) {
        // If the top of src doesn't appear in dst, don't bother with anything other than a POP.
        return vec![StackOp::Pop]
    }

    if is_permutation(src, dst) {
        // The transpositions are right-associative, so the last one gets applied first, hence pop.
        return vec![get_stack_ops_for_perm(src, dst).pop().unwrap()];
    }

    let mut ops = vec![StackOp::Pop];

    ops.extend(
        unique_push_targets
            .iter()
            // Only consider pushing this target if we need more occurrences of it, otherwise swaps
            // will be a better way to rearrange the existing occurrences as needed.
            .filter(|push_target| {
                let item = StackItem::PushTarget((*push_target).clone());
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

    ops.extend(
        (1..src_len)
            .filter(|i| should_try_swap(src, dst, *i))
            .map(StackOp::Swap),
    );

    ops
}

/// Whether we should consider `SWAP_i` in the search.
fn should_try_swap(src: &[StackItem], dst: &[StackItem], i: u8) -> bool {
    if src.is_empty() {
        return false;
    }

    let i = i as usize;
    let i_from = src.len() - 1;
    let i_to = i_from - i;

    // Only consider a swap if it places one of the two affected elements in the desired position.
    let top_correct_pos = i_to < dst.len() && src[i_from] == dst[i_to];
    let other_correct_pos = i_from < dst.len() && src[i_to] == dst[i_from];
    top_correct_pos | other_correct_pos
}

impl StackOp {
    fn cost(&self) -> u32 {
        let (cpu_rows, memory_rows) = match self {
            StackOp::Push(target) => {
                let bytes = match target {
                    PushTarget::Literal(n) => u256_to_trimmed_be_bytes(n).len() as u32,
                    PushTarget::Label(_) => BYTES_PER_OFFSET as u32,
                    PushTarget::MacroLabel(_)
                    | PushTarget::MacroVar(_)
                    | PushTarget::Constant(_) => {
                        panic!("Target should have been expanded already: {:?}", target)
                    }
                };
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
            StackOp::Push(target) => {
                stack.push(StackItem::PushTarget(target.clone()));
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
            StackOp::Push(target) => Item::Push(target),
            Pop => Item::StandardOp("POP".into()),
            StackOp::Dup(n) => Item::StandardOp(format!("DUP{}", n)),
            StackOp::Swap(n) => Item::StandardOp(format!("SWAP{}", n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};

    use crate::cpu::kernel::stack::stack_manipulation::StackItem::NamedItem;
    use crate::cpu::kernel::stack::stack_manipulation::{shortest_path, StackItem};

    #[test]
    fn test_shortest_path() {
        init_logger();
        shortest_path(
            vec![named("ret"), named("a"), named("b"), named("d")],
            vec![named("ret"), named("b"), named("a")],
            vec![],
        );
    }

    #[test]
    fn test_shortest_path_permutation() {
        init_logger();
        shortest_path(
            vec![named("a"), named("b"), named("c")],
            vec![named("c"), named("a"), named("b")],
            vec![],
        );
    }

    fn named(name: &str) -> StackItem {
        NamedItem(name.into())
    }

    fn init_logger() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));
    }
}
