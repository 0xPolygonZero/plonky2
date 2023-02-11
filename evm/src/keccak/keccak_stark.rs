use std::marker::PhantomData;

use itertools::Itertools;
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
    reg_b, reg_c, reg_c_prime, reg_input_limb, reg_output_limb, reg_preimage, reg_step,
    NUM_COLUMNS, REG_FILTER,
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

pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res: Vec<_> = (0..2 * NUM_INPUTS).map(reg_input_limb).collect();
    res.extend(Column::singles((0..2 * NUM_INPUTS).map(reg_output_limb)));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(REG_FILTER)
}

#[derive(Copy, Clone, Default)]
pub struct KeccakStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakStark<F, D> {
    /// Generate the rows of the trace. Note that this does not generate the permuted columns used
    /// in our lookup arguments, as those are computed after transposing to column-wise form.
    fn generate_trace_rows(
        &self,
        inputs: Vec<[u64; NUM_INPUTS]>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let num_rows = (inputs.len() * NUM_ROUNDS)
            .max(min_rows)
            .next_power_of_two();
        let mut rows = Vec::with_capacity(num_rows);
        for input in inputs.iter() {
            let mut rows_for_perm = self.generate_trace_rows_for_perm(*input);
            // Since this is a real operation, not padding, we set the filter to 1 on the last row.
            rows_for_perm[NUM_ROUNDS - 1][REG_FILTER] = F::ONE;
            rows.extend(rows_for_perm);
        }

        let pad_rows = self.generate_trace_rows_for_perm([0; NUM_INPUTS]);
        while rows.len() < num_rows {
            rows.extend(pad_rows.clone());
        }
        rows.drain(num_rows..);
        rows
    }

    fn generate_trace_rows_for_perm(&self, input: [u64; NUM_INPUTS]) -> Vec<[F; NUM_COLUMNS]> {
        let mut rows = vec![[F::ZERO; NUM_COLUMNS]; NUM_ROUNDS];

        // Populate the preimage for each row.
        for round in 0..24 {
            for x in 0..5 {
                for y in 0..5 {
                    let input_xy = input[y * 5 + x];
                    let reg_preimage_lo = reg_preimage(x, y);
                    let reg_preimage_hi = reg_preimage_lo + 1;
                    rows[round][reg_preimage_lo] = F::from_canonical_u64(input_xy & 0xFFFFFFFF);
                    rows[round][reg_preimage_hi] = F::from_canonical_u64(input_xy >> 32);
                }
            }
        }

        // Populate the round input for the first round.
        for x in 0..5 {
            for y in 0..5 {
                let input_xy = input[y * 5 + x];
                let reg_lo = reg_a(x, y);
                let reg_hi = reg_lo + 1;
                rows[0][reg_lo] = F::from_canonical_u64(input_xy & 0xFFFFFFFF);
                rows[0][reg_hi] = F::from_canonical_u64(input_xy >> 32);
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
                let in_lo = reg_a(x, y);
                let in_hi = in_lo + 1;
                let out_lo = reg_a_prime_prime_prime(x, y);
                let out_hi = out_lo + 1;
                next_row[in_lo] = prev_row[out_lo];
                next_row[in_hi] = prev_row[out_hi];
            }
        }
    }

