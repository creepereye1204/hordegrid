//! Player input as a 16-bit command. Layout: docs/generated/net-protocol.md.

use crate::fx::Fixed;

/// One player's input for one frame.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerInput(pub u16);

impl PlayerInput {
    /// Mask for the 4-bit direction (0 = none, 1..=8).
    pub const DIR_MASK: u16 = 0x000F;
    /// Fire held.
    pub const FIRE: u16 = 1 << 4;
    /// Place held (reserved for placeables).
    pub const PLACE: u16 = 1 << 5;
    /// Keep facing while moving.
    pub const STRAFE: u16 = 1 << 6;
    /// Next weapon held.
    pub const WEAPON_NEXT: u16 = 1 << 7;
    /// Previous weapon held.
    pub const WEAPON_PREV: u16 = 1 << 8;
    /// Reload pressed.
    pub const RELOAD: u16 = 1 << 9;
    /// Reserved bits that must be zero.
    pub const RESERVED: u16 = 0x7C00;
    /// Player is connected. GGRS feeds `Default` (0) for disconnected players.
    pub const PRESENT: u16 = 1 << 15;

    /// Sanitize untrusted bits: invalid directions and reserved bits are cleared.
    #[must_use]
    pub const fn sanitized(self) -> Self {
        let mut v = self.0 & !Self::RESERVED;
        if (v & Self::DIR_MASK) > 8 {
            v &= !Self::DIR_MASK;
        }
        Self(v)
    }

    /// Direction index 0..=8.
    pub const fn dir(self) -> u8 {
        (self.0 & Self::DIR_MASK) as u8
    }

    /// Whether a flag is set.
    pub const fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Rising edge of `flag` compared to the previous frame.
    pub const fn pressed(self, prev: Self, flag: u16) -> bool {
        self.has(flag) && !prev.has(flag)
    }
}

const D: Fixed = Fixed::ratio(46_341, 65_536); // cos(45°) in Q16.16

/// Unit vectors for directions 0..=8 (N, NE, E, SE, S, SW, W, NW); screen y grows downward.
pub const DIR8: [(Fixed, Fixed); 9] = [
    (Fixed::ZERO, Fixed::ZERO),
    (Fixed::ZERO, Fixed(-Fixed::ONE.0)),
    (D, Fixed(-D.0)),
    (Fixed::ONE, Fixed::ZERO),
    (D, D),
    (Fixed::ZERO, Fixed::ONE),
    (Fixed(-D.0), D),
    (Fixed(-Fixed::ONE.0), Fixed::ZERO),
    (Fixed(-D.0), Fixed(-D.0)),
];

/// Quantize a delta vector to the nearest of 8 directions (1..=8), 0 if zero.
pub fn dir_from_delta(dx: Fixed, dy: Fixed) -> u8 {
    let (ax, ay) = (
        i64::from(dx.0.unsigned_abs()),
        i64::from(dy.0.unsigned_abs()),
    );
    if ax == 0 && ay == 0 {
        return 0;
    }
    // tan(67.5°) ≈ 2.4142 → 24142/10000
    let horizontal = ax * 10_000 > ay * 24_142;
    let vertical = ay * 10_000 > ax * 24_142;
    match (horizontal, vertical, dx.0 >= 0, dy.0 >= 0) {
        (true, _, true, _) => 3,
        (true, _, false, _) => 7,
        (_, true, _, false) => 1,
        (_, true, _, true) => 5,
        (_, _, true, false) => 2,
        (_, _, true, true) => 4,
        (_, _, false, true) => 6,
        (_, _, false, false) => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_bits_invalid_then_sanitized() {
        assert_eq!(PlayerInput(0x000F).sanitized().dir(), 0);
        assert_eq!(PlayerInput(PlayerInput::RESERVED | 3).sanitized().0, 3);
        assert!(PlayerInput(PlayerInput::RELOAD)
            .sanitized()
            .has(PlayerInput::RELOAD));
        assert!(PlayerInput(PlayerInput::PRESENT)
            .sanitized()
            .has(PlayerInput::PRESENT));
    }

    #[test]
    fn when_quantizing_cardinal_and_diagonal_then_matches_table() {
        for d in 1..=8u8 {
            let (x, y) = DIR8[d as usize];
            assert_eq!(dir_from_delta(x, y), d, "dir {d}");
        }
    }
}
