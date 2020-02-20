use snarkos_models::curves::{
    AffineCurve,
    Field,
    PrimeField,
    ProjectiveCurve,
    SWModelParameters as Parameters,
    SquareRootField,
};
use snarkos_utilities::{
    bititerator::BitIterator,
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Derivative)]
#[derivative(
    Copy(bound = "P: Parameters"),
    Clone(bound = "P: Parameters"),
    PartialEq(bound = "P: Parameters"),
    Eq(bound = "P: Parameters"),
    Debug(bound = "P: Parameters"),
    Hash(bound = "P: Parameters")
)]
pub struct GroupAffine<P: Parameters> {
    pub x: P::BaseField,
    pub y: P::BaseField,
    pub infinity: bool,
    #[derivative(Debug = "ignore")]
    _params: PhantomData<P>,
}

impl<P: Parameters> Display for GroupAffine<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.infinity {
            write!(f, "GroupAffine(Infinity)")
        } else {
            write!(f, "GroupAffine(x={}, y={})", self.x, self.y)
        }
    }
}

impl<P: Parameters> GroupAffine<P> {
    pub fn new(x: P::BaseField, y: P::BaseField, infinity: bool) -> Self {
        Self {
            x,
            y,
            infinity,
            _params: PhantomData,
        }
    }

    pub fn scale_by_cofactor(&self) -> <Self as AffineCurve>::Projective {
        self.mul_bits(BitIterator::new(P::COFACTOR))
    }

    pub(crate) fn mul_bits<S: AsRef<[u64]>>(&self, bits: BitIterator<S>) -> <Self as AffineCurve>::Projective {
        let mut res = GroupProjective::zero();
        for i in bits {
            res.double_in_place();
            if i {
                res.add_assign_mixed(self)
            }
        }
        res
    }

    /// Attempts to construct an affine point given an x-coordinate. The
    /// point is not guaranteed to be in the prime order subgroup.
    ///
    /// If and only if `greatest` is set will the lexicographically
    /// largest y-coordinate be selected.
    #[allow(dead_code)]
    pub(crate) fn get_point_from_x(x: P::BaseField, greatest: bool) -> Option<Self> {
        // Compute x^3 + ax + b
        let x3b = P::add_b(&((x.square() * &x) + &P::mul_by_a(&x)));

        x3b.sqrt().map(|y| {
            let negy = -y;

            let y = if (y < negy) ^ greatest { y } else { negy };
            Self::new(x, y, false)
        })
    }

    /// Checks that the current point is on the elliptic curve.
    pub fn is_on_curve(&self) -> bool {
        if self.is_zero() {
            true
        } else {
            // Check that the point is on the curve
            let y2 = self.y.square();
            let x3b = P::add_b(&((self.x.square() * &self.x) + &P::mul_by_a(&self.x)));
            y2 == x3b
        }
    }
}

impl<P: Parameters> AffineCurve for GroupAffine<P> {
    type BaseField = P::BaseField;
    type Projective = GroupProjective<P>;
    type ScalarField = P::ScalarField;

    fn zero() -> Self {
        Self::new(Self::BaseField::zero(), Self::BaseField::one(), true)
    }

    fn prime_subgroup_generator() -> Self {
        Self::new(P::AFFINE_GENERATOR_COEFFS.0, P::AFFINE_GENERATOR_COEFFS.1, false)
    }

    fn is_zero(&self) -> bool {
        self.infinity
    }

    fn mul<S: Into<<Self::ScalarField as PrimeField>::BigInt>>(&self, by: S) -> GroupProjective<P> {
        let bits = BitIterator::new(by.into());
        self.mul_bits(bits)
    }

    fn mul_by_cofactor(&self) -> Self {
        self.scale_by_cofactor().into()
    }

    fn mul_by_cofactor_inv(&self) -> Self {
        self.mul(P::COFACTOR_INV).into()
    }

    fn into_projective(&self) -> GroupProjective<P> {
        (*self).into()
    }

    fn is_in_correct_subgroup_assuming_on_curve(&self) -> bool {
        self.mul_bits(BitIterator::new(P::ScalarField::characteristic()))
            .is_zero()
    }

