use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;

/// A trait for viewing an evaluation frame of a STARK table.
///
/// It allows to access the current and next rows at a given step
/// and can be used to implement constraint evaluation both natively
/// and recursively.
pub trait StarkEvaluationFrame<F, P>: Sized
where
    F: Field,
    P: PackedField<Scalar = F>,
{
    /// The number of columns for the STARK table this evaluation frame views.
    const COLUMNS: usize;
    const PUBLIC_INPUTS: usize;

    /// Returns the local values (i.e. current row) for this evaluation frame.
    fn get_local_values(&self) -> &[P];
    /// Returns the next values (i.e. next row) for this evaluation frame.
    fn get_next_values(&self) -> &[P];

    fn get_public_inputs(&self) -> &[P::Scalar];

    /// Outputs a new evaluation frame from the provided local and next values.
    ///
    /// **NOTE**: Concrete implementations of this method SHOULD ensure that
    /// the provided slices lengths match the `Self::COLUMNS` value.
    fn from_values(lv: &[P], nv: &[P], public_inputs: &[P::Scalar]) -> Self;
}
pub struct StarkFrame<F, P, const N: usize, const N2: usize>
where
    F: Field,
    P: PackedField<Scalar = F>,
    [(); N]:,
    [(); N2]:,
{
    local_values: [P; N],
    next_values: [P; N],
    public_inputs: [P::Scalar; N2],
}

impl<F, P, const N: usize, const N2: usize> StarkEvaluationFrame<F, P> for StarkFrame<F, P, N, N2>
where
    F: Field,
    P: PackedField<Scalar = F>,
{
    const COLUMNS: usize = N;
    const PUBLIC_INPUTS: usize = N2;

    fn get_local_values(&self) -> &[P] {
        &self.local_values
    }

    fn get_next_values(&self) -> &[P] {
        &self.next_values
    }

    fn get_public_inputs(&self) -> &[P::Scalar] {
        &self.public_inputs
    }

    fn from_values(lv: &[P], nv: &[P], public_inputs: &[P::Scalar]) -> Self {
        assert_eq!(lv.len(), Self::COLUMNS);
        assert_eq!(nv.len(), Self::COLUMNS);
        assert_eq!(public_inputs.len(), Self::PUBLIC_INPUTS);

        Self {
            local_values: lv.try_into().unwrap(),
            next_values: nv.try_into().unwrap(),
            public_inputs: public_inputs.try_into().unwrap(),
        }
    }
}

/// A trait for viewing an evaluation frame of a STARK table.
///
/// It allows to access the current and next rows at a given step
/// and can be used to implement constraint evaluation both natively
/// and recursively.
pub trait StarkEvaluationFrameTarget<T: Copy + Clone + Default>: Sized {
    /// The number of columns for the STARK table this evaluation frame views.
    const COLUMNS: usize;
    const PUBLIC_INPUTS: usize;

    /// Returns the local values (i.e. current row) for this evaluation frame.
    fn get_local_values(&self) -> &[T];
    /// Returns the next values (i.e. next row) for this evaluation frame.
    fn get_next_values(&self) -> &[T];

    fn get_public_inputs(&self) -> &[T];

    /// Outputs a new evaluation frame from the provided local and next values.
    ///
    /// **NOTE**: Concrete implementations of this method SHOULD ensure that
    /// the provided slices lengths match the `Self::COLUMNS` value.
    fn from_values(lv: &[T], nv: &[T], public_inputs: &[T]) -> Self;
}

pub struct StarkFrameTarget<T: Copy + Clone + Default, const N: usize, const N2: usize> {
    local_values: [T; N],
    next_values: [T; N],
    public_inputs: [T; N2],
}

impl<T: Copy + Clone + Default, const N: usize, const N2: usize> StarkEvaluationFrameTarget<T>
    for StarkFrameTarget<T, N, N2>
{
    const COLUMNS: usize = N;
    const PUBLIC_INPUTS: usize = N2;

    fn get_local_values(&self) -> &[T] {
        &self.local_values
    }

    fn get_next_values(&self) -> &[T] {
        &self.next_values
    }

    fn get_public_inputs(&self) -> &[T] {
        &self.public_inputs
    }

    fn from_values(lv: &[T], nv: &[T], public_inputs: &[T]) -> Self {
        assert_eq!(lv.len(), Self::COLUMNS);
        assert_eq!(nv.len(), Self::COLUMNS);
        assert_eq!(public_inputs.len(), Self::PUBLIC_INPUTS);

        Self {
            local_values: lv.try_into().unwrap(),
            next_values: nv.try_into().unwrap(),
            public_inputs: public_inputs.try_into().unwrap(),
        }
    }
}
