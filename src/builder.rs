use {BiMap, DEFAULT_HASH_MAP_SIZE, MAX_LOAD_FACTOR};
use bitfield::{BitField, DefaultBitField};
use bucket::Bucket;

use std::cmp;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::marker::PhantomData;

/// A builder for the bimap. Allows for the parameters used to tune the BiMap to be configured.
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
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// let map: BiMap<String, String> = BiMapBuilder::new().finish();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }
}

impl<LH: BuildHasher, RH: BuildHasher, B: BitField> BiMapBuilder<LH, RH, B> {
    /// Sets the initial capacity of the bimap. It is not guaranteed that at least `capacity`
    /// elements can be inserted before the map needs to be resized, but it is likely. The only
    /// reason the map would need to be resized before that number of elements was inserted is due
    /// to a large number of hash collisions.
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// let map: BiMap<String, String> = BiMapBuilder::new().capacity(1024).finish();
    /// ```
    pub fn capacity(self, capacity: usize) -> Self {
        BiMapBuilder { capacity, ..self }
    }

    /// Sets the hasher used for left values. By default, the hashmap will use the hashing
    /// algorithm used in the standard library hashmap, which is randomly generated and designed to
    /// be resistant to DoS attacks. Changing this hasher may lead to hash collisions and
    /// performance issues, so do so with care.
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let map: BiMap<String, String> = BiMapBuilder::new()
    ///             .left_hasher(RandomState::new())
    ///             .finish();
    /// ```
    pub fn left_hasher<LH2: BuildHasher>(self, hasher: LH2) -> BiMapBuilder<LH2, RH, B> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: hasher,
            right_hasher: self.right_hasher,
            bit_field: self.bit_field,
        }
    }

    /// Sets the hasher used for right values. By default, the hashmap will use the hashing
    /// algorithm used in the standard library hashmap, which is randomly generated and designed to
    /// be resistant to DoS attacks. Changing this hasher may lead to hash collisions and
    /// performance issues, so do so with care.
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let map: BiMap<String, String> = BiMapBuilder::new()
    ///             .right_hasher(RandomState::new())
    ///             .finish();
    /// ```
    pub fn right_hasher<RH2: BuildHasher>(self, hasher: RH2) -> BiMapBuilder<LH, RH2, B> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: self.left_hasher,
            right_hasher: hasher,
            bit_field: self.bit_field,
        }
    }

    /// Sets the size of the bitfield used internall by the hopscotch hashing algorithm. The
    /// hopscotch hashing algorithm guarantees that each key is stored within the same
    /// "neighbourhood" as its ideal location, regardless of hash collisions. The size of the
    /// neighbourhood - and therefore the maximum offset between a key's real location and its
    /// ideal location - is equal to the number of bits in this bitfield type. This can be tuned to
    /// control the expected number of cache misses needed to do a lookup.
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// let map: BiMap<String, String, _, _, u16> = BiMapBuilder::new()
    ///             .bitfield::<u16>()
    ///             .finish();
    /// ```
    pub fn bitfield<B2: BitField>(self) -> BiMapBuilder<LH, RH, B2> {
        BiMapBuilder {
            capacity: self.capacity,
            left_hasher: self.left_hasher,
            right_hasher: self.right_hasher,
            bit_field: PhantomData,
        }
    }

    /// Takes a completely configured builder, and creates a new `BiMap` with the specified
    /// configurations.
    ///
    /// ```
    /// # use bimap::{BiMap, BiMapBuilder};
    /// let map: BiMap<String, String> = BiMapBuilder::new().finish();
    /// ```
    pub fn finish<L, R>(self) -> BiMap<L, R, LH, RH, B> {
        let capacity = match self.capacity {
            0 => 0,
            cap => (cmp::max(DEFAULT_HASH_MAP_SIZE, cap) as f32 * MAX_LOAD_FACTOR).ceil() as usize,
        };
        BiMap {
            len: 0,
            left_data: Bucket::empty_vec(capacity),
            right_data: Bucket::empty_vec(capacity),
            left_hasher: self.left_hasher,
            right_hasher: self.right_hasher,
        }
    }
}
