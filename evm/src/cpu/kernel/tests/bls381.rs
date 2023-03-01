use crate::bls381_arithmetic::Fp;
use rand::Rng;

#[test]
fn test_bls_mul() -> Result<(),()> {
    let mut rng = rand::thread_rng();
    let f: Fp = rng.gen::<Fp>();
    let g: Fp = rng.gen::<Fp>();
    let fg = f*g;

    println!("{:#?}", f);
    println!("{:#?}", g);
    println!("{:#?}", fg);

    Ok(())
}