    fn to_x_coordinate(&self) -> Self::BaseField {
        self.x.clone()
    }
}

impl<P: Parameters> Neg for GroupAffine<P> {
    type Output = Self;

    fn neg(self) -> Self {
        if !self.is_zero() {
            Self::new(self.x, -self.y, false)
        } else {
            self
        }
    }
}

impl<P: Parameters> ToBytes for GroupAffine<P> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.x.write(&mut writer)?;
        self.y.write(&mut writer)?;
        self.infinity.write(writer)
    }
}

impl<P: Parameters> FromBytes for GroupAffine<P> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let x = P::BaseField::read(&mut reader)?;
        let y = P::BaseField::read(&mut reader)?;
        let infinity = bool::read(reader)?;
        Ok(Self::new(x, y, infinity))
    }
}

impl<P: Parameters> Default for GroupAffine<P> {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Derivative)]
#[derivative(
    Copy(bound = "P: Parameters"),
    Clone(bound = "P: Parameters"),
    Eq(bound = "P: Parameters"),
    Debug(bound = "P: Parameters"),
    Hash(bound = "P: Parameters")
)]
pub struct GroupProjective<P: Parameters> {
    pub x: P::BaseField,
    pub y: P::BaseField,
    pub z: P::BaseField,
    _params: PhantomData<P>,
}

impl<P: Parameters> Display for GroupProjective<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.into_affine())
    }
}

impl<P: Parameters> PartialEq for GroupProjective<P> {
    fn eq(&self, other: &Self) -> bool {
        if self.is_zero() {
            return other.is_zero();
        }

        if other.is_zero() {
            return false;
        }

        // x1/z1 == x2/z2  <==> x1 * z2 == x2 * z1
        if (self.x * &other.z) != (other.x * &self.z) {
            false
        } else if (self.y * &other.z) != (other.y * &self.z) {
            false
        } else {
            true
        }
    }
}

impl<P: Parameters> Distribution<GroupProjective<P>> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GroupProjective<P> {
        let res = GroupProjective::prime_subgroup_generator() * &P::ScalarField::rand(rng);
        debug_assert!(res.into_affine().is_in_correct_subgroup_assuming_on_curve());
        res
    }
}

impl<P: Parameters> ToBytes for GroupProjective<P> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.x.write(&mut writer)?;
        self.y.write(&mut writer)?;
        self.z.write(writer)
    }
}

impl<P: Parameters> FromBytes for GroupProjective<P> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let x = P::BaseField::read(&mut reader)?;
        let y = P::BaseField::read(&mut reader)?;
        let z = P::BaseField::read(reader)?;
        Ok(Self::new(x, y, z))
    }
}

impl<P: Parameters> Default for GroupProjective<P> {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl<P: Parameters> GroupProjective<P> {
    pub fn new(x: P::BaseField, y: P::BaseField, z: P::BaseField) -> Self {
        Self {
            x,
            y,
            z,
            _params: PhantomData,
        }
    }
}

impl<P: Parameters> ProjectiveCurve for GroupProjective<P> {
    type Affine = GroupAffine<P>;
    type BaseField = P::BaseField;
    type ScalarField = P::ScalarField;

    // The point at infinity is always represented by Z = 0.
    #[inline]
    fn zero() -> Self {
        Self::new(P::BaseField::zero(), P::BaseField::one(), P::BaseField::zero())
    }

    #[inline]
    fn prime_subgroup_generator() -> Self {
        GroupAffine::prime_subgroup_generator().into()
    }

    // The point at infinity is always represented by
    // Z = 0.
    #[inline]
    fn is_zero(&self) -> bool {
        self.z.is_zero()
    }

    #[inline]
    fn is_normalized(&self) -> bool {
        self.is_zero() || self.z.is_one()
    }

