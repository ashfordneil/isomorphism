use ::{BiMap, DEFAULT_HASH_MAP_SIZE, MAX_LOAD_FACTOR};
use bitfield::{BitField, DefaultBitField};
use bucket::Bucket;

use std::cmp;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::marker::PhantomData;

/// A builder for the bimap.
#[derive(Debug)]
pub struct BiMapBuilder<LH, RH, B> {
    capacity: usize,
    left_hasher: LH,
    right_hasher: RH,
    bit_field: PhantomData<B>,
}

impl Default for BiMapBuilder<RandomState, RandomState, DefaultBitField> {
    fn default() -> Self {
        BiMapBuilder {
            capacity: DEFAULT_HASH_MAP_SIZE,
            left_hasher: Default::default(),
            right_hasher: Default::default(),
            bit_field: Default::default(),
        }
    }
}

impl BiMapBuilder<RandomState, RandomState, DefaultBitField> {
    /// Create new builder, ready to be configured.
    pub fn new() -> Self {
        Default::default()
    }
}

impl<LH: BuildHasher, RH: BuildHasher, B: BitField> BiMapBuilder<LH, RH, B> {
    /// Sets the initial capacity of the bimap. It is guaranteed that at least capacity elements
    /// can be inserted before the map needs to be resized.
    pub fn capacity(self, capacity: usize) -> Self {
        BiMapBuilder { capacity, ..self }
    }

    /// Sets the hasher used for left values.
    pub fn left_hasher<LH2: BuildHasher>(self, hasher: LH2) -> BiMapBuilder<LH2, RH, B> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: hasher,
            right_hasher: self.right_hasher,
            bit_field: self.bit_field,
        }
    }

    /// Sets the hasher used for right values.
    pub fn right_hasher<RH2: BuildHasher>(self, hasher: RH2) -> BiMapBuilder<LH, RH2, B> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: self.left_hasher,
            right_hasher: hasher,
            bit_field: self.bit_field,
        }
    }

    /// Sets the size of the bitfield used internall by the hopscotch hashing algorithm.
    pub fn bitfield<B2: BitField>(self) -> BiMapBuilder<LH, RH, B2> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: self.left_hasher,
            right_hasher: self.right_hasher,
            bit_field: PhantomData,
        }
    }

    /// Builds the BiMap.
    pub fn finish<L, R>(self) -> BiMap<L, R, LH, RH, B> {
        let capacity = match self.capacity {
            0 => 0,
            cap => (cmp::max(DEFAULT_HASH_MAP_SIZE, cap) as f32 * MAX_LOAD_FACTOR).ceil() as usize,
        };
        BiMap {
            left_data: Bucket::empty_vec(capacity),
            right_data: Bucket::empty_vec(capacity),
            left_hasher: self.left_hasher,
            right_hasher: self.right_hasher,
        }
    }
}
