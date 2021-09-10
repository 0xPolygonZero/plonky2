//! Implementation of the Poseidon hash function, as described in
//! https://eprint.iacr.org/2019/458.pdf

use unroll::unroll_for_loops;

use crate::field::crandall_field::CrandallField;
use crate::field::field_types::Field;
use crate::field::field_types::PrimeField;

// The number of full rounds and partial rounds is given by the
// calc_round_numbers.py script. They happen to be the same for both
// width 8 and width 12 with s-box x^7.
//
// NB: Changing any of these values will require regenerating all of
// the precomputed constant arrays in this file.
const HALF_N_FULL_ROUNDS: usize = 4;
const N_FULL_ROUNDS_TOTAL: usize = 2 * HALF_N_FULL_ROUNDS;
const N_PARTIAL_ROUNDS: usize = 22;
const N_ROUNDS: usize = N_FULL_ROUNDS_TOTAL + N_PARTIAL_ROUNDS;
const MAX_WIDTH: usize = 12; // we only have width 8 and 12, and 12 is bigger. :)

// The round constants are the same as for GMiMC (hash.rs):
// generated from ChaCha8 with a seed of 0. In this case we need
// to generate more though. We include enough for a WIDTH of 12;
// smaller widths just use a subset.
// TODO: These are specific to CrandallField; for other fields they wouldn't represent uniformly
// random numbers.
#[rustfmt::skip]
const ALL_ROUND_CONSTANTS: [u64; MAX_WIDTH * N_ROUNDS]  = [
    0xb585f767417ee042, 0x7746a55f77c10331, 0xb2fb0d321d356f7a, 0x0f6760a486f1621f,
    0xe10d6666b36abcdf, 0x8cae14cb455cc50b, 0xd438539cf2cee334, 0xef781c7d4c1fd8b4,
    0xcdc4a23a0aca4b1f, 0x277fa208d07b52e3, 0xe17653a300493d38, 0xc54302f27c287dc1,
    0x8628782231d47d10, 0x59cd1a8a690b49f2, 0xc3b919ad9efec0b0, 0xa484c4c637641d97,
    0x308bbd23f191398b, 0x6e4a40c1bf713cf1, 0x9a2eedb7510414fb, 0xe360c6e111c2c63b,
    0xd5c771901d4d89aa, 0xc35eae076e7d6b2f, 0x849c2656d0a09cad, 0xc0572c8c5cf1df2b,
    0xe9fa634a883b8bf3, 0xf56f6d4900fb1fdd, 0xf7d713e872a72a1b, 0x8297132b6ba47612,
    0xad6805e12ee8af1c, 0xac51d9f6485c22b9, 0x502ad7dc3bd56bf8, 0x57a1550c3761c577,
    0x66bbd30e99d311da, 0x0da2abef5e948f87, 0xf0612750443f8e94, 0x28b8ec3afb937d8c,
    0x92a756e6be54ca18, 0x70e741ec304e925d, 0x019d5ee2b037c59f, 0x6f6f2ed7a30707d1,
    0x7cf416d01e8c169c, 0x61df517bb17617df, 0x85dc499b4c67dbaa, 0x4b959b48dad27b23,
    0xe8be3e5e0dd779a0, 0xf5c0bc1e525ed8e6, 0x40b12cbf263cf853, 0xa637093f13e2ea3c,
    0x3cc3f89232e3b0c8, 0x2e479dc16bfe86c0, 0x6f49de07d6d39469, 0x213ce7beecc232de,
    0x5b043134851fc00a, 0xa2de45784a861506, 0x7103aaf97bed8dd5, 0x5326fc0dbb88a147,
    0xa9ceb750364cb77a, 0x27f8ec88cc9e991f, 0xfceb4fda8c93fb83, 0xfac6ff13b45b260e,
    0x7131aa455813380b, 0x93510360d5d68119, 0xad535b24fb96e3db, 0x4627f5c6b7efc045,
    0x645cf794e4da78a9, 0x241c70ed1ac2877f, 0xacb8e076b009e825, 0x3737e9db6477bd9d,
    0xe7ea5e344cd688ed, 0x90dee4a009214640, 0xd1b1edf7c77e74af, 0x0b65481bab42158e,
    0x99ad1aab4b4fe3e7, 0x438a7c91f1a360cd, 0xb60de3bd159088bf, 0xc99cab6b47a3e3bb,
    0x69a5ed92d5677cef, 0x5e7b329c482a9396, 0x5fc0ac0829f893c9, 0x32db82924fb757ea,
    0x0ade699c5cf24145, 0x7cc5583b46d7b5bb, 0x85df9ed31bf8abcb, 0x6604df501ad4de64,
    0xeb84f60941611aec, 0xda60883523989bd4, 0x8f97fe40bf3470bf, 0xa93f485ce0ff2b32,
    0x6704e8eebc2afb4b, 0xcee3e9ac788ad755, 0x510d0e66062a270d, 0xf6323f48d74634a0,
    0x0b508cdf04990c90, 0xf241708a4ef7ddf9, 0x60e75c28bb368f82, 0xa6217d8c3f0f9989,
    0x7159cd30f5435b53, 0x839b4e8fe97ec79f, 0x0d3f3e5e885db625, 0x8f7d83be1daea54b,
    0x780f22441e8dbc04, 0xeb9158465aedacd3, 0xd19e120d826c1b6c, 0x016ee53a7f007110,
    0xcb5fd54ed22dd1ca, 0xacb84178c58de144, 0x9c22190c2c463227, 0x5d693c1bcc98406d,
    0xdcef0798235f321a, 0x3d639263f55e0b1e, 0xe273fd977edb8fda, 0x418f027049d10fe7,
    0x8c25fda3f253a284, 0x2cbaed4dc25a884e, 0x5f58e6aff78dc2af, 0x284650ac6fb9d206,
    0x635b337f1391c13c, 0x9f9a036f1ac6361f, 0xb93e260cff6747b4, 0xb0a7eae8c7272e33,
    0xd0762cbce7da0a9f, 0x34c6efb829c754d6, 0x40bf0ab6166855c1, 0xb6b570fccc46a242,
    0x5a27b90055549545, 0xb1a5b166048b306f, 0x8722e0ad24f1006d, 0x788ee3b3b315049a,
    0x14a726661e5b0351, 0x98b7672fe1c3f13e, 0xbb93ae77bdc3aa8f, 0x28fd3b04756fc222,
    0x30a46805a86d7109, 0x337dc00c7844a0e7, 0xd5eca245253c861b, 0x77626382990d8546,
    0xc1e434bf33c3ae7a, 0x0299351a54dbf35e, 0xb2d456e4fb620184, 0x3e9ed1fdc00265ea,
    0x2972a92bb672e8db, 0x20216dd789f333ec, 0xadffe8cf746494a1, 0x1c4dbb1c5889d420,
    0x15a16a8a8c9972f5, 0x388a128b98960e26, 0x2300e5d6ca3e5589, 0x2f63aa865c9ceb9f,
    0xf1c36ce8d894420f, 0x271811252953f84a, 0xe5840293d5466a8e, 0x4d9bbc3e24e5f20e,
    0xea35bc29cfa2794b, 0x18e21b4bf59e2d28, 0x1e3b9fc632ef6adb, 0x25d643627a05e678,
    0x5a3f1bb1ecb63263, 0xdb7f0238ca031e31, 0xb462065960bfc4c4, 0x49c24ae463c280f4,
    0xd793862c6f7b901a, 0xaadd1106bdce475e, 0xc43b6e0eed8ad58f, 0xe29024c1f2060cb7,
    0x5e50c2755efbe17a, 0x10383f20ac183625, 0x38e8ee9d8a8a435d, 0xdd511837bcc52452,
    0x7750059861a7da6a, 0x86ab99b518d1dbef, 0xb1204f608ccfe33b, 0xef61ac84d8dfca49,
    0x1bbcd90f1f4eff36, 0x0cd1dabd9be9850a, 0x11a3ae5bf354bb11, 0xf755bfef11bb5516,
    0xa3b832506e2f3adb, 0x516306f4b617e6ba, 0xddb4ac4a2aeead3a, 0x64bb6dec62af4430,
    0xf9cc95c29895a152, 0x08d37f75632771b9, 0xeec49b619cee6b56, 0xf143933b56b3711a,
    0xe4c5dd82b9f6570c, 0xe7ad775756eefdc4, 0x92c2318bc834ef78, 0x739c25f93007aa0a,
    0x5636caca1725f788, 0xdd8f909af47cd0b6, 0xc6401fe16bc24d4e, 0x8ad97b342e6b3a3c,
    0x0c49366bb7be8ce2, 0x0784d3d2f4b39fb5, 0x530fb67ec5d77a58, 0x41049229b8221f3b,
    0x139542347cb606a3, 0x9cb0bd5ee62e6438, 0x02e3f615c4d3054a, 0x985d4f4adefb64a0,
    0x775b9feb32053cde, 0x304265a64d6c1ba6, 0x593664c3be7acd42, 0x4f0a2e5fd2bd6718,
    0xdd611f10619bf1da, 0xd8185f9b3e74f9a4, 0xef87139d126ec3b3, 0x3ba71336dd67f99b,
    0x7d3a455d8d808091, 0x660d32e15cbdecc7, 0x297a863f5af2b9ff, 0x90e0a736e6b434df,
    0x549f80ce7a12182e, 0x0f73b29235fb5b84, 0x16bf1f74056e3a01, 0x6d1f5a593019a39f,
    0x02ff876fa73f6305, 0xc5cb72a2fb9a5bd7, 0x8470f39d674dfaa3, 0x25abb3f1e41aea30,
    0x23eb8cc9c32951c7, 0xd687ba56242ac4ea, 0xda8d9e915d2de6b7, 0xe3cbdc7d938d8f1e,
    0xb9a8c9b4001efad6, 0xc0d28a5c64f2285c, 0x45d7ac9b878575b8, 0xeeb76e39d8da283e,
    0x3d06c8bd2fc7daac, 0x9c9c9820c13589f5, 0x65700b51db40bae3, 0x911f451579044242,
    0x7ae6849ff1fee8cc, 0x3bb340ebba896ae5, 0xb46e9d8bb71f0b4b, 0x8dcf22f9e1bde2a3,
    0x77bdaeda8cc55427, 0xf19e400ababa0e12, 0xc368a34939eb5c7f, 0x9ef1cd612c03bc5e,
    0xe89cd8553b94bbd8, 0x5cd377dcb4550713, 0xa7b0fb78cd4c5665, 0x7684403ef76c7128,
    0x5fa3f06f79c4f483, 0x8df57ac159dbade6, 0x2db01efa321b2625, 0x54846de4cfd58cb6,
    0xba674538aa20f5cd, 0x541d4963699f9777, 0xe9096784dadaa548, 0xdfe8992458bf85ff,
    0xece5a71e74a35593, 0x5ff98fd5ff1d14fd, 0x83e89419524c06e1, 0x5922040b6ef03286,
    0xf97d750eab002858, 0x5080d4c2dba7b3ec, 0xa7de115ba038b508, 0x6a9242acb5f37ec0,
    0xf7856ef865619ed0, 0x2265fc930dbd7a89, 0x17dfc8e5022c723b, 0x9001a64248f2d676,
    0x90004c13b0b8b50e, 0xb932b7cfc63485b0, 0xa0b1df81fd4c2bc5, 0x8ef1dd26b594c383,
    0x0541a4f9d20ba562, 0x9e611061be0a3c5b, 0xb3767e80e1e1624a, 0x0098d57820a88c6b,
    0x31d191cd71e01691, 0x410fefafbf90a57a, 0xbdf8f2433633aea8, 0x9e8cd55b9cc11c28,
    0xde122bec4acb869f, 0x4d001fd5b0b03314, 0xca66370067416209, 0x2f2339d6399888c6,
    0x6d1a7918f7c98a13, 0xdf9a493995f688f3, 0xebc2151f4ded22ca, 0x03cc2ba8a2bab82f,
    0xd341d03844ad9a9b, 0x387cb5d273ab3f58, 0xbba2515f74a7a221, 0x7248fe7737f37d9c,
    0x4d61e56a7437f6b9, 0x262e963c9e54bef8, 0x59e89b097477d296, 0x055d5b52b9e47452,
    0x82b27eb36e430708, 0xd30094caf3080f94, 0xcf5cb38227c2a3be, 0xfeed4db701262c7c,
    0x41703f5391dd0154, 0x5eeea9412666f57b, 0x4cd1f1b196abdbc4, 0x4a20358594b3662b,
    0x1478d361e4b47c26, 0x6f02dc0801d2c79f, 0x296a202eeb03c4b6, 0x2afd6799aec20c38,
    0x7acfd96f3050383d, 0x6798ba0c380dfdd3, 0x34c6f57b3de02c88, 0x5736e1baf82eb8a0,
    0x20057d2a0e58b8de, 0x3dea5bd5eb6e1404, 0x16e50d89874a6a98, 0x29bff3eccbfba19a,
    0x475cd3207974793c, 0x18a42105cde34cfa, 0x023e7414b0618331, 0x151471081b52594b,
    0xe4a3dff23bdeb0f3, 0x01a8d1a588c232ef, 0x11b4c74ee221d621, 0xe587cc0dce129c8c,
    0x1ff7327025a65080, 0x594e29c44b8602b1, 0xf6f31db1f5a56fd3, 0xc02ac5e4c7258a5e,
    0xe70201e9c5dc598f, 0x6f90ff3b9b3560b2, 0x42747a7262faf016, 0xd1f507e496927d26,
    0x1c86d265fdd24cd9, 0x3996ce73f6b5266e, 0x8e7fba02d68a061e, 0xba0dec71548b7546,
    0x9e9cbd785b8d8f40, 0xdae86459f6b3828c, 0xdebe08541314f71d, 0xa49229d29501358f,
    0x7be5ba0010c4df7c, 0xa3c95eaf09ecc39c, 0x0230bca8f5d457cd, 0x4135c2bedc68cdf9,
    0x166fc0cc4d5b20cc, 0x3762b59aa3236e6e, 0xe8928a4ceed163d2, 0x2a440b51b71223d9,
    0x80cefd2bb5f48e46, 0xbb9879c738328b71, 0x6e7c8f1ab47cced0, 0x164bb2de257ffc0a,
    0xf3c12fe5b800ea30, 0x40b9e92309e8c7e1, 0x551f5b0fe3b8d017, 0x25032aa7d4fc7aba,
    0xaaed340795de0a0a, 0x8ffd96bc38c8ba0f, 0x70fc91eb8aa58833, 0x7f795e2a97566d73,
    0x4543d9df72c4831d, 0xf172d73e69f20739, 0xdfd1c4ff1eb3d868, 0xbc8dfb62d26376f7,
];

