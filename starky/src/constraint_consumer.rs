use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;

pub struct ConstraintConsumer<P: PackedField> {
    /// Random values used to combine multiple constraints into one.
    alphas: Vec<P::Scalar>,

    /// Running sums of constraints that have been emitted so far, scaled by powers of alpha.
    // TODO(JN): This is pub so it can be used in a test. Once we have an API for accessing this
    // result, it should be made private.
    pub constraint_accs: Vec<P>,

    /// The evaluation of `X - g^(n-1)`.
    z_last: P,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the first trace row, and zero at other points in the subgroup.
    lagrange_basis_first: P,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the last trace row, and zero at other points in the subgroup.
    lagrange_basis_last: P,
}

impl<P: PackedField> ConstraintConsumer<P> {
    pub fn new(
        alphas: Vec<P::Scalar>,
        z_last: P,
        lagrange_basis_first: P,
        lagrange_basis_last: P,
    ) -> Self {
        Self {
            constraint_accs: vec![P::ZEROS; alphas.len()],
            alphas,
            z_last,
            lagrange_basis_first,
            lagrange_basis_last,
        }
    }

    pub fn accumulators(self) -> Vec<P> {
        self.constraint_accs
    }

    /// Add one constraint valid on all rows except the last.
    pub fn constraint_transition(&mut self, constraint: P) {
        self.constraint(constraint * self.z_last);
    }

    /// Add one constraint on all rows.
    pub fn constraint(&mut self, constraint: P) {
        for (&alpha, acc) in self.alphas.iter().zip(&mut self.constraint_accs) {
            *acc *= alpha;
            *acc += constraint;
        }
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// first row of the trace.
    pub fn constraint_first_row(&mut self, constraint: P) {
        self.constraint(constraint * self.lagrange_basis_first);
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// last row of the trace.
    pub fn constraint_last_row(&mut self, constraint: P) {
        self.constraint(constraint * self.lagrange_basis_last);
    }
}

pub struct RecursiveConstraintConsumer<F: RichField + Extendable<D>, const D: usize> {
    /// A random value used to combine multiple constraints into one.
    alphas: Vec<Target>,

    /// A running sum of constraints that have been emitted so far, scaled by powers of alpha.
    constraint_accs: Vec<ExtensionTarget<D>>,

    /// The evaluation of `X - g^(n-1)`.
    z_last: ExtensionTarget<D>,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the first trace row, and zero at other points in the subgroup.
    lagrange_basis_first: ExtensionTarget<D>,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the last trace row, and zero at other points in the subgroup.
    lagrange_basis_last: ExtensionTarget<D>,

    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> RecursiveConstraintConsumer<F, D> {
    pub fn new(
        zero: ExtensionTarget<D>,
        alphas: Vec<Target>,
        z_last: ExtensionTarget<D>,
        lagrange_basis_first: ExtensionTarget<D>,
        lagrange_basis_last: ExtensionTarget<D>,
    ) -> Self {
        Self {
            constraint_accs: vec![zero; alphas.len()],
            alphas,
            z_last,
            lagrange_basis_first,
            lagrange_basis_last,
            _phantom: Default::default(),
        }
    }

    pub fn accumulators(self) -> Vec<ExtensionTarget<D>> {
        self.constraint_accs
    }

    /// Add one constraint valid on all rows except the last.
    pub fn constraint_transition(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.z_last);
        self.constraint(builder, filtered_constraint);
    }

    /// Add one constraint valid on all rows.
    pub fn constraint(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        for (&alpha, acc) in self.alphas.iter().zip(&mut self.constraint_accs) {
            *acc = builder.scalar_mul_add_extension(alpha, *acc, constraint);
        }
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// first row of the trace.
    pub fn constraint_first_row(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.lagrange_basis_first);
        self.constraint(builder, filtered_constraint);
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// last row of the trace.
    pub fn constraint_last_row(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.lagrange_basis_last);
        self.constraint(builder, filtered_constraint);
    }
}
