use std::ops::{Add, AddAssign, Div, Mul, Neg};

#[derive(Debug, Clone, Copy)]
pub struct Rational32(pub i32, pub i32);

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn lcm(a: i32, b: i32) -> i32 {
    let a64 = i64::from(a);
    let b64 = i64::from(b);
    let g64 = i64::from(gcd(a, b));
    let l = (a64 / g64).checked_mul(b64).expect("LCM overflow").abs();
    i32::try_from(l).expect("LCM overflow")
}

impl Rational32 {
    pub fn zero() -> Self {
        Self(0, 1)
    }

    pub fn numer(&self) -> &i32 {
        &self.0
    }

    pub fn denom(&self) -> &i32 {
        &self.1
    }

    pub fn new(num: i32, denom: i32) -> Self {
        if denom == 0 {
            panic!("Denominator cannot be zero");
        }
        if denom < 0 {
            Self(-num, -denom)
        } else {
            Self(num, denom)
        }
    }

    pub fn from_int<T>(num: T) -> Self
    where
        T: Into<i32>,
    {
        Self(num.into(), 1)
    }

    pub fn to_f32(&self) -> Option<f32> {
        if self.1 == 0 {
            None
        } else {
            Some(self.0 as f32 / self.1 as f32)
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn reduce(self) -> Self {
        let g = gcd(self.0, self.1);
        let mut num = self.0 / g;
        let mut den = self.1 / g;
        if den < 0 {
            num = -num;
            den = -den;
        }
        Self(num, den)
    }

    pub fn reduct_to(self, denom: i32) -> Self {
        if denom == 0 {
            panic!("Denominator cannot be zero");
        }
        let reduced = self.reduce();
        let target_denom = lcm(reduced.1.abs(), denom.abs());
        let factor = target_denom / reduced.1.abs();
        let sign = if reduced.1 < 0 { -1 } else { 1 };
        Self(reduced.0 * factor * sign, target_denom)
    }

    pub fn from_integer(s: i32) -> Self {
        Self(s, 1)
    }
}

impl Default for Rational32 {
    fn default() -> Self {
        Self::zero()
    }
}

impl Neg for Rational32 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Rational32(-self.0, self.1)
    }
}

impl Add<Rational32> for Rational32 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let lhs = self.reduce();
        let rhs = rhs.reduce();

        let common_denom = lcm(lhs.1, rhs.1);
        let lhs_factor = common_denom / lhs.1;
        let rhs_factor = common_denom / rhs.1;

        let lhs_num = i64::from(lhs.0) * i64::from(lhs_factor);
        let rhs_num = i64::from(rhs.0) * i64::from(rhs_factor);
        let num = lhs_num
            .checked_add(rhs_num)
            .and_then(|v| i32::try_from(v).ok())
            .expect("Rational addition overflow");

        Rational32(num, common_denom)
    }
}

impl AddAssign<Rational32> for Rational32 {
    fn add_assign(&mut self, rhs: Rational32) {
        *self = *self + rhs;
    }
}

impl Mul<Rational32> for Rational32 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let lhs = self.reduce();
        let rhs = rhs.reduce();

        let g1 = gcd(lhs.0, rhs.1);
        let g2 = gcd(rhs.0, lhs.1);

        let lhs_num = lhs.0 / g1;
        let lhs_den = lhs.1 / g2;
        let rhs_num = rhs.0 / g2;
        let rhs_den = rhs.1 / g1;

        let num = i64::from(lhs_num)
            .checked_mul(i64::from(rhs_num))
            .and_then(|v| i32::try_from(v).ok())
            .expect("Rational multiplication overflow");
        let den = i64::from(lhs_den)
            .checked_mul(i64::from(rhs_den))
            .and_then(|v| i32::try_from(v).ok())
            .expect("Rational multiplication overflow");

        Rational32(num, den)
    }
}

impl Div<Rational32> for Rational32 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.0 == 0 {
            panic!("Cannot divide by zero");
        }
        Rational32(self.0 * rhs.1, self.1 * rhs.0)
    }
}