pub trait Poseidon<const WIDTH: usize>: PrimeField
where
    // magic to get const generic expressions to work
    [(); WIDTH - 1]: ,
{
    // Total number of round constants required: width of the input
    // times number of rounds.
    const N_ROUND_CONSTANTS: usize = WIDTH * N_ROUNDS;

    // Use the MDS matrix which is circulant with entries 2^x for each
    // x in MDS_MATRIX_EXPS.
    const MDS_MATRIX_EXPS: [u64; WIDTH];

    // Precomputed constants for the fast Poseidon calculation. See
    // the paper.
    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; WIDTH];
    const FAST_PARTIAL_ROUND_CONSTANTS: [u64; N_PARTIAL_ROUNDS - 1];
    const FAST_PARTIAL_ROUND_VS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS];
    const FAST_PARTIAL_ROUND_W_HATS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS];
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; WIDTH - 1]; WIDTH - 1];

    #[inline(always)]
    #[unroll_for_loops]
    fn mds_row_shf(r: usize, v: &[u64; WIDTH]) -> u128 {
        debug_assert!(r < WIDTH);
        // The values of MDS_MATRIX_EXPS are known to be small, so we can
        // accumulate all the products for each row and reduce just once
        // at the end (done by the caller).

        // NB: Unrolling this, calculating each term independently, and
        // summing at the end, didn't improve performance for me.
        let mut res = 0u128;

        // This is a hacky way of fully unrolling the loop.
        assert!(WIDTH <= 12);
        for i in 0..12 {
            if i < WIDTH {
                res += (v[(i + r) % WIDTH] as u128) << Self::MDS_MATRIX_EXPS[i];
            }
        }

        res
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_layer(state_: &[Self; WIDTH]) -> [Self; WIDTH] {
        let mut result = [Self::ZERO; WIDTH];

        let mut state = [0u64; WIDTH];
        for r in 0..WIDTH {
            state[r] = state_[r].to_noncanonical_u64();
        }

        // This is a hacky way of fully unrolling the loop.
        assert!(WIDTH <= 12);
        for r in 0..12 {
            if r < WIDTH {
                result[r] = Self::from_noncanonical_u128(Self::mds_row_shf(r, &state));
            }
        }

        result
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_first_constant_layer(state: &mut [Self; WIDTH]) {
        for i in 0..WIDTH {
            state[i] += Self::from_canonical_u64(Self::FAST_PARTIAL_FIRST_ROUND_CONSTANT[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_partial_layer_init(state: &[Self; WIDTH]) -> [Self; WIDTH] {
        let mut result = [Self::ZERO; WIDTH];

        // Initial matrix has first row/column = [1, 0, ..., 0];

        // c = 0
        result[0] = state[0];

        for c in 1..WIDTH {
            for r in 1..WIDTH {
                // NB: FAST_PARTIAL_ROUND_INITIAL_MATRIX is stored in
                // column-major order so that this dot product is cache
                // friendly.
                let t =
                    Self::from_canonical_u64(Self::FAST_PARTIAL_ROUND_INITIAL_MATRIX[c - 1][r - 1]);
                result[c] += state[r] * t;
            }
        }
        result
    }

    /// Computes s*A where s is the state row vector and A is the matrix
    ///
    ///    [ M_00  | v  ]
    ///    [ ------+--- ]
    ///    [ w_hat | Id ]
    ///
    /// M_00 is a scalar, v is 1x(t-1), w_hat is (t-1)x1 and Id is the
    /// (t-1)x(t-1) identity matrix.
    #[inline]
    #[unroll_for_loops]
    fn mds_partial_layer_fast(state: &[Self; WIDTH], r: usize) -> [Self; WIDTH] {
        // Set d = [M_00 | w^] dot [state]

        let s0 = state[0].to_noncanonical_u64() as u128;
        let mut d = Self::from_noncanonical_u128(s0 << Self::MDS_MATRIX_EXPS[0]);
        for i in 1..WIDTH {
            let t = Self::from_canonical_u64(Self::FAST_PARTIAL_ROUND_W_HATS[r][i - 1]);
            d += state[i] * t;
        }

        // result = [d] concat [state[0] * v + state[shift up by 1]]
        let mut result = [Self::ZERO; WIDTH];
        result[0] = d;
        for i in 1..WIDTH {
            let t = Self::from_canonical_u64(Self::FAST_PARTIAL_ROUND_VS[r][i - 1]);
            result[i] = state[0] * t + state[i];
        }
        result
    }

    #[inline]
    #[unroll_for_loops]
    fn constant_layer(state: &mut [Self; WIDTH], round_ctr: usize) {
        for i in 0..WIDTH {
            state[i] += Self::from_canonical_u64(ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr]);
        }
    }

    #[inline]
    fn sbox_monomial(x: Self) -> Self {
        // x |--> x^7
        let x2 = x * x;
        let x4 = x2 * x2;
        let x3 = x * x2;
        x3 * x4
    }

    #[inline]
    #[unroll_for_loops]
    fn sbox_layer(state: &mut [Self; WIDTH]) {
        for i in 0..WIDTH {
            state[i] = Self::sbox_monomial(state[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn full_rounds(state: &mut [Self; WIDTH], round_ctr: &mut usize) {
        for _ in 0..HALF_N_FULL_ROUNDS {
            Self::constant_layer(state, *round_ctr);
            Self::sbox_layer(state);
            *state = Self::mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds_fast(state: &mut [Self; WIDTH], round_ctr: &mut usize) {
        Self::partial_first_constant_layer(state);
        *state = Self::mds_partial_layer_init(state);

        // One less than N_PARTIAL_ROUNDS because we do the last one
        // separately at the end.
        for i in 0..(N_PARTIAL_ROUNDS - 1) {
            state[0] = Self::sbox_monomial(state[0]);
            state[0] += Self::from_canonical_u64(Self::FAST_PARTIAL_ROUND_CONSTANTS[i]);
            *state = Self::mds_partial_layer_fast(state, i);
        }
        state[0] = Self::sbox_monomial(state[0]);
        *state = Self::mds_partial_layer_fast(state, N_PARTIAL_ROUNDS - 1);
        *round_ctr += N_PARTIAL_ROUNDS;
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds(state: &mut [Self; WIDTH], round_ctr: &mut usize) {
        for _ in 0..N_PARTIAL_ROUNDS {
            Self::constant_layer(state, *round_ctr);
            state[0] = Self::sbox_monomial(state[0]);
            *state = Self::mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    fn poseidon(input: [Self; WIDTH]) -> [Self; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        Self::full_rounds(&mut state, &mut round_ctr);
        Self::partial_rounds_fast(&mut state, &mut round_ctr);
        Self::full_rounds(&mut state, &mut round_ctr);
        debug_assert_eq!(round_ctr, N_ROUNDS);

        state
    }

    #[inline]
    fn poseidon_naive(input: [Self; WIDTH]) -> [Self; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        Self::full_rounds(&mut state, &mut round_ctr);
        Self::partial_rounds(&mut state, &mut round_ctr);
        Self::full_rounds(&mut state, &mut round_ctr);
        debug_assert_eq!(round_ctr, N_ROUNDS);

        state
    }
}

#[rustfmt::skip]
impl Poseidon<8> for CrandallField {
    // The MDS matrix we use is the circulant matrix with first row given by the vector
    // [ 2^x for x in MDS_MATRIX_EXPS] = [4, 1, 2, 256, 16, 8, 1, 1]
    //
    // WARNING: If the MDS matrix is changed, then the following
    // constants need to be updated accordingly:
    //  - FAST_PARTIAL_ROUND_CONSTANTS
    //  - FAST_PARTIAL_ROUND_VS
    //  - FAST_PARTIAL_ROUND_W_HATS
    //  - FAST_PARTIAL_ROUND_INITIAL_MATRIX
    const MDS_MATRIX_EXPS: [u64; 8] = [2, 0, 1, 8, 4, 3, 0, 0];

    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; 8]  = [
        0x66bbd30e99d311da, 0x1d6beb91f1441299, 0x1dfb41ac10a5bda8, 0xcbe9eb8f6bfd79fb,
        0x2c943b9a8d9ee4f4, 0x6d70fcb874f05f57, 0xf48e800880a87878, 0x24b1eb418f3994c3,
    ];

    const FAST_PARTIAL_ROUND_CONSTANTS: [u64; N_PARTIAL_ROUNDS - 1]  = [
        0x6d69d39f98b01c69, 0x7694ae5bbd92de89, 0x0b9bfb9fbb252451, 0xf547651a6893f655,
        0x44f4e70e9f77cd03, 0xd8e2801a322a6f39, 0xbd0f7e1bc9649171, 0x2eda14ffc32245e4,
        0x296e04e8222b9265, 0x9aa740fd9cf504ea, 0xe5e868a6d4315bcc, 0x7d430efe75c6ece5,
        0x37ca54f0b49f6214, 0xde83a9f01bfa62d2, 0xb0831b529dbb5b9c, 0xa1d590c3b2b945b5,
        0xa130846268961080, 0x79e6e27330006b7a, 0xba12695bd255613b, 0x0091d7aaf86c0e15,
        0xe9028a7b418aa9f2,
    ];

    const FAST_PARTIAL_ROUND_VS: [[u64; 8 - 1]; N_PARTIAL_ROUNDS] = [
        [0xa22ff49d0671165e, 0x90333ff5780b9eec, 0x919457d220ebe522, 0xd4d8b0a8abc35c6e,
         0x1eca5e0b617850b2, 0x0baa903332edef19, 0x09f1096d496c30f1, ],
        [0x1eb2711afa8d6426, 0x533628d26840e36f, 0xde3f8282ae0806b9, 0x5b96c6c0b7997a68,
         0xec6a242a596b9076, 0x739b857159e03511, 0x1d6bd33258b57d6d, ],
        [0x3603d13c3b01d6c6, 0xc0f959147dc91ee1, 0x7d3b89b472d64db2, 0x49e58d37e29f4c85,
         0xc718800efb8b1033, 0x73b0ca72c31e03fd, 0x9697330fea22a70e, ],
        [0x978123b0febeabd5, 0x8efbca443cb8bfca, 0x9f24c447f2d051bb, 0x45ee906ad9703d20,
         0x9ec6215940d574a8, 0x42b4738de9fed9a9, 0x1a23d296f0228cc0, ],
        [0xbf9c7b22a247969f, 0xf45041b94b8fac96, 0xce23d5f90c9f2a39, 0x64524dbaaef7a56d,
         0xc9b92c71390db85c, 0xd622ef228ef8a6df, 0x697f4e5dc08c2716, ],
        [0x1116bcc5722efae3, 0xfbb5345317c9d731, 0xf696a78eb4b4b9ed, 0xaa08800cfdb59ad3,
         0xf6e9c007ec8900aa, 0xdb35e3b5c9ac6745, 0x4193d903dcca4903, ],
        [0x44b28ca6cb051164, 0xa63264849056f8e7, 0x0cc5dd14b73e4b34, 0xb0c213ed14737e89,
         0xde4ba41535b7cd0f, 0x31de7a36cb4f7f3a, 0x08cc6af0fdcfd306, ],
        [0x97f04080745484ff, 0x40b15e2cd05957bb, 0x27f5279de5eb0729, 0x7b19caab04109b68,
         0xd326745e4bfb87bf, 0xd7e15457e6d7c2d0, 0x4919494236719d7d, ],
        [0x21e9432cc31e919c, 0x837603b01af94b56, 0xe6067467c4e0480a, 0x9dd36e1da07e354e,
         0xe529a3d3f271beb2, 0xa5acad30d3c3fb59, 0x6d5a82823faf4ff1, ],
        [0x88b05b29984e6429, 0x2635decc12d3e510, 0x823fe6152336d209, 0xe7cb76fca3c4fa32,
         0x30443c9279dafcb9, 0x6734acbbe14ba020, 0xcb11ff5d7b9c7a38, ],
        [0x82cdd90e134c2173, 0xff5acf3f7c55d923, 0x63220d8165dcf1a2, 0xe0ce19b67f54977b,
         0x9fbf941303d8b998, 0xf8eda0fc08ccd441, 0x265a0c94ca846c64, ],
        [0x2d43cc534ce9a518, 0x7e78354cb4fdb780, 0x7f33160cefcdb183, 0x33390d373007e718,
         0xe463fb62f8c5e845, 0x454ac64497ad7b5f, 0xa73d26ae995afb8b, ],
        [0x9482c8097651113c, 0x24764722d2be2d59, 0x65b49db52c50cf30, 0x54298f2c474c206f,
         0x13a426fc7958905f, 0xe876f9b953c377f0, 0x43cfbf90c880605f, ],
        [0xdb016a76619ed366, 0x92a1be5140bdb3d3, 0xd399da81f92fb81a, 0xd7a41636505c38bd,
         0x40358caf39317262, 0x8fbc20933dd7d4e9, 0xc957a47b7eaa161b, ],
        [0x11d39c39e8228a7a, 0x071b598d448b6c05, 0x211f8baa562a3196, 0x0cc5b375756650c5,
         0x059feb689a554440, 0x3ee782c2a0c19619, 0x336a46904e832094, ],
        [0x0034133810f09fc5, 0x0021fada430e3c09, 0x010540e301677234, 0x000db9ed138d5010,
         0x000331ce727cd24c, 0x001c07a12edb566a, 0x00098bb5328b02e5, ],
        [0x000016d34ac3c4e3, 0x000005c883b88ffe, 0x00001054167214c0, 0x00002a5a393b8713,
         0x000010d7401feb08, 0x000103f43d476cf3, 0x00000a2b0d83a7a3, ],
        [0x00000102d65ebdaa, 0x0000000663cf5317, 0x00000009aedd7f7a, 0x00000011fad0f867,
         0x00000002a1022998, 0x0000000ceff381ff, 0x00000021cf076bb8, ],
        [0x0000000008679bb3, 0x000000001567aab3, 0x0000000040a61838, 0x0000000101ac6a94,
         0x0000000002ec5178, 0x0000000006b2488e, 0x000000000d4a83cb, ],
        [0x0000000000043801, 0x0000000000089d36, 0x000000000003d5c0, 0x0000000000052021,
         0x00000000000a24e8, 0x0000000000306095, 0x000000000100fed1, ],
        [0x0000000000002026, 0x0000000000010055, 0x0000000000000442, 0x0000000000000255,
         0x0000000000000434, 0x0000000000000269, 0x0000000000000308, ],
        [0x0000000000000001, 0x0000000000000001, 0x0000000000000008, 0x0000000000000010,
         0x0000000000000100, 0x0000000000000002, 0x0000000000000001, ],
    ];

    const FAST_PARTIAL_ROUND_W_HATS: [[u64; 8 - 1]; N_PARTIAL_ROUNDS] = [
        [0xa243d101153eb562, 0x1f670d5d8c14c000, 0xced8026856dd6a07, 0x1b7c4f1704047b8e,
         0x41ea3a3855c2d39e, 0x066101717cef6c02, 0xee96a3b009f99df7, ],
        [0xb00328edb79d53bd, 0x1639f163c71eee14, 0x192788e832e46178, 0x7c68b41f104d68ec,
         0x41174fa1485efe00, 0x618f488d942ec9f2, 0x4bdb7e3318926ebe, ],
        [0x0b28434790c294ef, 0x9ba864afff9e233c, 0x629092442534bdc0, 0x3be9b41110ccdb7d,
         0x9aa3bdeb8d16ac39, 0x17429e3995825d63, 0x702b4284837a4846, ],
        [0xd75842068900c798, 0x8cce741a1060389b, 0xcf91c066d7913b3e, 0xe50be3cbf8a4ace7,
         0x4df095d852da37da, 0x84078b181ce8bbcf, 0x085fc2670b73de72, ],
        [0xd97455be3e610261, 0xceef7dfe63f37fd2, 0x52faf29e9aa95f11, 0xea9672f8f70dde6c,
         0x8c51444bebd1b0ee, 0x49f00564527f62e3, 0x8433762d9a2fcba3, ],
        [0x99eff4ff916d648e, 0x87c459e036359734, 0xd763393ed532616b, 0x0766745aef99a96a,
         0x16b83fbe31e685fb, 0xc2fee2ab39b9e5b3, 0x8a72cfd149535052, ],
        [0x8167512548df88b2, 0xf04b045f4d32a8a1, 0xf673ef6ce25e4806, 0x21c85f4ab3a3b118,
         0xc1d8c8d07113367d, 0x6af20849e27ac1b5, 0xcef5545ab2bdd1b3, ],
        [0xa81eb29c319d5be8, 0x0e0ad7ad11c3a5ab, 0xf770147f2cd4ea74, 0x69f20eddd45794e6,
         0xb174303b7286d9a2, 0x04013980dd7245c7, 0x5dd6a119eba2db57, ],
        [0xc9aff46079c3bea7, 0xe40268bb580ef209, 0x54e2aba7d4f0596f, 0x7e2b0e6fcc266ad5,
         0x0d3daa4a7ead5ad6, 0x84eb64836ab1a2d2, 0x5fc9b471a5a97c6a, ],
        [0xcfcfeaa93d0d1462, 0xaf1f5dd31517b872, 0x7a4c24137a5077e0, 0x55feca2820116ffc,
         0xdf0429c64c459ac5, 0xed5ce47466d0bc8e, 0x8189efb2fe5e0478, ],
        [0x68dc6e8fc5fa43f7, 0xb47798ff890af473, 0xf078d6404130f753, 0xd9213de357e084ef,
         0x51eb0ba5b3814334, 0x7e9885e240657b0c, 0x9de439a115165582, ],
        [0x046b28c7d8e50ccf, 0x616f43798beeddb8, 0x4627e1d7b9b6523c, 0x5a00ee6489a654d1,
         0x239bc16f685c02a3, 0xa3a71de01b6747a9, 0xcf6402218da7a24a, ],
        [0x08ed374a2ec320d1, 0xe5a31270acf40388, 0x13fadd610c48f98e, 0x1e95da99ae3a8c7e,
         0x9dc0de1504c07254, 0x97cd6d98b355eadb, 0x9bc070176cd3c501, ],
        [0xf2fdfc56a1b76de8, 0xfd581b091c3df5a0, 0x7e321d7d5684d10c, 0x5a61834a90f088bc,
         0x2302d1337a2d8cec, 0x725afc4089b3c89f, 0x4ddf24c735a5374d, ],
        [0x4175e1cb9310a009, 0xdddd1e503c04dc25, 0xca4b0ca8bccfe503, 0xe4bbab954b3bd636,
         0x2b05f339af75dbe1, 0x8e9ffe84ceeeccd5, 0x124129078120a6e6, ],
        [0x48bd19a1b761d33d, 0xeb499cf8a6725e0d, 0x32c937fe09983757, 0xda84fd5479d2dec3,
         0xbac726cfd8a8a826, 0x75cc0a190b812741, 0xbe6bbdc6be692697, ],
        [0x0a832806a4ace872, 0xa1e96219bb14acfb, 0x7251019780ae0c31, 0x547b2847470eaf47,
         0x34da63bb720b7bbc, 0xc927a26df327fab7, 0x39a42518adce0d14, ],
        [0xcab13a56ab3a6144, 0x366737df19a4f4db, 0xed08a988f6618092, 0x565b9c84939e7446,
         0xa0e71c6ce10a3fdf, 0xffc45cb3ff5515fa, 0xd373981245598077, ],
        [0x7ac75a736a509479, 0x5ec9215122f1fe35, 0xac8b72753e8924f5, 0xa0c81c2f8db1d81c,
         0x998cdabdb1d3fa4e, 0xe4ca3a836097e99e, 0x39730c86a3bdee63, ],
        [0xd2a127e3d4f1549e, 0xcf5673508860595f, 0xfe5d3387e1dbc076, 0xfe8812fecc245152,
         0xf3e6abb09d715f64, 0xf5f868062e1ebabe, 0xff93de5aff828886, ],
        [0x42735ce753f9bb15, 0x9f7a7533b471f7c6, 0x097d75da8c661359, 0x1ed051078c26b3eb,
         0x40ef3e78b2b13b0d, 0xb0104cc90708254d, 0x97503325810d1378, ],
        [0xf728fe2c1747bad4, 0x415cdecd416e0602, 0x9f9c2e9615e80c24, 0xe291b04af1c1e362,
         0x1e93b91509607640, 0x5ed95be1c52cf97e, 0x359f0220d53d82f4, ],
    ];

    // NB: This is in COLUMN-major order to support cache-friendly pre-multiplication.
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; 8 - 1]; 8 - 1] = [
        [0x3fc702a71c42c8df, 0xdc5d5c2cec372bd8, 0x61e9415bfc0d135a, 0x9b7a25991a49b57f,
         0xaaee943e6eccf7b8, 0x2be97f5416341131, 0x3f3fd62d28872386, ],
        [0xda6cfb436cf6973e, 0x5ed3accc77ae85d0, 0xd63481d84fa12429, 0x38d80c86e3eb1887,
         0xf8ad1187508f709c, 0xd0b8c098bdcf7407, 0x2be97f5416341131, ],
        [0x800fc4e2c9f585d8, 0xc768961eecdcb554, 0xc8e4a9f96ab57c10, 0xeae1feb52d6eb09a,
         0x7ffbbc7ce8823d72, 0xf8ad1187508f709c, 0xaaee943e6eccf7b8, ],
        [0x3864e0e53027baf7, 0x95af3551b40289ce, 0x29d0d07fd9b6e9ea, 0xda20f5c812c60b4e,
         0xeae1feb52d6eb09a, 0x38d80c86e3eb1887, 0x9b7a25991a49b57f, ],
        [0x44ae739518db1d10, 0xa3ae8c5444f37d9a, 0xa5aac4ccc8b791cc, 0x29d0d07fd9b6e9ea,
         0xc8e4a9f96ab57c10, 0xd63481d84fa12429, 0x61e9415bfc0d135a, ],
        [0x1d46b66c2ad3ef0c, 0x53c070eae0ad0c38, 0xa3ae8c5444f37d9a, 0x95af3551b40289ce,
         0xc768961eecdcb554, 0x5ed3accc77ae85d0, 0xdc5d5c2cec372bd8, ],
        [0xbc75b7bb6f92fb6b, 0x1d46b66c2ad3ef0c, 0x44ae739518db1d10, 0x3864e0e53027baf7,
         0x800fc4e2c9f585d8, 0xda6cfb436cf6973e, 0x3fc702a71c42c8df, ],
    ];

    #[cfg(target_feature="avx2")]
    #[inline]
    #[unroll_for_loops]
    fn mds_layer(state_: &[CrandallField; 8]) -> [CrandallField; 8] {
        let in_state = [
            state_[0].to_noncanonical_u64(),
            state_[1].to_noncanonical_u64(),
            state_[2].to_noncanonical_u64(),
            state_[3].to_noncanonical_u64(),
            state_[4].to_noncanonical_u64(),
            state_[5].to_noncanonical_u64(),
            state_[6].to_noncanonical_u64(),
            state_[7].to_noncanonical_u64(),
        ];
        let out_state = crate::hash::poseidon_avx2::crandall_poseidon8_mds_avx2(in_state);
        [
            Self::from_canonical_u64(out_state[0]),
            Self::from_canonical_u64(out_state[1]),
            Self::from_canonical_u64(out_state[2]),
            Self::from_canonical_u64(out_state[3]),
            Self::from_canonical_u64(out_state[4]),
            Self::from_canonical_u64(out_state[5]),
            Self::from_canonical_u64(out_state[6]),
            Self::from_canonical_u64(out_state[7]),
        ]
    }
}

#[rustfmt::skip]
impl Poseidon<12> for CrandallField {
    // The MDS matrix we use is the circulant matrix with first row given by the vector
    // [ 2^x for x in MDS_MATRIX_EXPS] = [1024, 8192, 4, 1, 16, 2, 256, 128, 32768, 32, 1, 1]
    //
    // WARNING: If the MDS matrix is changed, then the following
    // constants need to be updated accordingly:
    //  - FAST_PARTIAL_ROUND_CONSTANTS
    //  - FAST_PARTIAL_ROUND_VS
    //  - FAST_PARTIAL_ROUND_W_HATS
    //  - FAST_PARTIAL_ROUND_INITIAL_MATRIX
    const MDS_MATRIX_EXPS: [u64; 12] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];

    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; 12]  = [
        0x3cc3f89232e3b0c8, 0x62fbbf978e28f47d, 0x39fdb188ec8547ef, 0x39df2d6d45a69859,
        0x8f0728b06d02b8ef, 0xaef06dc095c5e82a, 0xbca538714a7b9590, 0xbac7d7e5a0dd105c,
        0x6b92ff930094a160, 0xdaf229f00331101e, 0xd39b0be8a5c868c6, 0x47b0452c32f4fddb,
    ];


    const FAST_PARTIAL_ROUND_CONSTANTS: [u64; N_PARTIAL_ROUNDS - 1]  = [
        0xa00e150786abac6c, 0xe71901e012a81740, 0x8c4517d65a4d4813, 0x62b1661b06dafd6b,
        0x25b991b65a886452, 0x51bcd73c6aaabd6e, 0xb8956d71320d9266, 0x62e603408b7b7092,
        0x9839210869008dc0, 0xc6b3ebc672dd2b86, 0x816bd6d0838e9e05, 0x0e80e96e5f3cc3fd,
        0x4c8ea37c218378c9, 0x21a24a8087e0e306, 0x30c877124f60bdfa, 0x8e92578bf67f43f3,
        0x79089cd2893d3cfa, 0x4a2da1f7351fe5b1, 0x7941de449fea07f0, 0x9f9fe970f90fe0b9,
        0x8aff5500f81c1181,
    ];

    const FAST_PARTIAL_ROUND_VS: [[u64; 12 - 1]; N_PARTIAL_ROUNDS] = [
        [0xe67f4c76dd37e266, 0x3787d63a462ddaba, 0x6a541a0fad3032c7, 0xff665c7a10448d53,
         0xd1cdb53d9ddb8a88, 0x36b8c12048426352, 0x4e9a00b9a8972548, 0xa371c3fc71ddba26,
         0xf42eacd3b91465b5, 0x13bbf44566e89fdd, 0x17d35dfc4057799b, ],
        [0x74d80822f5ac105b, 0xd236707412f3a047, 0xc1b3828a69443f42, 0xe92487f111b47bd4,
         0x8b544fcd845e00f6, 0xe6ae4706f80dbf42, 0x47f1b8a0545fe1fa, 0xde2ddf83cf7b9217,
         0x1b9fe67073a9d147, 0x2658f0e2dd45c018, 0x7ebd50cedd2631da, ],
        [0x4bc36dcb20e574a3, 0xabda0ed71b34deb0, 0x3005b75fa2cc2425, 0xf3e90f0501cc6f0f,
         0xefc00ccd7b68da02, 0x42c105686461b611, 0x9bd4213d99925ac2, 0xa4994f529e2a94c4,
         0xb46ef4cd4db7cfc2, 0x175044110fde562f, 0x6a8ae415ec65007a, ],
        [0x7e682d3a5ef73e41, 0xcf32352159d13a33, 0x49f474977e36f6c3, 0x7bb0effe3bd426ea,
         0x64eed711604ee775, 0x0b524f42edaf84fb, 0xdfd97a4aa5d8567d, 0x5fe9c9824d43521d,
         0xaf61e76b9cdbb138, 0xc01b70f1adebfeab, 0x95d24d00678da148, ],
        [0x3549287475671e52, 0x9ca854efc14122dd, 0xcd886b543c9beb77, 0xa409843ee3ce4f6a,
         0x9f1bea833646efa2, 0xbfe3c09f70220e1e, 0xe0b6a8f93e036acf, 0x554733da74d2c9da,
         0xeb510c6f857aa212, 0x53626d71ca4a38dd, 0xb6ae627bfc11f637, ],
        [0xce18b963c797243d, 0x51eb1f1ce97f2a80, 0x104cc3f8c10457b9, 0x12d3c8cee6ec5c16,
         0xd43e1f577234fb55, 0x54c8c76901c7524c, 0x960af4ea5ef01c1c, 0xef6e7bc29cc45dd1,
         0x3a5987955b6574a4, 0x1dc302592713e124, 0xeea7c20882911833, ],
        [0xd9c21ebfb1c2ae8f, 0x0b4b6b7afcc68799, 0xdbe081d54b0cadfc, 0x961c7b785812f275,
         0xbeaa33b9cd98553a, 0x0aeae6ff5dd491c1, 0x15eec8aebadf9834, 0x16ca6296360389fe,
         0x008bb53e94c1041a, 0x368bf0dae439b072, 0x51ff6c0c07d56ac4, ],
        [0x1cb8fba2362a103f, 0x897b392d5912b66d, 0x7fa38fe8471e4ebf, 0x4ffa98336474e161,
         0xadf92c983e466ee6, 0x43b22e3794bdd8b8, 0xe7fd4b4c2e3c8713, 0xe4f8b07872deed65,
         0x9e152c9cb7e0b7c4, 0x1b26081e35432ccc, 0x647acdb0f39e597a, ],
        [0xf31c02888392b995, 0x0207c944c27fe9f8, 0x62767aea825841d2, 0x6ca016ce1667e093,
         0xd4aa4062188ca548, 0x80ad041f7bc66390, 0x7e8b2bdf628bc084, 0x0edcf7a59d112492,
         0xe26437b6e13326cb, 0x78f2c6f4b9257f3a, 0x3d31ecb8b17cfa69, ],
        [0x452046f066aaa834, 0x1ee5a5891493eb3f, 0x72a59ce75aad55e1, 0x086b6f5ddbe5d4ea,
         0x72964667982c1e80, 0x4edabf2f250d80d6, 0x9d34853dc92eff2b, 0xba0bf1d6dfd4a83c,
         0xd8257069ba15d122, 0x344f8bbc786dd0c3, 0xa68e988d58740429, ],
        [0x82e4d8c6dc1ae6ed, 0xe0957181ddfef5de, 0x592e8187280bf64d, 0x5b41e7d00fb09752,
         0x8feddb14c160201e, 0x1c9ad02b3d10f701, 0x16f5a869b59b6c31, 0x4c3d6f04136d7771,
         0xc7727996396e15ea, 0x97e39df842444fbd, 0xbdde9f7586a874df, ],
        [0x9512d3c4d7cb437c, 0x6c45b0d267f28b4c, 0x4c0f2ca87c29175f, 0xa51335204643a8f8,
         0x500c3ad025688091, 0x0354b59cd97eb531, 0xf7776cf7c6e35c1b, 0xbd4438971095dba5,
         0xfc2be1c80ac8bcc9, 0x760db2349cbda06b, 0xd89a987e88d41186, ],
        [0x4f6a3f5ee2763bb2, 0x03297a357f2da20c, 0x76c05507038c84aa, 0x1a5043d142781537,
         0x397542d78dadb3a1, 0x887dd81d3c3f27d0, 0xe5d2879bf760629c, 0xf9211873dbe5e068,
         0x9d2d37dff8301264, 0x68c59f77a6dbe6ed, 0x077543cffe95edfc, ],
        [0xadd787768284cdee, 0x82585abf32a3020d, 0xfe20edcb9f6a2cea, 0x844cbf79ffef7d45,
         0xa62bf3ca3eb80b1c, 0x4dfbcd2cd29117f4, 0xf1d1028bc0c8839c, 0x62a0e817e8d77ef5,
         0xb5eb84c0789a93ed, 0xcf41f39f2e2fd6d1, 0x9e57aadb4c8dcfc2, ],
        [0xd772005559fcdfaa, 0x66c9a95222385666, 0x410f26abdd94c446, 0xec36cb430f46924e,
         0x575482bd3706c282, 0x9ead1e1880d6f587, 0xe45eebbac54ebaad, 0xb4acdc141bc29117,
         0xce305bf5696d5c6f, 0xf0ed1597cf810813, 0x0c9eaf677e2a6d2e, ],
        [0xcb1519b8f35e7515, 0xd7cb72656790acd0, 0x3d3c4972cfcb4cf7, 0xaac6c7c54cefb31d,
         0xf61b30c24c112777, 0x6129996980a9a26f, 0xf405b608d78fdd10, 0xfc411ea75de454df,
         0x808a5dcf02559826, 0xee69df55c1fb93e8, 0x2e97449d2e7f4bef, ],
        [0xc646d3807e3f63f8, 0x8b75f8ab8a670c0e, 0xa3463ae487b2eff1, 0xe9cbfbd0f1032068,
         0x9775e58aeb04e069, 0x06cb23d6d06603f9, 0x0474bc743bd2a597, 0xc709561ece9d291b,
         0x718100080c964a41, 0x3a5beca6171c74be, 0x2feed444497af7eb, ],
        [0x617c452b85c9d0a2, 0x9e97e4d7eae91a20, 0x83beea96a57ed657, 0x07f068abd6193935,
         0xa9a10751aab874d9, 0x1a2e6bfa534064c9, 0xdd1802545bf7a4b2, 0x8e3e06e8a89b8a7f,
         0xf6627102ecaf8f7e, 0x4ebfbf20512cf09a, 0xabbe52e572d5bf4a, ],
        [0x01653b4f4a999932, 0x0053f2a963638e1a, 0x001922cbf2c59efc, 0x00015fc3f40ff355,
         0x003531822ee190e8, 0x06612a21c3a9cafe, 0x012e62120d30bbf0, 0x0039ded9f9a7df37,
         0x000bc8d6c5739e4a, 0x001000e0be5d2a9e, 0x0c018651e998d5b8, ],
        [0x00000063775cfe99, 0x000006c0c4b6e7e4, 0x000001090a1416ee, 0x0000001a438450db,
         0x000000036280cbdf, 0x0000000ffea8b49d, 0x00001801427a72e3, 0x0000023059280d1b,
         0x000000e4e2f6fbee, 0x00000029ebd5c20c, 0x0000001e61472f75, ],
        [0x0000000000015900, 0x00000000000c2505, 0x0000000020008642, 0x0000000002200945,
         0x0000000000430070, 0x0000000000058581, 0x0000000000240b08, 0x000000004000a214,
         0x0000000000814424, 0x00000000050050a2, 0x000000000083040a, ],
        [0x0000000000000001, 0x0000000000000001, 0x0000000000000020, 0x0000000000008000,
         0x0000000000000080, 0x0000000000000100, 0x0000000000000002, 0x0000000000000010,
         0x0000000000000001, 0x0000000000000004, 0x0000000000002000, ],
    ];

    const FAST_PARTIAL_ROUND_W_HATS: [[u64; 12 - 1]; N_PARTIAL_ROUNDS] = [
        [0xf8c08a4101e2a5e4, 0x1d59fd32df7c1369, 0x22c9f355ee2603e9, 0x088f5c6c47afac6f,
         0xea0a086f009303c0, 0x2a04f88abd6341a3, 0x4893220de1d91824, 0xf153c2a717c08a1f,
         0x84f81d7b79459079, 0x6fb4ffed9b78d9f0, 0x1eaafffe5e1becf6, ],
        [0x0a98f6ce528a5af6, 0x235bae28135c7475, 0x7ace29ef814a2255, 0x6030aeaac50421f4,
         0x7987fd365fbf2539, 0x0f79e921a3239a77, 0xb11997d5f12b36a3, 0x984368cd38362bbf,
         0xa14e59e13570c297, 0x83a0cda0d47fadfa, 0x1dcfd6ba0e54133c, ],
        [0x1d8f384f837f49e2, 0xf8cfde4f45967d4c, 0xc1fee8f19fe21c43, 0x04363b9307aebeea,
         0x841cea2f6247b41a, 0xefad3917abc7a53c, 0x0f6d8258511ac0e6, 0x77c86f3704bbfe57,
         0x6c1b85ac9ef87dbf, 0x2b0ee517bdd38773, 0xd274576d9d7952c1, ],
        [0x5dd7aebeedd0eacb, 0xe7abcd4b0857dddc, 0x29f1a2e1a32ec8d5, 0x1181eed8c3a8e08b,
         0xcba331414a192658, 0xa47ccc727964ddbb, 0x8414892c9096aaef, 0x596b12214645218d,
         0xf41f19984365e6c3, 0x4719f61fdebf31a4, 0x9075d2ad73964a38, ],
        [0x09df8b108094522a, 0x1aca572b4c76988a, 0xd31c8fc7fd51eccf, 0xfeceefdcdc38770d,
         0x1d1b235a0eb031f4, 0x971bbc1112c36b29, 0x8c021c051da48779, 0xe89ec828cfbdd96f,
         0xe72956d332e2dc52, 0xc0b14ea64ab04ee5, 0x53233fda2a3c29ad, ],
        [0xb8a98dff72a17a51, 0x3a7860f384f03806, 0x1e58886bef1446c6, 0xc7910598dad5a1f4,
         0xae0642adc54989b0, 0xf4d768f139f5f4f9, 0xbff59ba7765b3e6b, 0x91b2d8424617ef7b,
         0x6fecea5e1ea32471, 0xe26667436d718c56, 0x581b8f91d7d7c6e4, ],
        [0xd8d9cc4462e55b75, 0x7a707e9faf86c8de, 0x3c1afbb7083058ce, 0x1274f5e1aaf581c1,
         0x274bb4597bd29568, 0x0c1ed5200aa0ca93, 0x5d73e0a4ee921248, 0xd8e88f02d831f72e,
         0x0920a407b6fc1d2f, 0x423dde535b3c0f86, 0x9046fb30c35098ef, ],
        [0x910871bda1a4dc66, 0x06ff1f4e195e1916, 0xaaeee5346ab403dd, 0x0e10c7d3172cc6ae,
         0x04999dd075d58fa9, 0x3da251b3ee6bf0e5, 0x9184e34946712416, 0x473fbaf135f61868,
         0xbbe66160875bc6fe, 0x4ad958365708aad9, 0xef9287c594553868, ],
        [0x10cb59cb3613bb08, 0x96e3ca98eb380cf9, 0x3153cc874088d97a, 0xc8c9d31008862ae9,
         0x29b662d09e3ce873, 0xfd25aa286a33c577, 0xe5cd6822fea38b6b, 0x49cb042f7e30d9ef,
         0x5c14b08062acf75c, 0xebeb59c698831c5d, 0xc51a7bfddcd53406, ],
        [0xf5b5132c50230980, 0xee13fdc497fc7ff7, 0x7aaaf371f4027bbe, 0xefbf9646d3eab1d4,
         0x0192b0c878f88990, 0x33a13ab409a95afb, 0xca3147bb5652e935, 0x1b6e0d178d166ea1,
         0x983a5eb800745372, 0xadc3b9f092da6ee9, 0xd53d2d9ae9b0b8b7, ],
        [0x4c823667ece9492b, 0x9515e5811fcf086f, 0xac71ccac616dbf01, 0x1818c85ae69d9610,
         0x2b97efe5cd0a9f61, 0x49d1a2ec7c1d8a9e, 0x215787a8272ef1c3, 0x7ebde6076499a32a,
         0xc1b81122cb7b43f6, 0x6fb37a243559d827, 0x970cd9b0339d2d05, ],
        [0x6aaa2e6a8c31c207, 0x26c0676a25426ea6, 0x5edda44cc885f665, 0x8e8b97c979ad532a,
         0xb9d9bf57b3eeafd0, 0x5656c6bb02989fd1, 0x70313b79197821d4, 0x7fe33766f7226b1f,
         0x7499a04a6b030f6b, 0x4c69391a8ed5c0f2, 0x4b4e96c68d1eb19d, ],
        [0xff199cee489a97d1, 0xca544f9410e9ea31, 0xb819ecc35beab037, 0xe746955c01f58adc,
         0x3d1812758140549f, 0x348e03c3750cff4a, 0xc648b624683bb31f, 0xae4ab9656117e784,
         0x8c02225fe885b95f, 0xf07f35e38b527e04, 0x97f2475a77d1fa3b, ],
        [0xd7a767d6f78d263c, 0xf01f29bc13f8d52e, 0x86df93d4be47e46e, 0xc7f42508cef87d3a,
         0xd68b87bb951a1eee, 0xbbf7aa5ea42f1936, 0x5cbd3e1051cdccbc, 0x917fd26537f1cb47,
         0xe872defb4073d680, 0x7a23790b9c2fcf11, 0x57372f64f1ba571e, ],
        [0x1141ce95e4f36268, 0x07dc03c4438b93ff, 0x1923d97cc980b788, 0x79e776a98bc81418,
         0x39ed107b4fc226ae, 0xfc49245486022c81, 0x581a344b413f1491, 0x36d13d5bd609823c,
         0x61c51cf0a912bdf3, 0x3e035096932c0675, 0xedfef9ed5176bcfa, ],
        [0xac20ba71ed5fffe1, 0x7ccf77683dd3c134, 0x35b660c9248693f6, 0xe3c3db8cd17abf36,
         0xe145283d080d4b94, 0xd6fdb1a4a101f81f, 0xa8b316f332519218, 0x63e25815404423d1,
         0x5099cd7de648979d, 0xae2a5fc0f336bb2e, 0x78624fe97e6727d7, ],
        [0xd50ec4091bee8eda, 0xa83d33121e0b98c6, 0x169f674d12527a05, 0xdda18a72ef29b26b,
         0x0001e1849d2ec83f, 0x06a4bdef8093bdeb, 0x02e55a872c5c16fd, 0xbc07fd6489c5e5ec,
         0xa9bf440c06ed9ad9, 0xa50a7c091a869b12, 0x52387f502106d171, ],
        [0xd88a917ecaec9164, 0x91cbd172c1c60db4, 0x089901176d11cbbe, 0xd9aa7a4e25d85fd2,
         0x76c8de23f4e46584, 0xdb58d95b54563760, 0x1ac4ec96160b0b5d, 0x47a18a07a663bd37,
         0xe1a0c0e1f1ad360c, 0xee9efd9bb2ff331b, 0x332516435912bb4e, ],
        [0x3ee7f239b3f72cb8, 0x8dd9a15c6b2cdf2c, 0xf34be27eb6089094, 0x2f316b9dfe26c6a2,
         0x6ef0a376d699d966, 0x6416ebfa513b7048, 0xa3a8b269c35bc569, 0xf9bd882d51a186f9,
         0x04016d660c8e9a04, 0x94a8d01bf1185c32, 0xd5dd630701e8e2f1, ],
        [0x2cb013d7fdda0dd1, 0x95aa522094977e0b, 0x40e3490b6d03abe2, 0x19c3390a981c8563,
         0x6178af85fdd6d8e2, 0xefede56f5ba88274, 0xe7fd4de4966ffcab, 0x8759e5befc06ecf9,
         0x933864bbe83a02b2, 0xd5c2f21adaf0fc0e, 0x10c0e6410a3a632a, ],
        [0x7648769e7d9a5a37, 0x14256df209909079, 0x46ffa1ea96331c95, 0xbdf534c6f8372297,
         0x45fd78f68986f2f5, 0xd960926124b727ae, 0x8139aca5f725e73f, 0xd3f23433928e0c54,
         0xa221614eb4379297, 0xe445f5b133e491f8, 0x7694bcd4a0245609, ],
        [0x6aaab9a9e8117836, 0x40a1c716c884730b, 0xd81303b2c9d46838, 0x346c1ba0cdc21317,
         0x726821a9c9aa0db6, 0x7db3ed5312178744, 0x0ce23bf6f9eed082, 0xb9e01dfc6bb98a90,
         0x2e97f1cb8689f623, 0xa2a9961db0d614d8, 0xf87c2101134b253c, ],
    ];

    // NB: This is in COLUMN-major order to support cache-friendly pre-multiplication.
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; 12 - 1]; 12 - 1] = [
        [0x8a041eb885fb24f5, 0xeb159cc540fb5e78, 0xf2bc5f8a1eb47c5f, 0x029914d117a17af3,
         0xf2bfc6a0100f3c6d, 0x2d506a5bb5b7480c, 0xda5e708c57dfe9f9, 0xd7b5feb73cd3a335,
         0xeab0a50ac0fa5244, 0xad929b347785656d, 0xa344593dadcaf3de, ],
        [0xb5d5efb12203ef9a, 0xe2afdb22f2e0801a, 0xac34f93c00842bef, 0xb908389bbeee3c9d,
         0xf88cbe5484d71f29, 0x3e815f5ac59316cf, 0xaa5a5bcedc8ce58c, 0x2f1dcb0b29bcce64,
         0x22f96387ab3046d8, 0x87b1d6bd50b96399, 0xad929b347785656d, ],
        [0xe7ad8152c5d50bed, 0x2b68f22b6b414b24, 0x397f9c162cea9170, 0x33ab6239c8b237f3,
         0x110365276c97b11f, 0x68bc309864072be8, 0xc0e1cb8013a75747, 0x10a1b57ff824d1f1,
         0x45a7029a0a30d66f, 0x22f96387ab3046d8, 0xeab0a50ac0fa5244, ],
        [0x685ef5a6b9e241d3, 0x9e085548eeb422b1, 0x3fd7af7763f724a2, 0x337d2955b1c463ae,
         0x1927309728b02b2c, 0x56a37505b7b907a7, 0xfd7c623553723df6, 0x35b7afed907102a9,
         0x10a1b57ff824d1f1, 0x2f1dcb0b29bcce64, 0xd7b5feb73cd3a335, ],
        [0x7e60116e98d5e20c, 0xb561d1111e413cce, 0xeb3a58c29ba7f596, 0xb2ec0e3facb37f22,
         0x95dc8a5e61463c07, 0xecd49c2975d75106, 0x7b298b0fc4b796ab, 0xfd7c623553723df6,
         0xc0e1cb8013a75747, 0xaa5a5bcedc8ce58c, 0xda5e708c57dfe9f9, ],
        [0xfdb6ca0a6d5cc865, 0xe351dee9a4f90434, 0x9fedefd5f6653e80, 0xb7f57d2afbb79622,
         0xa84c2200dfb57d3e, 0x0cdf9734cbbc0e07, 0xecd49c2975d75106, 0x56a37505b7b907a7,
         0x68bc309864072be8, 0x3e815f5ac59316cf, 0x2d506a5bb5b7480c, ],
        [0x857f31827fb3fe60, 0x6aa96c0125bddef7, 0xe629261862a9a8e1, 0xf285b4aa369079a1,
         0x01838a8c1d92d250, 0xa84c2200dfb57d3e, 0x95dc8a5e61463c07, 0x1927309728b02b2c,
         0x110365276c97b11f, 0xf88cbe5484d71f29, 0xf2bfc6a0100f3c6d, ],
        [0xe31988229a5fcb6e, 0x06f4e7db60b9d3b3, 0x4e093de640582a4f, 0xa3a167ee9469e711,
         0xf285b4aa369079a1, 0xb7f57d2afbb79622, 0xb2ec0e3facb37f22, 0x337d2955b1c463ae,
         0x33ab6239c8b237f3, 0xb908389bbeee3c9d, 0x029914d117a17af3, ],
        [0xc06fefcd7cea8405, 0xa7b2498836972dc4, 0x4e3662cf34ca1a70, 0x4e093de640582a4f,
         0xe629261862a9a8e1, 0x9fedefd5f6653e80, 0xeb3a58c29ba7f596, 0x3fd7af7763f724a2,
         0x397f9c162cea9170, 0xac34f93c00842bef, 0xf2bc5f8a1eb47c5f, ],
        [0x05adcaa7427c172c, 0xdc59b1fe6c753a07, 0xa7b2498836972dc4, 0x06f4e7db60b9d3b3,
         0x6aa96c0125bddef7, 0xe351dee9a4f90434, 0xb561d1111e413cce, 0x9e085548eeb422b1,
         0x2b68f22b6b414b24, 0xe2afdb22f2e0801a, 0xeb159cc540fb5e78, ],
        [0x4ff536f518f675c7, 0x05adcaa7427c172c, 0xc06fefcd7cea8405, 0xe31988229a5fcb6e,
         0x857f31827fb3fe60, 0xfdb6ca0a6d5cc865, 0x7e60116e98d5e20c, 0x685ef5a6b9e241d3,
         0xe7ad8152c5d50bed, 0xb5d5efb12203ef9a, 0x8a041eb885fb24f5, ],
    ];

    #[cfg(target_feature="avx2")]
    #[inline]
    #[unroll_for_loops]
    fn mds_layer(state_: &[CrandallField; 12]) -> [CrandallField; 12] {
        let in_state = [
            state_[0].to_noncanonical_u64(),
            state_[1].to_noncanonical_u64(),
            state_[2].to_noncanonical_u64(),
            state_[3].to_noncanonical_u64(),
            state_[4].to_noncanonical_u64(),
            state_[5].to_noncanonical_u64(),
            state_[6].to_noncanonical_u64(),
            state_[7].to_noncanonical_u64(),
            state_[8].to_noncanonical_u64(),
            state_[9].to_noncanonical_u64(),
            state_[10].to_noncanonical_u64(),
            state_[11].to_noncanonical_u64(),
        ];
        let out_state = crate::hash::poseidon_avx2::crandall_poseidon12_mds_avx2(in_state);
        [
            Self::from_canonical_u64(out_state[0]),
            Self::from_canonical_u64(out_state[1]),
            Self::from_canonical_u64(out_state[2]),
            Self::from_canonical_u64(out_state[3]),
            Self::from_canonical_u64(out_state[4]),
            Self::from_canonical_u64(out_state[5]),
            Self::from_canonical_u64(out_state[6]),
            Self::from_canonical_u64(out_state[7]),
            Self::from_canonical_u64(out_state[8]),
            Self::from_canonical_u64(out_state[9]),
            Self::from_canonical_u64(out_state[10]),
            Self::from_canonical_u64(out_state[11]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField as F;
    use crate::field::field_types::Field;
    use crate::hash::poseidon::Poseidon;

    fn check_test_vectors<const WIDTH: usize>(test_vectors: Vec<([u64; WIDTH], [u64; WIDTH])>)
    where
        F: Poseidon<WIDTH>,
        [(); WIDTH - 1]: ,
    {
        for (input_, expected_output_) in test_vectors.into_iter() {
            let mut input = [F::ZERO; WIDTH];
            for i in 0..WIDTH {
                input[i] = F::from_canonical_u64(input_[i]);
            }
            let output = F::poseidon(input);
            for i in 0..WIDTH {
                let ex_output = F::from_canonical_u64(expected_output_[i]);
                assert_eq!(output[i], ex_output);
            }
        }
    }

    #[test]
    fn test_vectors() {
        // Test inputs are:
        // 1. all zeros
        // 2. range 0..WIDTH
        // 3. random elements of CrandallField.
        // expected output calculated with (modified) hadeshash reference implementation.

        #[rustfmt::skip]
        let test_vectors8: Vec<([u64; 8], [u64; 8])> = vec![
            ([0, 0, 0, 0, 0, 0, 0, 0, ],
             [0x0751cebf68b361b0, 0x35d3c97c66539351, 0xd8658ef4a6240e92, 0x6781ebb9bbbb4e9f,
              0x274e5747ffc945ab, 0xf145287440599e51, 0xb193e521a83175a1, 0xcc133eb594e53a80, ]),
            ([0, 1, 2, 3, 4, 5, 6, 7, ],
             [0x1183fb3b5cbb3c6c, 0xa4ac49f197402036, 0xd752a2f6b9f1e6a2, 0x508da1afbebd9538,
              0xd32e183335ea3b8a, 0x79eb2ab985665a18, 0xa6a43cefcee4bfc2, 0x50521374c3cf82e1, ]),
            ([0xb69ed321abbeffbb, 0xfb496d8c39b64e42, 0x274f1cfbb925c789, 0x9e846d2b9a56b834,
              0xc7f297c0d48bc3b6, 0xb859ab1e45850a0a, 0x3244fe3bcb1244cb, 0xb98e1cfa647575de, ],
             [0xa7369ab44b1aadd2, 0x884abb3db138372d, 0x9fc2e4ee64df8608, 0x12a205150a1dbe5a,
              0x934ab794bd534b3c, 0xb39ef937e8caa038, 0x9e5fe73f4b03983c, 0x9539e39e93c28978, ]),
        ];

        check_test_vectors::<8>(test_vectors8);

        #[rustfmt::skip]
        let test_vectors12: Vec<([u64; 12], [u64; 12])> = vec![
            ([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ],
             [0x3e7b141d38447d8e, 0x66c245618877844a, 0xb8e1c45f458b0f13, 0x2f1d4710145a8698,
              0x7af9686a09b78693, 0xc0e5b9a1c728d4ea, 0x25a8a20844491890, 0x8e9d1b1b58ae2019,
              0x593286e9cfdd9e55, 0x131ac26134caca32, 0xc1c6e880dc77f0a6, 0x94db15af6ad9527b, ]),
            ([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, ],
             [0x8ca83bb7e510aff5, 0x68a7a9441166cc2c, 0xa1ba50df7e5d9f68, 0xbd14765ff1725536,
              0xcea83c5e2680f3da, 0xa7782c56559f6d32, 0x03d5cb8d13adf174, 0x298de89026c219a6,
              0x481f50c421e19bf7, 0x3ea5672a17888b27, 0x2f223e603dd1cd7e, 0x05826e3e65f9d4e7, ]),
            ([0xb69ed321abbeffbb, 0xfb496d8c39b64e42, 0x274f1cfbb925c789, 0x9e846d2b9a56b834,
              0xc7f297c0d48bc3b6, 0xb859ab1e45850a0a, 0x3244fe3bcb1244cb, 0xb98e1cfa647575de,
              0x3c9ed8013b0b366b, 0x6a242cb943c91b16, 0x404794ad562239f1, 0x209363e20945adf6, ],
             [0x402cd8c7a11a682a, 0xc25b92012a2ad940, 0x64a26e5d349a800d, 0x78fcf2d5fe54bd74,
              0x0724f91d1abd3154, 0xb1fa8e7a8853fe41, 0x0b82a2b53fa007f0, 0x226f2dbe1bae032f,
              0x8c86ef4f325ff4ce, 0xce2fe2273aed3f7a, 0x3f67b6b298ae64a6, 0xaaf13b4630e53e41, ]),
        ];

        check_test_vectors(test_vectors12);
    }

    fn check_consistency<const WIDTH: usize>()
    where
        F: Poseidon<WIDTH>,
        [(); WIDTH - 1]: ,
    {
        let mut input = [F::ZERO; WIDTH];
        for i in 0..WIDTH {
            input[i] = F::from_canonical_u64(i as u64);
        }
        let output = F::poseidon(input);
        let output_naive = F::poseidon_naive(input);
        for i in 0..WIDTH {
            assert_eq!(output[i], output_naive[i]);
        }
    }

    #[test]
    fn consistency() {
        check_consistency::<8>();
        check_consistency::<12>();
    }
}
