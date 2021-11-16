// Implementation of Poiseidon in Metal
// TODO: Const layer. It's cheap anyway and I didn't want to think about where to put the constants.

#include <metal_stdlib>
using namespace metal;

// Goldilocks multiplication
uint64_t mul(uint64_t x, uint64_t y) {
    uint64_t lo = x * y;
    uint64_t hi = mulhi(x, y);

    uint64_t hi_lo = static_cast<uint32_t>(hi);
    uint64_t hi_hi = hi >> 32;

    uint64_t res0 = lo - hi_hi;
    uint64_t res1 = res0 - 0xffffffff * (lo < res0);
    uint64_t res2 = res1 + hi_lo * 0xffffffff;
    uint64_t res3 = res2 + 0xffffffff * (res2 < res1);
    
    return res3;
}

uint64_t sbox_monomial(uint64_t x) {
    uint64_t x2 = mul(x, x);
    uint64_t x3 = mul(x, x2);
    uint64_t x4 = mul(x2, x2);
    uint64_t x7 = mul(x3, x4);
    return x7;
}

void sbox_layer_partial(uint64_t thread *state) {
    state[0] = sbox_monomial(state[0]);
}

void sbox_layer_full(uint64_t thread *state) {
    state[11] = sbox_monomial(state[11]);
    state[10] = sbox_monomial(state[10]);
    state[9] = sbox_monomial(state[9]);
    state[8] = sbox_monomial(state[8]);
    state[7] = sbox_monomial(state[7]);
    state[6] = sbox_monomial(state[6]);
    state[5] = sbox_monomial(state[5]);
    state[4] = sbox_monomial(state[4]);
    state[3] = sbox_monomial(state[3]);
    state[2] = sbox_monomial(state[2]);
    state[1] = sbox_monomial(state[1]);
    state[0] = sbox_monomial(state[0]);
}

