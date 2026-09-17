//! Axis-separated AABB-vs-tile movement (docs/design-docs/performance.md §4).

use crate::fx::Fixed;
use crate::map::MapData;

/// Move a box of half-size `r` at `(x, y)` by `(dx, dy)` against solid tiles.
/// Returns the new position. Per-call movement must be < 0.5 tile (asserted by config consts).
pub fn move_box(
    map: &MapData,
    x: Fixed,
    y: Fixed,
    dx: Fixed,
    dy: Fixed,
    r: Fixed,
) -> (Fixed, Fixed) {
    let nx = slide_x(map, x, y, dx, r);
    let ny = slide_y(map, nx, y, dy, r);
    (nx, ny)
}

fn slide_x(map: &MapData, x: Fixed, y: Fixed, dx: Fixed, r: Fixed) -> Fixed {
    if dx == Fixed::ZERO {
        return x;
    }
    let nx = x + dx;
    let top = (y - r).floor_int();
    let bottom = (y + r - Fixed::EPSILON).floor_int();
    if dx > Fixed::ZERO {
        let tile = (nx + r - Fixed::EPSILON).floor_int();
        if (top..=bottom).any(|ty| map.solid(tile, ty)) {
            return Fixed::from_int(tile) - r;
        }
    } else {
        let tile = (nx - r).floor_int();
        if (top..=bottom).any(|ty| map.solid(tile, ty)) {
            return Fixed::from_int(tile + 1) + r;
        }
    }
    nx
}

fn slide_y(map: &MapData, x: Fixed, y: Fixed, dy: Fixed, r: Fixed) -> Fixed {
    if dy == Fixed::ZERO {
        return y;
    }
    let ny = y + dy;
    let left = (x - r).floor_int();
    let right = (x + r - Fixed::EPSILON).floor_int();
    if dy > Fixed::ZERO {
        let tile = (ny + r - Fixed::EPSILON).floor_int();
        if (left..=right).any(|tx| map.solid(tx, tile)) {
            return Fixed::from_int(tile) - r;
        }
    } else {
        let tile = (ny - r).floor_int();
        if (left..=right).any(|tx| map.solid(tx, tile)) {
            return Fixed::from_int(tile + 1) + r;
        }
    }
    ny
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_walking_into_left_border_wall_then_stops_at_edge() {
        let map = MapData::by_id(0);
        let r = Fixed::ratio(35, 100);
        let (mut x, y) = (
            Fixed::from_int(2) + Fixed::HALF,
            Fixed::from_int(2) + Fixed::HALF,
        );
        for _ in 0..200 {
            (x, _) = move_box(&map, x, y, Fixed::ratio(-9, 100), Fixed::ZERO, r);
        }
        assert_eq!(
            x,
            Fixed::ONE + r,
            "tile 0 is wall, so box rests at x = 1 + r"
        );
    }
}
