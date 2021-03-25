use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::fft::coset_ifft;
use crate::field::field::Field;
use crate::gadgets::split_join::{split_le_constraints, split_le_generator_local_wires};
use crate::gates::deterministic_gate::{DeterministicGate, DeterministicGateAdapter};
use crate::gates::gate::GateRef;
use crate::gates::output_graph::{OutputGraph, GateOutputLocation, ExpandableOutputGraph};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;
use crate::gadgets::conditionals::conditional_multiply_poly;

/// Performs a FRI consistency check. The goal is to check consistency between polynomials `f^(i)`
/// and `f^(i + 1)`, where `f^(i + 1)` is supposed to be an `arity`-to-one reduction of `f^(i)`. See
/// the FRI paper for details.
///
/// This check involves `arity` openings of `f^(i)` and a single opening of `f^(i + 1)`. Let's call
/// the set of `f^(i)` opening locations `{s_j}`, and the `f^(i + 1)` opening location `y`. Note
/// that all of these points can be derived from the query path. So, we take as input an integer
/// whose binary decomposition represents the left and right turns in the query path.
///
/// Since our protocol uses a variant of FRI with `k` commit phases, this gate takes `k` opening
/// sets for `f^(i)`, and `k` openings for `f^(i + 1)`.
///
/// Putting it together, this gate does the following:
/// - Computes the binary decomposition of the input representing the query path.
/// - Computes `{s_j}` and `y` from the query path.
/// - For each commit phase:
///   - Non-deterministically interpolates a polynomial through each `(s_j, f^(i))`.
///   - Evaluates the purported interpolant at each `s_j`, and checks that the result matches the
///     associated opening of `f^(i)(s_j)`.
///   - Evaluates the purported interpolant at `y`, and checks that the result matches the opening
///     of `f^(i + 1)(y)`.
#[derive(Debug, Copy, Clone)]
pub(crate) struct FriConsistencyGate {
    /// The arity of this reduction step.
    arity_bits: usize,

    /// The number of commit phases.
    num_commits: usize,

    /// The maximum number of bits in any query path.
    max_path_bits: usize,
}

impl FriConsistencyGate {
    pub fn new<F: Field>(
        arity_bits: usize,
        num_commits: usize,
        max_path_bits: usize,
    ) -> GateRef<F> {
        let gate = Self { arity_bits, num_commits, max_path_bits };
        GateRef::new(DeterministicGateAdapter::new(gate))
    }

    fn arity(&self) -> usize {
        1 << self.arity_bits
    }

    /// Generator for the `i`'th layer of FRI.
    pub const CONST_GENERATOR_I: usize = 0;

    // Note: These methods relating to wire indices are ordered by wire index. The index
    // calculations are a little hairy since there are quite a few different "sections" of wires,
    // and each section has to calculate where the previous sections left off. To make it more
    // manageable, we have separate functions to calculate the start of each section. There is also
    // a test to make sure that they behave as expected, with no overlapping indices or what not.

    /// An integer representing the location of the `f^(i + 1)` node in the reduction tree. Its
    /// `i`th bit in little-endian form represents the direction of the `i`th turn in the query
    /// path, starting from the root.
    pub const WIRE_PATH: usize = 0;

    fn start_wire_f_i(&self) -> usize {
        1
    }

    pub fn wire_f_i(&self, commit_idx: usize, j: usize) -> usize {
        debug_assert!(commit_idx < self.num_commits);
        debug_assert!(j < self.arity());
        self.start_wire_f_i() + self.arity() * commit_idx + j
    }

    fn start_wire_f_i_plus_1(&self) -> usize {
        self.start_wire_f_i() + self.arity() * self.num_commits
    }

    pub fn wire_f_i_plus_1(&self, commit_idx: usize) -> usize {
        debug_assert!(commit_idx < self.num_commits);
        self.start_wire_f_i_plus_1() + commit_idx
    }

    fn start_wire_path_bits(&self) -> usize {
        self.start_wire_f_i_plus_1() + self.num_commits
    }

    /// The `i`th bit of the path.
    fn wire_path_bit_i(&self, i: usize) -> usize {
        self.start_wire_path_bits() + i
    }

    fn start_wire_s_j(&self) -> usize {
        self.start_wire_path_bits() + self.max_path_bits
    }

    /// The input index of `s_j` (see the FRI paper).
    fn wire_s_j(&self, j: usize) -> usize {
        debug_assert!(j < self.arity());
        self.start_wire_s_j() + j
    }

    fn start_wire_y(&self) -> usize {
        self.start_wire_s_j() + self.arity()
    }

    /// The input index of `y` (see the FRI paper).
    fn wire_y(&self) -> usize {
        self.start_wire_y()
    }

    fn start_wire_coefficient(&self) -> usize {
        self.start_wire_y() + 1
    }

