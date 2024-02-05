#include "def.cuh"

__global__
void ifft_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len, const GoldilocksField* root_table, GoldilocksField n_inv);

#if 1

#include <cassert>

__device__ inline
unsigned int bitrev(unsigned int num, const int log_len) {
    unsigned int reversedNum = 0;

    for (int i = 0; i < log_len; ++i) {
        if ((num & (1 << i)) != 0) {
            reversedNum |= 1 << ((log_len - 1) - i);
        }
    }

    return reversedNum;
}

__device__
void reverse_index_bits(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len)
{

    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    assert((1 << log_len) == values_num_per_poly);
    assert(thCnt >= poly_num);

    int perpoly_thcnt = thCnt / poly_num;
    int poly_idx     = gid / perpoly_thcnt;
    int value_idx    = gid % perpoly_thcnt;

    assert(poly_idx < poly_num);

    for (unsigned i = value_idx; i < values_num_per_poly; i += perpoly_thcnt) {
        unsigned idx = i;

        unsigned ridx = bitrev(idx, log_len);
        GoldilocksField *values = values_flatten + values_num_per_poly*poly_idx;
        assert(ridx < values_num_per_poly);
        if (idx < ridx) {
            auto tmp = values[idx];
            values[idx] = values[ridx];
            values[ridx] = tmp;
        }
    }

    __syncthreads();
}

__global__
void reverse_index_bits_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len) {
    reverse_index_bits(values_flatten, poly_num, values_num_per_poly, log_len);
}

__device__
void fft_dispatch(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len, const GoldilocksField* root_table, int r) {
    reverse_index_bits(values_flatten, poly_num, values_num_per_poly, log_len);
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    assert((1 << log_len) == values_num_per_poly);
    assert(thCnt >= poly_num);

    int perpoly_thcnt = thCnt / poly_num;
    int poly_idx     = gid / perpoly_thcnt;
    int value_idx    = gid % perpoly_thcnt;

    assert(poly_idx < poly_num);

    int lg_packed_width = 0;
    int packed_n = values_num_per_poly;

    GoldilocksField* packed_values = values_flatten + values_num_per_poly*poly_idx;

    if (r > 0) {
        // if r == 0 then this loop is a noop.
        uint64_t mask = ~((1 << r) - 1);
        for (int i = value_idx; i < values_num_per_poly; i += perpoly_thcnt) {
            if (i % (1<<r) > 0) {
                assert(packed_values[i].data == 0);
            }
            packed_values[i] = packed_values[i & mask];
        }
        __syncthreads();
    }

    int lg_half_m = r;
    for (; lg_half_m < log_len; ++lg_half_m) {
        int lg_m = lg_half_m + 1;
        int m = 1 << lg_m; // Subarray size (in field elements).
        int packed_m = m >> lg_packed_width; // Subarray size (in vectors).
        int half_packed_m = packed_m / 2;
        assert(half_packed_m != 0);

        const GoldilocksField* omega_table = root_table + ((1<<lg_half_m) - 1);
        if (lg_half_m > 0)
            omega_table += 1;

        for (int k = value_idx; k < packed_n/2;  k += perpoly_thcnt) {
            int kk = (k*2 / packed_m) * packed_m;
            int j  = k*2%packed_m / 2;
            GoldilocksField omega = omega_table[j];
            GoldilocksField t = omega * packed_values[kk + half_packed_m + j];
            GoldilocksField u = packed_values[kk + j];
            packed_values[kk + j] = u + t;
            packed_values[kk + half_packed_m + j] = u - t;
        }
        __syncthreads();
    }
}

