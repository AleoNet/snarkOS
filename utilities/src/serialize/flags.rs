pub trait Flags: Default + Clone + Copy + Sized {
    fn u8_bitmask(&self) -> u8;

    fn from_u8(value: u8) -> Self;

    fn from_u8_remove_flags(value: &mut u8) -> Self;

    fn len() -> usize;
}

/// Flags to be encoded into the serialization.
#[derive(Default, Clone, Copy)]
pub struct EmptyFlags;

impl Flags for EmptyFlags {
    #[inline]
    fn u8_bitmask(&self) -> u8 {
        0
    }

    #[inline]
    fn from_u8(_value: u8) -> Self {
        EmptyFlags
    }

    #[inline]
    fn from_u8_remove_flags(_value: &mut u8) -> Self {
        EmptyFlags
    }

    #[inline]
    fn len() -> usize {
        0
    }
}

/// Flags to be encoded into the serialization.
/// The default flags (empty) should not change the binary representation.
#[derive(Clone, Copy)]
pub enum SWFlags {
    Infinity,
    PositiveY,
    NegativeY,
}

impl SWFlags {
    #[inline]
    pub fn infinity() -> Self {
        SWFlags::Infinity
    }

    #[inline]
    pub fn from_y_sign(is_positive: bool) -> Self {
        if is_positive {
            SWFlags::PositiveY
        } else {
            SWFlags::NegativeY
        }
    }

    #[inline]
    pub fn is_infinity(&self) -> bool {
        match self {
            SWFlags::Infinity => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_positive(&self) -> Option<bool> {
        match self {
            SWFlags::Infinity => None,
            SWFlags::PositiveY => Some(true),
            SWFlags::NegativeY => Some(false),
        }
    }
}

impl Default for SWFlags {
    #[inline]
    fn default() -> Self {
        // NegativeY doesn't change the serialization
        SWFlags::NegativeY
    }
}

impl Flags for SWFlags {
    #[inline]
    fn u8_bitmask(&self) -> u8 {
        let mut mask = 0;
        match self {
            SWFlags::Infinity => mask |= 1 << 6,
            SWFlags::PositiveY => mask |= 1 << 7,
            _ => (),
        }
        mask
    }

    #[inline]
    fn from_u8(value: u8) -> Self {
        let x_sign = (value >> 7) & 1 == 1;
        let is_infinity = (value >> 6) & 1 == 1;
        match (x_sign, is_infinity) {
            (_, true) => SWFlags::Infinity,
            (true, false) => SWFlags::PositiveY,
            (false, false) => SWFlags::NegativeY,
        }
    }

    #[inline]
    fn from_u8_remove_flags(value: &mut u8) -> Self {
        let flags = Self::from_u8(*value);
        *value &= 0x3F;
        flags
    }

    /// Number of bits required for these flags.
    #[inline]
    fn len() -> usize {
        2
    }
}

/// Flags to be encoded into the serialization.
/// The default flags (empty) should not change the binary representation.
#[derive(Clone, Copy)]
pub enum EdwardsFlags {
    PositiveY,
    NegativeY,
}

impl EdwardsFlags {
    #[inline]
    pub fn from_y_sign(is_positive: bool) -> Self {
        if is_positive {
            EdwardsFlags::PositiveY
        } else {
            EdwardsFlags::NegativeY
        }
    }

    #[inline]
    pub fn is_positive(&self) -> bool {
        match self {
            EdwardsFlags::PositiveY => true,
            EdwardsFlags::NegativeY => false,
        }
    }
}

impl Default for EdwardsFlags {
    #[inline]
    fn default() -> Self {
        // NegativeY doesn't change the serialization
        EdwardsFlags::NegativeY
    }
}

impl Flags for EdwardsFlags {
    #[inline]
    fn u8_bitmask(&self) -> u8 {
        let mut mask = 0;
        match self {
            EdwardsFlags::PositiveY => mask |= 1 << 7,
            EdwardsFlags::NegativeY => (),
        }
        mask
    }

    #[inline]
    fn from_u8(value: u8) -> Self {
        let x_sign = (value >> 7) & 1 == 1;
        if x_sign {
            EdwardsFlags::PositiveY
        } else {
            EdwardsFlags::NegativeY
        }
    }

    #[inline]
    fn from_u8_remove_flags(value: &mut u8) -> Self {
        let flags = Self::from_u8(*value);
        *value &= 0x7F;
        flags
    }

    /// Number of bits required for these flags.
    #[inline]
    fn len() -> usize {
        1
    }
}