    fn batch_normalization(v: &mut [Self]) {
        // Montgomery’s Trick and Fast Implementation of Masked AES
        // Genelle, Prouff and Quisquater
        // Section 3.2

        // First pass: compute [a, ab, abc, ...]
        let mut prod = Vec::with_capacity(v.len());
        let mut tmp = P::BaseField::one();
        for g in v
            .iter_mut()
            // Ignore normalized elements
            .filter(|g| !g.is_normalized())
        {
            tmp.mul_assign(&g.z);
            prod.push(tmp);
        }

        // Invert `tmp`.
        tmp = tmp.inverse().unwrap(); // Guaranteed to be nonzero.

        // Second pass: iterate backwards to compute inverses
        for (g, s) in v
            .iter_mut()
            // Backwards
            .rev()
            // Ignore normalized elements
            .filter(|g| !g.is_normalized())
            // Backwards, skip last element, fill in one for last term.
            .zip(
                prod.into_iter()
                    .rev()
                    .skip(1)
                    .chain(Some(P::BaseField::one())),
            )
        {
            // tmp := tmp * g.z; g.z := tmp * s = 1/z
            let newtmp = tmp * &g.z;
            g.z = tmp * &s;
            tmp = newtmp;
        }

        // Perform affine transformations
        for g in v.iter_mut().filter(|g| !g.is_normalized()) {
            g.x *= &g.z; // x/z^2
            g.y *= &g.z;
            g.z = P::BaseField::one(); // z = 1
        }
    }

    fn double_in_place(&mut self) -> &mut Self {
        if self.is_zero() {
            self
        } else {
            // https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html#doubling-dbl-2007-bl

            // XX = X1^2
            let xx = self.x.square();
            // ZZ = Z1^2
            let zz = self.z.square();
            // w = a*ZZ + 3*XX
            let w = P::mul_by_a(&zz) + &(xx + &xx.double());
            // s = 2*Y1*Z1
            let mut s = self.y * &(self.z);
            s.double_in_place();
            // sss = s^3
            let mut sss = s.square();
            sss *= &s;
            // R = Y1*s
            let r = self.y * &s;
            // RR = R2
            let rr = r.square();
            // B = (X1+R)^2-XX-RR
            let b = (self.x + &r).square() - &xx - &rr;
            // h = w2-2*B
            let h = w.square() - &(b + &b);
            // X3 = h*s
            self.x = h * &s;
            // Y3 = w*(B-h)-2*RR
            self.y = w * &(b - &h) - &(rr + &rr);
            // Z3 = sss
            self.z = sss;

            self
        }
    }

    fn add_assign_mixed(&mut self, other: &Self::Affine) {
        if other.is_zero() {
            return;
        } else if self.is_zero() {
            self.x = other.x;
            self.y = other.y;
            self.z = P::BaseField::one();
            return;
        }
        let mut v = other.x * &self.z;
        let mut u = other.y * &self.z;
        if u == self.y && v == self.x {
            // x1 / z1 == x2 / z2 <==> x1 * z2 == x2 * z1;
            // Here, z2 = 1, so we have x1 == x2 * z1;
            self.double_in_place();
        } else {
            // https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html#addition-madd-1998-cmo
            // u = Y2*Z1-Y1
            u -= &self.y;
            // uu = u^2
            let uu = u.square();
            // v = X2*Z1-X1
            v -= &self.x;
            // vv = v2
            let vv = v.square();
            // vvv = v*vv
            let vvv = v * &vv;
            // r = vv*X1
            let r = vv * &self.x;
            // a = uu*Z1-vvv-2*r
            let a = uu * &self.z - &vvv - &r.double();
            // X3 = v*a
            self.x = v * &a;
            // Y3 = u*(R-A)-vvv*Y1
            self.y = u * &(r - &a) - &(vvv * &self.y);
            // Z3 = vvv*Z1
            self.z = vvv * &self.z;
        }
    }

    fn mul_assign<S: Into<<Self::ScalarField as PrimeField>::BigInt>>(&mut self, other: S) {
        let mut res = Self::zero();

        let mut found_one = false;

        for i in BitIterator::new(other.into()) {
            if found_one {
                res.double_in_place();
            } else {
                found_one = i;
            }

            if i {
                res.add_assign(self);
            }
        }

        *self = res;
    }

    fn into_affine(&self) -> GroupAffine<P> {
        (*self).into()
    }

    fn recommended_wnaf_for_scalar(scalar: <Self::ScalarField as PrimeField>::BigInt) -> usize {
        P::empirical_recommended_wnaf_for_scalar(scalar)
    }

