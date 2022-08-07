use std::marker::PhantomData;

use itertools::Itertools;
use log::info;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::plonk_common::reduce_with_powers_ext_circuit;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::keccak::columns::{
    reg_a, reg_a_prime, reg_a_prime_prime, reg_a_prime_prime_0_0_bit, reg_a_prime_prime_prime,
    reg_b, reg_c, reg_c_partial, reg_input_limb, reg_output_limb, reg_step, NUM_COLUMNS,
};
use crate::keccak::constants::{rc_value, rc_value_bit};
use crate::keccak::logic::{
    andn, andn_gen, andn_gen_circuit, xor, xor3_gen, xor3_gen_circuit, xor_gen, xor_gen_circuit,
};
use crate::keccak::round_flags::{eval_round_flags, eval_round_flags_recursively};
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Number of rounds in a Keccak permutation.
pub(crate) const NUM_ROUNDS: usize = 24;

/// Number of 64-bit elements in the Keccak permutation input.
pub(crate) const NUM_INPUTS: usize = 25;

pub(crate) const NUM_PUBLIC_INPUTS: usize = 0;

pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res: Vec<_> = (0..2 * NUM_INPUTS).map(reg_input_limb).collect();
    res.extend(Column::singles((0..2 * NUM_INPUTS).map(reg_output_limb)));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(reg_step(NUM_ROUNDS - 1))
}

#[derive(Copy, Clone, Default)]
pub struct KeccakStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakStark<F, D> {
    /// Generate the rows of the trace. Note that this does not generate the permuted columns used
    /// in our lookup arguments, as those are computed after transposing to column-wise form.
    pub(crate) fn generate_trace_rows(
        &self,
        inputs: Vec<[u64; NUM_INPUTS]>,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let num_rows = (inputs.len() * NUM_ROUNDS).next_power_of_two();
        info!("{} rows", num_rows);
        let mut rows = Vec::with_capacity(num_rows);
        for input in inputs.iter() {
            rows.extend(self.generate_trace_rows_for_perm(*input));
        }

        let pad_rows = self.generate_trace_rows_for_perm([0; NUM_INPUTS]);
        while rows.len() < num_rows {
            rows.extend(&pad_rows);
        }
        rows.drain(num_rows..);
        rows
    }

    fn generate_trace_rows_for_perm(&self, input: [u64; NUM_INPUTS]) -> Vec<[F; NUM_COLUMNS]> {
        let mut rows = vec![[F::ZERO; NUM_COLUMNS]; NUM_ROUNDS];

        for x in 0..5 {
            for y in 0..5 {
                let input_xy = input[x * 5 + y];
                for z in 0..64 {
                    rows[0][reg_a(x, y, z)] = F::from_canonical_u64((input_xy >> z) & 1);
                }
            }
        }

        self.generate_trace_row_for_round(&mut rows[0], 0);
        for round in 1..24 {
            self.copy_output_to_input(rows[round - 1], &mut rows[round]);
            self.generate_trace_row_for_round(&mut rows[round], round);
        }

        rows
    }

    fn copy_output_to_input(&self, prev_row: [F; NUM_COLUMNS], next_row: &mut [F; NUM_COLUMNS]) {
        for x in 0..5 {
            for y in 0..5 {
                let cur_lo = prev_row[reg_a_prime_prime_prime(x, y)];
                let cur_hi = prev_row[reg_a_prime_prime_prime(x, y) + 1];
                let cur_u64 = cur_lo.to_canonical_u64() | (cur_hi.to_canonical_u64() << 32);
                let bit_values: Vec<u64> = (0..64)
                    .scan(cur_u64, |acc, _| {
                        let tmp = *acc & 1;
                        *acc >>= 1;
                        Some(tmp)
                    })
                    .collect();

                for z in 0..64 {
                    next_row[reg_a(x, y, z)] = F::from_canonical_u64(bit_values[z]);
                }
            }
        }
    }

