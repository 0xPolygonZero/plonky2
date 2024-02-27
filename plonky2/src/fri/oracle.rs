#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};
use core::mem::transmute;

use itertools::Itertools;
use plonky2_field::types::Field;
use plonky2_maybe_rayon::*;
use rustacuda::memory::{AsyncCopyDestination, DeviceBuffer};
use rustacuda::prelude::*;

use crate::field::extension::Extendable;
use crate::field::fft::FftRootTable;
use crate::field::packed::PackedField;
use crate::field::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::fri::proof::FriProof;
use crate::fri::prover::fri_proof;
use crate::fri::structure::{FriBatchInfo, FriInstanceInfo};
use crate::fri::FriParams;
use crate::hash::hash_types::RichField;
use crate::hash::merkle_tree::{MerkleCap, MerkleTree};
use crate::iop::challenger::Challenger;
use crate::plonk::config::{GenericConfig, Hasher};
use crate::timed;
use crate::util::reducing::ReducingFactor;
use crate::util::timing::TimingTree;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place, transpose};

/// Four (~64 bit) field elements gives ~128 bit security.
pub const SALT_SIZE: usize = 4;

#[derive(Debug)]
pub struct CudaInnerContext {
    pub stream: rustacuda::stream::Stream,
    pub stream2: rustacuda::stream::Stream,
}

#[derive(Debug)]
#[repr(C)]
pub struct CudaInvContext<F: RichField + Extendable<D>, const D: usize> {
    pub inner: CudaInnerContext,
    pub root_table_device: DeviceBuffer<F>,
    pub root_table_device2: DeviceBuffer<F>,
    pub shift_powers_device: DeviceBuffer<F>,
    pub ctx: Context,
}