__global__
void ifft_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len, const GoldilocksField* root_table, GoldilocksField n_inv) {
    fft_dispatch(values_flatten, poly_num, values_num_per_poly, log_len, root_table, 0);

    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    assert((1 << log_len) == values_num_per_poly);
    assert(thCnt > poly_num);

    int perpoly_thcnt = thCnt / poly_num;
    int poly_idx     = gid / perpoly_thcnt;
    int value_idx    = gid % perpoly_thcnt;

    assert(perpoly_thcnt % 32 == 0);
    assert(poly_idx < poly_num);

    GoldilocksField* buffer = values_flatten + values_num_per_poly*poly_idx;

    if (value_idx == 0) {
        buffer[0] *= n_inv;
        buffer[values_num_per_poly / 2] *= n_inv;
    }

    if (perpoly_thcnt < values_num_per_poly) {
        for (int i = value_idx; i < values_num_per_poly/2; i += perpoly_thcnt) {
            if (i == 0)
                continue;
            int j = values_num_per_poly - i;
            GoldilocksField coeffs_i = buffer[j] * n_inv;
            GoldilocksField coeffs_j = buffer[i] * n_inv;
            buffer[i] = coeffs_i;
            buffer[j] = coeffs_j;
        }
    } else {
        // This is not good for perf, as perpoly_thcnt > values_num_per_poly menas some thread will not get any work
        int i = value_idx;
        if (i != 0 && i < values_num_per_poly/2) {
            int j = values_num_per_poly - i;
            GoldilocksField coeffs_i = buffer[j] * n_inv;
            GoldilocksField coeffs_j = buffer[i] * n_inv;
            buffer[i] = coeffs_i;
            buffer[j] = coeffs_j;
        }   
    }
}

__global__
void fft_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int log_len, const GoldilocksField* root_table, int r) {
    fft_dispatch(values_flatten, poly_num, values_num_per_poly, log_len, root_table, r);
}


__global__
void lde_kernel(const GoldilocksField* values_flatten, GoldilocksField* ext_values_flatten, int poly_num, int values_num_per_poly, int rate_bits)
{
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    int values_num_per_poly2 = values_num_per_poly * (1<<rate_bits);

    for (int i = gid; i < poly_num*values_num_per_poly; i += thCnt) {
        unsigned idx = i % values_num_per_poly;
        unsigned poly_idx = i / values_num_per_poly;
        assert(poly_idx < poly_num);
        const GoldilocksField *values = values_flatten + values_num_per_poly*poly_idx;
        ext_values_flatten[poly_idx*values_num_per_poly2 + idx] = values[idx];
    }
}

__global__
void init_lde_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int rate_bits)
{
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    assert(thCnt > poly_num);

    int values_num_per_poly2 = values_num_per_poly * (1<<rate_bits);
    for (int i = gid; i < poly_num*values_num_per_poly*7; i += thCnt) {
        unsigned idx = i % (values_num_per_poly*7);
        unsigned poly_idx = i / (values_num_per_poly*7);

        GoldilocksField* values = values_flatten + poly_idx*values_num_per_poly2 + values_num_per_poly;
        values[idx].data = 0;
    }

}
__global__
void mul_shift_kernel(GoldilocksField* values_flatten, int poly_num, int values_num_per_poly, int rate_bits, const GoldilocksField* shift_powers)
{
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    uint64_t values_num_per_poly2 = values_num_per_poly * (1<<rate_bits);
    for (int i = gid; i < poly_num*values_num_per_poly; i += thCnt) {
        unsigned idx = i % values_num_per_poly;
        unsigned poly_idx = i / values_num_per_poly;

        GoldilocksField* values = values_flatten + poly_idx*values_num_per_poly2;
        values[idx] *= shift_powers[idx];
    }
}