    fn generate_trace_row_for_round(&self, row: &mut [F; NUM_COLUMNS], round: usize) {
        row[reg_step(round)] = F::ONE;

        // Populate C partial and C.
        for x in 0..5 {
            for z in 0..64 {
                let a = [0, 1, 2, 3, 4].map(|i| row[reg_a(x, i, z)]);
                let c_partial = xor([a[0], a[1], a[2]]);
                let c = xor([c_partial, a[3], a[4]]);
                row[reg_c_partial(x, z)] = c_partial;
                row[reg_c(x, z)] = c;
            }
        }

        // Populate A'.
        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    row[reg_a_prime(x, y, z)] = xor([
                        row[reg_a(x, y, z)],
                        row[reg_c((x + 4) % 5, z)],
                        row[reg_c((x + 1) % 5, (z + 64 - 1) % 64)],
                    ]);
                }
            }
        }

        // Populate A''.
        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        for x in 0..5 {
            for y in 0..5 {
                let get_bit = |z| {
                    xor([
                        row[reg_b(x, y, z)],
                        andn(row[reg_b((x + 1) % 5, y, z)], row[reg_b((x + 2) % 5, y, z)]),
                    ])
                };

                let lo = (0..32)
                    .rev()
                    .fold(F::ZERO, |acc, z| acc.double() + get_bit(z));
                let hi = (32..64)
                    .rev()
                    .fold(F::ZERO, |acc, z| acc.double() + get_bit(z));

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                row[reg_lo] = lo;
                row[reg_hi] = hi;
            }
        }

        // For the XOR, we split A''[0, 0] to bits.
        let val_lo = row[reg_a_prime_prime(0, 0)].to_canonical_u64();
        let val_hi = row[reg_a_prime_prime(0, 0) + 1].to_canonical_u64();
        let val = val_lo | (val_hi << 32);
        let bit_values: Vec<u64> = (0..64)
            .scan(val, |acc, _| {
                let tmp = *acc & 1;
                *acc >>= 1;
                Some(tmp)
            })
            .collect();
        for i in 0..64 {
            row[reg_a_prime_prime_0_0_bit(i)] = F::from_canonical_u64(bit_values[i]);
        }

        // A''[0, 0] is additionally xor'd with RC.
        let in_reg_lo = reg_a_prime_prime(0, 0);
        let in_reg_hi = in_reg_lo + 1;
        let out_reg_lo = reg_a_prime_prime_prime(0, 0);
        let out_reg_hi = out_reg_lo + 1;
        let rc_lo = rc_value(round) & ((1 << 32) - 1);
        let rc_hi = rc_value(round) >> 32;
        row[out_reg_lo] = F::from_canonical_u64(row[in_reg_lo].to_canonical_u64() ^ rc_lo);
        row[out_reg_hi] = F::from_canonical_u64(row[in_reg_hi].to_canonical_u64() ^ rc_hi);
    }

    pub fn generate_trace(&self, inputs: Vec<[u64; NUM_INPUTS]>) -> Vec<PolynomialValues<F>> {
        let mut timing = TimingTree::new("generate trace", log::Level::Debug);

        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            &mut timing,
            "generate trace rows",
            self.generate_trace_rows(inputs)
        );

        let trace_polys = timed!(
            &mut timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        timing.print();
        trace_polys
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;
    const PUBLIC_INPUTS: usize = NUM_PUBLIC_INPUTS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        eval_round_flags(vars, yield_constr);

        // C_partial[x] = xor(A[x, 0], A[x, 1], A[x, 2])
        for x in 0..5 {
            for z in 0..64 {
                let c_partial = vars.local_values[reg_c_partial(x, z)];
                let a_0 = vars.local_values[reg_a(x, 0, z)];
                let a_1 = vars.local_values[reg_a(x, 1, z)];
                let a_2 = vars.local_values[reg_a(x, 2, z)];
                let xor_012 = xor3_gen(a_0, a_1, a_2);
                yield_constr.constraint(c_partial - xor_012);
            }
        }

        // C[x] = xor(C_partial[x], A[x, 3], A[x, 4])
        for x in 0..5 {
            for z in 0..64 {
                let c = vars.local_values[reg_c(x, z)];
                let xor_012 = vars.local_values[reg_c_partial(x, z)];
                let a_3 = vars.local_values[reg_a(x, 3, z)];
                let a_4 = vars.local_values[reg_a(x, 4, z)];
                let xor_01234 = xor3_gen(xor_012, a_3, a_4);
                yield_constr.constraint(c - xor_01234);
            }
        }

        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for z in 0..64 {
                let c_left = vars.local_values[reg_c((x + 4) % 5, z)];
                let c_right = vars.local_values[reg_c((x + 1) % 5, (z + 64 - 1) % 64)];
                let d = xor_gen(c_left, c_right);

                for y in 0..5 {
                    let a = vars.local_values[reg_a(x, y, z)];
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let xor = xor_gen(d, a);
                    yield_constr.constraint(a_prime - xor);
                }
            }
        }

        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        for x in 0..5 {
            for y in 0..5 {
                let get_bit = |z| {
                    xor_gen(
                        vars.local_values[reg_b(x, y, z)],
                        andn_gen(
                            vars.local_values[reg_b((x + 1) % 5, y, z)],
                            vars.local_values[reg_b((x + 2) % 5, y, z)],
                        ),
                    )
                };

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                let lo = vars.local_values[reg_lo];
                let hi = vars.local_values[reg_hi];
                let computed_lo = (0..32)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));
                let computed_hi = (32..64)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));

                yield_constr.constraint(computed_lo - lo);
                yield_constr.constraint(computed_hi - hi);
            }
        }

        // A'''[0, 0] = A''[0, 0] XOR RC
        let a_prime_prime_0_0_bits = (0..64)
            .map(|i| vars.local_values[reg_a_prime_prime_0_0_bit(i)])
            .collect_vec();
        let computed_a_prime_prime_0_0_lo = (0..32)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + a_prime_prime_0_0_bits[z]);
        let computed_a_prime_prime_0_0_hi = (32..64)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + a_prime_prime_0_0_bits[z]);
        let a_prime_prime_0_0_lo = vars.local_values[reg_a_prime_prime(0, 0)];
        let a_prime_prime_0_0_hi = vars.local_values[reg_a_prime_prime(0, 0) + 1];
        yield_constr.constraint(computed_a_prime_prime_0_0_lo - a_prime_prime_0_0_lo);
        yield_constr.constraint(computed_a_prime_prime_0_0_hi - a_prime_prime_0_0_hi);

        let get_xored_bit = |i| {
            let mut rc_bit_i = P::ZEROS;
            for r in 0..NUM_ROUNDS {
                let this_round = vars.local_values[reg_step(r)];
                let this_round_constant =
                    P::from(FE::from_canonical_u32(rc_value_bit(r, i) as u32));
                rc_bit_i += this_round * this_round_constant;
            }

            xor_gen(a_prime_prime_0_0_bits[i], rc_bit_i)
        };

        let a_prime_prime_prime_0_0_lo = vars.local_values[reg_a_prime_prime_prime(0, 0)];
        let a_prime_prime_prime_0_0_hi = vars.local_values[reg_a_prime_prime_prime(0, 0) + 1];
        let computed_a_prime_prime_prime_0_0_lo = (0..32)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + get_xored_bit(z));
        let computed_a_prime_prime_prime_0_0_hi = (32..64)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + get_xored_bit(z));
        yield_constr.constraint(computed_a_prime_prime_prime_0_0_lo - a_prime_prime_prime_0_0_lo);
        yield_constr.constraint(computed_a_prime_prime_prime_0_0_hi - a_prime_prime_prime_0_0_hi);

        // Enforce that this round's output equals the next round's input.
        for x in 0..5 {
            for y in 0..5 {
                let output_lo = vars.local_values[reg_a_prime_prime_prime(x, y)];
                let output_hi = vars.local_values[reg_a_prime_prime_prime(x, y) + 1];
                let input_bits = (0..64)
                    .map(|z| vars.next_values[reg_a(x, y, z)])
                    .collect_vec();
                let input_bits_combined_lo = (0..32)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + input_bits[z]);
                let input_bits_combined_hi = (32..64)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + input_bits[z]);
                let is_last_round = vars.local_values[reg_step(NUM_ROUNDS - 1)];
                yield_constr.constraint_transition(
                    (P::ONES - is_last_round) * (output_lo - input_bits_combined_lo),
                );
                yield_constr.constraint_transition(
                    (P::ONES - is_last_round) * (output_hi - input_bits_combined_hi),
                );
            }
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let two = builder.two();

        eval_round_flags_recursively(builder, vars, yield_constr);

        // C_partial[x] = xor(A[x, 0], A[x, 1], A[x, 2])
        for x in 0..5 {
            for z in 0..64 {
                let c_partial = vars.local_values[reg_c_partial(x, z)];
                let a_0 = vars.local_values[reg_a(x, 0, z)];
                let a_1 = vars.local_values[reg_a(x, 1, z)];
                let a_2 = vars.local_values[reg_a(x, 2, z)];

                let xor_012 = xor3_gen_circuit(builder, a_0, a_1, a_2);
                let diff = builder.sub_extension(c_partial, xor_012);
                yield_constr.constraint(builder, diff);
            }
        }

        // C[x] = xor(C_partial[x], A[x, 3], A[x, 4])
        for x in 0..5 {
            for z in 0..64 {
                let c = vars.local_values[reg_c(x, z)];
                let xor_012 = vars.local_values[reg_c_partial(x, z)];
                let a_3 = vars.local_values[reg_a(x, 3, z)];
                let a_4 = vars.local_values[reg_a(x, 4, z)];

                let xor_01234 = xor3_gen_circuit(builder, xor_012, a_3, a_4);
                let diff = builder.sub_extension(c, xor_01234);
                yield_constr.constraint(builder, diff);
            }
        }

        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for z in 0..64 {
                let c_left = vars.local_values[reg_c((x + 4) % 5, z)];
                let c_right = vars.local_values[reg_c((x + 1) % 5, (z + 64 - 1) % 64)];
                let d = xor_gen_circuit(builder, c_left, c_right);

                for y in 0..5 {
                    let a = vars.local_values[reg_a(x, y, z)];
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let xor = xor_gen_circuit(builder, d, a);
                    let diff = builder.sub_extension(a_prime, xor);
                    yield_constr.constraint(builder, diff);
                }
            }
        }

        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        for x in 0..5 {
            for y in 0..5 {
                let mut get_bit = |z| {
                    let andn = andn_gen_circuit(
                        builder,
                        vars.local_values[reg_b((x + 1) % 5, y, z)],
                        vars.local_values[reg_b((x + 2) % 5, y, z)],
                    );
                    xor_gen_circuit(builder, vars.local_values[reg_b(x, y, z)], andn)
                };

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                let lo = vars.local_values[reg_lo];
                let hi = vars.local_values[reg_hi];
                let bits_lo = (0..32).map(&mut get_bit).collect_vec();
                let bits_hi = (32..64).map(get_bit).collect_vec();
                let computed_lo = reduce_with_powers_ext_circuit(builder, &bits_lo, two);
                let computed_hi = reduce_with_powers_ext_circuit(builder, &bits_hi, two);
                let diff = builder.sub_extension(computed_lo, lo);
                yield_constr.constraint(builder, diff);
                let diff = builder.sub_extension(computed_hi, hi);
                yield_constr.constraint(builder, diff);
            }
        }

        // A'''[0, 0] = A''[0, 0] XOR RC
        let a_prime_prime_0_0_bits = (0..64)
            .map(|i| vars.local_values[reg_a_prime_prime_0_0_bit(i)])
            .collect_vec();
        let computed_a_prime_prime_0_0_lo =
            reduce_with_powers_ext_circuit(builder, &a_prime_prime_0_0_bits[0..32], two);
        let computed_a_prime_prime_0_0_hi =
            reduce_with_powers_ext_circuit(builder, &a_prime_prime_0_0_bits[32..64], two);
        let a_prime_prime_0_0_lo = vars.local_values[reg_a_prime_prime(0, 0)];
        let a_prime_prime_0_0_hi = vars.local_values[reg_a_prime_prime(0, 0) + 1];
        let diff = builder.sub_extension(computed_a_prime_prime_0_0_lo, a_prime_prime_0_0_lo);
        yield_constr.constraint(builder, diff);
        let diff = builder.sub_extension(computed_a_prime_prime_0_0_hi, a_prime_prime_0_0_hi);
        yield_constr.constraint(builder, diff);

        let mut get_xored_bit = |i| {
            let mut rc_bit_i = builder.zero_extension();
            for r in 0..NUM_ROUNDS {
                let this_round = vars.local_values[reg_step(r)];
                let this_round_constant = builder
                    .constant_extension(F::from_canonical_u32(rc_value_bit(r, i) as u32).into());
                rc_bit_i = builder.mul_add_extension(this_round, this_round_constant, rc_bit_i);
            }

            xor_gen_circuit(builder, a_prime_prime_0_0_bits[i], rc_bit_i)
        };

        let a_prime_prime_prime_0_0_lo = vars.local_values[reg_a_prime_prime_prime(0, 0)];
        let a_prime_prime_prime_0_0_hi = vars.local_values[reg_a_prime_prime_prime(0, 0) + 1];
        let bits_lo = (0..32).map(&mut get_xored_bit).collect_vec();
        let bits_hi = (32..64).map(get_xored_bit).collect_vec();
        let computed_a_prime_prime_prime_0_0_lo =
            reduce_with_powers_ext_circuit(builder, &bits_lo, two);
        let computed_a_prime_prime_prime_0_0_hi =
            reduce_with_powers_ext_circuit(builder, &bits_hi, two);
        let diff = builder.sub_extension(
            computed_a_prime_prime_prime_0_0_lo,
            a_prime_prime_prime_0_0_lo,
        );
        yield_constr.constraint(builder, diff);
        let diff = builder.sub_extension(
            computed_a_prime_prime_prime_0_0_hi,
            a_prime_prime_prime_0_0_hi,
        );
        yield_constr.constraint(builder, diff);

        // Enforce that this round's output equals the next round's input.
        for x in 0..5 {
            for y in 0..5 {
                let output_lo = vars.local_values[reg_a_prime_prime_prime(x, y)];
                let output_hi = vars.local_values[reg_a_prime_prime_prime(x, y) + 1];
                let input_bits = (0..64)
                    .map(|z| vars.next_values[reg_a(x, y, z)])
                    .collect_vec();
                let input_bits_combined_lo =
                    reduce_with_powers_ext_circuit(builder, &input_bits[0..32], two);
                let input_bits_combined_hi =
                    reduce_with_powers_ext_circuit(builder, &input_bits[32..64], two);
                let is_last_round = vars.local_values[reg_step(NUM_ROUNDS - 1)];
                let diff = builder.sub_extension(input_bits_combined_lo, output_lo);
                let filtered_diff = builder.mul_sub_extension(is_last_round, diff, diff);
                yield_constr.constraint_transition(builder, filtered_diff);
                let diff = builder.sub_extension(input_bits_combined_hi, output_hi);
                let filtered_diff = builder.mul_sub_extension(is_last_round, diff, diff);
                yield_constr.constraint_transition(builder, filtered_diff);
            }
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use keccak_rust::{KeccakF, StateBitsWidth};
    use plonky2::field::types::Field;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::keccak::columns::reg_output_limb;
    use crate::keccak::keccak_stark::{KeccakStark, NUM_INPUTS, NUM_ROUNDS};
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn keccak_correctness_test() -> Result<()> {
        let input: [u64; NUM_INPUTS] = rand::random();

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        let rows = stark.generate_trace_rows(vec![input.try_into().unwrap()]);
        let last_row = rows[NUM_ROUNDS - 1];
        let base = F::from_canonical_u64(1 << 32);
        let output = (0..NUM_INPUTS)
            .map(|i| last_row[reg_output_limb(2 * i)] + base * last_row[reg_output_limb(2 * i + 1)])
            .collect::<Vec<_>>();

        let mut keccak_input: [[u64; 5]; 5] = [
            input[0..5].try_into().unwrap(),
            input[5..10].try_into().unwrap(),
            input[10..15].try_into().unwrap(),
            input[15..20].try_into().unwrap(),
            input[20..25].try_into().unwrap(),
        ];

        let keccak = KeccakF::new(StateBitsWidth::F1600);
        keccak.permutations(&mut keccak_input);
        let expected: Vec<_> = keccak_input
            .iter()
            .flatten()
            .map(|&x| F::from_canonical_u64(x))
            .collect();

        assert_eq!(output, expected);

        Ok(())
    }
}
