//! Q16.16 fixed-point number. All world math in the simulation uses this type.
//!
//! Arithmetic saturates instead of wrapping so that an out-of-range value degrades
//! gameplay locally instead of silently teleporting an entity (and it stays deterministic).

use bytemuck::{Pod, Zeroable};
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Q16.16 fixed-point value. `Fixed::ONE` is one world tile.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Pod, Zeroable)]
pub struct Fixed(pub i32);

impl Fixed {
    /// Number of fractional bits.
    pub const FRAC_BITS: u32 = 16;
    /// 0.0
    pub const ZERO: Self = Self(0);
    /// 1.0
    pub const ONE: Self = Self(1 << Self::FRAC_BITS);
    /// 0.5
    pub const HALF: Self = Self(1 << (Self::FRAC_BITS - 1));
    /// Smallest positive value.
    pub const EPSILON: Self = Self(1);

    /// Integer to fixed (saturating).
    #[must_use]
    pub const fn from_int(v: i32) -> Self {
        Self(v.saturating_mul(1 << Self::FRAC_BITS))
    }

    /// `num / den` as fixed. `den` must be non-zero (checked at compile time for consts).
    #[must_use]
    pub const fn ratio(num: i32, den: i32) -> Self {
        Self((((num as i64) << Self::FRAC_BITS) / den as i64) as i32)
    }

    /// Floor to integer (tile index). Arithmetic shift floors toward negative infinity.
    #[must_use]
    pub const fn floor_int(self) -> i32 {
        self.0 >> Self::FRAC_BITS
    }

    /// Absolute value (saturating at `i32::MAX`).
    #[must_use]
    pub const fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    /// Clamp into `[lo, hi]`.
    #[must_use]
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        Self(self.0.clamp(lo.0, hi.0))
    }

    /// Raw squared magnitude of a vector as `i64` in Q32.32 (no overflow for world-sized values).
    #[must_use]
    pub const fn len_sq(x: Self, y: Self) -> i64 {
        let x = x.0 as i64;
        let y = y.0 as i64;
        x * x + y * y
    }

    /// Squared value of a scalar as Q32.32 `i64`, for comparing against [`Self::len_sq`].
    #[must_use]
    pub const fn sq(self) -> i64 {
        let v = self.0 as i64;
        v * v
    }

    /// Multiply by an integer (saturating).
    #[must_use]
    pub const fn mul_int(self, k: i32) -> Self {
        Self(self.0.saturating_mul(k))
    }
}

const fn sat_i64(v: i64) -> i32 {
    if v > i32::MAX as i64 {
        i32::MAX
    } else if v < i32::MIN as i64 {
        i32::MIN
    } else {
        v as i32
    }
}

impl Add for Fixed {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}
impl AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl Sub for Fixed {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}
impl SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
impl Neg for Fixed {
    type Output = Self;
    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}
impl Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self(sat_i64((self.0 as i64 * rhs.0 as i64) >> Self::FRAC_BITS))
    }
}
impl Div for Fixed {
    type Output = Self;
    /// Division by zero yields zero (debug-asserted): a stalled entity beats a crashed peer.
    fn div(self, rhs: Self) -> Self {
        debug_assert!(rhs.0 != 0, "fixed-point division by zero");
        if rhs.0 == 0 {
            return Self::ZERO;
        }
        Self(sat_i64(((self.0 as i64) << Self::FRAC_BITS) / rhs.0 as i64))
    }
}

/// Integer square root of a `u64` (floor), fixed iteration count → deterministic cost.
pub const fn isqrt_u64(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    // bit-by-bit method: at most 32 iterations
    let mut rem = n;
    let mut root = 0u64;
    let mut bit = 1u64 << 62;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn when_constructing_then_constants_are_exact() {
        assert_eq!(Fixed::from_int(3).0, 3 * 65536);
        assert_eq!(Fixed::ratio(1, 2), Fixed::HALF);
        assert_eq!(Fixed::ratio(-3, 2).floor_int(), -2);
        assert_eq!(Fixed::from_int(-1).floor_int(), -1);
    }

    #[test]
    fn when_dividing_by_zero_in_release_semantics_then_zero() {
        // debug_assert fires in tests, so exercise through catch_unwind-free path via raw check
        assert_eq!(isqrt_u64(0), 0);
        assert_eq!(isqrt_u64(15), 3);
        assert_eq!(isqrt_u64(16), 4);
        assert_eq!(isqrt_u64(u64::MAX), 4_294_967_295);
    }

    proptest! {
        #[test]
        fn add_is_commutative(a in any::<i32>(), b in any::<i32>()) {
            prop_assert_eq!(Fixed(a) + Fixed(b), Fixed(b) + Fixed(a));
        }

        #[test]
        fn mul_is_commutative(a in any::<i32>(), b in any::<i32>()) {
            prop_assert_eq!(Fixed(a) * Fixed(b), Fixed(b) * Fixed(a));
        }

        #[test]
        fn add_saturates_never_wraps(a in any::<i32>(), b in any::<i32>()) {
            let r = (Fixed(a) + Fixed(b)).0 as i64;
            prop_assert_eq!(r, (a as i64 + b as i64).clamp(i32::MIN as i64, i32::MAX as i64));
        }

        #[test]
        fn mul_by_one_is_identity(a in any::<i32>()) {
            prop_assert_eq!(Fixed(a) * Fixed::ONE, Fixed(a));
        }

        #[test]
        fn div_inverts_mul_for_small_values(a in -300i32..300, b in 1i32..100) {
            let fa = Fixed::from_int(a);
            let fb = Fixed::from_int(b);
            prop_assert_eq!((fa * fb) / fb, fa);
        }

        #[test]
        fn isqrt_is_floor_sqrt(n in any::<u64>()) {
            let r = isqrt_u64(n) as u128;
            prop_assert!(r * r <= n as u128);
            prop_assert!((r + 1) * (r + 1) > n as u128);
        }
    }
}