    fn generate_trace_row_for_round(&self, row: &mut [F; NUM_COLUMNS], round: usize) {
        row[reg_step(round)] = F::ONE;

        // Populate C[x] = xor(A[x, 0], A[x, 1], A[x, 2], A[x, 3], A[x, 4]).
        for x in 0..5 {
            for z in 0..64 {
                let is_high_limb = z / 32;
                let bit_in_limb = z % 32;
                let a = [0, 1, 2, 3, 4].map(|i| {
                    let reg_a_limb = reg_a(x, i) + is_high_limb;
                    let a_limb = row[reg_a_limb].to_canonical_u64() as u32;
                    F::from_bool(((a_limb >> bit_in_limb) & 1) != 0)
                });
                row[reg_c(x, z)] = xor(a);
            }
        }

        // Populate C'[x, z] = xor(C[x, z], C[x - 1, z], C[x + 1, z - 1]).
        for x in 0..5 {
            for z in 0..64 {
                row[reg_c_prime(x, z)] = xor([
                    row[reg_c(x, z)],
                    row[reg_c((x + 4) % 5, z)],
                    row[reg_c((x + 1) % 5, (z + 63) % 64)],
                ]);
            }
        }

        // Populate A'. To avoid shifting indices, we rewrite
        //     A'[x, y, z] = xor(A[x, y, z], C[x - 1, z], C[x + 1, z - 1])
        // as
        //     A'[x, y, z] = xor(A[x, y, z], C[x, z], C'[x, z]).
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    let is_high_limb = z / 32;
                    let bit_in_limb = z % 32;
                    let reg_a_limb = reg_a(x, y) + is_high_limb;
                    let a_limb = row[reg_a_limb].to_canonical_u64() as u32;
                    let a_bit = F::from_bool(((a_limb >> bit_in_limb) & 1) != 0);
                    row[reg_a_prime(x, y, z)] =
                        xor([a_bit, row[reg_c(x, z)], row[reg_c_prime(x, z)]]);
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

    pub fn generate_trace(
        &self,
        inputs: Vec<[u64; NUM_INPUTS]>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(inputs, min_rows)
        );
        let trace_polys = timed!(
            timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );
        trace_polys
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        eval_round_flags(vars, yield_constr);

        // The filter must be 0 or 1.
        let filter = vars.local_values[REG_FILTER];
        yield_constr.constraint(filter * (filter - P::ONES));

        // If this is not the final step, the filter must be off.
        let final_step = vars.local_values[reg_step(NUM_ROUNDS - 1)];
        let not_final_step = P::ONES - final_step;
        yield_constr.constraint(not_final_step * filter);

        // If this is not the final step, the local and next preimages must match.
        for x in 0..5 {
            for y in 0..5 {
                let preimage = reg_preimage(x, y);
                let diff = vars.local_values[preimage] - vars.next_values[preimage];
                yield_constr.constraint_transition(not_final_step * diff);
            }
        }

        // C'[x, z] = xor(C[x, z], C[x - 1, z], C[x + 1, z - 1]).
        for x in 0..5 {
            for z in 0..64 {
                let xor = xor3_gen(
                    vars.local_values[reg_c(x, z)],
                    vars.local_values[reg_c((x + 4) % 5, z)],
                    vars.local_values[reg_c((x + 1) % 5, (z + 63) % 64)],
                );
                let c_prime = vars.local_values[reg_c_prime(x, z)];
                yield_constr.constraint(c_prime - xor);
            }
        }

        // Check that the input limbs are consistent with A' and D.
        // A[x, y, z] = xor(A'[x, y, z], D[x, y, z])
        //            = xor(A'[x, y, z], C[x - 1, z], C[x + 1, z - 1])
        //            = xor(A'[x, y, z], C[x, z], C'[x, z]).
        // The last step is valid based on the identity we checked above.
        // It isn't required, but makes this check a bit cleaner.
        for x in 0..5 {
            for y in 0..5 {
                let a_lo = vars.local_values[reg_a(x, y)];
                let a_hi = vars.local_values[reg_a(x, y) + 1];
                let get_bit = |z| {
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let c = vars.local_values[reg_c(x, z)];
                    let c_prime = vars.local_values[reg_c_prime(x, z)];
                    xor3_gen(a_prime, c, c_prime)
                };
                let computed_lo = (0..32)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));
                let computed_hi = (32..64)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));
                yield_constr.constraint(computed_lo - a_lo);
                yield_constr.constraint(computed_hi - a_hi);
            }
        }

        // xor_{i=0}^4 A'[x, i, z] = C'[x, z], so for each x, z,
        // diff * (diff - 2) * (diff - 4) = 0, where
        // diff = sum_{i=0}^4 A'[x, i, z] - C'[x, z]
        for x in 0..5 {
            for z in 0..64 {
                let sum: P = [0, 1, 2, 3, 4]
                    .map(|i| vars.local_values[reg_a_prime(x, i, z)])
                    .into_iter()
                    .sum();
                let diff = sum - vars.local_values[reg_c_prime(x, z)];
                yield_constr
                    .constraint(diff * (diff - FE::TWO) * (diff - FE::from_canonical_u8(4)));
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
                let input_lo = vars.next_values[reg_a(x, y)];
                let input_hi = vars.next_values[reg_a(x, y) + 1];
                let is_last_round = vars.local_values[reg_step(NUM_ROUNDS - 1)];
                let not_last_round = P::ONES - is_last_round;
                yield_constr.constraint_transition(not_last_round * (output_lo - input_lo));
                yield_constr.constraint_transition(not_last_round * (output_hi - input_hi));
            }
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let one_ext = builder.one_extension();
        let two = builder.two();
        let two_ext = builder.two_extension();
        let four_ext = builder.constant_extension(F::Extension::from_canonical_u8(4));

        eval_round_flags_recursively(builder, vars, yield_constr);

        // The filter must be 0 or 1.
        let filter = vars.local_values[REG_FILTER];
        let constraint = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constraint);

        // If this is not the final step, the filter must be off.
        let final_step = vars.local_values[reg_step(NUM_ROUNDS - 1)];
        let not_final_step = builder.sub_extension(one_ext, final_step);
        let constraint = builder.mul_extension(not_final_step, filter);
        yield_constr.constraint(builder, constraint);

        // If this is not the final step, the local and next preimages must match.
        for x in 0..5 {
            for y in 0..5 {
                let preimage = reg_preimage(x, y);
                let diff =
                    builder.sub_extension(vars.local_values[preimage], vars.next_values[preimage]);
                let constraint = builder.mul_extension(not_final_step, diff);
                yield_constr.constraint_transition(builder, constraint);
            }
        }

        // C'[x, z] = xor(C[x, z], C[x - 1, z], C[x + 1, z - 1]).
        for x in 0..5 {
            for z in 0..64 {
                let xor = xor3_gen_circuit(
                    builder,
                    vars.local_values[reg_c(x, z)],
                    vars.local_values[reg_c((x + 4) % 5, z)],
                    vars.local_values[reg_c((x + 1) % 5, (z + 63) % 64)],
                );
                let c_prime = vars.local_values[reg_c_prime(x, z)];
                let diff = builder.sub_extension(c_prime, xor);
                yield_constr.constraint(builder, diff);
            }
        }

        // Check that the input limbs are consistent with A' and D.
        // A[x, y, z] = xor(A'[x, y, z], D[x, y, z])
        //            = xor(A'[x, y, z], C[x - 1, z], C[x + 1, z - 1])
        //            = xor(A'[x, y, z], C[x, z], C'[x, z]).
        // The last step is valid based on the identity we checked above.
        // It isn't required, but makes this check a bit cleaner.
        for x in 0..5 {
            for y in 0..5 {
                let a_lo = vars.local_values[reg_a(x, y)];
                let a_hi = vars.local_values[reg_a(x, y) + 1];
                let mut get_bit = |z| {
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let c = vars.local_values[reg_c(x, z)];
                    let c_prime = vars.local_values[reg_c_prime(x, z)];
                    xor3_gen_circuit(builder, a_prime, c, c_prime)
                };
                let bits_lo = (0..32).map(&mut get_bit).collect_vec();
                let bits_hi = (32..64).map(get_bit).collect_vec();
                let computed_lo = reduce_with_powers_ext_circuit(builder, &bits_lo, two);
                let computed_hi = reduce_with_powers_ext_circuit(builder, &bits_hi, two);
                let diff = builder.sub_extension(computed_lo, a_lo);
                yield_constr.constraint(builder, diff);
                let diff = builder.sub_extension(computed_hi, a_hi);
                yield_constr.constraint(builder, diff);
            }
        }

        // xor_{i=0}^4 A'[x, i, z] = C'[x, z], so for each x, z,
        // diff * (diff - 2) * (diff - 4) = 0, where
        // diff = sum_{i=0}^4 A'[x, i, z] - C'[x, z]
        for x in 0..5 {
            for z in 0..64 {
                let sum = builder.add_many_extension(
                    [0, 1, 2, 3, 4].map(|i| vars.local_values[reg_a_prime(x, i, z)]),
                );
                let diff = builder.sub_extension(sum, vars.local_values[reg_c_prime(x, z)]);
                let diff_minus_two = builder.sub_extension(diff, two_ext);
                let diff_minus_four = builder.sub_extension(diff, four_ext);
                let constraint =
                    builder.mul_many_extension([diff, diff_minus_two, diff_minus_four]);
                yield_constr.constraint(builder, constraint);
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
                let input_lo = vars.next_values[reg_a(x, y)];
                let input_hi = vars.next_values[reg_a(x, y) + 1];
                let is_last_round = vars.local_values[reg_step(NUM_ROUNDS - 1)];
                let diff = builder.sub_extension(input_lo, output_lo);
                let filtered_diff = builder.mul_sub_extension(is_last_round, diff, diff);
                yield_constr.constraint_transition(builder, filtered_diff);
                let diff = builder.sub_extension(input_hi, output_hi);
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
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::fri::oracle::PolynomialBatch;
    use plonky2::iop::challenger::Challenger;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::timed;
    use plonky2::util::timing::TimingTree;
    use tiny_keccak::keccakf;

    use crate::config::StarkConfig;
    use crate::cross_table_lookup::{CtlData, CtlZData};
    use crate::keccak::columns::reg_output_limb;
    use crate::keccak::keccak_stark::{KeccakStark, NUM_INPUTS, NUM_ROUNDS};
    use crate::permutation::GrandProductChallenge;
    use crate::prover::prove_single_table;
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

        let rows = stark.generate_trace_rows(vec![input.try_into().unwrap()], 8);
        let last_row = rows[NUM_ROUNDS - 1];
        let output = (0..NUM_INPUTS)
            .map(|i| {
                let hi = last_row[reg_output_limb(2 * i + 1)].to_canonical_u64();
                let lo = last_row[reg_output_limb(2 * i)].to_canonical_u64();
                (hi << 32) | lo
            })
            .collect::<Vec<_>>();

        let expected = {
            let mut state = input;
            keccakf(&mut state);
            state
        };

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn keccak_benchmark() -> Result<()> {
        const NUM_PERMS: usize = 85;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakStark<F, D>;
        let stark = S::default();
        let config = StarkConfig::standard_fast_config();

        init_logger();

        let input: Vec<[u64; NUM_INPUTS]> = (0..NUM_PERMS).map(|_| rand::random()).collect();

        let mut timing = TimingTree::new("prove", log::Level::Debug);
        let trace_poly_values = timed!(
            timing,
            "generate trace",
            stark.generate_trace(input.try_into().unwrap(), 8, &mut timing)
        );

        // TODO: Cloning this isn't great; consider having `from_values` accept a reference,
        // or having `compute_permutation_z_polys` read trace values from the `PolynomialBatch`.
        let cloned_trace_poly_values = timed!(timing, "clone", trace_poly_values.clone());

        let trace_commitments = timed!(
            timing,
            "compute trace commitment",
            PolynomialBatch::<F, C, D>::from_values(
                cloned_trace_poly_values,
                config.fri_config.rate_bits,
                false,
                config.fri_config.cap_height,
                &mut timing,
                None,
            )
        );
        let degree = 1 << trace_commitments.degree_log;

        // Fake CTL data.
        let ctl_z_data = CtlZData {
            z: PolynomialValues::zero(degree),
            challenge: GrandProductChallenge {
                beta: F::ZERO,
                gamma: F::ZERO,
            },
            columns: vec![],
            filter_column: None,
        };
        let ctl_data = CtlData {
            zs_columns: vec![ctl_z_data; config.num_challenges],
        };

        prove_single_table(
            &stark,
            &config,
            &trace_poly_values,
            &trace_commitments,
            &ctl_data,
            &mut Challenger::new(),
            &mut timing,
        )?;

        timing.print();
        Ok(())
    }

    fn init_logger() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));
    }
}
