#include "plonky2_gpu_impl.cuh"

struct CudaInvContext {
    cudaStream_t stream;
    cudaStream_t stream2;
};
template <class T>
struct DataSlice{
    T* ptr;
    int len;
};


#ifndef __CUDA_ARCH__
#include <string>
#include <mutex>

struct RustError { /* to be returned exclusively by value */
    int code;
    char *message;
#ifdef __cplusplus
    RustError(int e = 0) : code(e)
    {   message = nullptr;   }
    RustError(int e, const std::string& str) : code(e)
    {   message = str.empty() ? nullptr : strdup(str.c_str());   }
    RustError(int e, const char *str) : code(e)
    {   message = str==nullptr ? nullptr : strdup(str);   }
    // no destructor[!], Rust takes care of the |message|
#endif
};

#define CUDA_ASSERT(expr) \
do {\
    if (auto code = expr; code != cudaSuccess) {\
        printf("%s@%d failed: %s\n", #expr, __LINE__, cudaGetErrorString(code));\
        return RustError{code};\
    }\
} while(0)

#include <fstream>
#include <vector>

extern "C" {
    RustError ifft(
            GoldilocksField* d_values_flatten,
            int poly_num, int values_num_per_poly, int log_len,
            const GoldilocksField* d_root_table,
            GoldilocksField* p_inv,
            CudaInvContext* ctx
    ) {
        GoldilocksField n_inv = *p_inv;
        auto stream = ctx->stream;

        clock_t start = clock();
        ifft_kernel<<<poly_num, 32*8, 0, stream>>>(d_values_flatten, poly_num, values_num_per_poly, log_len, d_root_table, n_inv);
        cudaStreamSynchronize(stream);
        /* printf("ifft_kernel elapsed: %.2lf\n", (double )(clock()-start) / CLOCKS_PER_SEC * 1000); */

        return RustError{cudaSuccess};
    }

    RustError fft_blinding(
            GoldilocksField* d_values_flatten,
            GoldilocksField* d_ext_values_flatten,
            int poly_num, int values_num_per_poly, int log_len,
            const GoldilocksField* d_root_table2, const GoldilocksField* d_shift_powers,
            int rate_bits,
            int pad_extvalues_len,
            CudaInvContext* ctx
    ) {
        auto stream = ctx->stream;

        int thcnt = 0;
        int nthreads = 32;

        d_ext_values_flatten += pad_extvalues_len;


        clock_t start = clock();
        thcnt = values_num_per_poly * poly_num;
        nthreads = 32;
        lde_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(d_values_flatten, d_ext_values_flatten,
                                                                               poly_num, values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("lde_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly * poly_num;
        nthreads = 32;
        init_lde_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num,
                                                                                    values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("init_lde_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly * poly_num;
        nthreads = 32;
        mul_shift_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num,
                                                                                     values_num_per_poly, rate_bits,
                                                                                     d_shift_powers);
        cudaStreamSynchronize(stream);
        printf("mul_shift_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        fft_kernel<<<poly_num, 32 * 8, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly * (1 << rate_bits),
                                                    log_len + rate_bits, d_root_table2, rate_bits);
        cudaStreamSynchronize(stream);
        printf("fft_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);
    }


    RustError build_merkle_tree(
        GoldilocksField* d_ext_values_flatten,
        int poly_num, int values_num_per_poly, int log_len,
        int rate_bits, int salt_size,
        int cap_height,
        int pad_extvalues_len,
        CudaInvContext* ctx
    ) {
        int values_num_per_extpoly = values_num_per_poly * (1 << rate_bits);
        auto stream = ctx->stream;
        int ext_poly_num = poly_num + salt_size;

        int len_cap = 1 << cap_height;
        int num_digests = 2 * (values_num_per_extpoly - len_cap);

        int thcnt = 0;
        int nthreads = 32;
        d_ext_values_flatten += pad_extvalues_len;


        clock_t start = clock();
        thcnt = values_num_per_extpoly * poly_num;
        nthreads = 32;
        reverse_index_bits_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(d_ext_values_flatten,
                                                                                              poly_num,
                                                                                              values_num_per_extpoly,
                                                                                              log_len + rate_bits);
        cudaStreamSynchronize(stream);
        printf("reverse_index_bits_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);

        int log2_leaves_len = log_len + rate_bits;
        assert(cap_height <= log2_leaves_len);

        auto *d_digest_buf = (PoseidonHasher::HashOut *) (d_ext_values_flatten + values_num_per_extpoly * ext_poly_num);

        start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        hash_leaves_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(
                d_ext_values_flatten, poly_num + salt_size, values_num_per_extpoly, d_digest_buf, len_cap, num_digests);
        cudaStreamSynchronize(stream);
        printf("hash_leaves_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        nthreads = 32 * 8;
        thcnt = len_cap * nthreads;
        reduce_digests_kernel<<<(thcnt + nthreads - 1) / nthreads, nthreads, 0, stream>>>(values_num_per_extpoly,
                                                                                          d_digest_buf, len_cap,
                                                                                          num_digests);
        cudaStreamSynchronize(stream);
        printf("reduce_digests_kernel elapsed: %.2lf\n", (double) (clock() - start) / CLOCKS_PER_SEC * 1000);
    }


    RustError transpose(
            GoldilocksField* d_ext_values_flatten,
            int poly_num, int values_num_per_poly,
            int rate_bits, int salt_size,
            int pad_extvalues_len,
            CudaInvContext* ctx
    ){
        int values_num_per_extpoly = values_num_per_poly*(1<<rate_bits);
        auto stream = ctx->stream;
        int ext_poly_num = poly_num + salt_size;

        int thcnt = 0;
        int nthreads = 32;
        d_ext_values_flatten += pad_extvalues_len;


        clock_t start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        transpose_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, d_ext_values_flatten - pad_extvalues_len, ext_poly_num, values_num_per_extpoly);
        cudaStreamSynchronize(stream);
        printf("transpose_kernel elapsed: %.2lf\n",  (double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        return RustError{cudaSuccess};
    }

    RustError merkle_tree_from_values(
            GoldilocksField* d_values_flatten,
            GoldilocksField* d_ext_values_flatten,
            int poly_num, int values_num_per_poly, int log_len,
           const GoldilocksField* d_root_table, const GoldilocksField* d_root_table2, const GoldilocksField* d_shift_powers,
           GoldilocksField* p_inv, int rate_bits, int salt_size,
           int cap_height,
           int pad_extvalues_len,
           CudaInvContext* ctx
    ){
        assert(0);
        printf("start merkle_tree_from_values: poly_num:%d, values_num_per_poly:%d, log_len:%d, n_inv:%lu\n",
               poly_num, values_num_per_poly, log_len, p_inv->data);
        printf("d_values_flatten: %p, d_ext_values_flatten: %p\n", d_values_flatten, d_ext_values_flatten);

        int values_num_per_extpoly = values_num_per_poly*(1<<rate_bits);
        GoldilocksField n_inv = *p_inv;
        auto stream = ctx->stream;
        int ext_poly_num = poly_num + salt_size;

        int len_cap = 1 << cap_height;
        int num_digests = 2 * (values_num_per_extpoly - len_cap);

        int thcnt = 0;
        int nthreads = 32;
        double ifft_kernel_use, lde_kernel_use, mul_shift_kernel_use, fft_kernel_use, reverse_index_bits_kernel_use,
                hash_leaves_kernel_use, reduce_digests_kernel_use, transpose_kernel_use;

        d_ext_values_flatten += pad_extvalues_len;
        clock_t start = clock();
        ifft_kernel<<<poly_num, 32*8, 0, stream>>>(d_values_flatten, poly_num, values_num_per_poly, log_len, d_root_table, n_inv);
        cudaStreamSynchronize(stream);
        printf("ifft_kernel elapsed: %.2lf\n", ifft_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        lde_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_values_flatten, d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("lde_kernel elapsed: %.2lf\n", lde_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        init_lde_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("init_lde_kernel elapsed: %.2lf\n", (double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        mul_shift_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits, d_shift_powers);
        cudaStreamSynchronize(stream);
        printf("mul_shift_kernel elapsed: %.2lf\n", mul_shift_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        fft_kernel<<<poly_num, 32*8, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly*(1<<rate_bits), log_len+rate_bits, d_root_table2, rate_bits);
        cudaStreamSynchronize(stream);
        printf("fft_kernel elapsed: %.2lf\n", fft_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_extpoly*poly_num;
        nthreads = 32;
        reverse_index_bits_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_extpoly, log_len+rate_bits);
        cudaStreamSynchronize(stream);
        printf("reverse_index_bits_kernel elapsed: %.2lf\n", reverse_index_bits_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        int log2_leaves_len = log_len + rate_bits;
        assert(cap_height <= log2_leaves_len);

        auto *d_digest_buf = (PoseidonHasher::HashOut*)(d_ext_values_flatten + values_num_per_extpoly * ext_poly_num);

        start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        hash_leaves_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(
                d_ext_values_flatten, poly_num+salt_size, values_num_per_extpoly, d_digest_buf, len_cap, num_digests);
        cudaStreamSynchronize(stream);
        printf("hash_leaves_kernel elapsed: %.2lf\n", hash_leaves_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        nthreads = 32*8;
        thcnt = len_cap * nthreads;
        reduce_digests_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(values_num_per_extpoly, d_digest_buf, len_cap, num_digests);
        cudaStreamSynchronize(stream);
        printf("reduce_digests_kernel elapsed: %.2lf\n", reduce_digests_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        transpose_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, d_ext_values_flatten - pad_extvalues_len, ext_poly_num, values_num_per_extpoly);
        cudaStreamSynchronize(stream);
        printf("transpose_kernel elapsed: %.2lf\n", transpose_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        double total_use =
                ifft_kernel_use+
                lde_kernel_use+
                mul_shift_kernel_use+
                fft_kernel_use+
                reverse_index_bits_kernel_use+
                hash_leaves_kernel_use+
                reduce_digests_kernel_use+
                transpose_kernel_use;

        printf("total use:%.2lf\n", total_use);
        return RustError{cudaSuccess};
    }

    RustError merkle_tree_from_coeffs(
            GoldilocksField* d_values_flatten,
            GoldilocksField* d_ext_values_flatten,
            int poly_num, int values_num_per_poly, int log_len,
            const GoldilocksField* d_root_table, const GoldilocksField* d_root_table2, const GoldilocksField* d_shift_powers,
            int rate_bits, int salt_size,
            int cap_height,
            int pad_extvalues_len,
            CudaInvContext* ctx
    ){
        printf("start merkle_tree_from_coeffs: poly_num:%d, values_num_per_poly:%d, log_len:%d\n",
               poly_num, values_num_per_poly, log_len);
        printf("d_values_flatten: %p, d_ext_values_flatten: %p\n", d_values_flatten, d_ext_values_flatten);

        int values_num_per_extpoly = values_num_per_poly*(1<<rate_bits);
        auto stream = ctx->stream;
        int ext_poly_num = poly_num + salt_size;

        int len_cap = 1 << cap_height;
        int num_digests = 2 * (values_num_per_extpoly - len_cap);

        int thcnt = 0;
        int nthreads = 32;
        double lde_kernel_use, mul_shift_kernel_use, fft_kernel_use, reverse_index_bits_kernel_use,
                hash_leaves_kernel_use, reduce_digests_kernel_use, transpose_kernel_use;

        d_ext_values_flatten += pad_extvalues_len;


        clock_t start;

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        lde_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_values_flatten, d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("lde_kernel elapsed: %.2lf\n", lde_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        init_lde_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits);
        cudaStreamSynchronize(stream);
        printf("init_lde_kernel elapsed: %.2lf\n", (double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_poly*poly_num;
        nthreads = 32;
        mul_shift_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly, rate_bits, d_shift_powers);
        cudaStreamSynchronize(stream);
        printf("mul_shift_kernel elapsed: %.2lf\n", mul_shift_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        fft_kernel<<<poly_num, 32*8, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_poly*(1<<rate_bits), log_len+rate_bits, d_root_table2, rate_bits);
        cudaStreamSynchronize(stream);
        printf("fft_kernel elapsed: %.2lf\n", fft_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        thcnt = values_num_per_extpoly*poly_num;
        nthreads = 32;
        reverse_index_bits_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, poly_num, values_num_per_extpoly, log_len+rate_bits);
        cudaStreamSynchronize(stream);
        printf("reverse_index_bits_kernel elapsed: %.2lf\n", reverse_index_bits_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        int log2_leaves_len = log_len + rate_bits;
        assert(cap_height <= log2_leaves_len);

        auto *d_digest_buf = (PoseidonHasher::HashOut*)(d_ext_values_flatten + values_num_per_extpoly * ext_poly_num);

        start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        hash_leaves_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(
                d_ext_values_flatten, poly_num+salt_size, values_num_per_extpoly, d_digest_buf, len_cap, num_digests);
        cudaStreamSynchronize(stream);
        printf("hash_leaves_kernel elapsed: %.2lf\n", hash_leaves_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        start = clock();
        nthreads = 32*8;
        thcnt = len_cap * nthreads;
        reduce_digests_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(values_num_per_extpoly, d_digest_buf, len_cap, num_digests);
        cudaStreamSynchronize(stream);
        printf("reduce_digests_kernel elapsed: %.2lf\n", reduce_digests_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);

        cudaStreamSynchronize(ctx->stream2);

        start = clock();
        thcnt = values_num_per_extpoly;
        nthreads = 32;
        transpose_kernel<<<(thcnt+nthreads-1)/nthreads, nthreads, 0, stream>>>(d_ext_values_flatten, d_ext_values_flatten - pad_extvalues_len, ext_poly_num, values_num_per_extpoly);
        cudaStreamSynchronize(stream);
        printf("transpose_kernel elapsed: %.2lf\n", transpose_kernel_use=(double )(clock()-start) / CLOCKS_PER_SEC * 1000);
        
       /* GoldilocksField * leaves = d_ext_values_flatten -pad_extvalues_len; */
       /* for (int i =0; i < ext_poly_num; ++i) { */
       /*      printf("leaves: "); */
       /*      for (int j=0; j < values_num_per_extpoly; ++j) { */
       /*      printf("%lld, ", leaves+(i * values_num_per_extpoly + j)); */
       /*      } */
       /*      printf("\n"); */
       /* } */

        double total_use =
                lde_kernel_use+
                mul_shift_kernel_use+
                fft_kernel_use+
                reverse_index_bits_kernel_use+
                hash_leaves_kernel_use+
                reduce_digests_kernel_use+
                transpose_kernel_use;

        printf("total use:%.2lf\n", total_use);
        return RustError{cudaSuccess};
    }

}

#endif
