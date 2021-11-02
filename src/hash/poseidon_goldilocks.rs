//! Implementations for Poseidon over Goldilocks field of widths 8 and 12.
//!
//! These contents of the implementations *must* be generated using the
//! `poseidon_constants.sage` script in the `mir-protocol/hash-constants`
//! repository.

use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::poseidon::{Poseidon, N_PARTIAL_ROUNDS};

#[rustfmt::skip]
impl Poseidon for GoldilocksField {
    // The MDS matrix we use is the circulant matrix with first row given by the vector
    // [ 2^x for x in MDS_MATRIX_EXPS] = [1, 1, 2, 1, 8, 32, 2, 256, 4096, 8, 65536, 1024]
    //
    // WARNING: If the MDS matrix is changed, then the following
    // constants need to be updated accordingly:
    //  - FAST_PARTIAL_ROUND_CONSTANTS
    //  - FAST_PARTIAL_ROUND_VS
    //  - FAST_PARTIAL_ROUND_W_HATS
    //  - FAST_PARTIAL_ROUND_INITIAL_MATRIX
    const MDS_MATRIX_EXPS: [u64; 12] = [0, 0, 1, 0, 3, 5, 1, 8, 12, 3, 16, 10];

    const FAST_PARTIAL_FIRST_ROUND_CONSTANT: [u64; 12]  = [
        0x3cc3f89232e3b0c8, 0x3a8304bc56985013, 0x2a9f75c2280d2a8e, 0x53b9e0fac07c9b2b,
        0x276ef5190ab36dd6, 0xdccc95c1f434ce8d, 0x28d717d689301db6, 0x2662f1723650b872,
        0xc6b0375cf47850da, 0xbdfcca7661d81f17, 0x911992a4f6d9591f, 0xb718e4720c9f542f,
    ];

    const FAST_PARTIAL_ROUND_CONSTANTS: [u64; N_PARTIAL_ROUNDS]  = [
        0x1c92804be083d129, 0x81d932f4620fcfc6, 0x29f58a72045f76a0, 0x434472d6c6e34f30,
        0xc82c90fad781bb5c, 0xe6dfefae3135c450, 0xd0a0c9c9fff4798f, 0x97517f4034e7c8e6,
        0xae8b5030952e5949, 0xf77251b77cc297e2, 0x879c3a97606f1160, 0xed4e1e98780bdc19,
        0x5a9120e0c05b1660, 0xc4b244ea04b27221, 0x7fe9d55a335d7b82, 0xd69ff91c66ec999a,
        0x4c389b1b8180f1f5, 0x1b289f8c7fdeea1e, 0x3d464c75140b20e7, 0x74d158e1be40eb73,
        0xfc787193d2a84ea4, 0x0,
    ];

