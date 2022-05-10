use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::plonk_common::reduce_with_powers;
use plonky2::util::reducing::ReducingFactor;

use crate::all_stark::Table;
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::PermutationChallenge;
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub struct CrossTableLookup {
    pub looking_table: Table,
    pub looking_columns: Vec<usize>,
    pub looked_table: Table,
    pub looked_columns: Vec<usize>,
}

impl CrossTableLookup {
    pub fn new(
        looking_table: Table,
        looking_columns: Vec<usize>,
        looked_table: Table,
        looked_columns: Vec<usize>,
    ) -> Self {
        assert_eq!(looking_columns.len(), looked_columns.len());
        Self {
            looking_table,
            looking_columns,
            looked_table,
            looked_columns,
        }
    }
}

/// Lookup data for one table.
#[derive(Clone)]
pub struct LookupData<F: Field> {
    pub zs_beta_gammas: Vec<(PolynomialValues<F>, F, F, Vec<usize>)>,
}

impl<F: Field> Default for LookupData<F> {
    fn default() -> Self {
        Self {
            zs_beta_gammas: Vec::new(),
        }
    }
}

impl<F: Field> LookupData<F> {
    pub fn len(&self) -> usize {
        self.zs_beta_gammas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zs_beta_gammas.is_empty()
    }

    pub fn z_polys(&self) -> Vec<PolynomialValues<F>> {
        self.zs_beta_gammas
            .iter()
            .map(|(p, _, _, _)| p.clone())
            .collect()
    }
}

pub fn cross_table_lookup_zs<F: RichField, C: GenericConfig<D, F = F>, const D: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>],
    cross_table_lookups: &[CrossTableLookup],
    challenger: &mut Challenger<F, C::Hasher>,
) -> Vec<LookupData<F>> {
    cross_table_lookups.iter().fold(
        vec![LookupData::default(); trace_poly_values.len()],
        |mut acc, cross_table_lookup| {
            let CrossTableLookup {
                looking_table,
                looking_columns,
                looked_table,
                looked_columns,
                ..
            } = cross_table_lookup;

            for _ in 0..config.num_challenges {
                let beta = challenger.get_challenge();
                let gamma = challenger.get_challenge();
                let z_looking = partial_products(
                    &trace_poly_values[*looking_table as usize],
                    looking_columns,
                    beta,
                    gamma,
                );
                let z_looked = partial_products(
                    &trace_poly_values[*looked_table as usize],
                    looked_columns,
                    beta,
                    gamma,
                );

                acc[*looking_table as usize].zs_beta_gammas.push((
                    z_looking,
                    beta,
                    gamma,
                    looking_columns.clone(),
                ));
                acc[*looked_table as usize].zs_beta_gammas.push((
                    z_looked,
                    beta,
                    gamma,
                    looked_columns.clone(),
                ));
            }
            acc
        },
    )
}

fn partial_products<F: Field>(
    trace: &[PolynomialValues<F>],
    columns: &[usize],
    beta: F,
    gamma: F,
) -> PolynomialValues<F> {
    let mut partial_prod = F::ONE;
    let mut res = Vec::new();
    for i in 0..trace[0].len() {
        partial_prod *=
            gamma + reduce_with_powers(columns.iter().map(|&j| &trace[j].values[i]), beta);
        res.push(partial_prod);
    }
    res.into()
}

pub struct CTLCheckVars<F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_z: P,
    pub(crate) next_z: P,
    pub(crate) challenges: PermutationChallenge<F>,
    pub(crate) columns: Vec<usize>,
}

pub(crate) fn eval_cross_table_lookup_checks<F, FE, P, C, S, const D: usize, const D2: usize>(
    vars: StarkEvaluationVars<FE, P>,
    lookup_data: &[CTLCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    for lookup_datum in lookup_data {
        let CTLCheckVars {
            local_z,
            next_z,
            challenges,
            columns,
        } = lookup_datum;
        let mut factor = ReducingFactor::new(challenges.beta);
        let mut combine = |v: &[P]| -> P {
            factor.reduce_ext(columns.iter().map(|&i| v[i])) + FE::from_basefield(challenges.gamma)
        };

        // Check value of `Z(1)`
        consumer.constraint_first_row(*local_z - combine(vars.local_values));
        // Check `Z(gw) = combination * Z(w)`
        consumer.constraint_transition(*next_z - *local_z * combine(vars.next_values));
    }
}
