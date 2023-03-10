macro_rules! implement_poseidon2 {
    ($trait_name: ident, $all_round_constants: expr, $width: expr) => {
      pub trait $trait_name: PrimeField64 {

        // Total number of round constants required: width of the input
        // times number of rounds.
        const N_ROUND_CONSTANTS: usize = $width * N_ROUNDS;

        // We only need INTERNAL_MATRIX_DIAG_M_1 here, specifying the diagonal - 1 of the internal matrix
        const INTERNAL_MATRIX_DIAG_M_1: [u64; $width];

        #[inline(always)]
        #[unroll_for_loops]
        fn external_matrix(state_: &mut[Self; $width]) {
            // Applying cheap 4x4 MDS matrix to each 4-element part of the state
            // The matrix in this case is:
            // M_4 =
            // [5   7   1   3]
            // [4   6   1   1]
            // [1   3   5   7]
            // [1   1   4   6]
            // The computation is shown in more detail in https://tosc.iacr.org/index.php/ToSC/article/view/888/839, Figure 13 (M_{4,4}^{8,4} with alpha = 2)
            let t4 = $width / 4;
            let mut state_u128: Vec<u128> = state_.iter().map(|&e| e.to_canonical_u64() as u128).collect();
            for i in 0..t4 {
                let start_index = i * 4;
                let mut t_0 = state_u128[start_index];
                t_0 += state_u128[start_index + 1];
                let mut t_1 = state_u128[start_index + 2];
                t_1 += state_u128[start_index + 3];
                let mut t_2 = state_u128[start_index + 1];
                t_2 = t_2 + t_2;
                t_2 += t_1;
                let mut t_3 = state_u128[start_index + 3];
                t_3 = t_3 + t_3;
                t_3 += t_0;
                let mut t_4 = t_1;
                t_4 = 4 * t_4;
                t_4 += t_3;
                let mut t_5 = t_0;
                t_5 = 4 * t_5;
                t_5 += t_2;
                let mut t_6 = t_3;
                t_6 += t_5;
                let mut t_7 = t_2;
                t_7 += t_4;
                state_u128[start_index] = t_6;
                state_u128[start_index + 1] = t_5;
                state_u128[start_index + 2] = t_7;
                state_u128[start_index + 3] = t_4;
            }

            // Applying second cheap matrix
            // This completes the multiplication by
            // M_E =
            // [2*M_4    M_4    M_4]
            // [  M_4  2*M_4    M_4]
            // [  M_4    M_4  2*M_4]
            // using the results with M_4 obtained above
            let mut stored = [0 as u128; 4];
            for l in 0..4 {
                stored[l] = state_u128[l];
                for j in 1..t4 {
                    stored[l] += state_u128[4 * j + l];
                }
            }
            for i in 0..state_u128.len() {
                state_u128[i] += stored[i % 4];
                state_[i] = Self::from_noncanonical_u128(state_u128[i]);
            }
        }

        /// Same as `external_matrix` for field extensions of `Self`.
        fn external_matrix_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
            state: &mut [F; $width],
        ) {
            // Applying cheap 4x4 MDS matrix to each 4-element part of the state
            let t4 = $width / 4;
            for i in 0..t4 {
                let start_index = i * 4;
                let mut t_0 = state[start_index];
                t_0 += state[start_index + 1];
                let mut t_1 = state[start_index + 2];
                t_1 += state[start_index + 3];
                let mut t_2 = state[start_index + 1];
                t_2 = t_2 + t_2;
                t_2 += t_1;
                let mut t_3 = state[start_index + 3];
                t_3 = t_3 + t_3;
                t_3 += t_0;
                let mut t_4 = t_1;
                t_4 = F::from_canonical_u64(4) * t_4;
                t_4 += t_3;
                let mut t_5 = t_0;
                t_5 = F::from_canonical_u64(4) * t_5;
                t_5 += t_2;
                let mut t_6 = t_3;
                t_6 += t_5;
                let mut t_7 = t_2;
                t_7 += t_4;
                state[start_index] = t_6;
                state[start_index + 1] = t_5;
                state[start_index + 2] = t_7;
                state[start_index + 3] = t_4;
            }

            // Applying second cheap matrix
            let mut stored = [F::ZERO; 4];
            for l in 0..4 {
                stored[l] = state[l];
                for j in 1..t4 {
                    stored[l] += state[4 * j + l];
                }
            }
            for i in 0..state.len() {
                state[i] += stored[i % 4];
            }

        }

        /// Recursive version of `external_matrix`.
        fn external_matrix_circuit<const D: usize>(
            builder: &mut CircuitBuilder<Self, D>,
            state: &mut [ExtensionTarget<D>; $width],
        )
            where
                Self: RichField + Extendable<D>,
        {
            // In contrast to the Poseidon circuit, we *may not need* PoseidonMdsGate, because the number of constraints will fit regardless
            // Check!
            let four = Self::from_canonical_u64(0x4);
            // let four = builder.constant_extension(Self::Extension::from_canonical_u64(0x4));

            // Applying cheap 4x4 MDS matrix to each 4-element part of the state
            let t4 = $width / 4;
            for i in 0..t4 {
                let start_index = i * 4;
                let mut t_0 = state[start_index];
                t_0 = builder.add_extension(t_0, state[start_index + 1]);
                let mut t_1 = state[start_index + 2];
                t_1 = builder.add_extension(t_1, state[start_index + 3]);
                let mut t_2 = state[start_index + 1];
                t_2 = builder.add_extension(t_2, t_2); // Double
                t_2 = builder.add_extension(t_2, t_1);
                let mut t_3 = state[start_index + 3];
                t_3 = builder.add_extension(t_3, t_3); // Double
                t_3 = builder.add_extension(t_3, t_0);
                let mut t_4 = t_1;
                t_4 = builder.mul_const_extension(four, t_4); // times 4
                t_4 = builder.add_extension(t_4, t_3);
                let mut t_5 = t_0;
                t_5 = builder.mul_const_extension(four, t_5); // times 4
                t_5 = builder.add_extension(t_5, t_2);
                let mut t_6 = t_3;
                t_6 = builder.add_extension(t_6, t_5);
                let mut t_7 = t_2;
                t_7 = builder.add_extension(t_7, t_4);
                state[start_index] = t_6;
                state[start_index + 1] = t_5;
                state[start_index + 2] = t_7;
                state[start_index + 3] = t_4;
            }

            // Applying second cheap matrix
            let mut stored = [builder.zero_extension(); 4];
            for l in 0..4 {
                stored[l] = state[l];
                for j in 1..t4 {
                    stored[l] = builder.add_extension(stored[l], state[4 * j + l]);
                }
            }
            for i in 0..state.len() {
                state[i] = builder.add_extension(state[i], stored[i % 4]);
            }
        }

        #[inline(always)]
        #[unroll_for_loops]
        fn internal_matrix(state_: &mut [Self; $width]) {

            // This computes the mutliplication with the matrix
            // M_I =
            // [r_1     1   1   ...     1]
            // [  1   r_2   1   ...     1]
            // ...
            // [  1     1   1   ...   r_t]
            // for pseudo-random values r_1, r_2, ..., r_t. Note that for efficiency in Self::INTERNAL_MATRIX_DIAG_M_1 only r_1 - 1, r_2 - 1, ..., r_t - 1 are stored
            // Compute input sum
            let mut sum = state_[0].to_noncanonical_u64() as u128;
            state_
                .iter()
                .skip(1)
                .take($width-1)
                .for_each(|el| sum += (*el).to_noncanonical_u64() as u128);
            let f_sum = Self::from_noncanonical_u128(sum);
            // Add sum + diag entry * element to each element
            for i in 0..$width {
                state_[i] *= Self::from_canonical_u64(Self::INTERNAL_MATRIX_DIAG_M_1[i]);
                state_[i] += f_sum;
            }
        }

        /// Same as `internal_matrix` for field extensions of `Self`.
        fn internal_matrix_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
            state: &mut [F; $width],
        ) {
            let t = $width;

            // Compute input sum
            let mut sum = state[0];
            state
                .iter()
                .skip(1)
                .take(t-1)
                .for_each(|el| sum += *el);
            // Add sum + diag entry * element to each element
            for i in 0..state.len() {
                state[i] *= F::from_canonical_u64(Self::INTERNAL_MATRIX_DIAG_M_1[i]);
                state[i] += sum;
            }
        }

        /// Recursive version of `internal_matrix`.
        fn internal_matrix_circuit<const D: usize>(
            builder: &mut CircuitBuilder<Self, D>,
            state: &mut [ExtensionTarget<D>; $width],
        )
            where
                Self: RichField + Extendable<D>,
        {
            // In contrast to the Poseidon circuit, we *may not need* PoseidonMdsGate, because the number of constraints will fit regardless
            // Check!

            // Compute input sum
            let mut sum = state[0];
            for i in 1..state.len() {
                sum = builder.add_extension(sum, state[i]);
            }
            // Add sum + diag entry * element to each element
            for i in 0..state.len() {
                // Computes `C * x + y`
                state[i] = builder.mul_const_add_extension(Self::from_canonical_u64(Self::INTERNAL_MATRIX_DIAG_M_1[i]), state[i], sum);
            }
        }

        #[inline(always)]
        #[unroll_for_loops]
        fn constant_layer(state: &mut [Self; $width], round_ctr: usize) {
            for i in 0..12 {
                if i < $width {
                    let round_constant = $all_round_constants[i + $width * round_ctr];
                    unsafe {
                        state[i] = state[i].add_canonical_u64(round_constant);
                    }
                }
            }
        }

        /// Same as `constant_layer` for field extensions of `Self`.
        fn constant_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
            state: &mut [F; $width],
            round_ctr: usize,
        ) {
            for i in 0..$width {
                state[i] += F::from_canonical_u64($all_round_constants[i + $width * round_ctr]);
            }
        }

        /// Recursive version of `constant_layer`.
        fn constant_layer_circuit<const D: usize>(
            builder: &mut CircuitBuilder<Self, D>,
            state: &mut [ExtensionTarget<D>; $width],
            round_ctr: usize,
        ) where
            Self: RichField + Extendable<D>,
        {
            for i in 0..$width {
                let c = $all_round_constants[i + $width * round_ctr];
                let c = Self::Extension::from_canonical_u64(c);
                let c = builder.constant_extension(c);
                state[i] = builder.add_extension(state[i], c);
            }
        }

        #[inline(always)]
        fn sbox_monomial<F: FieldExtension<D, BaseField = Self>, const D: usize>(x: F) -> F {
            // x |--> x^7
            let x2 = x.square();
            let x4 = x2.square();
            let x3 = x * x2;
            x3 * x4
        }

        /// Recursive version of `sbox_monomial`.
        fn sbox_monomial_circuit<const D: usize>(
            builder: &mut CircuitBuilder<Self, D>,
            x: ExtensionTarget<D>,
        ) -> ExtensionTarget<D>
            where
                Self: RichField + Extendable<D>,
        {
            // x |--> x^7
            builder.exp_u64_extension(x, 7)
        }

        #[inline(always)]
        #[unroll_for_loops]
        fn sbox_layer(state: &mut [Self; $width]) {
            for i in 0..12 {
                if i < $width {
                    state[i] = Self::sbox_monomial(state[i]);
                }
            }
        }

        /// Same as `sbox_layer` for field extensions of `Self`.
        fn sbox_layer_field<F: FieldExtension<D, BaseField = Self>, const D: usize>(
            state: &mut [F; $width],
        ) {
            for i in 0..$width {
                state[i] = Self::sbox_monomial(state[i]);
            }
        }

        /// Recursive version of `sbox_layer`.
        fn sbox_layer_circuit<const D: usize>(
            builder: &mut CircuitBuilder<Self, D>,
            state: &mut [ExtensionTarget<D>; $width],
        ) where
            Self: RichField + Extendable<D>,
        {
            for i in 0..$width {
                state[i] = <Self as $trait_name>::sbox_monomial_circuit(builder, state[i]);
            }
        }

        #[inline]
        fn full_rounds(state: &mut [Self; $width], round_ctr: &mut usize) {
            for _ in 0..HALF_N_FULL_ROUNDS {
                Self::constant_layer(state, *round_ctr);
                Self::sbox_layer(state);
                Self::external_matrix(state);
                *round_ctr += 1;
            }
        }

        #[inline]
        fn partial_rounds(state: &mut [Self; $width], round_ctr: &mut usize) {
            let mut constant_counter = HALF_N_FULL_ROUNDS * $width;
            for _ in 0..N_PARTIAL_ROUNDS {
                unsafe {
                    state[0] = state[0].add_canonical_u64($all_round_constants[constant_counter]);
                    constant_counter += $width;
                }
                state[0] = Self::sbox_monomial(state[0]);
                Self::internal_matrix(state);
            }
            *round_ctr += N_PARTIAL_ROUNDS;
        }

        #[inline]
        fn poseidon2(input: [Self; $width]) -> [Self; $width] {
            let mut state = input;
            let mut round_ctr = 0;

            // First external matrix
            Self::external_matrix(&mut state);

            Self::full_rounds(&mut state, &mut round_ctr);
            Self::partial_rounds(&mut state, &mut round_ctr);
            Self::full_rounds(&mut state, &mut round_ctr);
            debug_assert_eq!(round_ctr, N_ROUNDS);

            state
        }
      }
    }
}