    const FAST_PARTIAL_ROUND_VS: [[u64; 12 - 1]; N_PARTIAL_ROUNDS] = [
        [0x9a5dd25dc32e6569, 0xd4b82de00e7510fa, 0x165bdcd7b344404a, 0xa85b4c126b8edfd4,
         0xcd2735bf92ab4f96, 0xdc07742c7da8ac41, 0x953fc266fc5ae49f, 0x0a151c20bfc847bf,
         0x0c550caef5afedb5, 0x74d28901888c5fa8, 0xdc51b68c30cc1741, ],
        [0x4f765e0a4246c828, 0xbbdc8cbadd477a84, 0x052a5abd7de2344c, 0xab88daa04d9c7fab,
         0xbc8fd7acbee798ef, 0xe55d796c0d8a7a09, 0x40824732ed2c556c, 0x298a94d56eabeaa4,
         0x719fcd5e11312b6c, 0x1ec9a560131d1ac7, 0xabc54a42497f7fd1, ],
        [0xb51f81e6eeeeb0d6, 0xc6f3c34e7161d1ef, 0x1e93b9e2255eed5b, 0xa78338e63ec48cc2,
         0xea6e89d1c7220a56, 0xaa52f6a1c2814bc5, 0x5896b6395e09fba0, 0xf7fc97a18d5f1eee,
         0xf2712e64111823e8, 0x4f84821bf1f857f4, 0x02041415d72da206, ],
        [0x39286a4a4a391e77, 0x4ac16c7bebc97214, 0x7427cbbcb895a01f, 0x2ef8491d0b14759b,
         0xbec7625ee20fa616, 0x7c64393faf749b6f, 0x0f61c751c9826dc5, 0x700e6f3ee8ccb8a7,
         0x5bdea3b447ef8667, 0xa0f569a5a6e97588, 0xcc9e78115d7cae2d, ],
        [0x0933079ab678e5ee, 0xed6861bf33c54a28, 0x62503e6e1749a497, 0x745a9c65dea83ac6,
         0x20ce351f6e700cf0, 0x2ec0b18d30fafb8a, 0x0312f54c22b5f299, 0x5222977218fd6cd5,
         0x82662e8445868eec, 0xc4cab6335040265d, 0x12e5790e9efb9217, ],
        [0x0d829aec63871f55, 0x384d8a425086dd8c, 0x13e78b54657bfd3e, 0x2a45a17a03093566,
         0x7b6872656233b9be, 0xddc0281bb12bbb4c, 0xa224ebff0652d7c8, 0xc5ca97207780ea5c,
         0x484236194d3586ba, 0x432a56d44a44f3f7, 0xc41f926f862fc532, ],
        [0x9366cd7ed9ef5e06, 0xd7f941098175f223, 0x9af7dda3e1c9f2b1, 0x9a0ec6d0a03525f5,
         0x3ab244f4fb0fb387, 0xd8c4e357eb1d5778, 0xe62157e2e25edbbb, 0xafcd6630f841f1f8,
         0xc3969199738708fb, 0xa8224d311e6a551f, 0xc2c0a01fc655fd9f, ],
        [0xd78498f2013cd9b6, 0x675d21a200b2908c, 0x70bfd23b9e88c707, 0x85472dcbcfd078e3,
         0x5658c961cfffd574, 0x89e05a2cda3ca315, 0x1b51ae1ff8186a9f, 0xca648f8c6c7822cb,
         0x7233c92647957f4d, 0x520bf21c62d37ffa, 0x897496c7407a2ca7, ],
        [0x8e80cf5bca4eee19, 0x754779126bc1afcf, 0x07e887764b379cb0, 0x7dc7c14e12f91d5e,
         0xc8f5dab5fb6b0264, 0x1c842cf8021f9176, 0x69b56a7e2e2db2c0, 0xf30253f77fef3445,
         0x14bb3a62919efb99, 0xff9976d424a5d89c, 0x59dde7be0331a202, ],
        [0xdbe04b62126330a2, 0x0409b2138da1eaec, 0x7bd4558eb2262691, 0xafa86cfa8d52b05b,
         0xb83f570197d8c584, 0xb3ded6cc13990ac1, 0xfd33937cb072c9e1, 0xe3b3989341d92952,
         0xd26e76d6ca949ad9, 0x35c89a8548f88e86, 0x8af785bd940c3b43, ],
        [0xcbf3b86701c790da, 0x63634f67e29f4005, 0x008f903982363b81, 0xc2b07f99d6eb0229,
         0xa8344b83d15e2558, 0x880f4e5fd103b7b0, 0xd40eddb0a5929072, 0x476e27ccee571f49,
         0xe71439b4b989f9eb, 0x97e55074f852b2fe, 0xdd258c2137e1a2c5, ],
        [0x982b90366d23259b, 0xb2667eacaa76b306, 0xecf233e82020ede1, 0x3cee7ac07d4a88c7,
         0x31428be2fe5a5854, 0xf1beea1d55c4c4db, 0x584fd6b580f1ffd2, 0x6e2381c3c8ba0d0b,
         0x21ab749cbafc0611, 0x8ed389f39aba3001, 0xa24ba694f2b42f13, ],
        [0xdb30cd9db02606f9, 0x1b0d6736682ba257, 0x0d3bcdecf5808443, 0x31c330001dbd3dbd,
         0x9684d22370447946, 0xde0e24e6426c6935, 0xf487270dd081ef69, 0xd943f4ef48f2b252,
         0x4c52a7fdd1c52d24, 0xc293082029ea139d, 0xc2ba73ab3da0468a, ],
        [0xd093bd0dcc74e0d1, 0xe91428f9ce6a98e5, 0x673dee716909dc21, 0xf22e3223548219d7,
         0x3297978d881a1300, 0x51157b1e8218d77c, 0x0e3b0a5c07843889, 0x273b48dfa36752b6,
         0x5dbf2c6323576866, 0x1c032b70763df9a7, 0x1a8d7ed4159ecbf4, ],
        [0x8e40b29fa6c4f3ad, 0x43bc06dba91daa9b, 0x445df1620dd6d846, 0xae1e72ed68c45c46,
         0x496ee4e593ade46d, 0x1d3642eddce9118f, 0x71a88114bd8fd755, 0x4a10d6b22514943d,
         0x56dca305d4d72fee, 0xe2e4d9ce95fa62bf, 0xfb6bfffd47b50b0a, ],
        [0x4c6c14946cc557ee, 0x9b1bcbaac7ba3226, 0xdd7410361fa0dd20, 0x9c8a098cbaf95b26,
         0x3da4f26593503adf, 0xffb07b45cd3bf859, 0xaf034373af54a559, 0xd6b9bace407146bb,
         0x7b92c04c972f4ec6, 0xfe71df71165b9845, 0xad0134b9dc9ebe51, ],
        [0xfdaa64ceec88aa7c, 0x565342e2d815525c, 0xe382458f259429a8, 0x0f6ba5afd5d1d1ca,
         0xcba85de412439a41, 0x212d3c62049ccb1a, 0x930c0bf5950267e3, 0x60f87fe43fc560d8,
         0x8f1fbdbcd878a33b, 0xd28b789abf9af16f, 0xd921f0434fa0eb07, ],
        [0xd69c2c80635e7c18, 0x5a3d78c8772f293f, 0x844fe5e72ad1ceb5, 0x81b217e5910dc916,
         0x2951409fb7c8ba85, 0x5c135dd95693e367, 0xc2e8a723f9f7ebd2, 0x10bb79bf5d63f38d,
         0x34625b1550385a89, 0xdc6235328d791163, 0x1eb12b7aed4d5133, ],
        [0x01426faca89577d0, 0x003ca90136ac4fd0, 0x00289223dc45a17f, 0x0009921704320612,
         0x0007efae3669e451, 0x006499f206b3349d, 0x1001120d9b5dcfe1, 0x000e3aa47db4da94,
         0x0320dc8339d35692, 0x4030a0a16247ecbd, 0x04368a659c160a6b, ],
        [0x0000001237b408f0, 0x00000004c8f1b79c, 0x0000000446de5309, 0x00000032a3e2d4ac,
         0x00000c007600eeb7, 0x000100040ee771b0, 0x00000198394d0817, 0x0000301810a981ba,
         0x0000030f37d86f5a, 0x0000030ab1cc04d4, 0x000000c0e7c0b7e9, ],
        [0x00000000000234a0, 0x0000000000114630, 0x000000000800260c, 0x0000000100005288,
         0x0000000000900194, 0x00000000200800a3, 0x0000000002011034, 0x000000000105100e,
         0x0000000000604025, 0x0000000000114a03, 0x0000000000061481, ],
        [0x0000000000000400, 0x0000000000010000, 0x0000000000000008, 0x0000000000001000,
         0x0000000000000100, 0x0000000000000002, 0x0000000000000020, 0x0000000000000008,
         0x0000000000000001, 0x0000000000000002, 0x0000000000000001, ],
    ];

