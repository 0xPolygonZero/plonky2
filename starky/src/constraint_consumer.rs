use std::marker::PhantomData;

use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;

pub struct ConstraintConsumer<P: PackedField> {
    /// A random value used to combine multiple constraints into one.
    alpha: P::Scalar,

    /// A running sum of constraints that have been emitted so far, scaled by powers of alpha.
    constraint_acc: P,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the first trace row, and zero at other points in the subgroup.
    lagrange_basis_first: P::Scalar,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the last trace row, and zero at other points in the subgroup.
    lagrange_basis_last: P::Scalar,
}

impl<P: PackedField> ConstraintConsumer<P> {
    /// Add one constraint.
    pub fn one(&mut self, constraint: P) {
        self.constraint_acc *= self.alpha;
        self.constraint_acc += constraint;
    }

    /// Add a series of constraints.
    pub fn many(&mut self, constraints: impl IntoIterator<Item = P>) {
        constraints
            .into_iter()
            .for_each(|constraint| self.one(constraint));
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// first row of the trace.
    pub fn one_first_row(&mut self, constraint: P) {
        self.one(constraint * self.lagrange_basis_first);
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// last row of the trace.
    pub fn one_last_row(&mut self, constraint: P) {
        self.one(constraint * self.lagrange_basis_last);
    }
}

pub struct RecursiveConstraintConsumer<F: RichField + Extendable<D>, const D: usize> {
    /// A random value used to combine multiple constraints into one.
    alpha: Target,

    /// A running sum of constraints that have been emitted so far, scaled by powers of alpha.
    constraint_acc: ExtensionTarget<D>,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the first trace row, and zero at other points in the subgroup.
    lagrange_basis_first: ExtensionTarget<D>,

    /// The evaluation of the Lagrange basis polynomial which is nonzero at the point associated
    /// with the last trace row, and zero at other points in the subgroup.
    lagrange_basis_last: ExtensionTarget<D>,

    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> RecursiveConstraintConsumer<F, D> {
    /// Add one constraint.
    pub fn one(&mut self, builder: &mut CircuitBuilder<F, D>, constraint: ExtensionTarget<D>) {
        self.constraint_acc =
            builder.scalar_mul_add_extension(self.alpha, self.constraint_acc, constraint);
    }

    /// Add a series of constraints.
    pub fn many(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraints: impl IntoIterator<Item = ExtensionTarget<D>>,
    ) {
        constraints
            .into_iter()
            .for_each(|constraint| self.one(builder, constraint));
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// first row of the trace.
    pub fn one_first_row(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.lagrange_basis_first);
        self.one(builder, filtered_constraint);
    }

    /// Add one constraint, but first multiply it by a filter such that it will only apply to the
    /// last row of the trace.
    pub fn one_last_row(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        constraint: ExtensionTarget<D>,
    ) {
        let filtered_constraint = builder.mul_extension(constraint, self.lagrange_basis_last);
        self.one(builder, filtered_constraint);
    }
}
