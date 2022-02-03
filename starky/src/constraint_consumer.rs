use std::marker::PhantomData;

use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;

pub struct ConstraintConsumer<P: PackedField> {
    /// Random values used to combine multiple constraints into one.
    alphas: Vec<P::Scalar>,

    /// Running sums of constraints that have been emitted so far, scaled by powers of alpha.
    constraint_accs: Vec<P>,

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

    // TODO: Do this correctly.
    pub fn accumulators(self) -> Vec<P::Scalar> {
        self.constraint_accs
            .into_iter()
            .map(|acc| acc.as_slice()[0])
            .collect()
    }

    /// Add one constraint valid on all rows except the last.
    pub fn constraint(&mut self, constraint: P) {
        self.constraint_wrapping(constraint * self.z_last);
    }

    /// Add one constraint on all rows.
    pub fn constraint_wrapping(&mut self, constraint: P) {
        for (&alpha, acc) in self.alphas.iter().zip(&mut self.constraint_accs) {
            *acc *= alpha;
            *acc += constraint;
        }
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// first row of the trace.
    pub fn constraint_first_row(&mut self, constraint: P) {
        self.constraint_wrapping(constraint * self.lagrange_basis_first);
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// last row of the trace.
    pub fn constraint_last_row(&mut self, constraint: P) {
        self.constraint_wrapping(constraint * self.lagrange_basis_last);
    }
}

pub struct RecursiveConstraintConsumer<F: RichField + Extendable<D>, const D: usize> {
    /// A random value used to combine multiple constraints into one.
    alpha: Target,

    /// A running sum of constraints that have been emitted so far, scaled by powers of alpha.
    constraint_acc: ExtensionTarget<D>,

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
    /// Add one constraint valid on all rows except the last.
    pub fn constraint(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.z_last);
        self.constraint(builder, filtered_constraint);
    }

    /// Add one constraint valid on all rows.
    pub fn constraint_wrapping(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        self.constraint_acc =
            builder.scalar_mul_add_extension(self.alpha, self.constraint_acc, constraint);
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
