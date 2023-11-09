use plonky2_field::{polynomial::PolynomialCoeffs, goldilocks_field::GoldilocksField, types::Field};

fn main() {
    let v: Vec<u64> = vec![1,2,3,4,5,6,7,8,9,10,11,12,13,14,15, 16];
    let arr: Vec<GoldilocksField> = v.iter().map(|x| GoldilocksField::from_canonical_u64(*x)).collect();
    let coeffs = PolynomialCoeffs::new(arr);
    let ret = coeffs.fft();
    println!("{:?}", ret);
}