    /// The wire input index of the j'th coefficient of the interpolant.
    fn wire_coefficient(&self, commit_idx: usize, j: usize) -> usize {
        debug_assert!(commit_idx < self.num_commits);
        debug_assert!(j < self.arity());
        self.start_wire_coefficient() + commit_idx * self.arity() + j
    }

    fn start_unnamed_wires(&self) -> usize {
        self.start_wire_coefficient() + self.num_commits * self.arity()
    }

    fn add_s_j_outputs<F: Field>(&self, output_graph: &mut ExpandableOutputGraph<F>) {
        // Each s_j = g^path, where g is the generator for s_j's layer (see CONST_GENERATOR), and
        // path is an integer representing the location of s_j in the reduction tree. This assumes
        // that path is encoded such that its less significant bits are closer to the root of the
        // tree.

        // Note about bit ordering: in a FRI reduction tree, the `j`th node in a layer can be
        // written as `g^rev(j)`, where `rev` reverses the bits of its inputs. One way to think of
        // this is that squaring left-shifts the exponent, so for adjacent nodes to have the same
        // square, they must differ only in the left-most bit (which will "overflow" after the
        // left-shift). FFT trees have the same property.

        // We start by computing g^0, g^10, g^100, g^1000, ...
        let mut squares = vec![ConstraintPolynomial::local_constant(0)];
        for _ in 1..self.max_path_bits {
            let prev_square = squares.last().unwrap();
            let next_square = output_graph.add(prev_square.square());
            squares.push(next_square)
        }

        // We can think of path as having two parts: a less significant part that is common to all
        // {s_j}, and a more significant part that depends on j. We start by computing
        // g^path_common:
        let mut g_exp_path_common = ConstraintPolynomial::zero();
        let shared_path_bits = self.max_path_bits - self.arity_bits;
        for i in 0..shared_path_bits {
            let bit = ConstraintPolynomial::local_wire(self.wire_path_bit_i(i));
            g_exp_path_common = conditional_multiply_poly(&g_exp_path_common, &squares[i], &bit);
            g_exp_path_common = output_graph.add(g_exp_path_common);
        }

        // Then, we factor in the "extra" powers of g specific to each child.
        for j in 0..self.arity() {
            let mut s_j = g_exp_path_common.clone();
            for bit_index in 0..self.arity_bits {
                let bit = (j >> bit_index & 1) != 0;
                if bit {
                    // See the comment near the top about bit ordering.
                    s_j *= &squares[shared_path_bits + self.arity_bits - 1 - bit_index];
                }
            }
            let s_j_loc = GateOutputLocation::LocalWire(self.wire_s_j(j));
            output_graph.output_graph.add(s_j_loc, s_j);
        }
    }

    fn add_y_output<F: Field>(&self, output_graph: &mut ExpandableOutputGraph<F>) {
        let loc = GateOutputLocation::LocalWire(self.wire_y());
        // We can start with any s_j and repeatedly square it. We arbitrary pick s_0.
        let mut out = ConstraintPolynomial::local_wire(self.wire_s_j(0));
        for _ in 0..self.arity_bits {
            out = out.square();
        }
        output_graph.output_graph.add(loc, out);
    }

    fn evaluate_each_poly<F: Field>(&self, commit_idx: usize) -> Vec<ConstraintPolynomial<F>> {
        let coefficients = (0..self.arity())
            .map(|i| ConstraintPolynomial::local_wire(self.wire_coefficient(commit_idx, i)))
            .collect::<Vec<ConstraintPolynomial<F>>>();
        let mut constraints = Vec::new();

        for j in 0..self.arity() {
            // Check the evaluation of f^(i) at s_j.
            let expected = ConstraintPolynomial::local_wire(self.wire_f_i(commit_idx, j));
            let actual = self.evaluate_poly(&coefficients,
                                            ConstraintPolynomial::local_wire(self.wire_s_j(j)));
            constraints.push(actual - expected);
        }

        // Check the evaluation of f^(i + 1) at y.
        let expected = ConstraintPolynomial::local_wire(self.wire_f_i_plus_1(commit_idx));
        let actual = self.evaluate_poly(&coefficients,
                                        ConstraintPolynomial::local_wire(self.wire_y()));
        constraints.push(actual - expected);

        constraints
    }

    /// Given a polynomial's coefficients, naively evaluate it at a point.
    fn evaluate_poly<F: Field>(
        &self,
        coefficients: &[ConstraintPolynomial<F>],
        point: ConstraintPolynomial<F>,
    ) -> ConstraintPolynomial<F> {
        coefficients.iter()
            .enumerate()
            .map(|(i, coeff)| coeff * point.exp(i))
            .sum()
    }
}