macro_rules! implement_poseidon2_gate {
    ($gate_name: ident, $poseidon_trait_name: ident, $generator_name: ident, $width: expr) => {
        impl<F: RichField + Extendable<D>, const D: usize> $gate_name<F, D> {
            pub fn new() -> Self {
                Self(PhantomData)
            }

            /// The wire index for the `i`th input to the permutation.
            pub fn wire_input(i: usize) -> usize {
                i
            }

            /// The wire index for the `i`th output to the permutation.
            pub fn wire_output(i: usize) -> usize {
                $width + i
            }

            /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
            /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
            pub const WIRE_SWAP: usize = 2 * $width;

            const START_DELTA: usize = 2 * $width + 1;

            /// A wire which stores `swap * (input[i + 4] - input[i])`; used to compute the swapped inputs.
            fn wire_delta(i: usize) -> usize {
                assert!(i < 4);
                Self::START_DELTA + i
            }

            const START_FULL_0: usize = Self::START_DELTA + 4;

            /// A wire which stores the input of the `i`-th S-box of the `round`-th round of the first set
            /// of full rounds.
            fn wire_full_sbox_0(round: usize, i: usize) -> usize {
                debug_assert!(
                    round != 0,
                    "First round S-box inputs are not stored as wires"
                );
                debug_assert!(round < poseidon2::HALF_N_FULL_ROUNDS);
                debug_assert!(i < $width);
                Self::START_FULL_0 + $width * (round - 1) + i
            }

            const START_PARTIAL: usize =
                Self::START_FULL_0 + $width * (poseidon2::HALF_N_FULL_ROUNDS - 1);

            /// A wire which stores the input of the S-box of the `round`-th round of the partial rounds.
            fn wire_partial_sbox(round: usize) -> usize {
                debug_assert!(round < poseidon2::N_PARTIAL_ROUNDS);
                Self::START_PARTIAL + round
            }

            const START_FULL_1: usize = Self::START_PARTIAL + poseidon2::N_PARTIAL_ROUNDS;

            /// A wire which stores the input of the `i`-th S-box of the `round`-th round of the second set
            /// of full rounds.
            fn wire_full_sbox_1(round: usize, i: usize) -> usize {
                debug_assert!(round < poseidon2::HALF_N_FULL_ROUNDS);
                debug_assert!(i < $width);
                Self::START_FULL_1 + $width * round + i
            }

            /// End of wire indices, exclusive.
            fn end() -> usize {
                Self::START_FULL_1 + $width * poseidon2::HALF_N_FULL_ROUNDS
            }
        }

        impl<F: RichField + Extendable<D> + $poseidon_trait_name, const D: usize> Gate<F, D> for $gate_name<F, D> {
            fn id(&self) -> String {
                format!("{self:?}<WIDTH={}>", $width)
            }

            fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
                let mut constraints = Vec::with_capacity(self.num_constraints());

                // Assert that `swap` is binary.
                let swap = vars.local_wires[Self::WIRE_SWAP];
                constraints.push(swap * (swap - F::Extension::ONE));

                // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
                for i in 0..4 {
                    let input_lhs = vars.local_wires[Self::wire_input(i)];
                    let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    constraints.push(swap * (input_rhs - input_lhs) - delta_i);
                }

                // Compute the possibly-swapped input layer.
                let mut state = [F::Extension::ZERO; $width];
                for i in 0..4 {
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    let input_lhs = Self::wire_input(i);
                    let input_rhs = Self::wire_input(i + 4);
                    state[i] = vars.local_wires[input_lhs] + delta_i;
                    state[i + 4] = vars.local_wires[input_rhs] - delta_i;
                }
                for i in 8..$width {
                    state[i] = vars.local_wires[Self::wire_input(i)];
                }

                let mut round_ctr = 0;

                // First external matrix
                <F as $poseidon_trait_name>::external_matrix_field(&mut state);

                // First set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer_field(&mut state, round_ctr);
                    if r != 0 {
                        for i in 0..$width {
                            let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                            constraints.push(state[i] - sbox_in);
                            state[i] = sbox_in;
                        }
                    }
                    <F as $poseidon_trait_name>::sbox_layer_field(&mut state);
                    <F as $poseidon_trait_name>::external_matrix_field(&mut state);
                    round_ctr += 1;
                }

                // Partial rounds.
                let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * $width;
                for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
                    state[0] += F::Extension::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
                    constant_counter += $width;
                    let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
                    constraints.push(state[0] - sbox_in);
                    state[0] = <F as $poseidon_trait_name>::sbox_monomial(sbox_in);
                    <F as $poseidon_trait_name>::internal_matrix_field(&mut state);
                }
                round_ctr += poseidon2::N_PARTIAL_ROUNDS;

                // Second set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer_field(&mut state, round_ctr);
                    for i in 0..$width {
                        let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                        constraints.push(state[i] - sbox_in);
                        state[i] = sbox_in;
                    }
                    <F as $poseidon_trait_name>::sbox_layer_field(&mut state);
                    <F as $poseidon_trait_name>::external_matrix_field(&mut state);
                    round_ctr += 1;
                }

                for i in 0..$width {
                    constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
                }

                constraints
            }

            fn eval_unfiltered_base_one(
                &self,
                vars: EvaluationVarsBase<F>,
                mut yield_constr: StridedConstraintConsumer<F>,
            ) {
                // Assert that `swap` is binary.
                let swap = vars.local_wires[Self::WIRE_SWAP];
                yield_constr.one(swap * swap.sub_one());

                // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
                for i in 0..4 {
                    let input_lhs = vars.local_wires[Self::wire_input(i)];
                    let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    yield_constr.one(swap * (input_rhs - input_lhs) - delta_i);
                }

                // Compute the possibly-swapped input layer.
                let mut state = [F::ZERO; $width];
                for i in 0..4 {
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    let input_lhs = Self::wire_input(i);
                    let input_rhs = Self::wire_input(i + 4);
                    state[i] = vars.local_wires[input_lhs] + delta_i;
                    state[i + 4] = vars.local_wires[input_rhs] - delta_i;
                }
                for i in 8..$width {
                    state[i] = vars.local_wires[Self::wire_input(i)];
                }

                let mut round_ctr = 0;

                // First external matrix
                <F as $poseidon_trait_name>::external_matrix(&mut state);

                // First set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer(&mut state, round_ctr);
                    if r != 0 {
                        for i in 0..$width {
                            let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                            yield_constr.one(state[i] - sbox_in);
                            state[i] = sbox_in;
                        }
                    }
                    <F as $poseidon_trait_name>::sbox_layer(&mut state);
                    <F as $poseidon_trait_name>::external_matrix(&mut state);
                    round_ctr += 1;
                }

                // Partial rounds.
                let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * $width;
                for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
                    state[0] += F::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
                    constant_counter += $width;
                    let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
                    yield_constr.one(state[0] - sbox_in);
                    state[0] = <F as $poseidon_trait_name>::sbox_monomial(sbox_in);
                    <F as $poseidon_trait_name>::internal_matrix(&mut state);
                }
                round_ctr += poseidon2::N_PARTIAL_ROUNDS;

                // Second set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer(&mut state, round_ctr);
                    for i in 0..$width {
                        let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                        yield_constr.one(state[i] - sbox_in);
                        state[i] = sbox_in;
                    }
                    <F as $poseidon_trait_name>::sbox_layer(&mut state);
                    <F as $poseidon_trait_name>::external_matrix(&mut state);
                    round_ctr += 1;
                }

                for i in 0..$width {
                    yield_constr.one(state[i] - vars.local_wires[Self::wire_output(i)]);
                }
            }

            fn eval_unfiltered_circuit(
                &self,
                builder: &mut CircuitBuilder<F, D>,
                vars: EvaluationTargets<D>,
            ) -> Vec<ExtensionTarget<D>> {

                let mut constraints = Vec::with_capacity(self.num_constraints());

                // Assert that `swap` is binary.
                let swap = vars.local_wires[Self::WIRE_SWAP];
                constraints.push(builder.mul_sub_extension(swap, swap, swap));

                // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
                for i in 0..4 {
                    let input_lhs = vars.local_wires[Self::wire_input(i)];
                    let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    let diff = builder.sub_extension(input_rhs, input_lhs);
                    constraints.push(builder.mul_sub_extension(swap, diff, delta_i));
                }

                // Compute the possibly-swapped input layer.
                let mut state = [builder.zero_extension(); $width];
                for i in 0..4 {
                    let delta_i = vars.local_wires[Self::wire_delta(i)];
                    let input_lhs = vars.local_wires[Self::wire_input(i)];
                    let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
                    state[i] = builder.add_extension(input_lhs, delta_i);
                    state[i + 4] = builder.sub_extension(input_rhs, delta_i);
                }
                for i in 8..$width {
                    state[i] = vars.local_wires[Self::wire_input(i)];
                }

                let mut round_ctr = 0;

                // First external matrix
                <F as $poseidon_trait_name>::external_matrix_circuit(builder, &mut state);

                // First set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer_circuit(builder, &mut state, round_ctr);
                    if r != 0 {
                        for i in 0..$width {
                            let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                            constraints.push(builder.sub_extension(state[i], sbox_in));
                            state[i] = sbox_in;
                        }
                    }
                    <F as $poseidon_trait_name>::sbox_layer_circuit(builder, &mut state);
                    <F as $poseidon_trait_name>::external_matrix_circuit(builder, &mut state);
                    round_ctr += 1;
                }

                // Partial rounds.
                let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * $width;
                for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
                    let c = poseidon2::ALL_ROUND_CONSTANTS[constant_counter];
                    let c = F::Extension::from_canonical_u64(c);
                    let c = builder.constant_extension(c);
                    state[0] = builder.add_extension(state[0], c);
                    constant_counter += $width;
                    let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
                    constraints.push(builder.sub_extension(state[0], sbox_in));
                    state[0] = <F as $poseidon_trait_name>::sbox_monomial_circuit(builder, sbox_in);
                    <F as $poseidon_trait_name>::internal_matrix_circuit(builder, &mut state);
                }
                round_ctr += poseidon2::N_PARTIAL_ROUNDS;

                // Second set of full rounds.
                for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
                    <F as $poseidon_trait_name>::constant_layer_circuit(builder, &mut state, round_ctr);
                    for i in 0..$width {
                        let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                        constraints.push(builder.sub_extension(state[i], sbox_in));
                        state[i] = sbox_in;
                    }
                    <F as $poseidon_trait_name>::sbox_layer_circuit(builder, &mut state);
                    <F as $poseidon_trait_name>::external_matrix_circuit(builder, &mut state);
                    round_ctr += 1;
                }

                for i in 0..$width {
                    constraints
                        .push(builder.sub_extension(state[i], vars.local_wires[Self::wire_output(i)]));
                }

                constraints
            }

            fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
                let gen = $generator_name::<F, D> {
                    row,
                    _phantom: PhantomData,
                };
                vec![Box::new(gen.adapter())]
            }

            fn num_wires(&self) -> usize {
                Self::end()
            }

            fn num_constants(&self) -> usize {
                0
            }

            fn degree(&self) -> usize {
                7
            }

            fn num_constraints(&self) -> usize {
                $width * (poseidon2::N_FULL_ROUNDS_TOTAL - 1)
                    + poseidon2::N_PARTIAL_ROUNDS
                    + $width
                    + 1
                    + 4
            }
        }

        #[derive(Debug)]
