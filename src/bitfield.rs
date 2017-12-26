use std::ops::{BitAnd, BitOr, Not, Shl};

/// A bit field trait for use in hashmap buckets.
pub trait BitField: BitAnd + BitOr + Sized {

    /// Should return a constant value describing how big the bitfield of this type is.
    fn size() -> usize;

    /// Should return a bitfield that is all zeroes, except for a single one at a given index.
    fn one_at(index: usize) -> Self;

    /// Should return a bitfield that is all ones, except for a single zero at a given index.
    fn zero_at(index: usize) -> Self;
}

/// Helper trait to reduce code duplication when implementing Bitfield for integer types.
pub trait BitSized {
    /// Returns how many bits are in the type.
    fn size() -> usize;
}

impl BitSized for u8 {
    fn size() -> usize {
        8
    }
}

impl BitSized for u16 {
    fn size() -> usize {
        16
    }
}

impl BitSized for u32 {
    fn size() -> usize {
        32
    }
}

impl BitSized for u64 {
    fn size() -> usize {
        64
    }
}

impl <T> BitField for T where
    T: BitSized + BitAnd + BitOr + Not<Output=T> + Shl<usize, Output=T> + From<u8>
{
    fn size() -> usize {
        <T as BitSized>::size()
    }

    fn one_at(index: usize) -> Self {
        Self::from(1u8) << index
    }

    fn zero_at(index: usize) -> Self {
        !Self::one_at(index)
    }
}