void mds_layer(uint64_t thread *state) {
    uint64_t state_lo[12] = {
        static_cast<uint32_t>(state[0]), static_cast<uint32_t>(state[1]),
        static_cast<uint32_t>(state[2]), static_cast<uint32_t>(state[3]),
        static_cast<uint32_t>(state[4]), static_cast<uint32_t>(state[5]),
        static_cast<uint32_t>(state[6]), static_cast<uint32_t>(state[7]),
        static_cast<uint32_t>(state[8]), static_cast<uint32_t>(state[9]),
        static_cast<uint32_t>(state[10]), static_cast<uint32_t>(state[11])
    };
    uint64_t state_hi[12] = {
        state[0] >> 32, state[1] >> 32,
        state[2] >> 32, state[3] >> 32,
        state[4] >> 32, state[5] >> 32,
        state[6] >> 32, state[7] >> 32,
        state[8] >> 32, state[9] >> 32,
        state[10] >> 32, state[11] >> 32
    };
    
    uint64_t res_lo[12];
    uint64_t res_hi[12];
    
    res_lo[11] = state_lo[11] << 0;
    res_hi[11] = state_hi[11] << 0;
    res_lo[10] = state_lo[11] << 0;
    res_hi[10] = state_hi[11] << 0;
    res_lo[9] = state_lo[11] << 1;
    res_hi[9] = state_hi[11] << 1;
    res_lo[8] = state_lo[11] << 0;
    res_hi[8] = state_hi[11] << 0;
    res_lo[7] = state_lo[11] << 3;
    res_hi[7] = state_hi[11] << 3;
    res_lo[6] = state_lo[11] << 5;
    res_hi[6] = state_hi[11] << 5;
    res_lo[5] = state_lo[11] << 1;
    res_hi[5] = state_hi[11] << 1;
    res_lo[4] = state_lo[11] << 8;
    res_hi[4] = state_hi[11] << 8;
    res_lo[3] = state_lo[11] << 12;
    res_hi[3] = state_hi[11] << 12;
    res_lo[2] = state_lo[11] << 3;
    res_hi[2] = state_hi[11] << 3;
    res_lo[1] = state_lo[11] << 16;
    res_hi[1] = state_hi[11] << 16;
    res_lo[0] = state_lo[11] << 10;
    res_hi[0] = state_hi[11] << 10;
    
    res_lo[11] += state_lo[10] << 10;
    res_hi[11] += state_hi[10] << 10;
    res_lo[10] += state_lo[10] << 0;
    res_hi[10] += state_hi[10] << 0;
    res_lo[9] += state_lo[10] << 0;
    res_hi[9] += state_hi[10] << 0;
    res_lo[8] += state_lo[10] << 1;
    res_hi[8] += state_hi[10] << 1;
    res_lo[7] += state_lo[10] << 0;
    res_hi[7] += state_hi[10] << 0;
    res_lo[6] += state_lo[10] << 3;
    res_hi[6] += state_hi[10] << 3;
    res_lo[5] += state_lo[10] << 5;
    res_hi[5] += state_hi[10] << 5;
    res_lo[4] += state_lo[10] << 1;
    res_hi[4] += state_hi[10] << 1;
    res_lo[3] += state_lo[10] << 8;
    res_hi[3] += state_hi[10] << 8;
    res_lo[2] += state_lo[10] << 12;
    res_hi[2] += state_hi[10] << 12;
    res_lo[1] += state_lo[10] << 3;
    res_hi[1] += state_hi[10] << 3;
    res_lo[0] += state_lo[10] << 16;
    res_hi[0] += state_hi[10] << 16;
    
    res_lo[11] += state_lo[9] << 16;
    res_hi[11] += state_hi[9] << 16;
    res_lo[10] += state_lo[9] << 10;
    res_hi[10] += state_hi[9] << 10;
    res_lo[9] += state_lo[9] << 0;
    res_hi[9] += state_hi[9] << 0;
    res_lo[8] += state_lo[9] << 0;
    res_hi[8] += state_hi[9] << 0;
    res_lo[7] += state_lo[9] << 1;
    res_hi[7] += state_hi[9] << 1;
    res_lo[6] += state_lo[9] << 0;
    res_hi[6] += state_hi[9] << 0;
    res_lo[5] += state_lo[9] << 3;
    res_hi[5] += state_hi[9] << 3;
    res_lo[4] += state_lo[9] << 5;
    res_hi[4] += state_hi[9] << 5;
    res_lo[3] += state_lo[9] << 1;
    res_hi[3] += state_hi[9] << 1;
    res_lo[2] += state_lo[9] << 8;
    res_hi[2] += state_hi[9] << 8;
    res_lo[1] += state_lo[9] << 12;
    res_hi[1] += state_hi[9] << 12;
    res_lo[0] += state_lo[9] << 3;
    res_hi[0] += state_hi[9] << 3;
    
    res_lo[11] += state_lo[8] << 3;
    res_hi[11] += state_hi[8] << 3;
    res_lo[10] += state_lo[8] << 16;
    res_hi[10] += state_hi[8] << 16;
    res_lo[9] += state_lo[8] << 10;
    res_hi[9] += state_hi[8] << 10;
    res_lo[8] += state_lo[8] << 0;
    res_hi[8] += state_hi[8] << 0;
    res_lo[7] += state_lo[8] << 0;
    res_hi[7] += state_hi[8] << 0;
    res_lo[6] += state_lo[8] << 1;
    res_hi[6] += state_hi[8] << 1;
    res_lo[5] += state_lo[8] << 0;
    res_hi[5] += state_hi[8] << 0;
    res_lo[4] += state_lo[8] << 3;
    res_hi[4] += state_hi[8] << 3;
    res_lo[3] += state_lo[8] << 5;
    res_hi[3] += state_hi[8] << 5;
    res_lo[2] += state_lo[8] << 1;
    res_hi[2] += state_hi[8] << 1;
    res_lo[1] += state_lo[8] << 8;
    res_hi[1] += state_hi[8] << 8;
    res_lo[0] += state_lo[8] << 12;
    res_hi[0] += state_hi[8] << 12;
    
    res_lo[11] += state_lo[7] << 12;
    res_hi[11] += state_hi[7] << 12;
    res_lo[10] += state_lo[7] << 3;
    res_hi[10] += state_hi[7] << 3;
    res_lo[9] += state_lo[7] << 16;
    res_hi[9] += state_hi[7] << 16;
    res_lo[8] += state_lo[7] << 10;
    res_hi[8] += state_hi[7] << 10;
    res_lo[7] += state_lo[7] << 0;
    res_hi[7] += state_hi[7] << 0;
    res_lo[6] += state_lo[7] << 0;
    res_hi[6] += state_hi[7] << 0;
    res_lo[5] += state_lo[7] << 1;
    res_hi[5] += state_hi[7] << 1;
    res_lo[4] += state_lo[7] << 0;
    res_hi[4] += state_hi[7] << 0;
    res_lo[3] += state_lo[7] << 3;
    res_hi[3] += state_hi[7] << 3;
    res_lo[2] += state_lo[7] << 5;
    res_hi[2] += state_hi[7] << 5;
    res_lo[1] += state_lo[7] << 1;
    res_hi[1] += state_hi[7] << 1;
    res_lo[0] += state_lo[7] << 8;
    res_hi[0] += state_hi[7] << 8;
    
    res_lo[11] += state_lo[6] << 8;
    res_hi[11] += state_hi[6] << 8;
    res_lo[10] += state_lo[6] << 12;
    res_hi[10] += state_hi[6] << 12;
    res_lo[9] += state_lo[6] << 3;
    res_hi[9] += state_hi[6] << 3;
    res_lo[8] += state_lo[6] << 16;
    res_hi[8] += state_hi[6] << 16;
    res_lo[7] += state_lo[6] << 10;
    res_hi[7] += state_hi[6] << 10;
    res_lo[6] += state_lo[6] << 0;
    res_hi[6] += state_hi[6] << 0;
    res_lo[5] += state_lo[6] << 0;
    res_hi[5] += state_hi[6] << 0;
    res_lo[4] += state_lo[6] << 1;
    res_hi[4] += state_hi[6] << 1;
    res_lo[3] += state_lo[6] << 0;
    res_hi[3] += state_hi[6] << 0;
    res_lo[2] += state_lo[6] << 3;
    res_hi[2] += state_hi[6] << 3;
    res_lo[1] += state_lo[6] << 5;
    res_hi[1] += state_hi[6] << 5;
    res_lo[0] += state_lo[6] << 1;
    res_hi[0] += state_hi[6] << 1;
    
    res_lo[11] += state_lo[5] << 1;
    res_hi[11] += state_hi[5] << 1;
    res_lo[10] += state_lo[5] << 8;
    res_hi[10] += state_hi[5] << 8;
    res_lo[9] += state_lo[5] << 12;
    res_hi[9] += state_hi[5] << 12;
    res_lo[8] += state_lo[5] << 3;
    res_hi[8] += state_hi[5] << 3;
    res_lo[7] += state_lo[5] << 16;
    res_hi[7] += state_hi[5] << 16;
    res_lo[6] += state_lo[5] << 10;
    res_hi[6] += state_hi[5] << 10;
    res_lo[5] += state_lo[5] << 0;
    res_hi[5] += state_hi[5] << 0;
    res_lo[4] += state_lo[5] << 0;
    res_hi[4] += state_hi[5] << 0;
    res_lo[3] += state_lo[5] << 1;
    res_hi[3] += state_hi[5] << 1;
    res_lo[2] += state_lo[5] << 0;
    res_hi[2] += state_hi[5] << 0;
    res_lo[1] += state_lo[5] << 3;
    res_hi[1] += state_hi[5] << 3;
    res_lo[0] += state_lo[5] << 5;
    res_hi[0] += state_hi[5] << 5;
    
    res_lo[11] += state_lo[4] << 5;
    res_hi[11] += state_hi[4] << 5;
    res_lo[10] += state_lo[4] << 1;
    res_hi[10] += state_hi[4] << 1;
    res_lo[9] += state_lo[4] << 8;
    res_hi[9] += state_hi[4] << 8;
    res_lo[8] += state_lo[4] << 12;
    res_hi[8] += state_hi[4] << 12;
    res_lo[7] += state_lo[4] << 3;
    res_hi[7] += state_hi[4] << 3;
    res_lo[6] += state_lo[4] << 16;
    res_hi[6] += state_hi[4] << 16;
    res_lo[5] += state_lo[4] << 10;
    res_hi[5] += state_hi[4] << 10;
    res_lo[4] += state_lo[4] << 0;
    res_hi[4] += state_hi[4] << 0;
    res_lo[3] += state_lo[4] << 0;
    res_hi[3] += state_hi[4] << 0;
    res_lo[2] += state_lo[4] << 1;
    res_hi[2] += state_hi[4] << 1;
    res_lo[1] += state_lo[4] << 0;
    res_hi[1] += state_hi[4] << 0;
    res_lo[0] += state_lo[4] << 3;
    res_hi[0] += state_hi[4] << 3;
    
    res_lo[11] += state_lo[3] << 3;
    res_hi[11] += state_hi[3] << 3;
    res_lo[10] += state_lo[3] << 5;
    res_hi[10] += state_hi[3] << 5;
    res_lo[9] += state_lo[3] << 1;
    res_hi[9] += state_hi[3] << 1;
    res_lo[8] += state_lo[3] << 8;
    res_hi[8] += state_hi[3] << 8;
    res_lo[7] += state_lo[3] << 12;
    res_hi[7] += state_hi[3] << 12;
    res_lo[6] += state_lo[3] << 3;
    res_hi[6] += state_hi[3] << 3;
    res_lo[5] += state_lo[3] << 16;
    res_hi[5] += state_hi[3] << 16;
    res_lo[4] += state_lo[3] << 10;
    res_hi[4] += state_hi[3] << 10;
    res_lo[3] += state_lo[3] << 0;
    res_hi[3] += state_hi[3] << 0;
    res_lo[2] += state_lo[3] << 0;
    res_hi[2] += state_hi[3] << 0;
    res_lo[1] += state_lo[3] << 1;
    res_hi[1] += state_hi[3] << 1;
    res_lo[0] += state_lo[3] << 0;
    res_hi[0] += state_hi[3] << 0;
    
    res_lo[11] += state_lo[2] << 0;
    res_hi[11] += state_hi[2] << 0;
    res_lo[10] += state_lo[2] << 3;
    res_hi[10] += state_hi[2] << 3;
    res_lo[9] += state_lo[2] << 5;
    res_hi[9] += state_hi[2] << 5;
    res_lo[8] += state_lo[2] << 1;
    res_hi[8] += state_hi[2] << 1;
    res_lo[7] += state_lo[2] << 8;
    res_hi[7] += state_hi[2] << 8;
    res_lo[6] += state_lo[2] << 12;
    res_hi[6] += state_hi[2] << 12;
    res_lo[5] += state_lo[2] << 3;
    res_hi[5] += state_hi[2] << 3;
    res_lo[4] += state_lo[2] << 16;
    res_hi[4] += state_hi[2] << 16;
    res_lo[3] += state_lo[2] << 10;
    res_hi[3] += state_hi[2] << 10;
    res_lo[2] += state_lo[2] << 0;
    res_hi[2] += state_hi[2] << 0;
    res_lo[1] += state_lo[2] << 0;
    res_hi[1] += state_hi[2] << 0;
    res_lo[0] += state_lo[2] << 1;
    res_hi[0] += state_hi[2] << 1;
    
    res_lo[11] += state_lo[1] << 1;
    res_hi[11] += state_hi[1] << 1;
    res_lo[10] += state_lo[1] << 0;
    res_hi[10] += state_hi[1] << 0;
    res_lo[9] += state_lo[1] << 3;
    res_hi[9] += state_hi[1] << 3;
    res_lo[8] += state_lo[1] << 5;
    res_hi[8] += state_hi[1] << 5;
    res_lo[7] += state_lo[1] << 1;
    res_hi[7] += state_hi[1] << 1;
    res_lo[6] += state_lo[1] << 8;
    res_hi[6] += state_hi[1] << 8;
    res_lo[5] += state_lo[1] << 12;
    res_hi[5] += state_hi[1] << 12;
    res_lo[4] += state_lo[1] << 3;
    res_hi[4] += state_hi[1] << 3;
    res_lo[3] += state_lo[1] << 16;
    res_hi[3] += state_hi[1] << 16;
    res_lo[2] += state_lo[1] << 10;
    res_hi[2] += state_hi[1] << 10;
    res_lo[1] += state_lo[1] << 0;
    res_hi[1] += state_hi[1] << 0;
    res_lo[0] += state_lo[1] << 0;
    res_hi[0] += state_hi[1] << 0;
    
    res_lo[11] += state_lo[0] << 0;
    res_hi[11] += state_hi[0] << 0;
    res_lo[10] += state_lo[0] << 1;
    res_hi[10] += state_hi[0] << 1;
    res_lo[9] += state_lo[0] << 0;
    res_hi[9] += state_hi[0] << 0;
    res_lo[8] += state_lo[0] << 3;
    res_hi[8] += state_hi[0] << 3;
    res_lo[7] += state_lo[0] << 5;
    res_hi[7] += state_hi[0] << 5;
    res_lo[6] += state_lo[0] << 1;
    res_hi[6] += state_hi[0] << 1;
    res_lo[5] += state_lo[0] << 8;
    res_hi[5] += state_hi[0] << 8;
    res_lo[4] += state_lo[0] << 12;
    res_hi[4] += state_hi[0] << 12;
    res_lo[3] += state_lo[0] << 3;
    res_hi[3] += state_hi[0] << 3;
    res_lo[2] += state_lo[0] << 16;
    res_hi[2] += state_hi[0] << 16;
    res_lo[1] += state_lo[0] << 10;
    res_hi[1] += state_hi[0] << 10;
    res_lo[0] += state_lo[0] << 0;
    res_hi[0] += state_hi[0] << 0;
    
    res_hi[11] += res_lo[11] >> 32;
    res_hi[10] += res_lo[10] >> 32;
    res_hi[9] += res_lo[9] >> 32;
    res_hi[8] += res_lo[8] >> 32;
    res_hi[7] += res_lo[7] >> 32;
    res_hi[6] += res_lo[6] >> 32;
    res_hi[5] += res_lo[5] >> 32;
    res_hi[4] += res_lo[4] >> 32;
    res_hi[3] += res_lo[3] >> 32;
    res_hi[2] += res_lo[2] >> 32;
    res_hi[1] += res_lo[1] >> 32;
    res_hi[0] += res_lo[0] >> 32;
    
    res_lo[11] = (res_hi[11] << 32) | (res_lo[11] & 0xffffffff);
    res_lo[10] = (res_hi[10] << 32) | (res_lo[10] & 0xffffffff);
    res_lo[9] = (res_hi[9] << 32) | (res_lo[9] & 0xffffffff);
    res_lo[8] = (res_hi[8] << 32) | (res_lo[8] & 0xffffffff);
    res_lo[7] = (res_hi[7] << 32) | (res_lo[7] & 0xffffffff);
    res_lo[6] = (res_hi[6] << 32) | (res_lo[6] & 0xffffffff);
    res_lo[5] = (res_hi[5] << 32) | (res_lo[5] & 0xffffffff);
    res_lo[4] = (res_hi[4] << 32) | (res_lo[4] & 0xffffffff);
    res_lo[3] = (res_hi[3] << 32) | (res_lo[3] & 0xffffffff);
    res_lo[2] = (res_hi[2] << 32) | (res_lo[2] & 0xffffffff);
    res_lo[1] = (res_hi[1] << 32) | (res_lo[1] & 0xffffffff);
    res_lo[0] = (res_hi[0] << 32) | (res_lo[0] & 0xffffffff);
    
    res_hi[11] >>= 32;
    res_hi[10] >>= 32;
    res_hi[9] >>= 32;
    res_hi[8] >>= 32;
    res_hi[7] >>= 32;
    res_hi[6] >>= 32;
    res_hi[5] >>= 32;
    res_hi[4] >>= 32;
    res_hi[3] >>= 32;
    res_hi[2] >>= 32;
    res_hi[1] >>= 32;
    res_hi[0] >>= 32;
    
    res_hi[11] = res_hi[11] * 0xffffffff + res_lo[11];
    res_hi[10] = res_hi[10] * 0xffffffff + res_lo[10];
    res_hi[9] = res_hi[9] * 0xffffffff + res_lo[9];
    res_hi[8] = res_hi[8] * 0xffffffff + res_lo[8];
    res_hi[7] = res_hi[7] * 0xffffffff + res_lo[7];
    res_hi[6] = res_hi[6] * 0xffffffff + res_lo[6];
    res_hi[5] = res_hi[5] * 0xffffffff + res_lo[5];
    res_hi[4] = res_hi[4] * 0xffffffff + res_lo[4];
    res_hi[3] = res_hi[3] * 0xffffffff + res_lo[3];
    res_hi[2] = res_hi[2] * 0xffffffff + res_lo[2];
    res_hi[1] = res_hi[1] * 0xffffffff + res_lo[1];
    res_hi[0] = res_hi[0] * 0xffffffff + res_lo[0];
    
    state[11] = res_hi[11] + 0xffffffff * (res_lo[11] > res_hi[11]);
    state[10] = res_hi[10] + 0xffffffff * (res_lo[10] > res_hi[10]);
    state[9] = res_hi[9] + 0xffffffff * (res_lo[9] > res_hi[9]);
    state[8] = res_hi[8] + 0xffffffff * (res_lo[8] > res_hi[8]);
    state[7] = res_hi[7] + 0xffffffff * (res_lo[7] > res_hi[7]);
    state[6] = res_hi[6] + 0xffffffff * (res_lo[6] > res_hi[6]);
    state[5] = res_hi[5] + 0xffffffff * (res_lo[5] > res_hi[5]);
    state[4] = res_hi[4] + 0xffffffff * (res_lo[4] > res_hi[4]);
    state[3] = res_hi[3] + 0xffffffff * (res_lo[3] > res_hi[3]);
    state[2] = res_hi[2] + 0xffffffff * (res_lo[2] > res_hi[2]);
    state[1] = res_hi[1] + 0xffffffff * (res_lo[1] > res_hi[1]);
    state[0] = res_hi[0] + 0xffffffff * (res_lo[0] > res_hi[0]);
}

