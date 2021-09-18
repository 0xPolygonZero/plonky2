use crate::field::crandall_field::CrandallField;

const OMEGA1: CrandallField = CrandallField(0xac4df974d3152fee);
const OMEGA2: CrandallField = CrandallField(0x55d12e4569c958c5);
const OMEGA3: CrandallField = CrandallField(0x89cbc2f9e9a71802);
const INV8: CrandallField = CrandallField(0xdfffffff82000001);

const MDS_MATRIX_FFT: [CrandallField; 8] = [
    CrandallField(0x0000000000000121), CrandallField(0xffffffff6fffff0e),
    CrandallField(0x22a4d36d2b0dfe96), CrandallField(0xdd5b2c9244f2018d),
    CrandallField(0xcd15aad2a7ad91d4), CrandallField(0x8747f8a164bfbc8c),
    CrandallField(0xc9fbc93ca1ecbec2), CrandallField(0xe1a6934da1a5f2b1),
];

const ROUND_CONSTANTS_FFT: [[CrandallField; 8]; 22] = [
    [CrandallField(0x0213b9182a1cd763), CrandallField(0xd4afa73a1f2188e4),
     CrandallField(0x0cf180760bb6d8cb), CrandallField(0x01d7c70ebbaa36b3),
     CrandallField(0xc47a69549ed9a6ff), CrandallField(0xe044e9ec18d6c7df),
     CrandallField(0xf3d905ea992efc85), CrandallField(0xb7b99771dd19b3a9)],
    [CrandallField(0x75ac7dacd192ba59), CrandallField(0xe2d319661c7e0e16),
     CrandallField(0x6943d638471ba253), CrandallField(0xd505e76f2c61d62b),
     CrandallField(0x5dbbca456e2d5807), CrandallField(0xd0f5617a7e7c212d),
     CrandallField(0x8674666aba4b9e96), CrandallField(0x9bb1cf9b5bdd5c2a)],
    [CrandallField(0xbd9f79cf899402f9), CrandallField(0x328bebc1abf52325),
     CrandallField(0x4dcff91328fde409), CrandallField(0x212548771186b920),
     CrandallField(0x4884bff9b411bcbc), CrandallField(0xc55b4f904e7fe88f),
     CrandallField(0xc158e44d66c463dd), CrandallField(0xb7c6299d9db9b9d3)],
    [CrandallField(0xc177f15b654acf6a), CrandallField(0xc90627d107cace58),
     CrandallField(0x31f827be7ab62ff6), CrandallField(0xaf8b456c71b3f05a),
     CrandallField(0x968c28d9f54e0b27), CrandallField(0x645246dfcc58bb60),
     CrandallField(0x2fbf853aece94c9b), CrandallField(0xb7d63f363a55ea9b)],
    [CrandallField(0xc24aabbd8cd6ff51), CrandallField(0xd3199cb4059bbd7f),
     CrandallField(0xa72e24a629fd0cef), CrandallField(0xf48aea0e2a543c97),
     CrandallField(0x9197a5902448f546), CrandallField(0x19c6e73d7f0315b3),
     CrandallField(0x48091538c1aff09b), CrandallField(0xfe62c379bb13c360)],
    [CrandallField(0xb79f7530e169ad63), CrandallField(0x7aa3bad78f174d56),
     CrandallField(0x28c1ad676994a633), CrandallField(0xb2474389c8c7e26a),
     CrandallField(0x5a95c522664bc597), CrandallField(0xc9e14266b4b98206),
     CrandallField(0x02a072b7acf4e70b), CrandallField(0x99053a207fa76d39)],
    [CrandallField(0x724504d91fc4d3dc), CrandallField(0xa570f49b833c1d97),
     CrandallField(0x9753468ef8151e13), CrandallField(0x2a843e93fe37613c),
     CrandallField(0xa638ce256b89b769), CrandallField(0x0e14313425d7bc8a),
     CrandallField(0x6f03206b7597e1b4), CrandallField(0x5a15ae84974b43c2)],
    [CrandallField(0x81c2f76a9ffd41dd), CrandallField(0xc6d0c94ef44c3af6),
     CrandallField(0x82033857b5ee1fd7), CrandallField(0xfebede2528d882c3),
     CrandallField(0x72c55c91041328ae), CrandallField(0xef6f165ae347c1d9),
     CrandallField(0x6ec75d4866984160), CrandallField(0x9dd5a00a00548f06)],
    [CrandallField(0xc85f4fb1afb673b2), CrandallField(0xc82d3011ad7f5e1c),
     CrandallField(0x24e5baa792519557), CrandallField(0xf0318369efbcf636),
     CrandallField(0xb958242b71d29b31), CrandallField(0x3fb5abf1d59deb30),
     CrandallField(0x91822632b44bc0d6), CrandallField(0x5a9ab5623f1a3607)],
    [CrandallField(0xaff905f5b2040297), CrandallField(0x9dd0e1225f59892e),
     CrandallField(0x62c3afc10d76f031), CrandallField(0xf0addcc4f75f9396),
     CrandallField(0x43879f72599b57b2), CrandallField(0xe305cb172bdcb52f),
     CrandallField(0xf48a5307befa9e00), CrandallField(0x9eab7947c6c7d3e2)],
    [CrandallField(0xad5b6a33c0dc2cc6), CrandallField(0x62d5118de8d8af7d),
     CrandallField(0xe3a3a770aafab3c6), CrandallField(0xca30a15a52e5fef6),
     CrandallField(0x348d30e68d02299b), CrandallField(0x1e3a9c340616a4cc),
     CrandallField(0x0112c717dc910d8c), CrandallField(0x4f5094610b5da92d)],
    [CrandallField(0x0830c6f16636222f), CrandallField(0xdccedb513ad9c9f2),
     CrandallField(0x1fc4810bf34d7193), CrandallField(0xa5b373a8105d21d9),
     CrandallField(0xdf5e126151f5aee4), CrandallField(0x406f149419cb4aa6),
     CrandallField(0x1463637e54a97ea3), CrandallField(0xa509447e89ab5d3b)],
    [CrandallField(0x4360a4edc24e7e8e), CrandallField(0x6a3719654142cb77),
     CrandallField(0x8049be0723775fef), CrandallField(0xe74cbd5464192775),
     CrandallField(0x7835a801f8e59ffd), CrandallField(0x9cbcea26598bee7a),
     CrandallField(0x08592b51ec912b4f), CrandallField(0x72bf3c0678b38f5c)],
    [CrandallField(0xc9d24dac71588ecc), CrandallField(0xce83ed93f2a1cc25),
     CrandallField(0xe61623ce8a1a27a5), CrandallField(0x2eef189d4ac5dabd),
     CrandallField(0x8c315fcb70e93991), CrandallField(0xd484f4eb56660961),
     CrandallField(0xfe0b56c73bc5b1ef), CrandallField(0x030482cff22e219b)],
    [CrandallField(0xec8b4a54681f5936), CrandallField(0x33483569514590fd),
     CrandallField(0xff5afa7426c2fa1f), CrandallField(0xfe64e39c448eefbd),
     CrandallField(0x9ed5c6584b295001), CrandallField(0x95a75acc3140a553),
     CrandallField(0x9cbe64eb46a897a2), CrandallField(0xbe3c7073ad0236a8)],
    [CrandallField(0x7b0c29c89d718e0f), CrandallField(0xb318d22fb29e2888),
     CrandallField(0xf6b598ff9bc8ca0f), CrandallField(0xecf8ca78258a2e10),
     CrandallField(0xe387bba6d963e3d1), CrandallField(0x597962d4251d424b),
     CrandallField(0x00d116ccf5ab9cf0), CrandallField(0x02084c9827845893)],
    [CrandallField(0xadff32719f3e3908), CrandallField(0xb812182c9ddadbf5),
     CrandallField(0xe8606beccf78e52d), CrandallField(0x891f6bfd4d4bcc24),
     CrandallField(0xfbfc77c09503c671), CrandallField(0xef9060e0ab0be502),
     CrandallField(0x55e9bb53edcb9345), CrandallField(0xa3947ae684237bc9)],
    [CrandallField(0xd005bd4c5071f841), CrandallField(0xdb9bfb7c41c4f796),
     CrandallField(0xfd5f7706d350c006), CrandallField(0xa3324ace0e53b6a4),
     CrandallField(0x8fcb8094f80858f9), CrandallField(0xec55ec6cdb5690dd),
     CrandallField(0x0a49449c697112a6), CrandallField(0xe7e200863c937055)],
    [CrandallField(0x1a33975371440275), CrandallField(0xb9c8882d8bffe6ff),
     CrandallField(0xd486c59f3d906a15), CrandallField(0xcd903b2e203f1d27),
     CrandallField(0x3a26041985c0ff53), CrandallField(0x170b79fe8634cffc),
     CrandallField(0x836c2963ef51f180), CrandallField(0xd310cab9ab1ea558)],
    [CrandallField(0x57b1a2dc7ef23e15), CrandallField(0xd04c5099cb34d89b),
     CrandallField(0x40c79f9796bd5d82), CrandallField(0x832d0e27138cc61b),
     CrandallField(0x8a684a8efcd5e404), CrandallField(0x5b188fcbf29d0a5d),
     CrandallField(0x7f5058be1c06ad97), CrandallField(0xd56b17c97fc7e218)],
    [CrandallField(0xf36997dbc11e9aee), CrandallField(0xf83ab28d2d1f8b61),
     CrandallField(0x5e4cd8a7b681a610), CrandallField(0x3588bf6f0d1281b7),
     CrandallField(0x752caf56a2b84be2), CrandallField(0x48498cd8995069cf),
     CrandallField(0x39a4b2ce3cbb98d5), CrandallField(0xebb4e1de535dca78)],
    [CrandallField(0x30863e36e09135a6), CrandallField(0x0a6e3084b88449af),
     CrandallField(0x7079038540613eb6), CrandallField(0xa78589afb50dfcd1),
     CrandallField(0x249dbf5c541e403d), CrandallField(0x2057d9ad5a35804f),
     CrandallField(0xd6ce993e51d32a55), CrandallField(0x4c25d121917e4132)],
];

