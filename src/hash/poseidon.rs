//! Implementation of the Poseidon hash function, as described in
//! https://eprint.iacr.org/2019/458.pdf

use crate::field::field_types::Field;

// The number of full rounds and partial rounds is given by the
// calc_round_numbers.py script. They happen to be the same for both
// widths with s-box x^7.
const HALF_N_FULL_ROUNDS: usize = 4;
const N_FULL_ROUNDS_TOTAL: usize = 2 * HALF_N_FULL_ROUNDS;
const N_PARTIAL_ROUNDS: usize = 22;
const N_ROUNDS: usize = N_FULL_ROUNDS_TOTAL + N_PARTIAL_ROUNDS;
const MAX_WIDTH: usize = 12;

// The round constants are the same as for GMiMC (hash.rs):
// generated from ChaCha8 with a seed of 0. In this case we need
// to generate more though. We include enough for a WIDTH of 12;
// smaller widths just use a subset.
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

mod poseidon_width8 {
    use unroll::unroll_for_loops;
    use crate::field::field_types::Field;
    use std::mem::MaybeUninit;

    use crate::hash::poseidon::{
        ALL_ROUND_CONSTANTS, N_ROUNDS, N_PARTIAL_ROUNDS, HALF_N_FULL_ROUNDS
    };

    const WIDTH: usize = 8;
    const N_ROUND_CONSTANTS: usize = WIDTH * N_ROUNDS;