void full_round(uint64_t thread *state) {
    sbox_layer_full(state);
    mds_layer(state);
}

void partial_round(uint64_t thread *state) {
    sbox_layer_partial(state);
    mds_layer(state);
}

kernel void poseidon(device const uint64_t* inA,
                     device uint64_t* result,
                     uint index [[thread_position_in_grid]])
{
    uint64_t state[12] = {inA[12 * index], inA[12 * index + 1], inA[12 * index + 2], inA[12 * index + 3],
                          inA[12 * index + 4], inA[12 * index + 5], inA[12 * index + 6], inA[12 * index + 7],
                          inA[12 * index + 8], inA[12 * index + 9], inA[12 * index + 10], inA[12 * index + 11]};
    
    for (int i = 0; i < 4; ++i) {
        full_round(state);
    }
    for (int i = 4; i < 26; ++i) {
        partial_round(state);
    }
    for (int i = 26; i < 30; ++i) {
        full_round(state);
    }
    
    result[12 * index] = state[0];
    result[12 * index + 1] = state[1];
    result[12 * index + 2] = state[2];
    result[12 * index + 3] = state[3];
    result[12 * index + 4] = state[4];
    result[12 * index + 5] = state[5];
    result[12 * index + 6] = state[6];
    result[12 * index + 7] = state[7];
    result[12 * index + 8] = state[8];
    result[12 * index + 9] = state[9];
    result[12 * index + 10] = state[10];
    result[12 * index + 11] = state[11];
}