/// Represents a FRI oracle, i.e. a batch of polynomials which have been Merklized.
#[derive(Eq, PartialEq, Debug)]
pub struct PolynomialBatch<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    pub polynomials: Vec<PolynomialCoeffs<F>>,
    pub merkle_tree: MerkleTree<F, C::Hasher>,
    pub degree_log: usize,
    pub rate_bits: usize,
    pub blinding: bool,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> Default
    for PolynomialBatch<F, C, D>
{
    fn default() -> Self {
        PolynomialBatch {
            polynomials: Vec::new(),
            merkle_tree: MerkleTree::default(),
            degree_log: 0,
            rate_bits: 0,
            blinding: false,
        }
    }
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    PolynomialBatch<F, C, D>
{
    pub fn from_values(
        values: Vec<PolynomialValues<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Self {
        let coeffs;
        coeffs = timed!(
            timing,
            "IFFT",
            values.into_par_iter().map(|v| v.ifft()).collect::<Vec<_>>()
        );
        // log::info!("coeffs {:?}", coeffs);

        Self::from_coeffs(
            coeffs,
            rate_bits,
            blinding,
            cap_height,
            timing,
            fft_root_table,
        )
    }
    /// Creates a list polynomial commitment for the polynomials interpolating the values in `values`.
    #[cfg(feature = "cuda")]
    pub fn from_values_cuda(
        values: Vec<PolynomialValues<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        poly_num: usize,
        values_num_per_poly: usize,
        ctx: &mut CudaInvContext<F, D>,
    ) -> Self {
        let coeffs;

        {
            // merkle tree for reference
            /*
                            cap_1                             cap_2
                         /        \                        /        \
                   digest_9       digest_10              digest_11     digest_12
                  /      \        /     \              /      \     /        \
            digest_1 digest_2 digest_3 digest_4 digest_5 digest_6 digest_7 digest_8
               |        |       |         |        |        |        |         |
            leaf_1   leaf_2   leaf_3   leaf_4   leaf_5    leaf_6   leaf_7   leaf_8

            important formulas (with using the above example too)

            num of caps = 2^cap_height = 2 ^ 1 = 2 caps
            num of digests = 2* (num of leaves - num of caps) = 2 * (8 - 2) = 12

            In the context of committing to trace polynomials as merkle tree,
            trace polynomials are represented as evaluation over domain H
            we first convert the trace into evaluation over a larger domain D, and create a table out of it.

            The domain is larger by factor of 2^rate_bits
            we can visualize that by our columns getting bigger by factor of 2^rate_bits

            so now number of leaves = `values_num_per_poly` * (1 << rate_bits)
            each leaf is Vec of size `poly_num`. we can visualize a leaf as row of this new table
            */
            // assume always zero for now
            let salt_size = if blinding { SALT_SIZE } else { 0 };
            use rustacuda::memory::DeviceSlice;
            let len_cap = 1 << cap_height;
            let num_digests = 2 * (values_num_per_poly * (1 << rate_bits) - len_cap);
            let num_digests_and_caps = num_digests + len_cap;

            let values_flatten_len = poly_num * values_num_per_poly;
            let ext_values_flatten_len =
                (values_flatten_len + salt_size * values_num_per_poly) * (1 << rate_bits);
            let digests_and_caps_buf_len = num_digests_and_caps;
            let mut digests_and_caps_buf: Vec<
                <<C as GenericConfig<D>>::Hasher as Hasher<F>>::Hash,
            > = Vec::with_capacity(num_digests_and_caps);
            unsafe {
                digests_and_caps_buf.set_len(num_digests_and_caps);
            }

            let pad_extvalues_len = ext_values_flatten_len;
            let mut ext_values_flatten: Vec<F> = Vec::with_capacity(ext_values_flatten_len);
            unsafe {
                ext_values_flatten.set_len(ext_values_flatten_len);
            }
            let ext_values_device_offset = 0;
            let lg_n = log2_strict(values_num_per_poly);
            let n_inv = F::inverse_2exp(lg_n);
            let n_inv_ptr: *const F = &n_inv;
            let mut values_device = unsafe {
                DeviceBuffer::<F>::uninitialized(
                    pad_extvalues_len + ext_values_flatten_len + digests_and_caps_buf.len() * 4,
                )
                .unwrap()
            };
            // root table device to be used for FFT over original domain H
            let root_table_device = &ctx.root_table_device;
            // root table device to be used for FFT over larger domain D
            let root_table_device2 = &ctx.root_table_device2;
            // coset shift needed to evaluate over D
            let shift_powers_device = &ctx.shift_powers_device;

            let mut values_flatten = timed!(
                timing,
                "flat map",
                values
                    .into_par_iter()
                    .flat_map(|poly| poly.values)
                    .collect::<Vec<F>>()
            );

            timed!(timing, "copy values to GPU", unsafe {
                transmute::<&mut DeviceSlice<F>, &mut DeviceSlice<u64>>(
                    &mut values_device[0..values_flatten_len],
                )
                .async_copy_from(
                    transmute::<&Vec<F>, &Vec<u64>>(&values_flatten),
                    &ctx.inner.stream,
                )
                .unwrap();
                ctx.inner.stream.synchronize().unwrap();
            });
            // ifft to retrieve coefficients of trace polynomials
            let ctx_ptr: *mut CudaInnerContext = &mut ctx.inner;
            timed!(timing, "IFFT on GPU", unsafe {
                plonky2_cuda::ifft(
                    values_device.as_mut_ptr() as *mut u64,
                    poly_num as i32,
                    values_num_per_poly as i32,
                    lg_n as i32,
                    root_table_device.as_ptr() as *const u64,
                    n_inv_ptr as *const u64,
                    ctx_ptr as *mut core::ffi::c_void,
                )
            });
            timed!(timing, "copy values to CPU", unsafe {
                transmute::<&mut DeviceSlice<F>, &mut DeviceSlice<u64>>(
                    &mut values_device[0..values_flatten_len],
                )
                .async_copy_to(
                    transmute::<&mut Vec<F>, &mut Vec<u64>>(&mut values_flatten),
                    &ctx.inner.stream2,
                )
                .unwrap();
            });
            // this should internally
            // 1. Evaluate the polynomials over larger domain D with FFT + coset shift
            // 2. build merkle tree out of the table with columns corresponding
            //    to evaluations of polynomials over larger domain D
            // 3. Return the leaves as Vec<Vec<F>> (each vec would be row of the above table)
            // 4. All the digests, including caps
            // TODO: figure out whether above is done, and the offsets from where we can retrieve
            //       leaves and digests.
            timed!(timing, "Building MerkleTree + Transpose with GPU", unsafe {
                plonky2_cuda::merkle_tree_from_coeffs(
                    values_device.as_mut_ptr() as *mut u64,
                    values_device.as_mut_ptr() as *mut u64,
                    poly_num as i32,
                    values_num_per_poly as i32,
                    lg_n as i32,
                    root_table_device.as_ptr() as *const u64,
                    root_table_device2.as_ptr() as *const u64,
                    shift_powers_device.as_ptr() as *const u64,
                    rate_bits as i32,
                    salt_size as i32,
                    cap_height as i32,
                    pad_extvalues_len as i32,
                    ctx_ptr as *mut core::ffi::c_void,
                )
            });

            coeffs = timed!(
                timing,
                "unflat map",
                values_flatten
                    .chunks(values_num_per_poly)
                    .map(|values| PolynomialCoeffs::new(values.to_vec()))
                    .collect::<Vec<_>>()
            );
            // log::info!("coeffs {:?}", coeffs);
            timed!(timing, "copy result", {
                let mut alllen = ext_values_flatten_len;
                assert!(ext_values_flatten.len() == ext_values_flatten_len);

                alllen += pad_extvalues_len;

                let len_with_f = digests_and_caps_buf_len * 4;
                let fs =
                    unsafe { transmute::<&mut Vec<_>, &mut Vec<F>>(&mut digests_and_caps_buf) };
                let leaves =
                    unsafe { transmute::<&mut Vec<_>, &mut Vec<F>>(&mut ext_values_flatten) };

                unsafe {
                    fs.set_len(len_with_f);
                }
                println!(
                    "alllen: {}, digest_and_cap_buf_len: {}, diglen: {}",
                    alllen, len_with_f, digests_and_caps_buf_len
                );
                unsafe {
                    transmute::<&DeviceSlice<F>, &DeviceSlice<u64>>(
                        &values_device[alllen..alllen + len_with_f],
                    )
                    .async_copy_to(
                        transmute::<&mut Vec<F>, &mut Vec<u64>>(fs),
                        &ctx.inner.stream,
                    )
                    .unwrap();
                    ctx.inner.stream.synchronize().unwrap();
                }

                unsafe {
                    fs.set_len(len_with_f / 4);
                }
                unsafe {
                    transmute::<&DeviceSlice<F>, &DeviceSlice<u64>>(
                        &values_device[0..pad_extvalues_len],
                    )
                    .async_copy_to(
                        transmute::<&mut Vec<F>, &mut Vec<u64>>(leaves),
                        &ctx.inner.stream,
                    )
                    .unwrap();
                    ctx.inner.stream.synchronize().unwrap();
                }
            });
            let my_leaves_dev_offset = ext_values_device_offset as isize;
            let merkle_tree = MerkleTree {
                leaves: vec![],
                digests: vec![],
                cap: MerkleCap(digests_and_caps_buf[num_digests..num_digests_and_caps].to_vec()),
                my_leaf_len: poly_num + salt_size,
                my_leaves: ext_values_flatten.into(),
                my_leaves_len: pad_extvalues_len,
                my_leaves_dev_offset,
                my_digests: digests_and_caps_buf.into(),
            };
            Self {
                polynomials: coeffs,
                merkle_tree,
                degree_log: lg_n,
                rate_bits,
                blinding,
            }
        }
        // Self::from_coeffs(
        //     coeffs,
        //     rate_bits,
        //     blinding,
        //     cap_height,
        //     timing,
        //     fft_root_table,
        // )
    }

    /// Creates a list polynomial commitment for the polynomials `polynomials`.
    pub fn from_coeffs(
        polynomials: Vec<PolynomialCoeffs<F>>,
        rate_bits: usize,
        blinding: bool,
        cap_height: usize,
        timing: &mut TimingTree,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Self {
        let degree = polynomials[0].len();
        let lde_values = timed!(
            timing,
            "FFT + blinding",
            Self::lde_values(&polynomials, rate_bits, blinding, fft_root_table)
        );

        let mut leaves = timed!(timing, "transpose LDEs", transpose(&lde_values));
        reverse_index_bits_in_place(&mut leaves);
        // reverse_index_bits_in_place(&mut lde_values);
        // let leaves = timed!(timing, "transpose LDEs", transpose(&lde_values));
        let merkle_tree = timed!(
            timing,
            "build Merkle tree",
            MerkleTree::new(leaves, cap_height)
        );

        Self {
            polynomials,
            merkle_tree,
            degree_log: log2_strict(degree),
            rate_bits,
            blinding,
        }
    }

    fn lde_values(
        polynomials: &[PolynomialCoeffs<F>],
        rate_bits: usize,
        blinding: bool,
        fft_root_table: Option<&FftRootTable<F>>,
    ) -> Vec<Vec<F>> {
        let degree = polynomials[0].len();

        // If blinding, salt with two random elements to each leaf vector.
        let salt_size = if blinding { SALT_SIZE } else { 0 };

        polynomials
            .par_iter()
            .map(|p| {
                assert_eq!(p.len(), degree, "Polynomial degrees inconsistent");
                p.lde(rate_bits)
                    .coset_fft_with_options(F::coset_shift(), Some(rate_bits), fft_root_table)
                    .values
            })
            .chain(
                (0..salt_size)
                    .into_par_iter()
                    .map(|_| F::rand_vec(degree << rate_bits)),
            )
            .collect()
    }

    /// Fetches LDE values at the `index * step`th point.
    pub fn get_lde_values(&self, index: usize, step: usize) -> &[F] {
        let index = index * step;
        let index = reverse_bits(index, self.degree_log + self.rate_bits);
        let slice = {
            if self.merkle_tree.my_leaves.is_empty() {
                self.merkle_tree.leaves[index].as_slice()
            } else {
                &self.merkle_tree.my_leaves[index * self.merkle_tree.my_leaf_len
                    ..(index + 1) * self.merkle_tree.my_leaf_len]
            }
        };
        &slice[..slice.len() - if self.blinding { SALT_SIZE } else { 0 }]
    }

    /// Like `get_lde_values`, but fetches LDE values from a batch of `P::WIDTH` points, and returns
    /// packed values.
    pub fn get_lde_values_packed<P>(&self, index_start: usize, step: usize) -> Vec<P>
    where
        P: PackedField<Scalar = F>,
    {
        let row_wise = (0..P::WIDTH)
            .map(|i| self.get_lde_values(index_start + i, step))
            .collect_vec();

        // This is essentially a transpose, but we will not use the generic transpose method as we
        // want inner lists to be of type P, not Vecs which would involve allocation.
        let leaf_size = row_wise[0].len();
        (0..leaf_size)
            .map(|j| {
                let mut packed = P::ZEROS;
                packed
                    .as_slice_mut()
                    .iter_mut()
                    .zip(&row_wise)
                    .for_each(|(packed_i, row_i)| *packed_i = row_i[j]);
                packed
            })
            .collect_vec()
    }

    /// Produces a batch opening proof.
    pub fn prove_openings(
        instance: &FriInstanceInfo<F, D>,
        oracles: &[&Self],
        challenger: &mut Challenger<F, C::Hasher>,
        fri_params: &FriParams,
        timing: &mut TimingTree,
        ctx: &mut Option<&mut crate::fri::oracle::CudaInvContext<F, D>>,
    ) -> FriProof<F, C::Hasher, D> {
        assert!(D > 1, "Not implemented for D=1.");
        let alpha = challenger.get_extension_challenge::<D>();
        let mut alpha = ReducingFactor::new(alpha);

        // Final low-degree polynomial that goes into FRI.
        let mut final_poly = PolynomialCoeffs::empty();

        // Each batch `i` consists of an opening point `z_i` and polynomials `{f_ij}_j` to be opened at that point.
        // For each batch, we compute the composition polynomial `F_i = sum alpha^j f_ij`,
        // where `alpha` is a random challenge in the extension field.
        // The final polynomial is then computed as `final_poly = sum_i alpha^(k_i) (F_i(X) - F_i(z_i))/(X-z_i)`
        // where the `k_i`s are chosen such that each power of `alpha` appears only once in the final sum.
        // There are usually two batches for the openings at `zeta` and `g * zeta`.
        // The oracles used in Plonky2 are given in `FRI_ORACLES` in `plonky2/src/plonk/plonk_common.rs`.
        for FriBatchInfo { point, polynomials } in &instance.batches {
            // Collect the coefficients of all the polynomials in `polynomials`.
            let polys_coeff = polynomials.iter().map(|fri_poly| {
                &oracles[fri_poly.oracle_index].polynomials[fri_poly.polynomial_index]
            });
            let composition_poly = timed!(
                timing,
                &format!("reduce batch of {} polynomials", polynomials.len()),
                alpha.reduce_polys_base(polys_coeff)
            );
            let mut quotient = composition_poly.divide_by_linear(*point);
            quotient.coeffs.push(F::Extension::ZERO); // pad back to power of two
            alpha.shift_poly(&mut final_poly);
            final_poly += quotient;
        }

        let lde_final_poly = final_poly.lde(fri_params.config.rate_bits);
        let lde_final_values = timed!(
            timing,
            &format!("perform final FFT {}", lde_final_poly.len()),
            lde_final_poly.coset_fft(F::coset_shift().into())
        );

        let fri_proof = fri_proof::<F, C, D>(
            &oracles
                .par_iter()
                .map(|c| &c.merkle_tree)
                .collect::<Vec<_>>(),
            lde_final_poly,
            lde_final_values,
            challenger,
            fri_params,
            timing,
            ctx,
        );

        fri_proof
    }
}
