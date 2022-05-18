use anyhow::{ensure, Result};
use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;

use crate::all_stark::Table;
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::permutation::{
    get_grand_product_challenge_set, GrandProductChallenge, GrandProductChallengeSet,
};
use crate::proof::StarkProofWithPublicInputs;
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

#[derive(Clone)]
pub struct CrossTableLookup<F: Field> {
    pub looking_table: Table,
    pub looking_columns: Vec<usize>,
    pub looked_table: Table,
    pub looked_columns: Vec<usize>,
    pub default: Vec<F>,
}

impl<F: Field> CrossTableLookup<F> {
    pub fn new(
        looking_table: Table,
        looking_columns: Vec<usize>,
        looked_table: Table,
        looked_columns: Vec<usize>,
        default: Vec<F>,
    ) -> Self {
        assert_eq!(looking_columns.len(), looked_columns.len());
        Self {
            looking_table,
            looking_columns,
            looked_table,
            looked_columns,
            default,
        }
    }
}

/// Cross-table lookup data for one table.
#[derive(Clone)]
pub struct CtlData<F: Field> {
    /// Challenges used in the argument.
    pub(crate) challenges: GrandProductChallengeSet<F>,
    /// Vector of `(Z, columns)` where `Z` is a Z-polynomial for a lookup on columns `columns`.
    pub zs_columns: Vec<(PolynomialValues<F>, Vec<usize>)>,
}