    const FAST_PARTIAL_ROUND_W_HATS: [[u64; 12 - 1]; N_PARTIAL_ROUNDS] = [
        [0x54accab273d3aeca, 0x12fecae33b1f1da9, 0x573bb85449ea9a27, 0x6b5ddc139f172aad,
         0xd2b6d0ca34465d4c, 0x51cf0aafbddfc269, 0x6075e64679e7a403, 0x678316c041900ac9,
         0x10019c84b343fc57, 0xde5b81280922f644, 0x42490a86b2f2f305, ],
        [0x337c5930f7bacc46, 0x334792a4f1afb921, 0xc97ea5f1426e540e, 0x5fc74568337bd780,
         0xfd5718cc391d80ef, 0xef90b77a337d923c, 0xb28561998f153fea, 0xed5f65b8894345aa,
         0x7e2aacb5985893a7, 0xcbde536cb644fcf0, 0x07338300a07fc43b, ],
        [0xd4c9ad02fcc8b4c1, 0x2890dac7a1caa815, 0x7d62bc45c45f5db2, 0x0a902300db5deac2,
         0x663f3726307f62a4, 0x050bda7dc7d8eb3b, 0xd9db68f3f051c5b6, 0xc5110194a38210aa,
         0x403862136533be0e, 0x20039e053d9b227d, 0xe2c90d16262c5f3c, ],
        [0x6578da963396c755, 0xea6b546e6bc1e86f, 0x4e562ef0c66c2be3, 0x35b839dae0f9d22e,
         0x4aab3d88857b058c, 0x4f7443e07ac462d3, 0x93c2c5bbc385e50f, 0xc0c0c5c8ea023ce2,
         0x8409c53d4b62965d, 0x0489f2258135dcd1, 0x32958358c736aec9, ],
        [0xe13b50ca15b0a455, 0x9878071e2b5d4547, 0xb8e50d27b4172b30, 0xbf312f828d3ea142,
         0x5b8510573020e6e8, 0x7c3091c29d8d6afa, 0x7e2d900a50f194fa, 0xb236d5080d0b0409,
         0x08f148b6c3b99320, 0x679c6b9cadbe604c, 0x6b0313be2ad9b9f2, ],
        [0x12038ac320459b0e, 0x7abd36c6b25cd8e0, 0x37cc3583930e5a13, 0xafe725c4446a691d,
         0x99d89ccadeb38d80, 0x96c820be5528ec36, 0x9b63969fdc84ede6, 0x8f8f21cf5ad78c48,
         0x1a4d3573bc3c2d8b, 0x9f5a7bd9e771866e, 0x5bcef938b72497fc, ],
        [0x5f969817be6add7a, 0x572b04c1ae5a4c6d, 0x8d219b8fac9a287b, 0x4566b3c56372f434,
         0xdd3f46f108bf4441, 0xd7e1469baa3912c4, 0xac36377b68e071fc, 0xf348c609201d771a,
         0x0bb926a5e2ebdd96, 0x30efa780aee4705a, 0xb24ff2673691146a, ],
        [0x5d0324b3a1dab6e2, 0xbd1491a0cc9e564b, 0xb8699e13b528ef99, 0x7743d9a8753ee023,
         0xce577363cdb5bcbc, 0xc056688d4f006774, 0x61f9363c10d7fdf2, 0x5f730e5530f6e06d,
         0x25efb9ef3adf0072, 0xcf971d58e21a8aa7, 0xd830d7e8d0d70680, ],
        [0x36e69157ac42f39d, 0x3e7aca69ddf62d3e, 0xbbbef86cac42bb30, 0xa2e793ae56c27043,
         0x2a315dc4bc40c8a0, 0x84022758f3b3af55, 0x668809e74e7a470d, 0xf2d91eaafdee1820,
         0x50f19afd16d03294, 0x30c087d3223bcd4b, 0xf5739d95458cc633, ],
        [0x15266b5a75028317, 0x8059f198c9f88799, 0x437a070386c65244, 0xc70e0bb73942929d,
         0xa8b32cb37ae137ea, 0xc2e556278323a459, 0xbc486da754091692, 0x7815a23467d6b541,
         0x3e6dba4e930e8be6, 0x6b4277b0915d56ba, 0x20212bfac7922ea0, ],
        [0xeeba270c067b0c8b, 0xa4d576458941f29a, 0xecdf04a28c8c83be, 0xc808f0af215d7dda,
         0x424f4bfbecced0fb, 0xe4cbf6c0c10e58b3, 0x66a87bebfa09c031, 0x614ffc9443d5f0a4,
         0x96c96636f7b7975a, 0x58d4222a6f860cc5, 0x2d4f51c75bf50169, ],
        [0xab43452aec55310f, 0x0a719e77ec2b398c, 0x8f946888a3f5f74f, 0x7b447e0d9f7ad4fb,
         0x7a2887ceb40ef226, 0x8840b904c1c49e50, 0xd91ea2510b0eaddc, 0x6617fa40a1a220fb,
         0xb1c41a72a845cb45, 0x02c2715281868092, 0xaf5b1b6c46ca37bd, ],
        [0xe27649b9dbcbe631, 0x4afdf11d1d5e73b2, 0x05285a0e99160910, 0x23bfd6197ed8d3ba,
         0xb1e6292028792aab, 0xc997f6cc14e05cae, 0x34793ec255a555bd, 0xeb4f2da35a76dd03,
         0x767a5552c9910f3a, 0x4c4cc6987c30a447, 0x64da2b6920578f8d, ],
        [0xe97ce2fecc0720ac, 0x99fc5741fcdeae8a, 0x0ac47be58b345692, 0x75a446121f2cccda,
         0xf38e40a102691c8e, 0xdbe5d707594714ef, 0x6ab183bdab92e450, 0x0aed83850dc10451,
         0x66e16941a4373c93, 0x22af15bb3e1034a1, 0xab2136f22ed23ccc, ],
        [0xb0d3214d3c4c46c1, 0x3983bffd4053346c, 0xab1239b72a6a9e64, 0x669bcbda2406c089,
         0xf3118af8e563feda, 0x58323dbdd43a9c95, 0x5438aa910b51fd8c, 0xcbf071f9573f7e4f,
         0x476c8fde40075e51, 0xa10f54d3c77d8bed, 0xfecafe7ec7346beb, ],
        [0x79e00c6916f68fa8, 0x80e39c20c11400d6, 0x242e2b46a7c116b7, 0xea660990074fcff6,
         0x18e3369da4c9272b, 0xfa6471be8be33b80, 0xede2ed2a83a4574a, 0x9e595d610deaaed6,
         0xc7d2cf35fcacdc58, 0xc65cf113a9af2302, 0x35a74c3d0cac5fde, ],
        [0x35d6cf1a9aeabd4b, 0x4dc004b0b64954c3, 0xcb67ab54210b4c8f, 0xa2359b770621d28e,
         0x027a0a0a5e315bf6, 0xed6aad0492a86ef6, 0x127074e28969232c, 0x3e3d68e6354d396f,
         0x3cf204ab96edf7c6, 0x513a9050b70c18bf, 0x73b3b7399a3f5281, ],
        [0x0af9319d5b7cd620, 0x0514fbcecd8a897d, 0x542dd32e46738f8d, 0x49248ae425e9bd45,
         0x8bb9ef7ac36e53ea, 0x97981020c414a723, 0xe587f186c024e0c8, 0x14f01dd28e990ad2,
         0x4d3fca72e19ea756, 0x01a3824f1ee8e7f1, 0xb048d25b575f250e, ],
        [0xe78a4cfe6c6aa236, 0x4840deffdefd3b04, 0x6e0952d028e63e47, 0x249d49fb1d93304d,
         0xd41ce9ed49f7fbb3, 0xba255e808ea77466, 0x5ce52e6dc2005436, 0x8b5bf13acd881a04,
         0xf80f439f3ac011d1, 0x1d3618fb2cc3f916, 0xf41489c837e14938, ],
        [0x41e065665af15054, 0x71752ac86d1bba64, 0x9bfddd30f8ceadeb, 0x4f59dd5e6c985767,
         0x8aa3e0718ecaa657, 0x355f734ed4199ca2, 0x110f361baec4d693, 0x283a46e9e134b5b1,
         0x4fda33376f5c6514, 0xcca192f9565e7d13, 0x2251835db1c24c39, ],
        [0xc583f62f5970a849, 0xb6cc325741cd89dd, 0xf83288467f07ac1f, 0xfd82624964b845e7,
         0x11967e4e00a49fdd, 0x2fb200fae9f72577, 0xd6fb31913c7d5da7, 0xfad9ae578dd090cc,
         0xcd13b2be741ea5d8, 0xc1c54f9cf54b0c27, 0x29520a761b657cce, ],
        [0x0ac0e496a2b39f4a, 0x20571abb59e27953, 0xe9971143579a1d30, 0x980359c3dba518cb,
         0x05ecee5a85b427c4, 0x4620dd90ad0b5366, 0x95c98f9c5b859365, 0x0fbb1806fbc56995,
         0xfe4526fd802afae2, 0x70e3786431084092, 0xa8d78a0494939111, ],
    ];