#[inline(always)]
fn dif(mut x: [CrandallField; 8]) -> [CrandallField; 8] {
    (x[0], x[4]) = (x[0] + x[4], x[0] - x[4]);
    (x[1], x[5]) = (x[1] + x[5], x[1] - x[5]);
    (x[2], x[6]) = (x[2] + x[6], x[2] - x[6]);
    (x[3], x[7]) = (x[3] + x[7], x[3] - x[7]);

    x[6] *= OMEGA2;
    x[7] *= OMEGA2;
    (x[0], x[2]) = (x[0] + x[2], x[0] - x[2]);
    (x[4], x[6]) = (x[4] + x[6], x[4] - x[6]);
    (x[1], x[3]) = (x[1] + x[3], x[1] - x[3]);
    (x[5], x[7]) = (x[5] + x[7], x[5] - x[7]);

    x[3] *= OMEGA2;
    x[5] *= OMEGA1;
    x[7] *= OMEGA3;
    (x[0], x[1]) = (x[0] + x[1], x[0] - x[1]);
    (x[2], x[3]) = (x[2] + x[3], x[2] - x[3]);
    (x[4], x[5]) = (x[4] + x[5], x[4] - x[5]);
    (x[6], x[7]) = (x[6] + x[7], x[6] - x[7]);

    x
}

