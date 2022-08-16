use std::io::Result;
use plonky2_field::extension::Extendable;
use crate::hash::hash_types::RichField;
use crate::gates::gate::GateRef;
use crate::util::serialization::Buffer;

pub trait GateSerializer<F: RichField + Extendable<D>, const D: usize> {
    fn read_gate(&self, buf: &mut Buffer) -> Result<GateRef<F, D>>;
    fn write_gate(&self, buf: &mut Buffer, gate: &GateRef<F, D>) -> Result<()>; 
}

macro_rules! read_gate_impl {
    ($buf:expr, $tag:expr, $($gate_types:ty),*) => {{
        let tag = $tag;
        let buf = $buf;
        let mut i = 0..;
        $(if tag == i.next().unwrap() {
            let gate = <$gate_types as $crate::gates::gate::Gate<F, D>>::deserialize(buf)?;
            Ok($crate::gates::gate::GateRef::<F, D>::new(gate))
        } else)*
        { Err(std::io::Error::from(std::io::ErrorKind::InvalidData)) }
    }}
}

macro_rules! get_gate_tag_impl {
    ($gate_any:expr, $($gate_types:ty),*) => {{
        match $gate_any {
            gate_any => {
                let mut i = 0..;
                $(if let (tag, true) = (i.next().unwrap(), gate_any.is::<$gate_types>()) {
                    Ok(tag)
                } else)*
                {
                    Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
                }
            }
        }
    }}; 
}

#[macro_export]
macro_rules! impl_gate_serializer {
    ($target:ty, $($gate_types:ty),+) => {
        fn read_gate(&self, buf: &mut $crate::util::serialization::Buffer) -> std::io::Result<$crate::gates::gate::GateRef<F, D>> {
            let tag = buf.read_u32()?;
            read_gate_impl!(buf, tag, $($gate_types),+)
        }

        fn write_gate(&self, buf: &mut $crate::util::serialization::Buffer, gate: &$crate::gates::gate::GateRef<F, D>) -> std::io::Result<()> {
            let gate_as_any = gate.as_any();
            let tag = get_gate_tag_impl!(gate_as_any, $($gate_types),+)?;

            buf.write_u32(tag)?;
            gate.0.serialize(buf)?;
            Ok(())
        }
    };
}


pub mod default {
    use plonky2_field::extension::Extendable;
    use crate::util::gate_serialization::GateSerializer;
    use crate::hash::hash_types::RichField;
    use crate::gates::arithmetic_base::ArithmeticGate;
    use crate::gates::arithmetic_extension::ArithmeticExtensionGate;
    use crate::gates::assert_le::AssertLessThanGate;
    use crate::gates::constant::ConstantGate;
    use crate::gates::exponentiation::ExponentiationGate;
    use crate::gates::interpolation::HighDegreeInterpolationGate;
    use crate::gates::low_degree_interpolation::LowDegreeInterpolationGate;
    use crate::gates::multiplication_extension::MulExtensionGate;
    use crate::gates::noop::NoopGate;
    use crate::gates::poseidon::PoseidonGate;
    use crate::gates::poseidon_mds::PoseidonMdsGate;
    use crate::gates::public_input::PublicInputGate;
    use crate::gates::random_access::RandomAccessGate;
    use crate::gates::reducing::ReducingGate;
    use crate::gates::reducing_extension::ReducingExtensionGate;
    
    pub struct DefaultGateSerializer;
    impl<F: RichField + Extendable<D>, const D: usize> GateSerializer<F, D> for DefaultGateSerializer {
        impl_gate_serializer!{
            DefaultGateSerializer,
            ArithmeticGate, 
            ArithmeticExtensionGate<D>,
            AssertLessThanGate<F, D>,
            ConstantGate,
            ExponentiationGate<F, D>,
            HighDegreeInterpolationGate<F, D>,
            LowDegreeInterpolationGate<F, D>,
            MulExtensionGate<D>,
            NoopGate,
            PoseidonMdsGate<F, D>,
            PoseidonGate<F, D>,
            PublicInputGate,
            RandomAccessGate<F, D>,
            ReducingExtensionGate<D>,
            ReducingGate<D>
        }
    }
}
