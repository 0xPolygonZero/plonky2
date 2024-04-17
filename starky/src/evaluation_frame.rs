//! Implementation of constraint evaluation frames for STARKs.

/// A trait for viewing an evaluation frame of a STARK table.
///
/// It allows to access the current and next rows at a given step
/// and can be used to implement constraint evaluation both natively
/// and recursively.
pub trait StarkEvaluationFrame<T: Copy + Clone + Default, U: Copy + Clone + Default>:
    Sized
{
    /// The number of columns for the STARK table this evaluation frame views.
    const COLUMNS: usize;
    /// The number of public inputs for the STARK.
    const PUBLIC_INPUTS: usize;

    /// Returns the local values (i.e. current row) for this evaluation frame.
    fn get_local_values(&self) -> &[T];
    /// Returns the next values (i.e. next row) for this evaluation frame.
    fn get_next_values(&self) -> &[T];

    /// Returns the public inputs for this evaluation frame.
    fn get_public_inputs(&self) -> &[U];

    /// Outputs a new evaluation frame from the provided local and next values.
    ///
    /// **NOTE**: Concrete implementations of this method SHOULD ensure that
    /// the provided slices lengths match the `Self::COLUMNS` value.
    fn from_values(lv: &[T], nv: &[T], pis: &[U]) -> Self;
}

/// An evaluation frame to be used when defining constraints of a STARK system, that
/// implements the [`StarkEvaluationFrame`] trait.
#[derive(Debug)]
pub struct StarkFrame<
    T: Copy + Clone + Default,
    U: Copy + Clone + Default,
    const N: usize,
    const N2: usize,
> {
    local_values: [T; N],
    next_values: [T; N],
    public_inputs: [U; N2],
}

impl<T: Copy + Clone + Default, U: Copy + Clone + Default, const N: usize, const N2: usize>
    StarkEvaluationFrame<T, U> for StarkFrame<T, U, N, N2>
{
    const COLUMNS: usize = N;
    const PUBLIC_INPUTS: usize = N2;

    fn get_local_values(&self) -> &[T] {
        &self.local_values
    }

    fn get_next_values(&self) -> &[T] {
        &self.next_values
    }

    fn get_public_inputs(&self) -> &[U] {
        &self.public_inputs
    }

    fn from_values(lv: &[T], nv: &[T], pis: &[U]) -> Self {
        assert_eq!(lv.len(), Self::COLUMNS);
        assert_eq!(nv.len(), Self::COLUMNS);
        assert_eq!(pis.len(), Self::PUBLIC_INPUTS);

        Self {
            local_values: lv.try_into().unwrap(),
            next_values: nv.try_into().unwrap(),
            public_inputs: pis.try_into().unwrap(),
        }
    }
}
