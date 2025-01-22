//! A module to help with GateRef serialization

#[cfg(not(feature = "std"))]
pub use alloc::vec::Vec;
#[cfg(feature = "std")]
pub use std::vec::Vec; // For macros below

pub use log;
use plonky2_field::extension::Extendable;

use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::util::serialization::{Buffer, IoResult};

pub trait GateSerializer<F: RichField + Extendable<D>, const D: usize> {
    fn read_gate(
        &self,
        buf: &mut Buffer,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<GateRef<F, D>>;
    fn write_gate(
        &self,
        buf: &mut Vec<u8>,
        gate: &GateRef<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()>;
}

#[macro_export]
macro_rules! read_gate_impl {
    ($buf:expr, $tag:expr, $common:expr, $($gate_types:ty),+) => {{
        let tag = $tag;
        let buf = $buf;
        let mut i = 0..;
        $(if tag == i.next().unwrap() {
            let gate = <$gate_types as $crate::gates::gate::Gate<F, D>>::deserialize(buf, $common)?;
            Ok($crate::gates::gate::GateRef::<F, D>::new(gate))
        } else)*
        {
            Err($crate::util::serialization::IoError)
        }
    }}
}

#[macro_export]
macro_rules! get_gate_tag_impl {
    ($gate:expr, $($gate_types:ty),+) => {{
        let gate_any = $gate.0.as_any();
        let mut i = 0..;
        $(if let (tag, true) = (i.next().unwrap(), gate_any.is::<$gate_types>()) {
            Ok(tag)
        } else)*
        {
            $crate::util::serialization::gate_serialization::log::log!(
                $crate::util::serialization::gate_serialization::log::Level::Error,
                "attempted to serialize gate with id `{}` which is unsupported by this gate serializer",
                $gate.0.id()
            );
            Err($crate::util::serialization::IoError)
        }
    }};
}

#[macro_export]
/// Macro implementing the [`GateSerializer`] trait.
/// To serialize a list of gates used for a circuit,
/// this macro should be called with a struct on which to implement
/// this as first argument, followed by all the targeted gates.
macro_rules! impl_gate_serializer {
    ($target:ty, $($gate_types:ty),+) => {
        fn read_gate(
            &self,
            buf: &mut $crate::util::serialization::Buffer,
            common: &$crate::plonk::circuit_data::CommonCircuitData<F, D>,
        ) -> $crate::util::serialization::IoResult<$crate::gates::gate::GateRef<F, D>> {
            let tag = $crate::util::serialization::Read::read_u32(buf)?;
            read_gate_impl!(buf, tag, common, $($gate_types),+)
        }

        fn write_gate(
            &self,
            buf: &mut $crate::util::serialization::gate_serialization::Vec<u8>,
            gate: &$crate::gates::gate::GateRef<F, D>,
            common: &$crate::plonk::circuit_data::CommonCircuitData<F, D>,
        ) -> $crate::util::serialization::IoResult<()> {
            let tag = get_gate_tag_impl!(gate, $($gate_types),+)?;

            $crate::util::serialization::Write::write_u32(buf, tag)?;
            gate.0.serialize(buf, common)?;
            Ok(())
        }
    };
}

pub mod default {
    use plonky2_field::extension::Extendable;

    use crate::gates::arithmetic_base::ArithmeticGate;
    use crate::gates::arithmetic_extension::ArithmeticExtensionGate;
    use crate::gates::base_sum::BaseSumGate;
    use crate::gates::constant::ConstantGate;
    use crate::gates::coset_interpolation::CosetInterpolationGate;
    use crate::gates::exponentiation::ExponentiationGate;
    use crate::gates::lookup::LookupGate;
    use crate::gates::lookup_table::LookupTableGate;
    use crate::gates::multiplication_extension::MulExtensionGate;
    use crate::gates::noop::NoopGate;
    use crate::gates::poseidon::PoseidonGate;
    use crate::gates::poseidon_mds::PoseidonMdsGate;
    use crate::gates::public_input::PublicInputGate;
    use crate::gates::random_access::RandomAccessGate;
    use crate::gates::reducing::ReducingGate;
    use crate::gates::reducing_extension::ReducingExtensionGate;
    use crate::hash::hash_types::RichField;
    use crate::util::serialization::GateSerializer;
    /// A gate serializer that can be used to serialize all default gates supported
    /// by the `plonky2` library.
    /// Being a unit struct, it can be simply called as
    /// ```rust
    /// use plonky2::util::serialization::DefaultGateSerializer;
    /// let gate_serializer = DefaultGateSerializer;
    /// ```
    /// Applications using custom gates should define their own serializer implementing
    /// the `GateSerializer` trait. This can be easily done through the `impl_gate_serializer` macro.
    #[derive(Debug)]
    pub struct DefaultGateSerializer;
    impl<F: RichField + Extendable<D>, const D: usize> GateSerializer<F, D> for DefaultGateSerializer {
        impl_gate_serializer! {
            DefaultGateSerializer,
            ArithmeticGate,
            ArithmeticExtensionGate<D>,
            BaseSumGate<2>,
            ConstantGate,
            CosetInterpolationGate<F, D>,
            ExponentiationGate<F, D>,
            LookupGate,
            LookupTableGate,
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
