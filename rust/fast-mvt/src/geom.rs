use usize_cast::IntoUsize as _;

use crate::{MvtCoord, MvtError, MvtResult};

pub(crate) fn signed_area(coords: &[MvtCoord]) -> i64 {
    #[inline]
    fn cross_product(a: MvtCoord, b: MvtCoord) -> i64 {
        i64::from(a.x) * i64::from(b.y) - i64::from(a.y) * i64::from(b.x)
    }

    let Some((&first, rest)) = coords.split_first() else {
        return 0;
    };
    let mut prev = first;
    let mut area = 0_i64;
    for &coord in rest {
        area += cross_product(prev, coord);
        prev = coord;
    }
    area + cross_product(prev, first)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Command {
    MoveTo,
    LineTo,
    ClosePath,
}

impl Command {
    #[cfg(feature = "reader")]
    pub(crate) fn decode(value: u32) -> MvtResult<(Self, usize)> {
        Ok((
            match value & 0x7 {
                1 => Self::MoveTo,
                2 => Self::LineTo,
                7 => Self::ClosePath,
                _ => return Err(MvtError::InvalidGeometry),
            },
            (value >> 3).into_usize(),
        ))
    }

    #[cfg(feature = "writer")]
    pub(crate) fn encode(self, count: u32) -> MvtResult<u32> {
        if count > 0x1fff_ffff {
            return Err(MvtError::CommandCount(count));
        }
        let value = match self {
            Self::MoveTo => 1,
            Self::LineTo => 2,
            Self::ClosePath => 7,
        };
        Ok(value | (count << 3))
    }
}

#[cfg(all(test, feature = "writer", feature = "reader"))]
mod tests {
    use super::*;
    use crate::geom_reader::saturating_add_delta;
    use crate::geom_writer::encode_parameter;

    #[test]
    fn command_values_round_trip() {
        let move_to = Command::MoveTo.encode(1).unwrap();
        assert_eq!(move_to, 9);
        assert_eq!(Command::decode(move_to).unwrap(), (Command::MoveTo, 1));
        assert_eq!(Command::LineTo.encode(3).unwrap(), 26);
        assert_eq!(Command::ClosePath.encode(1).unwrap(), 15);
        assert!(matches!(
            Command::MoveTo.encode(0x2000_0000),
            Err(MvtError::CommandCount(0x2000_0000))
        ));
    }

    #[test]
    fn parameter_values_round_trip() {
        assert_eq!(encode_parameter(25), 50);
        assert_eq!(saturating_add_delta(0, 50), 25);
        assert_eq!(encode_parameter(-5), 9);
        assert_eq!(saturating_add_delta(0, 9), -5);
    }

    #[test]
    fn coordinate_overflow_saturates_at_i32_bounds() {
        assert_eq!(
            saturating_add_delta(i32::MAX, encode_parameter(1)),
            i32::MAX
        );
        assert_eq!(
            saturating_add_delta(i32::MIN, encode_parameter(-1)),
            i32::MIN
        );
    }
}
