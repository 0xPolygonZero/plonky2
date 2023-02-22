//! A module to help with WitnessGeneratorRef serialization

use plonky2_field::extension::Extendable;

use crate::hash::hash_types::RichField;
use crate::iop::generator::WitnessGeneratorRef;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::util::serialization::{Buffer, IoResult};

pub trait WitnessGeneratorSerializer<F: RichField + Extendable<D>, const D: usize> {
    fn read_generator(
        &self,
        buf: &mut Buffer,
        common: &CommonCircuitData<F, D>,
    ) -> IoResult<WitnessGeneratorRef<F>>;
    fn write_generator(
        &self,
        buf: &mut Vec<u8>,
        generator: &WitnessGeneratorRef<F>,
    ) -> IoResult<()>;
}

#[macro_export]
macro_rules! read_generator_impl {
    ($buf:expr, $tag:expr, $common:expr, $($generator_types:ty),+) => {{
        let tag = $tag;
        let buf = $buf;
        let mut i = 0..;

        if tag == 0 {
            let generator: $crate::recursion::dummy_circuit::DummyProofGenerator<F, C, D> =
                $crate::recursion::dummy_circuit::DummyProofGenerator::deserialize_with_circuit_data(buf, $common)?;
            return Ok($crate::iop::generator::WitnessGeneratorRef::<F>::new(
                $crate::iop::generator::SimpleGenerator::<F>::adapter(generator),
            ));
        }

        $(if tag == i.next().unwrap() {
        let generator =
            <$generator_types as $crate::iop::generator::SimpleGenerator<F>>::deserialize(buf)?;
        Ok($crate::iop::generator::WitnessGeneratorRef::<F>::new(
            $crate::iop::generator::SimpleGenerator::<F>::adapter(generator),
        ))
        } else)*
        {
            Err($crate::util::serialization::IoError)
        }
    }};
}

#[macro_export]
macro_rules! get_generator_tag_impl {
    ($generator:expr, $($generator_types:ty),+) => {{
        let mut i = 0..;
        $(if let (tag, true) = (i.next().unwrap(), $generator.0.id() == $crate::iop::generator::SimpleGenerator::<F>::id(&<$generator_types>::default())) {
            Ok(tag)
        } else)*
        {
            log::log!(log::Level::Error, "attempted to serialize generator with id {} which is unsupported by this generator serializer", $generator.0.id());
            Err($crate::util::serialization::IoError)
        }
    }};
}

#[macro_export]
macro_rules! impl_generator_serializer {
    ($target:ty, $($generator_types:ty),+) => {
        fn read_generator(
            &self,
            buf: &mut $crate::util::serialization::Buffer,
            common: &$crate::plonk::circuit_data::CommonCircuitData<F, D>,
        ) -> $crate::util::serialization::IoResult<$crate::iop::generator::WitnessGeneratorRef<F>> {
            let tag = $crate::util::serialization::Read::read_u32(buf)?;
            read_generator_impl!(buf, tag, common, $($generator_types),+)
        }

        fn write_generator(
            &self,
            buf: &mut Vec<u8>,
            generator: &$crate::iop::generator::WitnessGeneratorRef<F>,
        ) -> $crate::util::serialization::IoResult<()> {
            let tag = get_generator_tag_impl!(generator, $($generator_types),+)?;

            $crate::util::serialization::Write::write_u32(buf, tag)?;
            generator.0.serialize(buf)?;
            Ok(())
        }
    };
}

pub mod default {
    use core::marker::PhantomData;

    use plonky2_field::extension::Extendable;

    use crate::gadgets::arithmetic::EqualityGenerator;
    use crate::gadgets::arithmetic_extension::QuotientGeneratorExtension;
    use crate::gadgets::range_check::LowHighGenerator;
    use crate::gadgets::split_base::BaseSumGenerator;
    use crate::gadgets::split_join::{SplitGenerator, WireSplitGenerator};
    use crate::gates::arithmetic_base::ArithmeticBaseGenerator;
    use crate::gates::arithmetic_extension::ArithmeticExtensionGenerator;
    use crate::gates::base_sum::BaseSplitGenerator;
    use crate::gates::coset_interpolation::InterpolationGenerator;
    use crate::gates::exponentiation::ExponentiationGenerator;
    use crate::gates::multiplication_extension::MulExtensionGenerator;
    use crate::gates::poseidon::PoseidonGenerator;
    use crate::gates::poseidon_mds::PoseidonMdsGenerator;
    use crate::gates::random_access::RandomAccessGenerator;
    use crate::gates::reducing::ReducingGenerator;
    use crate::gates::reducing_extension::ReducingGenerator as ReducingExtensionGenerator;
    use crate::hash::hash_types::RichField;
    use crate::iop::generator::{
        ConstantGenerator, CopyGenerator, NonzeroTestGenerator, RandomValueGenerator,
    };
    use crate::plonk::config::{AlgebraicHasher, GenericConfig};
    use crate::recursion::dummy_circuit::DummyProofGenerator;
    use crate::util::generator_serialization::WitnessGeneratorSerializer;

    pub struct DefaultGeneratorSerializer<C: GenericConfig<D>, const D: usize> {
        pub _phantom: PhantomData<C>,
    }

    impl<F, C, const D: usize> WitnessGeneratorSerializer<F, D> for DefaultGeneratorSerializer<C, D>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        C::Hasher: AlgebraicHasher<F, C::HCO>,
    {
        impl_generator_serializer! {
            DefaultGateSerializer,
            DummyProofGenerator<F, C, D>,
            ArithmeticBaseGenerator<F, D>,
            ArithmeticExtensionGenerator<F, D>,
            BaseSplitGenerator<2>,
            BaseSumGenerator<2>,
            ConstantGenerator<F>,
            CopyGenerator,
            EqualityGenerator,
            ExponentiationGenerator<F, D>,
            InterpolationGenerator<F, D>,
            LowHighGenerator,
            MulExtensionGenerator<F, D>,
            NonzeroTestGenerator,
            PoseidonGenerator<F, D>,
            PoseidonMdsGenerator<D>,
            QuotientGeneratorExtension<D>,
            RandomAccessGenerator<F, D>,
            RandomValueGenerator,
            ReducingGenerator<D>,
            ReducingExtensionGenerator<D>,
            SplitGenerator,
            WireSplitGenerator
        }
    }
}
