//! This module contains logic for finding the optimal sequence of swaps to get from one stack state
//! to another, specifically for the case where the source and destination states are permutations
//! of one another.
//!
//! We solve the problem in three steps:
//! 1. Find a permutation `P` such that `P A = B`.
//! 2. If `A` contains duplicates, optimize `P` by reducing the number of cycles.
//! 3. Convert each cycle into a set of `(0 i)` transpositions, which correspond to swap
//!    instructions in the EVM.
//!
//! We typically represent a permutation as a sequence of cycles. For example, the permutation
//! `(1 2 3)(1 2)(4 5)` acts as:
//!
//! ```ignore
//! (1 2 3)(1 2)(4 5)[A_0, A_1, A_2, A_3, A_4, A_5] = (1 2 3)(1 2)[A_0, A_1, A_2, A_3, A_5, A_4]
//!                                                 = (1 2 3)[A_0, A_2, A_1, A_3, A_5, A_4]
//!                                                 = [A_0, A_3, A_2, A_1, A_5, A_4]
//! ```
//!
//! We typically represent a `(0 i)` transposition as a single scalar `i`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::cpu::kernel::stack::stack_manipulation::{StackItem, StackOp};

/// Find the optimal sequence of stack operations to get from `src` to `dst`. Assumes that `src` and
/// `dst` are permutations of one another.
pub(crate) fn get_stack_ops_for_perm(src: &[StackItem], dst: &[StackItem]) -> Vec<StackOp> {
    // We store stacks with the tip at the end, but the permutation calls below use the opposite
    // convention. They're a bit simpler when SWAP are (0 i) transposes.
    let mut src = src.to_vec();
    let mut dst = dst.to_vec();
    src.reverse();
    dst.reverse();

    let perm = find_permutation(&src, &dst);
    let optimized_perm = combine_cycles(perm, &src);
    let trans = permutation_to_transpositions(optimized_perm);
    transpositions_to_stack_ops(trans)
}

/// Apply the given permutation to the given list.
#[cfg(test)]
fn apply_perm<T: Eq + Hash + Clone>(permutation: Vec<Vec<usize>>, mut lst: Vec<T>) -> Vec<T> {
    // Run through perm in REVERSE order.
    for cycl in permutation.iter().rev() {
        let n = cycl.len();
        let last = lst[cycl[n - 1]].clone();
        for i in (0..n - 1).rev() {
            let j = (i + 1) % n;
            lst[cycl[j]] = lst[cycl[i]].clone();
        }
        lst[cycl[0]] = last;
    }
    lst
}

/// This function does STEP 1.
/// Given 2 lists A, B find a permutation P such that P . A = B.
pub fn find_permutation<T: Eq + Hash + Clone>(lst_a: &[T], lst_b: &[T]) -> Vec<Vec<usize>> {
    // We should check to ensure that A and B are indeed rearrangments of each other.
    assert!(is_permutation(lst_a, lst_b));

    let n = lst_a.len();

    // Keep track of the A_i's which have been already placed into the correct position.
    let mut correct_a = HashSet::new();

    // loc_b is a dictionary where loc_b[b] is the indices i where b = B_i != A_i.
    // We need to swap appropriate A_j's into these positions.
    let mut loc_b: HashMap<T, Vec<usize>> = HashMap::new();

    for i in 0..n {
        if lst_a[i] == lst_b[i] {
            // If A_i = B_i, we never do SWAP_i as we are already in the correct position.
            correct_a.insert(i);
        } else {
            loc_b.entry(lst_b[i].clone()).or_default().push(i);
        }
    }

    // This will be a list of disjoint cycles.
    let mut permutation = vec![];

    // For technical reasons, it's handy to include [0] as a trivial cycle.
    // This is because if A_0 = A_i for some other i in a cycle,
    // we can save transpositions by expanding the cycle to include 0.
    if correct_a.contains(&0) {
        permutation.push(vec![0]);
    }

    for i in 0..n {
        // If i is both not in the correct position and not already in a cycle, it will start a new cycle.
        if correct_a.contains(&i) {
            continue;
        }

        correct_a.insert(i);
        let mut cycl = vec![i];

        // lst_a[i] need to be swapped into an index j such that lst_b[j] = lst_a[i].
        // This exactly means j should be an element of loc_b[lst_a[i]].
        // We pop as each j should only be used once.
        // In this step we simply find any permutation. We will improve it to an optimal one in STEP 2.
        let mut j = loc_b.get_mut(&lst_a[i]).unwrap().pop().unwrap();

        // Keep adding elements to the cycle until we return to our initial index
        while j != i {
            correct_a.insert(j);
            cycl.push(j);
            j = loc_b.get_mut(&lst_a[j]).unwrap().pop().unwrap();
        }

        permutation.push(cycl);
    }
    permutation
}

