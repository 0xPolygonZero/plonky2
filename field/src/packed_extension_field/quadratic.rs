pub struct PackedQuadraticExtension<P: PackedExtendable<2>>(pub [P; 2]);

impl<P: PackedExtendable<2>> PackedQuadraticExtension<P> {
    fn get(&self) -> [P; 2] {
        let [a, b] = self.0;
        let (x, y) = a.interleave(b, 1);
        [x, y]
    }
    const fn new(vals: [P; 2]) -> Self {
        let [a, b] = vals;
        let (x, y) = a.interleave(b, 1);
        Self([x, y])
    }
}

unsafe impl<P: PackedExtendable<2>> PackedField for PackedQuadraticExtension<P> {
    type Scalar = <P::Scalar as Extendable<2>>::Extension;

    const WIDTH: usize = P::WIDTH;
    const ZEROS: usize = Self::new([P::ZEROS; 2]);
    const ONES: usize = Self::new([P::ONES; P::ZEROS]);

    fn from_arr(arr: [Self::Scalar; Self::WIDTH]) -> Self {
        transmute(arr)
    }
    fn as_arr(&self) -> [Self::Scalar; Self::WIDTH] {
        transmute(*self)
    }

    fn from_slice(slice: &[Self::Scalar]) -> &Self {
        assert_eq!(slice.len(), Self::WIDTH);
        unsafe { &*slice.as_ptr().cast() }
    }
    fn from_slice_mut(slice: &mut [Self::Scalar]) -> &mut Self {
        assert_eq!(slice.len(), Self::WIDTH);
        unsafe { &mut *slice.as_mut_ptr().cast() }
    }
    fn as_slice(&self) -> &[Self::Scalar] {
        todo!()
    }
    fn as_slice_mut(&mut self) -> &mut [Self::Scalar] {
        todo!()
    }

    #[inline]
    fn interleave(&self, other: Self, block_len: usize) -> (Self, Self) {
        todo!()
    }
}

// ========== MATH ==========

impl<P: PackedExtendable<2>> Add for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        let [a0, a1] = self.get();
        let [b0, b1] = self.get();
        Self::new([a0 + b0, a1 + b1])
    }
}

impl<P: PackedExtendable<2>> Sub for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let [a0, a1] = self.get();
        let [b0, b1] = self.get();
        Self::new([a0 - b0, a1 - b1])
    }
}

impl<P: PackedExtendable<2>> Neg for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        let [a0, a1] = self.get();
        Self::new([-a0, -a1])
    }
}

impl<P: PackedExtendable<2>> Mul for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let [a0, a1] = self.get();
        let [b0, b1] = self.get();

        let c0 = a0 * b0 + <P::Scalar as Extendable<2>>::W * a1 * b1;
        let c1 = a0 * b1 + a1 * b0;

        Self::new([c0, c1])
    }
}

impl<P: PackedExtendable<2>> Square for PackedQuadraticExtension<P> {
    #[inline]
    fn square(&self) -> Self {
        let [a0, a1] = self.get();

        let c0 = a0.square() + <P::Scalar as Extendable<2>>::W * a1.square();
        let c1 = a0 * (a1 + a1);

        Self::new([c0, c1])
    }
}

// ========== BOILERPLATE MATH ==========
impl<P: PackedExtendable<2>> Add<Self::Scalar> for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self::Scalar) -> Self {
        self + Self::from(rhs)
    }
}
impl<P: PackedExtendable<2>> Add<PackedQuadraticExtension<P>> for PackedQuadraticExtension<P>::Scalar {
    type Output = PackedQuadraticExtension<P>;
    #[inline]
    fn add(self, rhs: PackedQuadraticExtension<P>) -> PackedQuadraticExtension<P> {
        PackedQuadraticExtension<P>::from(self) + rhs
    }
}
impl<P: PackedExtendable<2>> AddAssign<Self> for PackedQuadraticExtension<P> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl<P: PackedExtendable<2>> AddAssign<Self::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn add_assign(&mut self, rhs: Self::Scalar) {
        *self = *self + rhs;
    }
}

impl<P: PackedExtendable<2>> Sub<Self::Scalar> for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self::Scalar) -> Self {
        self - Self::from(rhs)
    }
}
impl<P: PackedExtendable<2>> Sub<PackedQuadraticExtension<P>> for PackedQuadraticExtension<P>::Scalar {
    type Output = PackedQuadraticExtension<P>;
    #[inline]
    fn sub(self, rhs: PackedQuadraticExtension<P>) -> PackedQuadraticExtension<P> {
        PackedQuadraticExtension<P>::from(self) - rhs
    }
}
impl<P: PackedExtendable<2>> SubAssign<Self> for PackedQuadraticExtension<P> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl<P: PackedExtendable<2>> SubAssign<Self::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self::Scalar) {
        *self = *self - rhs;
    }
}

impl<P: PackedExtendable<2>> Mul<Self::Scalar> for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self::Scalar) -> Self {
        self * Self::from(rhs)
    }
}
impl<P: PackedExtendable<2>> Mul<PackedQuadraticExtension<P>> for PackedQuadraticExtension<P>::Scalar {
    type Output = PackedQuadraticExtension<P>;
    #[inline]
    fn mul(self, rhs: PackedQuadraticExtension<P>) -> PackedQuadraticExtension<P> {
        PackedQuadraticExtension<P>::from(self) * rhs
    }
}
impl<P: PackedExtendable<2>> MulAssign<Self> for PackedQuadraticExtension<P> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl<P: PackedExtendable<2>> MulAssign<Self::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self::Scalar) {
        *self = *self * rhs;
    }
}

impl<P: PackedExtendable<2>> Div<Self::Scalar> for PackedQuadraticExtension<P> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self::Scalar) -> Self {
        self * rhs.inverse()
    }
}
impl<P: PackedExtendable<2>> DivAssign<Self::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn div_assign(&mut self, rhs: Self::Scalar) {
        *self = *self / rhs;
    }
}

// ========== MISCELLANEOUS ==========

impl<P: PackedExtendable<2>> Debug for PackedQuadraticExtension<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.get())
    }
}

impl<P: PackedExtendable<2>> Default for PackedQuadraticExtension<P> {
    #[inline]
    fn default() -> Self {
        Self::ZEROS
    }
}

impl<P: PackedExtendable<2>> From<P> for PackedQuadraticExtension<P> {
    #[inline]
    fn from(x: P) -> Self {
        Self::new([x, P::ZEROS])
    }
}

impl<P: PackedExtendable<2>> From<P::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn from(x: P::Scalar) -> Self {
        Self::new([x.into(), P::ZEROS])
    }
}

impl<P: PackedExtendable<2>> From<Self::Scalar> for PackedQuadraticExtension<P> {
    #[inline]
    fn from(x: Self::Scalar) -> Self {
        Self::new([x.0[0].into(), x.0[1].into()])
    }
}

impl<P: PackedExtendable<2>> Product for PackedQuadraticExtension<P> {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::ONES)
    }
}

impl<P: PackedExtendable<2>> Sum for PackedQuadraticExtension<P> {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::ZEROS)
    }
}