impl<F: Field> CtlData<F> {
    pub(crate) fn new(challenges: GrandProductChallengeSet<F>) -> Self {
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

pub fn cross_table_lookup_data<F: RichField, C: GenericConfig<D, F = F>, const D: usize>(
    config: &StarkConfig,
    trace_poly_values: &[Vec<PolynomialValues<F>>],
    cross_table_lookups: &[CrossTableLookup<F>],
    challenger: &mut Challenger<F, C::Hasher>,
) -> Vec<CtlData<F>> {
    let challenges = get_grand_product_challenge_set(challenger, config.num_challenges);
    let mut ctl_data_per_table = vec![CtlData::new(challenges.clone()); trace_poly_values.len()];
    for CrossTableLookup {
        looking_table,
        looking_columns,
        looked_table,
        looked_columns,
        default,
    } in cross_table_lookups
    {
        for &challenge in &challenges.challenges {
            let z_looking = partial_products(
                &trace_poly_values[*looking_table as usize],
                looking_columns,
                challenge,
            );
            let z_looked = partial_products(
                &trace_poly_values[*looked_table as usize],
                looked_columns,
                challenge,
            );

            debug_assert_eq!(
                *z_looking.values.last().unwrap(),
                *z_looked.values.last().unwrap()
                    * challenge.combine(default).exp_u64(
                        trace_poly_values[*looking_table as usize][0].len() as u64
                            - trace_poly_values[*looked_table as usize][0].len() as u64
                    )
            );

            ctl_data_per_table[*looking_table as usize]
                .zs_columns
                .push((z_looking, looking_columns.clone()));
            ctl_data_per_table[*looked_table as usize]
                .zs_columns
                .push((z_looked, looked_columns.clone()));
        }
    }
    ctl_data_per_table
}

fn partial_products<F: Field>(
    trace: &[PolynomialValues<F>],
    columns: &[usize],
    challenge: GrandProductChallenge<F>,
) -> PolynomialValues<F> {
    let mut partial_prod = F::ONE;
    let degree = trace[0].len();
    let mut res = Vec::with_capacity(degree);
    for i in 0..degree {
        partial_prod *= challenge.combine(columns.iter().map(|&j| &trace[j].values[i]));
        res.push(partial_prod);
    }
    res.into()
}

#[derive(Clone)]
pub struct CtlCheckVars<'a, F, FE, P, const D2: usize>
where
    F: Field,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    pub(crate) local_z: P,
    pub(crate) next_z: P,
    pub(crate) challenges: GrandProductChallenge<F>,
    pub(crate) columns: &'a [usize],
}

impl<'a, F: RichField + Extendable<D>, const D: usize>
    CtlCheckVars<'a, F, F::Extension, F::Extension, D>
{
    pub(crate) fn from_proofs<C: GenericConfig<D, F = F>>(
        proofs: &[&StarkProofWithPublicInputs<F, C, D>],
        cross_table_lookups: &'a [CrossTableLookup<F>],
        ctl_challenges: &'a GrandProductChallengeSet<F>,
        num_permutation_zs: &[usize],
    ) -> Vec<Vec<Self>> {
        debug_assert_eq!(proofs.len(), num_permutation_zs.len());
        let mut ctl_zs = proofs
            .iter()
            .zip(num_permutation_zs)
            .map(|(p, &num_perms)| {
                let openings = &p.proof.openings;
                let ctl_zs = openings.permutation_ctl_zs.iter().skip(num_perms);
                let ctl_zs_right = openings.permutation_ctl_zs_right.iter().skip(num_perms);
                ctl_zs.zip(ctl_zs_right)
            })
            .collect::<Vec<_>>();

        let mut ctl_vars_per_table = vec![vec![]; proofs.len()];
        for CrossTableLookup {
            looking_table,
            looking_columns,
            looked_table,
            looked_columns,
            ..
        } in cross_table_lookups
        {
            for &challenges in &ctl_challenges.challenges {
                let (looking_z, looking_z_next) = ctl_zs[*looking_table as usize].next().unwrap();
                ctl_vars_per_table[*looking_table as usize].push(Self {
                    local_z: *looking_z,
                    next_z: *looking_z_next,
                    challenges,
                    columns: looking_columns,
                });

                let (looked_z, looked_z_next) = ctl_zs[*looked_table as usize].next().unwrap();
                ctl_vars_per_table[*looked_table as usize].push(Self {
                    local_z: *looked_z,
                    next_z: *looked_z_next,
                    challenges,
                    columns: looked_columns,
                });
            }
        }
        ctl_vars_per_table
    }
}

pub(crate) fn eval_cross_table_lookup_checks<F, FE, P, C, S, const D: usize, const D2: usize>(
    vars: StarkEvaluationVars<FE, P, { S::COLUMNS }, { S::PUBLIC_INPUTS }>,
    ctl_vars: &[CtlCheckVars<F, FE, P, D2>],
    consumer: &mut ConstraintConsumer<P>,
) where
    F: RichField + Extendable<D>,
    FE: FieldExtension<D2, BaseField = F>,
    P: PackedField<Scalar = FE>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    for lookup_vars in ctl_vars {
        let CtlCheckVars {
            local_z,
            next_z,
            challenges,
            columns,
        } = lookup_vars;
        let combine = |v: &[P]| -> P { challenges.combine(columns.iter().map(|&i| &v[i])) };

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
    cross_table_lookups: Vec<CrossTableLookup<F>>,
    proofs: &[&StarkProofWithPublicInputs<F, C, D>],
    challenges: GrandProductChallengeSet<F>,
    config: &StarkConfig,
) -> Result<()> {
    let degrees_bits = proofs
        .iter()
        .map(|p| p.proof.recover_degree_bits(config))
        .collect::<Vec<_>>();
    let mut ctl_zs_openings = proofs
        .iter()
        .map(|p| p.proof.openings.ctl_zs_last.iter())
        .collect::<Vec<_>>();
    for (
        i,
        CrossTableLookup {
            looking_table,
            looked_table,
            default,
            ..
        },
    ) in cross_table_lookups.into_iter().enumerate()
    {
        let looking_degree = 1 << degrees_bits[looking_table as usize];
        let looked_degree = 1 << degrees_bits[looked_table as usize];
        let looking_z = *ctl_zs_openings[looking_table as usize].next().unwrap();
        let looked_z = *ctl_zs_openings[looked_table as usize].next().unwrap();
        let challenge = challenges.challenges[i % config.num_challenges];
        let combined_default = challenge.combine(default.iter());

        ensure!(
            looking_z == looked_z * combined_default.exp_u64(looking_degree - looked_degree),
            "Cross-table lookup verification failed."
        );
    }

    Ok(())
}
