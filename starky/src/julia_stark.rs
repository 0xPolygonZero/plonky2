use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub struct JuliaStark<F: RichField + Extendable<D>, const D: usize> {
    c: F,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> JuliaStark<F, D> {
    const NUM_COLUMNS: usize = 1;
    const NUM_ROWS: usize = 1 << 10;

    fn new(c: F) -> Self {
        Self {
            c,
            _phantom: PhantomData,
        }
    }

    fn generate_trace(&self) -> Vec<[F; Self::NUM_COLUMNS]> {
        (0..Self::NUM_ROWS)
            .scan([F::ZERO; Self::NUM_COLUMNS], |acc, _| {
                let tmp = *acc;
                acc[0] = acc[0] * acc[0] + self.c;
                Some(tmp)
            })
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for JuliaStark<F, D> {
    const COLUMNS: usize = Self::NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        yield_constr.one(
            vars.next_values[0]
                - vars.local_values[0] * vars.local_values[0]
                - FE::from_basefield(self.c),
        );
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::field_types::Field;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::julia_stark::JuliaStark;
    use crate::prover::prove;

    #[test]
    fn test_julia_stark() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = JuliaStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let stark = S::new(F::NEG_ONE);
        let trace = stark.generate_trace();
        prove::<F, C, S, D>(stark, config, trace, &mut TimingTree::default());
    }
}