#[inline(always)]
fn dit(mut x: [CrandallField; 8]) -> [CrandallField; 8] {
    (x[0], x[1]) = (x[0] + x[1], x[0] - x[1]);
    (x[2], x[3]) = (x[2] + x[3], x[2] - x[3]);
    (x[4], x[5]) = (x[4] + x[5], x[4] - x[5]);
    (x[6], x[7]) = (x[6] + x[7], x[6] - x[7]);
    x[3] *= OMEGA2;
    x[5] *= OMEGA3;
    x[7] *= OMEGA1;

    (x[0], x[2]) = (x[0] + x[2], x[0] - x[2]);
    (x[4], x[6]) = (x[4] + x[6], x[4] - x[6]);
    (x[1], x[3]) = (x[1] - x[3], x[1] + x[3]);
    (x[5], x[7]) = (x[5] + x[7], x[5] - x[7]);
    x[6] *= OMEGA2;
    x[7] *= OMEGA2;

    (x[0], x[4]) = (x[0] + x[4], x[0] - x[4]);
    (x[1], x[5]) = (x[1] - x[5], x[1] + x[5]);
    (x[2], x[6]) = (x[2] - x[6], x[2] + x[6]);
    (x[3], x[7]) = (x[3] + x[7], x[3] - x[7]);

    x[0] *= INV8;
    x[1] *= INV8;
    x[2] *= INV8;
    x[3] *= INV8;
    x[4] *= INV8;
    x[5] *= INV8;
    x[6] *= INV8;
    x[7] *= INV8;

    x
}

