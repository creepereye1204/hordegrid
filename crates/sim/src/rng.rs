//! xoshiro128** — small, fast, deterministic PRNG stored inside `State` so it rolls back with it.

use bytemuck::{Pod, Zeroable};

/// Deterministic PRNG state.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct Rng {
    s: [u32; 4],
}

impl Rng {
    /// Seed via splitmix32 so that small/similar seeds still produce well-mixed state.
    pub const fn from_seed(seed: u32) -> Self {
        let mut z = seed;
        let mut s = [0u32; 4];
        let mut i = 0;
        while i < 4 {
            z = z.wrapping_add(0x9E37_79B9);
            let mut x = z;
            x = (x ^ (x >> 16)).wrapping_mul(0x21F0_AAAD);
            x = (x ^ (x >> 15)).wrapping_mul(0x735A_2D97);
            s[i] = x ^ (x >> 15);
            i += 1;
        }
        if s[0] | s[1] | s[2] | s[3] == 0 {
            s[0] = 1; // all-zero state is a fixed point
        }
        Self { s }
    }

    /// Next 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        let s = &mut self.s;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 9;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(11);
        result
    }

    /// Uniform integer in `0..n` (Lemire's multiply-shift; tiny bias irrelevant for gameplay, but deterministic).
    pub fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.next_u32()) * u64::from(n)) >> 32) as u32
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        lo + self.below((hi - lo + 1) as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_same_seed_then_same_sequence() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn when_zero_seed_then_not_stuck() {
        let mut r = Rng::from_seed(0);
        let first = r.next_u32();
        assert!((0..10).any(|_| r.next_u32() != first));
    }

    #[test]
    fn when_below_then_in_range() {
        let mut r = Rng::from_seed(7);
        for _ in 0..10_000 {
            assert!(r.below(8) < 8);
            let v = r.range_i32(-3, 3);
            assert!((-3..=3).contains(&v));
        }
    }
}
