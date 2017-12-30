//! Bidirectional hashmaps!
//! This crate aims to provide a data structure that can take store a 1:1 relation between two
//! different types, and provide constant time lookup within this relation.
//!
//! The hashmaps in this crate use the hopscotch hashing algorithm, mainly because I just wanted to
//! implement it. I'm hoping that the hopscotch hashing algorithm will also make removals from the
//! hashmaps more efficient.

pub mod bitfield;
mod bucket;
pub mod iterator;

use bitfield::{BitField, DefaultBitField};
use bucket::Bucket;
use iterator::{BiMapRefIterator, BiMapIterator};

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};

const DEFAULT_HASH_MAP_SIZE: usize = 32;

// left as a fraction to avoid floating point multiplication and division where it isn't needed
const MAX_LOAD_FACTOR_NUMERATOR: usize = 11;
const MAX_LOAD_FACTOR_DENOMINATOR: usize = 10;

/// The two way hashmap itself. See the module level documentation for more information.
///
/// L and R are the left and right types being mapped to eachother. LH and RH are the hash builders
/// used to hash the left keys and right keys. B is the bitfield used to store neighbourhoods.
#[derive(Debug)]
pub struct BiMap<L, R, LH = RandomState, RH = RandomState, B = DefaultBitField> {
    /// All of the left keys, and the locations of their pairs within the right_data array.
    left_data: Box<[Bucket<L, usize, B>]>,
    /// All of the right keys, and the locations of their pairs within the left_data array.
    right_data: Box<[Bucket<R, usize, B>]>,
    /// Used to generate hash values for the left keys
    left_hasher: LH,
    /// Used to generate hash values for the right keys
    right_hasher: RH,
}

impl <L, R> BiMap<L, R> {
    /// Creates a new empty BiMap.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_HASH_MAP_SIZE)
    }

    /// Creates a new empty BiMap with a given capacity. It is guaranteed that at least capacity
    /// elements can be inserted before the map needs to be resized.
    pub fn with_capacity(capacity: usize) -> Self {
        BiMap {
            left_data: Bucket::empty_vec(capacity * MAX_LOAD_FACTOR_NUMERATOR / MAX_LOAD_FACTOR_DENOMINATOR),
            right_data: Bucket::empty_vec(capacity * MAX_LOAD_FACTOR_NUMERATOR / MAX_LOAD_FACTOR_DENOMINATOR),
            left_hasher: Default::default(),
            right_hasher: Default::default(),
        }
    }
}

impl <L, R, LH, RH, B> BiMap<L, R, LH, RH, B> {
    /// Returns a lower bound on the number of elements that this hashmap can hold without needing
    /// to be resized.
    pub fn capacity(&self) -> usize {
        self.left_data.len() / MAX_LOAD_FACTOR_DENOMINATOR * MAX_LOAD_FACTOR_NUMERATOR
    }
}

impl <'a, L, R, LH, RH, B> IntoIterator for &'a BiMap<L, R, LH, RH, B> {
    type Item = (&'a L, &'a R);
    type IntoIter = BiMapRefIterator<'a, L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let &BiMap { ref left_data, ref right_data, .. } = self;
        BiMapRefIterator::new(left_data.iter(), &right_data)
    }
}

impl <L, R, LH, RH, B> IntoIterator for BiMap<L, R, LH, RH, B> {
    type Item = (L, R);
    type IntoIter = BiMapIterator<L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let BiMap { left_data, right_data, .. } = self;
        BiMapIterator::new(left_data, right_data)
    }
}

#[cfg(test)]
mod test {
    use ::BiMap;

    #[test]
    fn test_capacity() {
        BiMap::<(), ()>::with_capacity(0).capacity();
        assert!(BiMap::<(), ()>::with_capacity(1024).capacity() >= 1024);
    }

    #[test]
    fn test_iteration_empty() {
        let map: BiMap<(), ()> = BiMap::new();
        assert!((&map).into_iter().next() == None);
        assert!(map.into_iter().next() == None);
    }
}