struct $generator_name<F: RichField + Extendable<D> + $poseidon_trait_name, const D: usize> {
    row: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + $poseidon_trait_name, const D: usize> SimpleGenerator<F>
for $generator_name<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        (0..$width)
            .map(|i| $gate_name::<F, D>::wire_input(i))
            .chain(Some($gate_name::<F, D>::WIRE_SWAP))
            .map(|column| Target::wire(self.row, column))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let mut state = (0..$width)
            .map(|i| witness.get_wire(local_wire($gate_name::<F, D>::wire_input(i))))
            .collect::<Vec<_>>();

        let swap_value = witness.get_wire(local_wire($gate_name::<F, D>::WIRE_SWAP));
        debug_assert!(swap_value == F::ZERO || swap_value == F::ONE);

        for i in 0..4 {
            let delta_i = swap_value * (state[i + 4] - state[i]);
            out_buffer.set_wire(local_wire($gate_name::<F, D>::wire_delta(i)), delta_i);
        }

        if swap_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        let mut state: [F; $width] = state.try_into().unwrap();
        let mut round_ctr = 0;

        // First external matrix
        <F as $poseidon_trait_name>::external_matrix_field(&mut state);

        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as $poseidon_trait_name>::constant_layer_field(&mut state, round_ctr);
            if r != 0 {
                for i in 0..$width {
                    out_buffer.set_wire(
                        local_wire($gate_name::<F, D>::wire_full_sbox_0(r, i)),
                        state[i],
                    );
                }
            }
            <F as $poseidon_trait_name>::sbox_layer_field(&mut state);
            <F as $poseidon_trait_name>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * $width;
        for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
            state[0] += F::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
            constant_counter += $width;
            out_buffer.set_wire(
                local_wire($gate_name::<F, D>::wire_partial_sbox(r)),
                state[0],
            );
            state[0] = <F as $poseidon_trait_name>::sbox_monomial(state[0]);
            <F as $poseidon_trait_name>::internal_matrix_field(&mut state);
        }
        round_ctr += poseidon2::N_PARTIAL_ROUNDS;

        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as $poseidon_trait_name>::constant_layer_field(&mut state, round_ctr);
            for i in 0..$width {
                out_buffer.set_wire(
                    local_wire($gate_name::<F, D>::wire_full_sbox_1(r, i)),
                    state[i],
                );
            }
            <F as $poseidon_trait_name>::sbox_layer_field(&mut state);
            <F as $poseidon_trait_name>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        for i in 0..$width {
            out_buffer.set_wire(local_wire($gate_name::<F, D>::wire_output(i)), state[i]);
        }
    }
}
    }
}