/// This function does STEP 2. It tests to see if cycles can be combined which might occur if A has duplicates.
fn combine_cycles<T: Eq + Hash + Clone>(mut perm: Vec<Vec<usize>>, lst_a: &[T]) -> Vec<Vec<usize>> {
    // If perm is a single cycle, there is nothing to combine.
    if perm.len() == 1 {
        return perm;
    }

    let n = lst_a.len();

    // Need a dictionary to keep track of duplicates in lst_a.
    let mut all_a_positions: HashMap<T, Vec<usize>> = HashMap::new();
    for i in 0..n {
        all_a_positions.entry(lst_a[i].clone()).or_default().push(i);
    }

    // For each element a which occurs at positions i1, ..., ij, combine cycles such that all
    // ik which occur in a cycle occur in the same cycle.
    for positions in all_a_positions.values() {
        if positions.len() == 1 {
            continue;
        }

        let mut joinedperm = vec![];
        let mut newperm = vec![];
        let mut pos = 0;
        for cycl in perm {
            // Does cycl include an element of positions?
            let mut disjoint = true;

            for term in positions {
                if cycl.contains(term) {
                    if joinedperm.is_empty() {
                        // This is the first cycle we have found including an element of positions.
                        joinedperm = cycl.clone();
                        pos = cycl.iter().position(|x| x == term).unwrap();
                    } else {
                        // Need to merge 2 cycles. If A_i = A_j then the permutations
                        // (C_1, ..., C_k1, i, C_{k1 + 1}, ... C_k2)(D_1, ..., D_k3, j, D_{k3 + 1}, ... D_k4)
                        // (C_1, ..., C_k1, i, D_{k3 + 1}, ... D_k4, D_1, ..., D_k3, j, C_{k1 + 1}, ... C_k2)
                        // lead to the same oupput but the second will require less transpositions.
                        let newpos = cycl.iter().position(|x| x == term).unwrap();
                        joinedperm = [
                            &joinedperm[..pos + 1],
                            &cycl[newpos + 1..],
                            &cycl[..newpos + 1],
                            &joinedperm[pos + 1..],
                        ]
                        .concat();
                    }
                    disjoint = false;
                    break;
                }
            }
            if disjoint {
                newperm.push(cycl);
            }
        }
        if !joinedperm.is_empty() {
            newperm.push(joinedperm);
        }
        perm = newperm;
    }
    perm
}

// This function does STEP 3. Converting all cycles to [0, i] transpositions.
fn permutation_to_transpositions(perm: Vec<Vec<usize>>) -> Vec<usize> {
    let mut trans = vec![];
    // The method is pretty simple, we have:
    // (0 C_1 ... C_i) = (0 C_i) ... (0 C_1)
    // (C_1 ... C_i) = (0 C_1) (0 C_i) ... (0\ C_1).
    // We simply need to check to see if 0 is in our cycle to see which one to use.
    for cycl in perm {
        let n = cycl.len();
        let zero_pos = cycl.iter().position(|x| *x == 0);
        if let Some(pos) = zero_pos {
            trans.extend((1..n).map(|i| cycl[(n + pos - i) % n]));
        } else {
            trans.extend((0..=n).map(|i| cycl[(n - i) % n]));
        }
    }
    trans
}

#[cfg(test)]
fn trans_to_perm(trans: Vec<usize>) -> Vec<Vec<usize>> {
    trans.into_iter().map(|i| vec![0, i]).collect()
}

fn transpositions_to_stack_ops(trans: Vec<usize>) -> Vec<StackOp> {
    trans.into_iter().map(|i| StackOp::Swap(i as u8)).collect()
}

pub fn is_permutation<T: Eq + Hash + Clone>(a: &[T], b: &[T]) -> bool {
    make_multiset(a) == make_multiset(b)
}

fn make_multiset<T: Eq + Hash + Clone>(vals: &[T]) -> HashMap<T, usize> {
    let mut counts = HashMap::new();
    for val in vals {
        *counts.entry(val.clone()).or_default() += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use rand::prelude::SliceRandom;
    use rand::thread_rng;

    use crate::cpu::kernel::stack::permutations::{
        apply_perm, combine_cycles, find_permutation, is_permutation,
        permutation_to_transpositions, trans_to_perm,
    };

    #[test]
    fn test_combine_cycles() {
        assert_eq!(
            combine_cycles(vec![vec![0, 2], vec![3, 4]], &['a', 'b', 'c', 'd', 'a']),
            vec![vec![0, 3, 4, 2]]
        );
    }

    #[test]
    fn test_is_permutation() {
        assert!(is_permutation(&['a', 'b', 'c'], &['b', 'c', 'a']));
        assert!(!is_permutation(&['a', 'b', 'c'], &['a', 'b', 'b', 'c']));
        assert!(!is_permutation(&['a', 'b', 'c'], &['a', 'd', 'c']));
    }

    #[test]
    fn test_all() {
        let mut test_lst = vec![
            'a', 'a', 'a', 'a', 'b', 'b', 'b', 'c', 'c', 'c', 'd', 'd', 'e', 'f', 'g', 'h', 'k',
        ];

        let mut rng = thread_rng();
        test_lst.shuffle(&mut rng);
        for _ in 0..1000 {
            let lst_a = test_lst.clone();
            test_lst.shuffle(&mut rng);
            let lst_b = test_lst.clone();

            let perm = find_permutation(&lst_a, &lst_b);
            assert_eq!(apply_perm(perm.clone(), lst_a.clone()), lst_b);

            let shortperm = combine_cycles(perm.clone(), &lst_a);
            assert_eq!(apply_perm(shortperm.clone(), lst_a.clone()), lst_b);

            let trans = trans_to_perm(permutation_to_transpositions(perm));
            assert_eq!(apply_perm(trans.clone(), lst_a.clone()), lst_b);

            let shorttrans = trans_to_perm(permutation_to_transpositions(shortperm));
            assert_eq!(apply_perm(shorttrans.clone(), lst_a.clone()), lst_b);

            assert!(shorttrans.len() <= trans.len());
        }
    }
}