impl<T> Mul<T> for Rational32
where
    T: Into<i32>,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        let num = i64::from(self.0)
            .checked_mul(i64::from(rhs))
            .and_then(|v| i32::try_from(v).ok())
            .expect("Rational multiplication overflow");
        Rational32(num, self.1)
    }
}

impl From<i32> for Rational32 {
    fn from(value: i32) -> Self {
        Rational32(value, 1)
    }
}

impl From<Rational32> for (i32, i32) {
    fn from(value: Rational32) -> Self {
        (value.0, value.1)
    }
}

impl std::fmt::Display for Rational32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl PartialEq<Rational32> for Rational32 {
    fn eq(&self, other: &Rational32) -> bool {
        let reduced_self = self.reduce();
        let reduced_other = other.reduce();
        reduced_self.0 == reduced_other.0 && reduced_self.1 == reduced_other.1
    }
}

impl PartialOrd<Rational32> for Rational32 {
    fn partial_cmp(&self, other: &Rational32) -> Option<std::cmp::Ordering> {
        let reduced_self = self.reduce();
        let reduced_other = other.reduce();
        (reduced_self.0 * reduced_other.1).partial_cmp(&(reduced_other.0 * reduced_self.1))
    }
}

#[cfg(test)]
mod tests {
    use super::Rational32;

    #[test]
    fn reduce_normalizes_fraction_and_sign() {
        assert_eq!(Rational32(2, 4).reduce(), Rational32(1, 2));
        assert_eq!(Rational32(2, -4).reduce(), Rational32(-1, 2));
        assert_eq!(Rational32(-2, -4).reduce(), Rational32(1, 2));
    }

    #[test]
    fn add_and_add_assign_work() {
        let sum = Rational32(1, 2) + Rational32(1, 3);
        assert_eq!(sum.reduce(), Rational32(5, 6));
        assert_eq!(sum, Rational32(5, 6));

        let mut acc = Rational32(1, 2);
        acc += Rational32(1, 3);
        assert_eq!(acc.reduce(), Rational32(5, 6));
    }

    #[test]
    fn add_uses_lcm_denominator() {
        let sum = Rational32(1, 6) + Rational32(1, 4);
        assert_eq!(sum, Rational32(5, 12));
    }

    #[test]
    fn mul_and_div_produce_expected_results() {
        let product = Rational32(1, 2) * Rational32(1, 3);
        assert_eq!(product.reduce(), Rational32(1, 6));

        let quotient = Rational32(1, 2) / Rational32(2, 3);
        assert_eq!(quotient.reduce(), Rational32(3, 4));
    }

    #[test]
    fn mul_by_integer_and_negation_work() {
        assert_eq!((Rational32(3, 5) * 2).reduce(), Rational32(6, 5));
        assert_eq!((-Rational32(3, 5)).reduce(), Rational32(-3, 5));
    }

    #[test]
    fn ordering_and_equality_use_reduced_form() {
        assert_eq!(Rational32(1, 2), Rational32(2, 4));
        assert!(Rational32(1, 3) < Rational32(1, 2));
        assert!(Rational32(-1, 2) < Rational32(1, 3));
    }

    #[test]
    fn reduct_to_converts_to_compatible_denominator() {
        let converted = Rational32(1, 2).reduct_to(6);
        assert_eq!(converted, Rational32(3, 6));
    }

    #[test]
    fn to_f32_and_zero_helpers_work() {
        assert_eq!(Rational32::zero(), Rational32(0, 1));
        assert!(Rational32::zero().is_zero());

        let value = Rational32(1, 4).to_f32().expect("expected valid f32 value");
        assert!((value - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic(expected = "Denominator cannot be zero")]
    fn new_panics_on_zero_denominator() {
        let _ = Rational32::new(1, 0);
    }

    #[test]
    #[should_panic(expected = "Cannot divide by zero")]
    fn div_panics_on_zero_numerator_rhs() {
        let _ = Rational32(1, 2) / Rational32(0, 3);
    }

    #[test]
    fn reduct_to() {
        assert_eq!(Rational32(1, 2).reduct_to(4), Rational32(2, 4));
        assert_eq!(Rational32(1, 3).reduct_to(6), Rational32(2, 6));
        assert_eq!(Rational32(2, 8).reduct_to(4), Rational32(1, 4));
    }
}
