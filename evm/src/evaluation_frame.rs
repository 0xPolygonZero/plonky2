/// A trait for viewing an evaluation frame of a STARK table.
///
/// It allows to access the current and next rows at a given step
/// and can be used to implement constraint evaluation both natively
/// and recursively.
pub trait StarkEvaluationFrame<T: Copy + Clone + Default>: Sized {
    /// The number of columns for the STARK table this evaluation frame views.
    const COLUMNS: usize;

    /// Returns the local values (i.e. current row) for this evaluation frame.
    fn get_local_values(&self) -> &[T];
    /// Returns the next values (i.e. next row) for this evaluation frame.
    fn get_next_values(&self) -> &[T];

    /// Outputs a new evaluation frame from the provided local and next values.
    ///
    /// **NOTE**: Concrete implementations of this method SHOULD ensure that
    /// the provided slices lengths match the `Self::COLUMNS` value.
    fn from_values(lv: &[T], nv: &[T]) -> Self;
}

pub struct StarkFrame<T: Copy + Clone + Default, const N: usize> {
    local_values: [T; N],
    next_values: [T; N],
}

impl<T: Copy + Clone + Default, const N: usize> StarkEvaluationFrame<T> for StarkFrame<T, N> {
    const COLUMNS: usize = N;

    fn get_local_values(&self) -> &[T] {
        &self.local_values
    }

    fn get_next_values(&self) -> &[T] {
        &self.next_values
    }

    fn from_values(lv: &[T], nv: &[T]) -> Self {
        assert_eq!(lv.len(), Self::COLUMNS);
        assert_eq!(nv.len(), Self::COLUMNS);

        Self {
            local_values: lv.try_into().unwrap(),
            next_values: nv.try_into().unwrap(),
        }
    }
}