#[inline(always)]
fn monomial(x: CrandallField) -> CrandallField {
    let x2 = x * x;
    (x * x2) * (x2 * x2)
}

#[inline(always)]
fn constant_layer_fft(mut x_fft: [CrandallField; 8], i: usize) -> [CrandallField; 8] {
    let constants = ROUND_CONSTANTS_FFT[i];
    x_fft[0] += constants[0];
    x_fft[1] += constants[1];
    x_fft[2] += constants[2];
    x_fft[3] += constants[3];
    x_fft[4] += constants[4];
    x_fft[5] += constants[5];
    x_fft[6] += constants[6];
    x_fft[7] += constants[7];
    x_fft
}

#[inline(always)]
fn partial_sbox_layer_fft(mut x_fft: [CrandallField; 8]) -> [CrandallField; 8] {
    let x0 = (((x_fft[0] + x_fft[1]) + (x_fft[2] + x_fft[3]))
              + ((x_fft[4] + x_fft[5]) + (x_fft[6] + x_fft[7]))) * INV8;
    let x0_monomial = monomial(x0);
    let diff = x0_monomial - x0;
    x_fft[0] += diff;
    x_fft[1] += diff;
    x_fft[2] += diff;
    x_fft[3] += diff;
    x_fft[4] += diff;
    x_fft[5] += diff;
    x_fft[6] += diff;
    x_fft[7] += diff;
    x_fft
}

#[inline(always)]
fn mds_layer_fft(mut x_fft: [CrandallField; 8]) -> [CrandallField; 8] {
    x_fft[0] *= MDS_MATRIX_FFT[0];
    x_fft[1] *= MDS_MATRIX_FFT[1];
    x_fft[2] *= MDS_MATRIX_FFT[2];
    x_fft[3] *= MDS_MATRIX_FFT[3];
    x_fft[4] *= MDS_MATRIX_FFT[4];
    x_fft[5] *= MDS_MATRIX_FFT[5];
    x_fft[6] *= MDS_MATRIX_FFT[6];
    x_fft[7] *= MDS_MATRIX_FFT[7];
    x_fft
}

#[inline(always)]
pub fn partial_rounds_fft(x: [CrandallField; 8]) -> [CrandallField; 8] {
    let mut x_fft = dif(x);
    for i in 0..22 {
        x_fft = constant_layer_fft(x_fft, i);
        x_fft = partial_sbox_layer_fft(x_fft);
        x_fft = mds_layer_fft(x_fft);
    }
    dit(x_fft)
}