    // NB: This is in ROW-major order to support cache-friendly pre-multiplication.
    const FAST_PARTIAL_ROUND_INITIAL_MATRIX: [[u64; 12 - 1]; 12 - 1] = [
        [0xb8dee12bf8e622dc, 0x2a0bcfdad25a7a77, 0x35f873e941f6055d, 0x99b7b85b6028982e,
         0x86d6993880e836f7, 0x1ef8de305b9c354d, 0x8b0a80ef933c37dc, 0x715c7164aacaf4a8,
         0x43845bd4f75ac7f5, 0x3e71bb7b0ec57a1a, 0xffc5b2f8946575c3, ],
        [0x863ca0992eae09b0, 0x68901dfa3ecc7696, 0x6ba9546fc13ba8be, 0x555b7567255c9650,
         0x4570c6ac5e80551b, 0x8e440c6cc2d0ed18, 0xbad8ae4dbfba0799, 0x8b71ed9e65a6ed7a,
         0xaade0f9eb69ee576, 0xdebe1855920c6e64, 0x3e71bb7b0ec57a1a, ],
        [0x2c3887c29246a985, 0x5aeb127ffeece78f, 0xa86e940514be2461, 0x2cb276ddf6094068,
         0x81e59e8f82a28b3c, 0x27bc037b1569fb52, 0x706ee8b692c2ebc7, 0xeba6949241aedb71,
         0xc416ad39f1f908f8, 0xaade0f9eb69ee576, 0x43845bd4f75ac7f5, ],
        [0x03df3a62e1ea48d2, 0xbb484c2d408e9b12, 0x0fbf2169623ec24c, 0x50955930c2f9eb19,
         0x3dfc3cc6123745cc, 0xa2a8d3774d197b2c, 0xd16417e43d20feab, 0xd998a362dba538ba,
         0xeba6949241aedb71, 0x8b71ed9e65a6ed7a, 0x715c7164aacaf4a8, ],
        [0xbbf73d77fc6c411c, 0xad7f124615d240ee, 0x4e413fcebe9020ee, 0x540bd8044c672f2b,
         0x6db739f6d2e9f37d, 0x9aa1b0a8f56ad33d, 0x53c179d92714378f, 0xd16417e43d20feab,
         0x706ee8b692c2ebc7, 0xbad8ae4dbfba0799, 0x8b0a80ef933c37dc, ],
        [0xab92e860ecde7bdc, 0xa58fc91c605c26d5, 0xfbe68b79a8d5e0b9, 0x3e7edc1407cbd848,
         0xf69c76d11eaf57bf, 0x941ef2c6beace374, 0x9aa1b0a8f56ad33d, 0xa2a8d3774d197b2c,
         0x27bc037b1569fb52, 0x8e440c6cc2d0ed18, 0x1ef8de305b9c354d, ],
        [0xb522132046b25eaf, 0x2b7b18e882c3e2c6, 0xe3322ad433ba15c8, 0x87355794faf87b1b,
         0x14f6e5ac86065fce, 0xf69c76d11eaf57bf, 0x6db739f6d2e9f37d, 0x3dfc3cc6123745cc,
         0x81e59e8f82a28b3c, 0x4570c6ac5e80551b, 0x86d6993880e836f7, ],
        [0x0084dd11f5c0d55c, 0x9d664d307df18036, 0x1d80d847dca52945, 0xee3eecb9b2df1658,
         0x87355794faf87b1b, 0x3e7edc1407cbd848, 0x540bd8044c672f2b, 0x50955930c2f9eb19,
         0x2cb276ddf6094068, 0x555b7567255c9650, 0x99b7b85b6028982e, ],
        [0xeb7c39655546eba5, 0xf07245b62d94cf71, 0x17db9b690f0031a3, 0x1d80d847dca52945,
         0xe3322ad433ba15c8, 0xfbe68b79a8d5e0b9, 0x4e413fcebe9020ee, 0x0fbf2169623ec24c,
         0xa86e940514be2461, 0x6ba9546fc13ba8be, 0x35f873e941f6055d, ],
        [0xcb7fc57923717f84, 0x795a850bf5f9e397, 0xf07245b62d94cf71, 0x9d664d307df18036,
         0x2b7b18e882c3e2c6, 0xa58fc91c605c26d5, 0xad7f124615d240ee, 0xbb484c2d408e9b12,
         0x5aeb127ffeece78f, 0x68901dfa3ecc7696, 0x2a0bcfdad25a7a77, ],
        [0x3107f5edca2f02b8, 0xcb7fc57923717f84, 0xeb7c39655546eba5, 0x0084dd11f5c0d55c,
         0xb522132046b25eaf, 0xab92e860ecde7bdc, 0xbbf73d77fc6c411c, 0x03df3a62e1ea48d2,
         0x2c3887c29246a985, 0x863ca0992eae09b0, 0xb8dee12bf8e622dc, ],
    ];

