use rand::Rng;

use crate::bls381_arithmetic::Fp;

#[test]
fn test_bls_mul() -> Result<(), ()> {
    let mut rng = rand::thread_rng();
    let f: Fp = rng.gen::<Fp>();
    let g: Fp = rng.gen::<Fp>();
    let fg = f * g;

    println!("{:#?}", f);
    println!("{:#?}", g);
    println!("{:#?}", fg);

    Ok(())
}
