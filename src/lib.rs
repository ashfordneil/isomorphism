//! Bidirectional hashmaps!
//! This crate aims to provide a data structure that can take store a 1:1 relation between two
//! different types, and provide constant time lookup within this relation.
//!
//! The hashmaps in this crate use the hopscotch hashing algorithm, mainly because I just wanted to
//! implement it. I'm hoping that the hopscotch hashing algorithm will also make removals from the
//! hashmaps more efficient.

pub mod bitfield;
mod bucket;

use bitfield::{BitField, DefaultBitField};
use bucket::Bucket;

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use std::iter::{Iterator, IntoIterator};
use std::mem;
use std::ptr;
use std::slice;

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

pub struct BiMapRefIterator<'a, L, R, B> where L: 'a, R: 'a, B: 'a {
    left_data: slice::Iter<'a, Bucket<L, usize, B>>,
    right_data: &'a [Bucket<R, usize, B>],
}

impl<'a, L, R, B> Iterator for BiMapRefIterator<'a, L, R, B> where L: 'a, R: 'a {
    type Item = (&'a L, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut BiMapRefIterator { ref mut left_data, ref right_data } = self;
        left_data
            .filter_map(|bucket| bucket.data.as_ref())
            .map(|&(ref key, value)| (key, &right_data[value].data.as_ref().unwrap().0))
            .next()
    }
}

impl <'a, L, R, LH, RH, B> IntoIterator for &'a BiMap<L, R, LH, RH, B> {
    type Item = (&'a L, &'a R);
    type IntoIter = BiMapRefIterator<'a, L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let &BiMap { ref left_data, ref right_data, .. } = self;
        BiMapRefIterator {
            left_data: left_data.iter(),
            right_data: &right_data,
        }
    }
}

unsafe fn duplicate<T>(input: &T) -> T {
    let mut output = mem::uninitialized();
    ptr::copy_nonoverlapping(input, &mut output, mem::size_of::<T>());
    output
}

pub struct BiMapIterator<L, R, B> {
    left_data: Box<[Bucket<L, usize, B>]>,
    right_data: Box<[Bucket<R, usize, B>]>,
    index: usize,
}

impl <L, R, B> Iterator for BiMapIterator<L, R, B> {
    type Item = (L, R);

    fn next(&mut self) -> Option<Self::Item> {
        let &mut BiMapIterator { ref left_data, ref right_data, ref mut index } = self;
        let output = loop {
            *index += 1;
            if *index >= left_data.len() {
                break None;
            }
            if let Some((ref left, value)) = left_data[*index].data {
                let right = &right_data[value].data.as_ref().unwrap().0;
                unsafe {
                    break Some((duplicate(left), duplicate(right)));
                }
            }
        };
        output
    }
}

impl <L, R, LH, RH, B> IntoIterator for BiMap<L, R, LH, RH, B> {
    type Item = (L, R);
    type IntoIter = BiMapIterator<L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let BiMap { left_data, right_data, .. } = self;
        let index = 0;
        BiMapIterator { left_data, right_data, index }
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
