use std::sync::Arc;

use unroll::unroll_for_loops;

use crate::field::field::Field;

pub fn gmimc_compress<F: Field, const R: usize>(a: [F; 4], b: [F; 4], constants: Arc<[F; R]>) -> [F; 4] {
    // Sponge with r=8, c=4.
    let state_0 = [a[0], a[1], a[2], a[3], b[0],
        b[1], b[2], b[3],
        F::ZERO, F::ZERO, F::ZERO, F::ZERO];
    let state_1 = gmimc_permute::<F, 12, R>(state_0, constants.clone());
    [state_1[0], state_1[1], state_1[2], state_1[3]]
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
    use crate::field::crandall_field::CrandallField;
    use crate::field::Field;
    use crate::gmimc::{gmimc_permute, gmimc_permute_naive};

    #[test]
    fn consistency() {
        type F = CrandallField;
        let mut xs = [F::ZERO; 12];
        for i in 0..12 {
            xs[i] = F::from_canonical_usize(i);
        }
        let out = gmimc_permute::<_, _, 108>(xs);
        let out_naive = gmimc_permute_naive::<_, _, 108>(xs);
        assert_eq!(out, out_naive);
    }
}
