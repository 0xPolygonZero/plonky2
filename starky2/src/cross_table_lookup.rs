use anyhow::{ensure, Result};
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
use crate::permutation::{
    get_permutation_challenge_set, PermutationChallenge, PermutationChallengeSet,
};
use crate::proof::StarkProofWithPublicInputs;
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

#[derive(Clone)]
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
    pub(crate) challenges: PermutationChallengeSet<F>,
    pub zs_columns: Vec<(PolynomialValues<F>, Vec<usize>)>,
}

impl<F: Field> LookupData<F> {
    pub(crate) fn new(challenges: PermutationChallengeSet<F>) -> Self {
        Self {
            challenges,
            zs_columns: vec![],
        }
    }

    pub fn len(&self) -> usize {
        self.zs_columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zs_columns.is_empty()
    }

    pub fn z_polys(&self) -> Vec<PolynomialValues<F>> {
        self.zs_columns.iter().map(|(p, _)| p.clone()).collect()
    }
}

pub fn cross_table_lookup_zs<F: RichField, C: GenericConfig<D, F = F>, const D: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>],
    cross_table_lookups: &[CrossTableLookup],
    challenger: &mut Challenger<F, C::Hasher>,
) -> Vec<LookupData<F>> {
    let challenges = get_permutation_challenge_set(challenger, config.num_challenges);
    cross_table_lookups.iter().fold(
        vec![LookupData::new(challenges.clone()); trace_poly_values.len()],
        |mut acc, cross_table_lookup| {
            let CrossTableLookup {
                looking_table,
                looking_columns,
                looked_table,
                looked_columns,
            } = cross_table_lookup;

            for &PermutationChallenge { beta, gamma } in &challenges.challenges {
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

                acc[*looking_table as usize]
                    .zs_columns
                    .push((z_looking, looking_columns.clone()));
                acc[*looked_table as usize]
                    .zs_columns
                    .push((z_looked, looked_columns.clone()));
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

#[derive(Clone)]
pub struct CTLCheckVars<'a, F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_z: P,
    pub(crate) next_z: P,
    pub(crate) challenges: PermutationChallenge<F>,
    pub(crate) columns: &'a [usize],
}

impl<'a, F: RichField + Extendable<D>, const D: usize>
    CTLCheckVars<'a, F, F::Extension, F::Extension, D>
{
    pub(crate) fn from_proofs<C: GenericConfig<D, F = F>>(
        proofs: &[&StarkProofWithPublicInputs<F, C, D>],
        cross_table_lookups: &'a [CrossTableLookup],
        ctl_challenges: &'a PermutationChallengeSet<F>,
    ) -> Vec<Vec<Self>> {
        let mut ctl_zs = proofs
            .iter()
            .map(|p| {
                p.proof
                    .openings
                    .permutation_lookup_zs
                    .as_ref()
                    .unwrap() // TODO: fix unwrap
                    .iter()
                    .zip(
                        p.proof
                            .openings
                            .permutation_lookup_zs_right
                            .as_ref()
                            .unwrap()
                            .iter(),
                    )
            })
            .collect::<Vec<_>>();

        cross_table_lookups
            .iter()
            .fold(vec![vec![]; proofs.len()], |mut acc, ctl| {
                let CrossTableLookup {
                    looking_table,
                    looking_columns,
                    looked_table,
                    looked_columns,
                } = ctl;

                for &challenges in &ctl_challenges.challenges {
                    let (looking_z, looking_z_next) =
                        ctl_zs[*looking_table as usize].next().unwrap();
                    acc[*looking_table as usize].push(Self {
                        local_z: *looking_z,
                        next_z: *looking_z_next,
                        challenges,
                        columns: &looking_columns,
                    });

                    let (looked_z, looked_z_next) = ctl_zs[*looked_table as usize].next().unwrap();
                    acc[*looked_table as usize].push(Self {
                        local_z: *looked_z,
                        next_z: *looked_z_next,
                        challenges,
                        columns: &looked_columns,
                    });
                }
                acc
            })
    }
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

pub(crate) fn verify_cross_table_lookups<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    cross_table_lookups: Vec<CrossTableLookup>,
    proofs: &[&StarkProofWithPublicInputs<F, C, D>],
    challenges: PermutationChallengeSet<F>,
    config: &StarkConfig,
) -> Result<()> {
    let degrees_bits = proofs
        .iter()
        .map(|p| p.proof.recover_degree_bits(config))
        .collect::<Vec<_>>();
    let mut lookup_zs_openings = proofs
        .iter()
        .map(|p| p.proof.openings.lookup_zs_last.iter())
        .collect::<Vec<_>>();
    for (
        i,
        CrossTableLookup {
            looking_table,
            looked_table,
            ..
        },
    ) in cross_table_lookups.into_iter().enumerate()
    {
        let looking_degree = 1 << degrees_bits[looking_table as usize];
        let looked_degree = 1 << degrees_bits[looked_table as usize];
        let looking_z = *lookup_zs_openings[looking_table as usize].next().unwrap();
        let looked_z = *lookup_zs_openings[looked_table as usize].next().unwrap();
        ensure!(
            looking_z
                == looked_z
                    * challenges.challenges[i % config.num_challenges]
                        .gamma
                        .exp_u64(looking_degree - looked_degree),
            "Cross-table lookup verification failed."
        );
    }

    Ok(())
}
