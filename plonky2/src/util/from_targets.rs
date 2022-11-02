use crate::iop::target::Target;

pub trait FromTargets<'a, F, const D: usize> {
    type Config: Copy;

    fn len(config: Self::Config) -> usize;

    fn from_targets<I: Iterator<Item = Target>>(targets: &mut I, config: Self::Config) -> Self;
}

impl<'a, F, const D: usize> FromTargets<'a, F, D> for Target {
    type Config = ();

    fn len(_config: Self::Config) -> usize {
        1
    }

    fn from_targets<I: Iterator<Item = Target>>(targets: &mut I, _config: Self::Config) -> Self {
        targets.next().unwrap()
    }
}

impl<'a, F, T: FromTargets<'a, F, D>, const D: usize> FromTargets<'a, F, D> for Vec<T> {
    type Config = (T::Config, usize); // (config, size)

    fn len(config: Self::Config) -> usize {
        T::len(config.0) * config.1
    }

    fn from_targets<I: Iterator<Item = Target>>(targets: &mut I, config: Self::Config) -> Self {
        (0..config.1)
            .map(|_| T::from_targets(targets, config.0))
            .collect()
    }
}

impl<'a, F, T: FromTargets<'a, F, D>, const D: usize, const N: usize> FromTargets<'a, F, D>
    for [T; N]
{
    type Config = T::Config;

    fn len(config: Self::Config) -> usize {
        T::len(config) * N
    }

    fn from_targets<I: Iterator<Item = Target>>(targets: &mut I, config: Self::Config) -> Self {
        std::array::from_fn(|_| T::from_targets(targets, config))
    }
}

impl<'a, F, T: FromTargets<'a, F, D>, S: FromTargets<'a, F, D>, const D: usize>
    FromTargets<'a, F, D> for (T, S)
{
    type Config = (T::Config, S::Config);

    fn len(config: Self::Config) -> usize {
        T::len(config.0) + S::len(config.1)
    }

    fn from_targets<I: Iterator<Item = Target>>(targets: &mut I, config: Self::Config) -> Self {
        (
            T::from_targets(targets, config.0),
            S::from_targets(targets, config.1),
        )
    }
}