    fn recommended_wnaf_for_num_scalars(num_scalars: usize) -> usize {
        P::empirical_recommended_wnaf_for_num_scalars(num_scalars)
    }
}

impl<P: Parameters> Neg for GroupProjective<P> {
    type Output = Self;

    fn neg(self) -> Self {
        if !self.is_zero() {
            Self::new(self.x, -self.y, self.z)
        } else {
            self
        }
    }
}

impl<'a, P: Parameters> Add<&'a Self> for GroupProjective<P> {
    type Output = Self;

    fn add(self, other: &'a Self) -> Self {
        let mut copy = self;
        copy += other;
        copy
    }
}

impl<'a, P: Parameters> AddAssign<&'a Self> for GroupProjective<P> {
    fn add_assign(&mut self, other: &'a Self) {
        if self.is_zero() {
            *self = *other;
            return;
        }

        if other.is_zero() {
            return;
        }
        // https://www.hyperelliptic.org/EFD/g1p/data/shortw/projective/addition/add-1998-cmo-2

        if self == other {
            self.double_in_place();
        } else {
            // Y1Z2 = Y1*Z2
            let y1z2 = self.y * &other.z;
            // X1Z2 = X1*Z2
            let x1z2 = self.x * &other.z;
            // Z1Z2 = Z1*Z2
            let z1z2 = self.z * &other.z;
            // u = Y2*Z1-Y1Z2
            let u = (self.z * &other.y) - &y1z2;
            // uu = u^2
            let uu = u.square();
            // v = X2*Z1-X1Z2
            let v = (self.z * &other.x) - &x1z2;
            // vv = v^2
            let vv = v.square();
            // vvv = v*vv
            let vvv = v * &vv;
            // R = vv*X1Z2
            let r = vv * &x1z2;
            // A = uu*Z1Z2-vvv-2*R
            let a = (uu * &z1z2) - &(vvv + &r + &r);
            // X3 = v*A
            self.x = v * &a;
            // Y3 = u*(R-A)-vvv*Y1Z2
            self.y = ((r - &a) * &u) - &(vvv * &y1z2);
            // Z3 = vvv*Z1Z2
            self.z = vvv * &z1z2;
        }
    }
}

impl<'a, P: Parameters> Sub<&'a Self> for GroupProjective<P> {
    type Output = Self;

    fn sub(self, other: &'a Self) -> Self {
        let mut copy = self;
        copy -= other;
        copy
    }
}

impl<'a, P: Parameters> SubAssign<&'a Self> for GroupProjective<P> {
    fn sub_assign(&mut self, other: &'a Self) {
        *self += &(-(*other));
    }
}

impl<'a, P: Parameters> Mul<&'a P::ScalarField> for GroupProjective<P> {
    type Output = Self;

    fn mul(self, other: &'a P::ScalarField) -> Self {
        let mut copy = self;
        copy *= other;
        copy
    }
}

impl<'a, P: Parameters> MulAssign<&'a P::ScalarField> for GroupProjective<P> {
    fn mul_assign(&mut self, other: &'a P::ScalarField) {
        <GroupProjective<P> as ProjectiveCurve>::mul_assign(self, other.into_repr());
    }
}

// The affine point X, Y is represented in the jacobian
// coordinates with Z = 1.
impl<P: Parameters> From<GroupAffine<P>> for GroupProjective<P> {
    fn from(p: GroupAffine<P>) -> GroupProjective<P> {
        if p.is_zero() {
            Self::zero()
        } else {
            Self::new(p.x, p.y, P::BaseField::one())
        }
    }
}

// The projective point X, Y, Z is represented in the affine
// coordinates as X/Z^2, Y/Z^3.
impl<P: Parameters> From<GroupProjective<P>> for GroupAffine<P> {
    fn from(p: GroupProjective<P>) -> GroupAffine<P> {
        if p.is_zero() {
            GroupAffine::zero()
        } else if p.z.is_one() {
            // If Z is one, the point is already normalized.
            GroupAffine::new(p.x, p.y, false)
        } else {
            // Z is nonzero, so it must have an inverse in a field.
            let z_inv = p.z.inverse().unwrap();
            let x = p.x * &z_inv;
            let y = p.y * &z_inv;
            GroupAffine::new(x, y, false)
        }
    }
}