static __device__ inline int find_digest_index(int layer, int idx, int cap_len, int digest_len)
{
    int d_idx = 0;
    int d_len = digest_len;
    int c_len = cap_len;

    assert(idx < cap_len/(1<<layer));
    idx *= (1<<layer);

    bool at_right;
    while (c_len > (1<<layer)) {
        assert(d_len % 2 == 0);
        at_right = false;
        if (idx >= c_len / 2) {
            d_idx += d_len/2 +(d_len>2);
            idx -= c_len/2;
            at_right = true;
        }
        c_len = c_len/2;
        d_len = d_len/2 - 1;
    }

    if (layer > 0) {
        d_len = 2*(d_len+1);
        if (at_right) {
            d_idx -= 1;
        } else
            d_idx += d_len/2 - 1;
    }
    assert(d_idx < digest_len && d_idx >= 0);
    return d_idx;
}
__global__
void hash_leaves_kernel(GoldilocksField* values_flatten, int poly_num, int leaves_len,
                        PoseidonHasher::HashOut* digest_buf, int len_cap, int num_digests)
{
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    assert(num_digests % len_cap == 0);

    const int cap_len = leaves_len/len_cap;
    const int digest_len = num_digests/len_cap;

    for (int i = gid; i < leaves_len; i += thCnt) {
        GoldilocksField state[SPONGE_WIDTH] = {0};

        for (int j = 0; j < poly_num; j += SPONGE_RATE) {
            for (int k = 0; k < SPONGE_RATE && (j+k)<poly_num; ++k)
                state[k] = *(values_flatten + leaves_len*(j+k) + i);
            PoseidonHasher::permute_poseidon(state);
        }

        const int ith_cap = i / cap_len;
        const int idx = i % cap_len;
        int d_idx = find_digest_index(0, idx, cap_len, digest_len);

        assert((d_idx < digest_len));
        digest_buf[d_idx + ith_cap*digest_len] = *(PoseidonHasher::HashOut*)state;
    }
}

__global__
void reduce_digests_kernel(int leaves_len, PoseidonHasher::HashOut* digest_buf, int len_cap, int num_digests) {
    int thCnt = get_global_thcnt();
    int gid = get_global_id();
    assert(num_digests % len_cap == 0);
    const int percap_thnum = thCnt / len_cap;
    assert(percap_thnum % 32 == 0);

    const int ith_cap = gid / percap_thnum;
    const int cap_idx = gid % percap_thnum;

    int cap_len = leaves_len/len_cap;
    const int digest_len = num_digests/len_cap;

    PoseidonHasher::HashOut* cap_buf = digest_buf + num_digests;
    digest_buf += digest_len * ith_cap;

    const int old_cap_len = cap_len;
    for (int layer = 0; cap_len > 1; ++layer, cap_len /= 2) {
        for (int i = cap_idx; i < cap_len/2; i += percap_thnum) {
            int idx1 = find_digest_index(layer, i*2,    old_cap_len, digest_len);
            int idx2 = find_digest_index(layer, i*2 +1, old_cap_len, digest_len);

            auto h1 = digest_buf[idx1];
            auto h2 = digest_buf[idx2];

            GoldilocksField perm_inputs[SPONGE_WIDTH] = {0};
            *((PoseidonHasher::HashOut*)&perm_inputs[0]) = h1;
            *((PoseidonHasher::HashOut*)&perm_inputs[4]) = h2;

            PoseidonHasher::permute_poseidon(perm_inputs);

            if (cap_len == 2) {
                assert(old_cap_len > (1<<layer));
                cap_buf[ith_cap] = *(PoseidonHasher::HashOut*)perm_inputs;
            } else {
                int idx3 = find_digest_index(layer+1, i, old_cap_len, digest_len);
                digest_buf[idx3] = *(PoseidonHasher::HashOut*)perm_inputs;
            }
        }
        __syncthreads();
    }

}

__global__
void transpose_kernel(GoldilocksField* src_values_flatten, GoldilocksField* dst_values_flatten, int poly_num, int values_num_per_poly)
{
    int thCnt = get_global_thcnt();
    int gid = get_global_id();

    for (int i = gid; i < poly_num*values_num_per_poly; i += thCnt) {
        unsigned val_idx = i / poly_num;
        unsigned poly_idx = i % poly_num;

        GoldilocksField *src_value = src_values_flatten + poly_idx * values_num_per_poly + val_idx;
        GoldilocksField *dst_value = dst_values_flatten + val_idx * poly_num + poly_idx;

        *dst_value = *src_value;
    }
}

#endif
