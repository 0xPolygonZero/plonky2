use plonky2::field::extension_field::FieldExtension;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::plonk_common::reduce_with_powers;

use crate::config::StarkConfig;
use crate::prover::CrossTableLookup;

/// Lookup data for one table.
#[derive(Clone)]
pub struct LookupData<F: Field> {
    zs_beta_gammas: Vec<(PolynomialValues<F>, F, F)>,
}

impl<F: Field> Default for LookupData<F> {
    fn default() -> Self {
        Self {
            zs_beta_gammas: Vec::new(),
        }
    }
}

impl<F: Field> LookupData<F> {
    pub fn is_empty(&self) -> bool {
        self.zs_beta_gammas.is_empty()
    }

    pub fn z_polys(&self) -> Vec<PolynomialValues<F>> {
        self.zs_beta_gammas
            .iter()
            .map(|(p, _, _)| p.clone())
            .collect()
    }
}

pub fn cross_table_lookup_zs<F: RichField, C: GenericConfig<D, F = F>, const D: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>],
    cross_table_lookups: &[CrossTableLookup<F>],
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
                    &looking_columns,
                    beta,
                    gamma,
                );
                let z_looked = partial_products(
                    &trace_poly_values[*looked_table as usize],
                    &looked_columns,
                    beta,
                    gamma,
                );

                acc[*looking_table as usize]
                    .zs_beta_gammas
                    .push((z_looking, beta, gamma));
                acc[*looked_table as usize]
                    .zs_beta_gammas
                    .push((z_looked, beta, gamma));
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
            gamma + reduce_with_powers(columns.iter().map(|&j| &trace[i].values[j]), beta);
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
    pub(crate) local_zs: Vec<P>,
    pub(crate) next_zs: Vec<P>,
    pub(crate) permutation_challenge_sets: Vec<PermutationChallengeSet<F>>,
}

pub(crate) fn eval_permutation_checks<F, FE, P, C, S, const D: usize, const D2: usize>(
    stark: &S,
    config: &StarkConfig,
    vars: StarkEvaluationVars<FE, P>,
    permutation_data: PermutationCheckVars<F, FE, P, D2>,
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let PermutationCheckVars {
        local_zs,
        next_zs,
        permutation_challenge_sets,
    } = permutation_data;

    // Check that Z(1) = 1;
    for &z in &local_zs {
        consumer.constraint_first_row(z - FE::ONE);
    }

    let permutation_pairs = stark.permutation_pairs();

    let permutation_batches = get_permutation_batches(
        &permutation_pairs,
        &permutation_challenge_sets,
        config.num_challenges,
        stark.permutation_batch_size(),
    );

    // Each zs value corresponds to a permutation batch.
    for (i, instances) in permutation_batches.iter().enumerate() {
        // Z(gx) * down = Z x  * up
        let (reduced_lhs, reduced_rhs): (Vec<P>, Vec<P>) = instances
            .iter()
            .map(|instance| {
                let PermutationInstance {
                    pair: PermutationPair { column_pairs },
                    challenge: PermutationChallenge { beta, gamma },
                } = instance;
                let mut factor = ReducingFactor::new(*beta);
                let (lhs, rhs): (Vec<_>, Vec<_>) = column_pairs
                    .iter()
                    .map(|&(i, j)| (vars.local_values[i], vars.local_values[j]))
                    .unzip();
                (
                    factor.reduce_ext(lhs.into_iter()) + FE::from_basefield(*gamma),
                    factor.reduce_ext(rhs.into_iter()) + FE::from_basefield(*gamma),
                )
            })
            .unzip();
        let constraint = next_zs[i] * reduced_rhs.into_iter().product::<P>()
            - local_zs[i] * reduced_lhs.into_iter().product::<P>();
        consumer.constraint(constraint);
    }
}
