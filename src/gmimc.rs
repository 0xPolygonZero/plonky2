use std::sync::Arc;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use unroll::unroll_for_loops;

use crate::field::field::Field;

pub(crate) fn gmimc_automatic_constants<F: Field, const R: usize>() -> [F; R] {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut constants = [F::ZERO; R];
    for i in 0..R {
        constants[i] = F::rand_from_rng(&mut rng);
    }
    constants
}

pub fn gmimc_compress<F: Field, const R: usize>(
    a: [F; 4],
    b: [F; 4],
    constants: Arc<[F; R]>,
) -> [F; 4] {
    // Sponge with r=8, c=4.
    let state_0 = [
        a[0],
        a[1],
        a[2],
        a[3],
        b[0],
        b[1],
        b[2],
        b[3],
        F::ZERO,
        F::ZERO,
        F::ZERO,
        F::ZERO,
    ];
    let state_1 = gmimc_permute::<F, 12, R>(state_0, constants.clone());
    [state_1[0], state_1[1], state_1[2], state_1[3]]
}

/// Like `gmimc_permute`, but takes constants as an owned array. May be faster.
#[unroll_for_loops]
pub fn gmimc_permute_array<F: Field, const W: usize, const R: usize>(
    mut xs: [F; W],
    constants: [u64; R],
) -> [F; W] {
    // Value that is implicitly added to each element.
    // See https://affine.group/2020/02/starkware-challenge
    let mut addition_buffer = F::ZERO;

    for r in 0..R {
        let active = r % W;
        let f = (xs[active] + addition_buffer + F::from_canonical_u64(constants[r])).cube();
        addition_buffer += f;
        xs[active] -= f;
    }

    for i in 0..W {
        xs[i] += addition_buffer;
    }

    xs
}

#[unroll_for_loops]
pub fn gmimc_permute<F: Field, const W: usize, const R: usize>(
    mut xs: [F; W],
    constants: Arc<[F; R]>,
) -> [F; W] {
    // Value that is implicitly added to each element.
    // See https://affine.group/2020/02/starkware-challenge
    let mut addition_buffer = F::ZERO;

    for r in 0..R {
        let active = r % W;
        let f = (xs[active] + addition_buffer + constants[r]).cube();
        addition_buffer += f;
        xs[active] -= f;
    }

    for i in 0..W {
        xs[i] += addition_buffer;
    }

    xs
}

#[unroll_for_loops]
pub fn gmimc_permute_naive<F: Field, const W: usize, const R: usize>(
    mut xs: [F; W],
    constants: Arc<[F; R]>,
) -> [F; W] {
    for r in 0..R {
        let active = r % W;
        let f = (xs[active] + constants[r]).cube();
        for i in 0..W {
            if i != active {
                xs[i] = xs[i] + f;
            }
        }
    }

    xs
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::gmimc::{gmimc_permute, gmimc_permute_naive};

    #[test]
    fn consistency() {
        type F = CrandallField;
        const W: usize = 12;
        const R: usize = 101;

        let mut constants = [F::ZERO; R];
        for i in 0..R {
            constants[i] = F::from_canonical_usize(i);
        }
        let constants = Arc::new(constants);

        let mut xs = [F::ZERO; W];
        for i in 0..W {
            xs[i] = F::from_canonical_usize(i);
        }

        let out = gmimc_permute::<F, W, R>(xs, constants.clone());
        let out_naive = gmimc_permute_naive::<F, W, R>(xs, constants);
        assert_eq!(out, out_naive);
    }
}