impl<F: Field> DeterministicGate<F> for FriConsistencyGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn outputs(&self, _config: CircuitConfig) -> OutputGraph<F> {
        let mut output_graph = ExpandableOutputGraph::new(self.start_unnamed_wires());
        self.add_s_j_outputs(&mut output_graph);
        self.add_y_output(&mut output_graph);
        output_graph.output_graph
    }

    fn additional_constraints(&self, _config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        let mut constraints = Vec::new();

        // Add constraints for splitting the path into its binary representation.
        let bits = (0..self.max_path_bits)
            .map(|i| ConstraintPolynomial::local_wire(self.wire_path_bit_i(i)))
            .collect::<Vec<_>>();
        let split_constraints = split_le_constraints(
            ConstraintPolynomial::local_wire(Self::WIRE_PATH),
            &bits);
        constraints.extend(split_constraints);

        // Add constraints for checking each polynomial evaluation.
        for commit_idx in 0..self.num_commits {
            constraints.extend(self.evaluate_each_poly(commit_idx));
        }

        constraints
    }

    fn additional_generators(
        &self,
        _config: CircuitConfig,
        gate_index: usize,
        local_constants: Vec<F>,
        _next_constants: Vec<F>,
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let interpolant_generator = Box::new(
            InterpolantGenerator {
                gate: *self,
                gate_index,
                generator_i: local_constants[Self::CONST_GENERATOR_I],
            }
        );

        let bit_input_indices = (0..self.max_path_bits)
            .map(|i| self.wire_path_bit_i(i))
            .collect::<Vec<_>>();
        let split_generator = split_le_generator_local_wires(
            gate_index, Self::WIRE_PATH, &bit_input_indices);

        vec![interpolant_generator, split_generator]
    }
}

#[derive(Debug)]
struct InterpolantGenerator<F: Field> {
    gate: FriConsistencyGate,
    gate_index: usize,
    generator_i: F,
}

impl<F: Field> InterpolantGenerator<F> {
    /// Convenience method for converting a wire input index into a Target with our gate index.
    fn local_wire(&self, input: usize) -> Target {
        Target::Wire(Wire { gate: self.gate_index, input })
    }
}

impl<F: Field> SimpleGenerator<F> for InterpolantGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        let mut deps = vec![self.local_wire(FriConsistencyGate::WIRE_PATH)];
        for i in 0..self.gate.arity() {
            deps.push(self.local_wire(self.gate.wire_s_j(i)));
            for commit_idx in 0..self.gate.num_commits {
                deps.push(self.local_wire(self.gate.wire_f_i(commit_idx, i)));
            }
        }
        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut result = PartialWitness::new();

        for commit_idx in 0..self.gate.num_commits {
            let values = (0..self.gate.arity())
                .map(|j| witness.get_target(self.local_wire(self.gate.wire_f_i(commit_idx, j))))
                .collect();

            let path = witness.get_target(self.local_wire(FriConsistencyGate::WIRE_PATH));
            let shift = self.generator_i.exp(path);
            let coeffs = coset_ifft(values, shift);

            for (i, coeff) in coeffs.into_iter().enumerate() {
                result.set_target(
                    self.local_wire(self.gate.wire_coefficient(commit_idx, i)),
                    coeff);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::gates::fri_consistency_gate::FriConsistencyGate;

    #[test]
    fn wire_indices() {
        let gate = FriConsistencyGate {
            arity_bits: 1,
            num_commits: 2,
            max_path_bits: 4,
        };

        // The actual indices aren't really important, but we want to make sure that
        // - there are no overlaps
        // - there are no gaps
        // - the routed inputs come first
        assert_eq!(0, FriConsistencyGate::WIRE_PATH);
        assert_eq!(1, gate.wire_f_i(0, 0));
        assert_eq!(2, gate.wire_f_i(0, 1));
        assert_eq!(3, gate.wire_f_i(1, 0));
        assert_eq!(4, gate.wire_f_i(1, 1));
        assert_eq!(5, gate.wire_f_i_plus_1(0));
        assert_eq!(6, gate.wire_f_i_plus_1(1));
        assert_eq!(7, gate.wire_path_bit_i(0));
        assert_eq!(8, gate.wire_path_bit_i(1));
        assert_eq!(9, gate.wire_path_bit_i(2));
        assert_eq!(10, gate.wire_path_bit_i(3));
        assert_eq!(11, gate.wire_s_j(0));
        assert_eq!(12, gate.wire_s_j(1));
        assert_eq!(13, gate.wire_y());
        assert_eq!(14, gate.wire_coefficient(0, 0));
        assert_eq!(15, gate.wire_coefficient(0, 1));
        assert_eq!(16, gate.wire_coefficient(1, 0));
        assert_eq!(17, gate.wire_coefficient(1, 1));
        assert_eq!(18, gate.start_unnamed_wires());
    }
}
