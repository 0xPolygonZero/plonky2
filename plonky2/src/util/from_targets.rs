use crate::iop::target::Target;

pub trait FromTargets<'a, F, const D: usize> {
    type Config;

    fn len(config: &Self::Config) -> usize;

    fn from_targets<I: Iterator<Item = Target>>(
        targets: &mut I,
        common_data: &Self::Config,
    ) -> Self;
}