    // The MDS matrix we use is the circulant matrix with first row given by the vector
    // [ 2^x for x in MDS_MATRIX_EXPS] = [4, 1, 2, 256, 16, 8, 1, 1]
    //
    // WARNING: If the MDS matrix is changed, then the following
    // constants need to be updated accordingly:
    //  - FAST_PARTIAL_ROUND_CONSTANTS
    //  - FAST_PARTIAL_ROUND_VS
    //  - FAST_PARTIAL_ROUND_W_HATS
    //  - FAST_PARTIAL_ROUND_INITIAL_MATRIX
    const MDS_MATRIX_EXPS: [u64; WIDTH] = [2, 0, 1, 8, 4, 3, 0, 0];

    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; WIDTH]  = [
        0x66bbd30e99d311da, 0x922cdd920d5ac419, 0x5cab27a0c15b168b, 0xa733ad3667055dc6,
        0x1a746ebd8adb885b, 0xfec96302d0793c32, 0x30d4dcccf965bece, 0x5505d4978a83ad06,
    ];

    const FAST_PARTIAL_ROUND_CONSTANTS: [u64; N_PARTIAL_ROUNDS - 1]  = [
        0xa9fca60179a34b27, 0x910f523e761d5cf7, 0xf4381501631c51c3, 0xc0807cefd7f480a7,
        0xdd16bf949cade59c, 0x3fab1a2e78b09325, 0xaa7f685d961479b3, 0x53f3d661745e253f,
        0x2213f04ad2f65b70, 0xe2ed250f09674faf, 0x776f2b2c2c3e3360, 0x7f941321c1eab8e9,
        0x472f7ff111dae518, 0xa7587380d00c7171, 0x3bc2301bb03c506a, 0x5e7548da8c740bd5,
        0x78b7bc1e27198977, 0xd543e5e950f31b97, 0x42f8d1bc07d79f7a, 0xb8b0aed3f19bc871,
        0xc6fd030a1a9f267c,
    ];

    const FAST_PARTIAL_ROUND_VS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS] = [
        [0xd988c89fed32790c, 0x9591f8e0d649b98c, 0xfc4168643da877c7, 0xedada09f7a299564,
         0xad77fff957050234, 0x4c7abaa24552bc93, 0xc14c8ec240605dd3, ],
        [0x269a8266e1f24a61, 0x92d584664cae4540, 0x8000a46f5e0d59be, 0x2c6f0f3ee91f18be,
         0xcd992a29c33e3668, 0x262754408c6fa3ab, 0xbf5b14d522b49c24, ],
        [0xe3676f32baab3bc5, 0x89de03dd1633bdb0, 0x5669c09ce55a8a5d, 0xec5542016500559f,
         0x75998153067af4ff, 0xf01b44ac771b97dc, 0xb72e5f1e2900244c, ],
        [0x26cd38cca2a8e873, 0xd6ec077d612a405f, 0x560fc75d2bca8b8e, 0xd2804351c8bad70f,
         0x288003159f41a7ab, 0xade9051b7ce5d812, 0x04a0817b69d542c3, ],
        [0x3ee18781eefd9b80, 0x65441e0deb9c1896, 0x4f577aec290e328c, 0xf2e72c41b27d5637,
         0xdd341f2cf7a9f285, 0x19e2f006cdaf010d, 0x42c54c87f1230ae3, ],
        [0xf89381bb46f7df6b, 0xdac45f5772bff325, 0xe9900763cca7c4cb, 0xd3ae7a9dac4f45d2,
         0x209bac72d29da249, 0x16c2a74dc27c7567, 0x2961a1507e4f56de, ],
        [0x1fd9cf0291a05b5a, 0x1df4da71ea88e852, 0x1ee2b69c7017bcf5, 0x1d7ec870b3d2d366,
         0x12347dcab71cab3a, 0x21bc50cd8d876139, 0x22ece57cce482578, ],
        [0x02021eca5670f1e5, 0x01e40136ebac9ba3, 0x01f329b6a6982471, 0x01dce277cac0f4d3,
         0x012623d00535cecf, 0x0220ec3d9bc27b19, 0x0233b75c8fe7731f, ],
        [0x002077652696b6d5, 0x001e82ab2d8593f8, 0x001f733584f16e63, 0x001e09b9165afb3c,
         0x00128ef5c791bf77, 0x00226663911114b4, 0x0023a04e5326b8a5, ],
        [0x00020c16fb7a21ab, 0x0001eded600ef1b7, 0x0001fd8333154956, 0x0001e66cd558845e,
         0x00012b8a9a089c1f, 0x00022ab64de2323f, 0x00023dbc856b2b68, ],
        [0x0000210be20b1fac, 0x00001f076ad50f4c, 0x00001ffb27ad8c81, 0x00001e995bd6d9b6,
         0x000012f3420f4338, 0x000023240d962916, 0x0000245e4d3e691b, ],
        [0x000002180ffadf8a, 0x000001f8dff7d091, 0x0000020883bd3622, 0x000001ef14c4a919,
         0x00000130350d9d61, 0x000002334154240e, 0x00000247f821189b, ],
        [0x00000021701b6c99, 0x0000001f8777cda2, 0x000000208c3a2996, 0x0000001f5d995a1c,
         0x000000136e779e7a, 0x00000023fef585fe, 0x0000002501494523, ],
        [0x000000022a01915d, 0x0000000202699a0e, 0x000000020fb21174, 0x00000001f1a13322,
         0x0000000133868466, 0x000000023b162f41, 0x0000000258278210, ],
        [0x0000000021637d50, 0x0000000020673c58, 0x0000000021b7295f, 0x0000000020fc5040,
         0x0000000013f34a1e, 0x00000000249beba2, 0x0000000024dc45e2, ],
        [0x0000000002417afa, 0x0000000001fceaa8, 0x0000000002009f1a, 0x0000000001e28790,
         0x000000000138f67b, 0x00000000024d45f2, 0x000000000284b148, ],
        [0x0000000000211948, 0x000000000023815f, 0x0000000000257e12, 0x0000000000243554,
         0x000000000013c96e, 0x000000000022fe00, 0x0000000000227a62, ],
        [0x0000000000022df8, 0x000000000001cc5d, 0x000000000001b67c, 0x000000000001b7f4,
         0x0000000000015c4e, 0x0000000000029cac, 0x000000000002f3db, ],
        [0x000000000000249a, 0x0000000000002e24, 0x0000000000002df4, 0x000000000000242f,
         0x000000000000114c, 0x0000000000001ab8, 0x0000000000001ce2, ],
        [0x000000000000014b, 0x000000000000018d, 0x00000000000000f4, 0x00000000000001af,
         0x0000000000000228, 0x00000000000003a8, 0x0000000000000329, ],
        [0x0000000000000046, 0x0000000000000048, 0x0000000000000016, 0x0000000000000009,
         0x0000000000000010, 0x000000000000000b, 0x0000000000000018, ],
        [0x0000000000000000, 0x0000000000000000, 0x0000000000000003, 0x0000000000000004,
         0x0000000000000008, 0x0000000000000001, 0x0000000000000000, ],
    ];

    const FAST_PARTIAL_ROUND_W_HATS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS] = [
        [0xa58aa8ac20026048, 0xd59d1e437f494b49, 0x074e020db8252bc1, 0x0deda591d62ae741,
         0xf708eb061e358c00, 0x59990aae2f59b396, 0x7aed7284903a81c8, ],
        [0x98b7b620c663c4c3, 0xce24bc7ba86e1721, 0xaca796e60f43d9a9, 0x8e960659cc61a15c,
         0x38736cec9ee41f1e, 0x42a43e71b333fa6b, 0xfa605fa18e725a73, ],
        [0xce818633f58f17d7, 0x2337b6969deb48d5, 0xa3751dcb9850d11c, 0xa02f991f2fa68f41,
         0xf1932d24ba0be3f0, 0x7bd54ec4d1cc0e3c, 0xbef0f00f852f8ae0, ],
        [0xe179f294a5d169e6, 0x0205b0310aa6cd5f, 0x84af6e5b1053103d, 0x1475eaa8d93fc6a5,
         0xda5f459d31ff3efa, 0xabe69200fe1f9f69, 0x686d71351ca4929d, ],
        [0x3174bcdbdd00b17d, 0x862f02eb08ee6a30, 0x90424a250368b30a, 0x70096dcbc19f61a8,
         0xcf83457330cda0bc, 0x3e431a09f426fa85, 0x67b8474fd6874269, ],
        [0x4ae3f08b29d5216c, 0xb1b678da191bb3e9, 0xaba3bc84ca6410c1, 0x29109b5337c8809f,
         0x2693dfd7107a0bef, 0x1bbc354bffeeac31, 0xf2d573ca93453d7e, ],
        [0x7c0050987fa69374, 0xdcc504c30d8a284a, 0xc0b5d8a5db7e3d74, 0x33778f3cf12578fb,
         0xb20f05344b8a4d90, 0x652c9f207cb492e8, 0x1875feabea2bea9a, ],
        [0x9c7e5b6cc0b749ec, 0x28198b07a1e4d9ae, 0x8ec9d9f705707179, 0x6d18a4077b8d75a9,
         0xfafd4b8e7d7e68aa, 0xea949788ab9ce5e4, 0x17dded58bfa55942, ],
        [0xd8be7acf569d1fd0, 0xaa6cc1a757e34ef3, 0xd9478b5a0d04438e, 0x2dd7768a3daa2020,
         0x378a3707727137e7, 0x2e71e13ad80411ac, 0x46f24f72c4a51b5d, ],
        [0xd3ad8ea6f4c32855, 0xecd228e8cc53672f, 0x326c73014b464960, 0xa0e67393560fdfc2,
         0xfebbf1e61b75aa2b, 0x88e60ae27938fb18, 0x988efca114c76aab, ],
        [0x64bd935bf81bdbfb, 0xbde8381ca6270e8c, 0x357487d81a185704, 0x95b87b2ffce2a952,
         0xe52bd8b677d97876, 0x746770ffc24b36a4, 0xc735ce882796d894, ],
        [0x89c551ada036236e, 0x36a032a3b9be259e, 0x49b4302ee189903a, 0x64e734f8c78fcabd,
         0x8204479f2b732c33, 0x5005367ac32b9cd4, 0x30e61702a8f2fbd8, ],
        [0xfb612b9e67383a51, 0xba92ce278a1c21e1, 0xd0d5b187709809bc, 0xe14b6f0d1567ca0b,
         0x29d39b445739a343, 0xca63e2fff1ce86f2, 0xd5bb76143d382b88, ],
        [0x6acd9f8ff73ac2db, 0xca0baac3fae0a588, 0x5326100292b490b7, 0xd9656b7866560b0b,
         0xe69807cbaccd4884, 0xd92f7cc9bbde5c38, 0x52b53071bd9cd730, ],
        [0x185cb77f513c81c9, 0x4246e3a578894ef2, 0xe77cf7bb86b255ad, 0xcb630f8d1a395800,
         0x1be9f3ff403f5961, 0x8d8c6ee554ea27db, 0x01e904cf562b82a4, ],
        [0x70d6e9a0df9f328b, 0x1dc5855a66d55e90, 0x97a006e5660daf41, 0xa1adcd2959629524,
         0x92fadae4c06d8720, 0x7495604d6f7b1637, 0xdf9d03eae2c66229, ],
        [0x96eaacac3bbb54da, 0xc3b93bf58312bc9a, 0x7c4b2802e5455c50, 0x86af7878aee743a4,
         0x6f1d8e07dc107c29, 0x1173974022f94d73, 0x1773f7afbdc5cd3c, ],
        [0x216aac9599fe9e13, 0xd6393c82c46a8d58, 0x21aba13286492ac1, 0xcec9795b580d5260,
         0xd3fad674b4e98cd1, 0x5f94dcefee81b1f7, 0x0fd9951dfd23dac6, ],
        [0x0b9d9bb290fcd0ff, 0x25831916aab75f29, 0xc2e6da42f6a57491, 0xaeae5341ebe5b2ff,
         0x0115e48d8b6b2766, 0x5875349ea7001be9, 0x735405e37600285f, ],
        [0x5e9b6e3c91586035, 0x4abce16f13b1e71b, 0x681b05a48b48af54, 0x39c8aedb1c69c639,
         0x1fa1dadec72a009f, 0x2fb4c60dcce57ef2, 0xd9553eb8c154ffd2, ],
        [0x4013c3aefd170cef, 0x7ddc617c3b1fca63, 0x2344c694e3ee3af6, 0x3f296d377b4774a5,
         0x77b37fcf3c8d6a3a, 0x20d03890734afbc2, 0x21348a0bf55aa4a6, ],
        [0x44281cd32deca367, 0xefb8750953e8e7a9, 0x6de022ba843f08b8, 0x606af788aa770914,
         0x01dde0d48a639638, 0x529ce3ffa513942e, 0xba103c6d8296a3ab, ],
    ];

    // NB: This is in COLUMN-major order to support cache-friendly pre-multiplication.
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; WIDTH - 1]; WIDTH - 1] = [
        [0x3ca7766c0f02a97f, 0x595c5ea991cb443c, 0x312171c7b15a9d42, 0xca18c09bda766f05,
         0xc49844ab6ce76933, 0xd786b14be0417613, 0x1fcc411469e49e7f, ],
        [0xba1e6a03b2377a00, 0x353defa584902c61, 0xd864d1052328b529, 0xca8ab74c79219c33,
         0x18e95f5796202b29, 0x7fbdddfff64f3c4e, 0xd786b14be0417613, ],
        [0xae995a4d6bcf1d6c, 0x126e6e68ced97421, 0x6ff28bf211012fe1, 0x95b3ce2379ab5862,
         0x49ee54a44dd3862a, 0x18e95f5796202b29, 0xc49844ab6ce76933, ],
        [0x15675de2f683c08d, 0x2ed3f1a0463a06d0, 0x08c50a4bc8296260, 0x4b1721226951eeac,
         0x95b3ce2379ab5862, 0xca8ab74c79219c33, 0xca18c09bda766f05, ],
        [0x14bdd436838abd95, 0x64178f2edb5c8cc8, 0x9802c0b0f8bde71f, 0x08c50a4bc8296260,
         0x6ff28bf211012fe1, 0xd864d1052328b529, 0x312171c7b15a9d42, ],
        [0x354b2c93b59db840, 0x859dcbe542ec635e, 0x64178f2edb5c8cc8, 0x2ed3f1a0463a06d0,
         0x126e6e68ced97421, 0x353defa584902c61, 0x595c5ea991cb443c, ],
        [0xad6830e3e470192d, 0x354b2c93b59db840, 0x14bdd436838abd95, 0x15675de2f683c08d,
         0xae995a4d6bcf1d6c, 0xba1e6a03b2377a00, 0x3ca7766c0f02a97f, ],
    ];

    #[inline]
    #[unroll_for_loops]
    fn mds_row_shf(r: usize, v: &[u64; WIDTH]) -> u128 {
        debug_assert!(r < WIDTH);
        // The values of MDS_MATRIX_EXPS are known to be small, so we can
        // accumulate all the products for each row and reduce just once
        // at the end (done by the caller).

        // NB: Unrolling this, calculating each term independently, and
        // summing at the end, didn't improve performance for me.
        let mut res = 0u128;
        for i in 0..WIDTH {
            res += (v[(i + r) % WIDTH] as u128) << MDS_MATRIX_EXPS[i];
        }
        res
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_layer<F: Field>(state_: &[F; WIDTH]) -> [F; WIDTH] {
        // TODO: Use MaybeUninit
        let mut result = [F::ZERO; WIDTH];

        // NB: This is a bit wasteful. Replacing it by initialising state
        // directly with the raw u64 only saved a few percent though.
        let mut state = [0u64; WIDTH];
        for r in 0..WIDTH {
            state[r] = state_[r].to_canonical_u64();
        }

        for r in 0..WIDTH {
            result[r] = F::from_canonical_u128(mds_row_shf(r, &state));
        }
        result
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_first_constant_layer<F: Field>(state: &mut [F; WIDTH]) {
        for i in 0..WIDTH {
            state[i] += F::from_canonical_u64(
                FAST_PARTIAL_FIRST_ROUND_CONSTANT[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_partial_layer_init<F: Field>(state: &[F; WIDTH]) -> [F; WIDTH] {
        // TODO: Use MaybeUninit
        let mut result = [F::ZERO; WIDTH];

        // Initial matrix has first row/column = [1, 0, ..., 0];

        // c = 0
        result[0] = state[0];

        for c in 1..WIDTH {
            for r in 1..WIDTH {
                // NB: FAST_PARTIAL_ROUND_INITAL_MATRIX is stored in
                // column-major order so that this dot product is cache
                // friendly.
                let t = F::from_canonical_u64(
                    FAST_PARTIAL_ROUND_INITIAL_MATRIX[c - 1][r - 1]);
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
    fn mds_partial_layer_fast<F: Field>(state: &[F; WIDTH], r: usize) -> [F; WIDTH] {
        // Set d = [M_00 | w^] dot [state]
        const MDS_TOP_LEFT: u64 = 1u64 << MDS_MATRIX_EXPS[0];
        let mut d = F::from_canonical_u64(MDS_TOP_LEFT) * state[0];
        for i in 1..WIDTH {
            let t = F::from_canonical_u64(
                FAST_PARTIAL_ROUND_W_HATS[r][i - 1]);
            d += state[i] * t;
        }

        // result = [d] concat [state[0] * v + state[shift up by 1]]
        let mut result = [F::ZERO; WIDTH];
        result[0] = d;
        for i in 1..WIDTH {
            let t = F::from_canonical_u64(
                FAST_PARTIAL_ROUND_VS[r][i - 1]);
            result[i] = state[0] * t + state[i];
        }
        result
    }

    #[inline]
    #[unroll_for_loops]
    fn constant_layer<F: Field>(state: &mut [F; WIDTH], round_ctr: usize) {
        for i in 0..WIDTH {
            state[i] += F::from_canonical_u64(
                ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr]);
        }
    }

    #[inline]
    fn sbox_monomial<F: Field>(x: F) -> F {
        // x |--> x^7
        let x2 = x * x;
        let x4 = x2 * x2;
        let x3 = x * x2;
        x3 * x4
    }

    #[inline]
    #[unroll_for_loops]
    fn sbox_layer<F: Field>(state: &mut [F; WIDTH]) {
        for i in 0..WIDTH {
            state[i] = sbox_monomial(state[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn full_rounds<F: Field>(state: &mut [F; WIDTH], round_ctr: &mut usize) {
        for _ in 0..HALF_N_FULL_ROUNDS {
            constant_layer(state, *round_ctr);
            sbox_layer(state);
            *state = mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds_fast<F: Field>(
        state: &mut [F; WIDTH],
        round_ctr: &mut usize)
    {
        partial_first_constant_layer(state);
        *state = mds_partial_layer_init(state);

        // One less than N_PARTIAL_ROUNDS because we do the last one
        // separately at the end.
        for i in 0..(N_PARTIAL_ROUNDS - 1) {
            state[0] = sbox_monomial(state[0]);
            state[0] += F::from_canonical_u64(
                FAST_PARTIAL_ROUND_CONSTANTS[i]);
            *state = mds_partial_layer_fast(state, i);
        }
        state[0] = sbox_monomial(state[0]);
        *state = mds_partial_layer_fast(state, N_PARTIAL_ROUNDS - 1);
        *round_ctr += N_PARTIAL_ROUNDS;
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds<F: Field>(state: &mut [F; WIDTH], round_ctr: &mut usize) {
        for _ in 0..N_PARTIAL_ROUNDS {
            constant_layer(state, *round_ctr);
            state[0] = sbox_monomial(state[0]);
            *state = mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    pub fn poseidon<F: Field>(input: [F; WIDTH]) -> [F; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        full_rounds(&mut state, &mut round_ctr);
        partial_rounds_fast(&mut state, &mut round_ctr);
        full_rounds(&mut state, &mut round_ctr);

        state
    }

    #[inline]
    pub fn poseidon_naive<F: Field>(input: [F; WIDTH]) -> [F; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        full_rounds(&mut state, &mut round_ctr);
        partial_rounds(&mut state, &mut round_ctr);
        full_rounds(&mut state, &mut round_ctr);

        state
    }
}

mod poseidon_width12 {
    use unroll::unroll_for_loops;
    use crate::field::field_types::Field;
    use std::mem::MaybeUninit;

    use crate::hash::poseidon::{
        ALL_ROUND_CONSTANTS, N_ROUNDS, N_PARTIAL_ROUNDS, HALF_N_FULL_ROUNDS
    };

    const WIDTH: usize = 12;
    const N_ROUND_CONSTANTS: usize = WIDTH * N_ROUNDS;

    // The MDS matrix we use is the circulant matrix with first row given by the vector
    // [ 2^x for x in MDS_MATRIX_EXPS] = [1024, 8192, 4, 1, 16, 2, 256, 128, 32768, 32, 1, 1]
    //
    // WARNING: If the MDS matrix is changed, then the following
    // constants need to be updated accordingly:
    //  - FAST_PARTIAL_ROUND_CONSTANTS
    //  - FAST_PARTIAL_ROUND_VS
    //  - FAST_PARTIAL_ROUND_W_HATS
    //  - FAST_PARTIAL_ROUND_INITIAL_MATRIX
    const MDS_MATRIX_EXPS: [u64; WIDTH] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];

    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; WIDTH]  = [
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

    const FAST_PARTIAL_ROUND_VS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS] = [
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
        // TODO: This is the same as [2^x for x in MDS_MATRIX_EXPS].reverse() and
        // could/should be treated separately
        [0x0000000000000001, 0x0000000000000001, 0x0000000000000020, 0x0000000000008000,
         0x0000000000000080, 0x0000000000000100, 0x0000000000000002, 0x0000000000000010,
         0x0000000000000001, 0x0000000000000004, 0x0000000000002000, ],
    ];

    const FAST_PARTIAL_ROUND_W_HATS: [[u64; WIDTH - 1]; N_PARTIAL_ROUNDS] = [
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
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; WIDTH - 1]; WIDTH - 1] = [
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

    #[inline]
    #[unroll_for_loops]
    fn mds_row_shf(r: usize, v: &[u64; WIDTH]) -> u128 {
        debug_assert!(r < WIDTH);
        // The values of MDS_MATRIX_EXPS are known to be small, so we can
        // accumulate all the products for each row and reduce just once
        // at the end (done by the caller).

        // NB: Unrolling this, calculating each term independently, and
        // summing at the end, didn't improve performance for me.
        let mut res = 0u128;
        for i in 0..WIDTH {
            res += (v[(i + r) % WIDTH] as u128) << MDS_MATRIX_EXPS[i];
        }
        res
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_layer<F: Field>(state_: &[F; WIDTH]) -> [F; WIDTH] {
        // TODO: Use MaybeUninit
        let mut result = [F::ZERO; WIDTH];

        // NB: This is a bit wasteful. Replacing it by initialising state
        // directly with the raw u64 only saved a few percent though.
        let mut state = [0u64; WIDTH];
        for r in 0..WIDTH {
            state[r] = state_[r].to_canonical_u64();
        }

        for r in 0..WIDTH {
            result[r] = F::from_canonical_u128(mds_row_shf(r, &state));
        }
        result
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_first_constant_layer<F: Field>(state: &mut [F; WIDTH]) {
        for i in 0..WIDTH {
            state[i] += F::from_canonical_u64(
                FAST_PARTIAL_FIRST_ROUND_CONSTANT[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn mds_partial_layer_init<F: Field>(state: &[F; WIDTH]) -> [F; WIDTH] {
        // TODO: Use MaybeUninit
        let mut result = [F::ZERO; WIDTH];

        // Initial matrix has first row/column = [1, 0, ..., 0];

        // c = 0
        result[0] = state[0];

        for c in 1..WIDTH {
            for r in 1..WIDTH {
                // NB: FAST_PARTIAL_ROUND_INITAL_MATRIX is stored in
                // column-major order so that this dot product is cache
                // friendly.
                let t = F::from_canonical_u64(
                    FAST_PARTIAL_ROUND_INITIAL_MATRIX[c - 1][r - 1]);
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
    fn mds_partial_layer_fast<F: Field>(state: &[F; WIDTH], r: usize) -> [F; WIDTH] {
        // Set d = [M_00 | w^] dot [state]
        const MDS_TOP_LEFT: u64 = 1u64 << MDS_MATRIX_EXPS[0];
        let mut d = F::from_canonical_u64(MDS_TOP_LEFT) * state[0];
        for i in 1..WIDTH {
            let t = F::from_canonical_u64(
                FAST_PARTIAL_ROUND_W_HATS[r][i - 1]);
            d += state[i] * t;
        }

        // result = [d] concat [state[0] * v + state[shift up by 1]]
        let mut result = [F::ZERO; WIDTH];
        result[0] = d;
        for i in 1..WIDTH {
            let t = F::from_canonical_u64(
                FAST_PARTIAL_ROUND_VS[r][i - 1]);
            result[i] = state[0] * t + state[i];
        }
        result
    }

    #[inline]
    #[unroll_for_loops]
    fn constant_layer<F: Field>(state: &mut [F; WIDTH], round_ctr: usize) {
        for i in 0..WIDTH {
            state[i] += F::from_canonical_u64(
                ALL_ROUND_CONSTANTS[i + WIDTH * round_ctr]);
        }
    }

    #[inline]
    fn sbox_monomial<F: Field>(x: F) -> F {
        // x |--> x^7
        let x2 = x * x;
        let x4 = x2 * x2;
        let x3 = x * x2;
        x3 * x4
    }

    #[inline]
    #[unroll_for_loops]
    fn sbox_layer<F: Field>(state: &mut [F; WIDTH]) {
        for i in 0..WIDTH {
            state[i] = sbox_monomial(state[i]);
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn full_rounds<F: Field>(state: &mut [F; WIDTH], round_ctr: &mut usize) {
        for _ in 0..HALF_N_FULL_ROUNDS {
            constant_layer(state, *round_ctr);
            sbox_layer(state);
            *state = mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds_fast<F: Field>(
        state: &mut [F; WIDTH],
        round_ctr: &mut usize)
    {
        partial_first_constant_layer(state);
        *state = mds_partial_layer_init(state);

        // One less than N_PARTIAL_ROUNDS because we do the last one
        // separately at the end.
        for i in 0..(N_PARTIAL_ROUNDS - 1) {
            state[0] = sbox_monomial(state[0]);
            state[0] += F::from_canonical_u64(
                FAST_PARTIAL_ROUND_CONSTANTS[i]);
            *state = mds_partial_layer_fast(state, i);
        }
        state[0] = sbox_monomial(state[0]);
        *state = mds_partial_layer_fast(state, N_PARTIAL_ROUNDS - 1);
        *round_ctr += N_PARTIAL_ROUNDS;
    }

    #[inline]
    #[unroll_for_loops]
    fn partial_rounds<F: Field>(state: &mut [F; WIDTH], round_ctr: &mut usize) {
        for _ in 0..N_PARTIAL_ROUNDS {
            constant_layer(state, *round_ctr);
            state[0] = sbox_monomial(state[0]);
            *state = mds_layer(state);
            *round_ctr += 1;
        }
    }

    #[inline]
    pub fn poseidon<F: Field>(input: [F; WIDTH]) -> [F; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        full_rounds(&mut state, &mut round_ctr);
        partial_rounds_fast(&mut state, &mut round_ctr);
        full_rounds(&mut state, &mut round_ctr);

        state
    }

    #[inline]
    pub fn poseidon_naive<F: Field>(input: [F; WIDTH]) -> [F; WIDTH] {
        let mut state = input;
        let mut round_ctr = 0;

        full_rounds(&mut state, &mut round_ctr);
        partial_rounds(&mut state, &mut round_ctr);
        full_rounds(&mut state, &mut round_ctr);

        state
    }
}

#[inline]
pub fn poseidon8<F: Field>(input: [F; 8]) -> [F; 8] {
    poseidon_width8::poseidon(input)
}

#[inline]
pub fn poseidon8_naive<F: Field>(input: [F; 8]) -> [F; 8] {
    poseidon_width8::poseidon_naive(input)
}

#[inline]
pub fn poseidon12<F: Field>(input: [F; 12]) -> [F; 12] {
    poseidon_width12::poseidon(input)
}

#[inline]
pub fn poseidon12_naive<F: Field>(input: [F; 12]) -> [F; 12] {
    poseidon_width12::poseidon_naive(input)
}


#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField as F;
    use crate::field::field_types::Field;
    use crate::hash::poseidon::{poseidon12, poseidon12_naive};

    #[test]
    fn test_vectors() {
        const WIDTH: usize = 12;
        const N_TEST_VECTORS: usize = 3;
        // Test inputs are:
        // 1. all zeros
        // 2. range 0..WIDTH
        // 3. random elements of CrandallField.
        let inputs: [[u64; WIDTH]; N_TEST_VECTORS] = [
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, ],
            [0xb69ed321abbeffbb, 0xfb496d8c39b64e42, 0x274f1cfbb925c789, 0x9e846d2b9a56b834,
             0xc7f297c0d48bc3b6, 0xb859ab1e45850a0a, 0x3244fe3bcb1244cb, 0xb98e1cfa647575de,
             0x3c9ed8013b0b366b, 0x6a242cb943c91b16, 0x404794ad562239f1, 0x209363e20945adf6, ],
        ];
        // expected_output calculated with (modified) hadeshash reference implementation.
        let expected_outputs: [[u64; WIDTH]; N_TEST_VECTORS] = [
            [0x733dbb2084c331ac, 0x3cbd068569889642, 0x0bc75ee3b4b351ab, 0xc3cbdf8b7a447540,
             0x2a6a9ddd090b6ca8, 0x66f55b580f0fcd31, 0x80935eec8cb4c86f, 0xc3b66cf805fb8332,
             0xbbabe999ab606d17, 0x9618aca73ce5d896, 0xc4a523675a92c0d5, 0x3be6d1cdc14ca266, ],
            [0xb03c984fae455fae, 0x79e7d53c5d25d456, 0x1ae40aa47d2bf9a5, 0x2ccda76dfcb2fc87,
             0x1b1c79f82ece56d6, 0xe8c12ce2fe88c79e, 0x878dbb782b5015bc, 0x79b0a229fffd51c7,
             0x606a66880f03946c, 0xe81378acf56dc99e, 0x29fd49a23025a4cb, 0x24a459927ee2dc66, ],
            [0x68f4332a44f578d6, 0x778a6cb3296c4b1c, 0xbcb896697757fd75, 0x8571e71af645b680,
             0x87e5fb8beeb58064, 0x1a773e0082e1bc3e, 0x49279a03c03740e0, 0xefc01f3e5bce40d6,
             0x9c94e9dc2f4e644e, 0x1045284ba253bc2d, 0x6345d5906c37d80d, 0x10fc9428d0de1df3, ],
        ];

        for tv in 0..N_TEST_VECTORS {
            let mut input = [F::ZERO; WIDTH];
            for i in 0..WIDTH {
                input[i] = F::from_canonical_u64(inputs[tv][i] as u64);
            }
            let output = poseidon12(input);
            for i in 0..WIDTH {
                let ex_output = F::from_canonical_u64(expected_outputs[tv][i]);
                assert_eq!(output[i], ex_output);
            }
        }
    }

    #[test]
    fn consistency() {
        const WIDTH: usize = 12;
        let mut input = [F::ZERO; WIDTH];
        for i in 0..WIDTH {
            input[i] = F::from_canonical_u64(i as u64);
        }
        let output = poseidon12(input);
        let output_fast = poseidon12_naive(input);
        for i in 0..WIDTH {
            assert_eq!(output[i], output_fast[i]);
        }
    }
}
