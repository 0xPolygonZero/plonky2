use crate::field_types::Field;
use crate::packed_field::PackedField;

pub fn test_binop<P: PackedField, ResField, ResPacked>(res_field: ResField, res_packed: ResPacked)
where
    ResField: Fn(P::Scalar, P::Scalar) -> P::Scalar,
    ResPacked: Fn(P, P) -> P,
{
    let mut rng = StdRng::seed_from_u64(0);
    let input0_arr: [P::Scalar; P::WIDTH] = repeat_with(|| P::Scalar::rand_from_rng(&mut rng))
        .take(P::WIDTH)
        .try_into()
        .unwrap();
    let input1_arr: [P::Scalar; P::WIDTH] = repeat_with(|| P::Scalar::rand_from_rng(&mut rng))
        .take(P::WIDTH)
        .try_into()
        .unwrap();
    let input0 = P::from_arr(input0_arr);
    let input1 = P::from_arr(input1_arr);
    let output = ResPacked(input0, input1);
    let output_arr = output.as_arr();

    for ((&in0, &in1), &packed_out) in input0_arr.into_iter().zip(input1_arr).zip(output_arr) {
        let field_out = ResField(in0, in1);
        assert_eq!(packed_out, field_out);
    }
}