    #[cfg(all(target_arch="x86_64", target_feature="avx2", target_feature="bmi2"))]
    #[inline]
    fn poseidon(input: [Self; 12]) -> [Self; 12] {
        unsafe {
            crate::hash::arch::x86_64::poseidon_goldilocks_avx2_bmi2::poseidon(&input)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::field::field_types::{Field, PrimeField};
    use crate::field::goldilocks_field::GoldilocksField as F;
    use crate::hash::poseidon::test_helpers::{check_consistency, check_test_vectors};

    #[test]
    fn test_vectors() {
        // Test inputs are:
        // 1. all zeros
        // 2. range 0..WIDTH
        // 3. all -1's
        // 4. random elements of GoldilocksField.
        // expected output calculated with (modified) hadeshash reference implementation.

        let neg_one: u64 = F::NEG_ONE.to_canonical_u64();

        #[rustfmt::skip]
        let test_vectors8: Vec<([u64; 8], [u64; 8])> = vec![
            ([0, 0, 0, 0, 0, 0, 0, 0, ],
             [0x649eec3229475d06, 0x72afe85b8b600222, 0x816d0a50ddd39228, 0x5083133a721a187c,
              0xbb69bd7d90c490a6, 0xea1d33a65d0a3287, 0xb4d27542d2fba3bc, 0xf9756d565d90c20a, ]),
            ([0, 1, 2, 3, 4, 5, 6, 7, ],
             [0xdfda4e2a7ec338f4, 0x3ac8d668054b1873, 0xeaaef2f72528e7ff, 0xee7bcc836ae165bc,
              0x95561d9377c3e696, 0x2e7d39c369dfccaa, 0x992178c050936f8f, 0x34e38ec33f572850, ]),
            ([neg_one, neg_one, neg_one, neg_one,
              neg_one, neg_one, neg_one, neg_one, ],
             [0x9d8553546c658f67, 0xd5f6422aea26962b, 0xffb40b4db302da75, 0x34f43bbd7882c16c,
              0xccb375313fa146b0, 0x87574c332e89201a, 0x60e9e6c0c0be3a16, 0xf0e2a741e90756ba, ]),
            ([0x016f2dde9ccdaf6f, 0x77e29cda821fece4, 0x2f6686f781255f78, 0xd2c4c9a53070b44f,
              0x4d7035c9fd01fc40, 0xc8d460945c91d509, 0x14855cd8a36a097f, 0x49f640d6a30f9cf0, ],
             [0x4c3c58a3fac4ba05, 0x3f26fc2bcb33a3d4, 0xe13fcddcd7a136bb, 0x27b05be73a91e2f2,
              0x37804ed8ca07fcd5, 0xe78ec2f213e28456, 0xecf67d2aacb4dbe3, 0xad14575187c496ca, ]),
        ];

        check_test_vectors::<F, 8>(test_vectors8);

        #[rustfmt::skip]
        let test_vectors12: Vec<([u64; 12], [u64; 12])> = vec![
            ([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ],
             [0x3901858a44be6b3a, 0xb3470607c5f0ba0e, 0xb3b3ac3d89b37e8e, 0xd389513a7f6fe6e9,
              0x1eceb92f5da1c96b, 0x55d0bdfc6a842adf, 0x0112c568afb8819c, 0x6ac21107619569ee,
              0x3de33babbb421a85, 0x83688eb15ffe4ca3, 0x47e285b477551fa9, 0x1dd3dda781901271, ]),
            ([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, ],
             [0x641772a94a77c7e5, 0x38d2cec9c47e7314, 0x3577218e825058c9, 0x1cdb3b4d22c54bcc,
              0x803234d4b16eb152, 0xbbb6c8438627c0f0, 0x1b219561c95a41fa, 0x9bdc97531bacc401,
              0x4251f4fac8271d9d, 0x0279ffa7ba5ce9aa, 0x63baf77c533b5874, 0xb7ada3e1f98b25e7, ]),
            ([neg_one, neg_one, neg_one, neg_one,
              neg_one, neg_one, neg_one, neg_one,
              neg_one, neg_one, neg_one, neg_one, ],
             [0xd2e4605ed1eb9613, 0x62510e8cbaf8a3b5, 0x64dc1e941dbaf46c, 0x1d6c5a5fd43cc4c5,
              0xac4b4f6bf503a6b4, 0x19e17983f5e52404, 0x927b08e033b29b6f, 0xa41bc2cb5ddb9bc0,
              0x270d528b1accc148, 0x022169acf46c71ae, 0xbbd4566e7b49ad7d, 0x0ed1ea54401533ef, ]),
            ([0xa48728856b047229, 0xc43ab5e4aa986608, 0x715f470f075c057f, 0x36e955a095478013,
              0x7c036db7200ba52d, 0x20377cd3410dc7dc, 0x058c0956659b05b2, 0xa66c880ee57e8399,
              0xb06521c88afbd610, 0xdfa4d72ba95c8895, 0x25b403dac3622acc, 0xda607d79268a8fce, ],
             [0xe85b56b0764df429, 0x7c0796201b43fe68, 0x231673b8300a6a16, 0x25db4745a952a677,
              0x01431a6817415a4d, 0xfdfbbe63602076eb, 0x82c643dabf1154c1, 0x896e7e87b3f3417d,
              0x27eca78818ef9c27, 0xf08c93583c24dc47, 0x1c9e1552c07a9f73, 0x7659179192cfdc88, ]),
        ];

        check_test_vectors::<F, 12>(test_vectors12);
    }

    #[test]
    fn consistency() {
        check_consistency::<F, 8>();
        check_consistency::<F, 12>();
    }
}
