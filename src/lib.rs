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
use iterator::{BiMapIterator, BiMapRefIterator};

use std::borrow::Borrow;
use std::cmp;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter;
use std::mem;

const DEFAULT_HASH_MAP_SIZE: usize = 32;
const RESIZE_GROWTH_FACTOR: usize = 2;

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

impl<L, R> BiMap<L, R> {
    /// Creates a new empty BiMap.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_HASH_MAP_SIZE)
    }

    /// Creates a new empty BiMap with a given capacity. It is guaranteed that at least capacity
    /// elements can be inserted before the map needs to be resized.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = if capacity == 0 {
            0
        } else {
            cmp::max(DEFAULT_HASH_MAP_SIZE, capacity) * MAX_LOAD_FACTOR_NUMERATOR
                / MAX_LOAD_FACTOR_DENOMINATOR
        };
        BiMap {
            left_data: Bucket::empty_vec(capacity),
            right_data: Bucket::empty_vec(capacity),
            left_hasher: Default::default(),
            right_hasher: Default::default(),
        }
    }
}

impl<L, R, LH, RH, B> BiMap<L, R, LH, RH, B> {
    /// Returns a lower bound on the number of elements that this hashmap can hold without needing
    /// to be resized.
    pub fn capacity(&self) -> usize {
        self.left_data.len() / MAX_LOAD_FACTOR_DENOMINATOR * MAX_LOAD_FACTOR_NUMERATOR
    }
}

impl<L, R, LH, RH, B> BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq,
    R: Hash + Eq,
    LH: BuildHasher,
    RH: BuildHasher,
    B: BitField,
{
    /// Finds the ideal position of a key within the hashmap.
    fn find_ideal_index<K: Hash, H: BuildHasher>(key: &K, hasher: &H, len: usize) -> usize {
        let mut hasher = hasher.build_hasher();
        key.hash(&mut hasher);
        hasher.finish() as usize % len
    }

    /// Find the bitfield associated with an ideal hash index in a hashmap array, and mark a given
    /// index as full.
    fn mark_as_full<K>(ideal_index: usize, actual_index: usize, data: &mut [Bucket<K, usize, B>]) {
        let offset = (data.len() + actual_index - ideal_index) % data.len();
        data[ideal_index].neighbourhood = data[ideal_index].neighbourhood | B::one_at(offset);
    }

    /// Finds (or makes) a free space in which to insert a key in the hashmap. If this is not
    /// possible, due to the hashmap being too full, then None is returned.
    fn find_insert_index<K, V>(
        ideal_index: usize,
        key_data: &mut [Bucket<K, usize, B>],
        value_data: &mut [Bucket<V, usize, B>],
    ) -> Option<usize> {
        let len = key_data.len();

        // check that the neighbourhood isn't full - if it is the hashmap needs to be resized
        if key_data[ideal_index].neighbourhood.full() {
            return None;
        }

        // find the nearest free space
        if let Some((offset, _)) = key_data[ideal_index..]
            .iter()
            .chain(key_data[..ideal_index].iter())
            .enumerate()
            .filter(|&(_, bucket)| bucket.data.is_none())
            .next()
        {
            // is this free space within the neighbourhood?
            if offset < B::size() {
                Some((offset + ideal_index) % len)
            } else {
                // reshuffle the hashmap to make room for the next element
                let mut offset_index = (ideal_index + offset) % len;
                loop {
                    // find an element that can be shuffled into the free space
                    match (0..)
                        .map(|i| (len + offset_index - i) % len)
                        .take(B::size())
                        .skip(1)
                        .filter(|&i| {
                            let &(_, _, ideal) = key_data[i].data.as_ref().unwrap();
                            ideal > ideal_index && ideal < offset_index
                        })
                        .next()
                    {
                        Some(index) => {
                            // move the found element into the free space
                            let bucket = key_data[index].data.take().unwrap();
                            value_data[bucket.1].data.as_mut().unwrap().1 = offset_index;
                            key_data[offset_index].data = Some(bucket);
                            offset_index = index;

                            // either return the newly created free space, if it's close enough, or
                            // repeat - finding another element to move into this free space
                            if (offset_index + len - ideal_index) % len < B::size() {
                                break Some(offset_index);
                            }
                        }
                        None => {
                            break None;
                        }
                    }
                }
            }
        } else {
            // the hashmap is entirely full - this should not be possible but if it happens just
            // resize
            None
        }
    }

    /// Inserts an (L, R) pair into the hashmap. Returned is a (R, L) tuple of options. The
    /// `Option<R>` is the value that was previously associated with the inserted L (or lack
    /// thereof), and vice versa for the `Option<L>`.
    pub fn insert(&mut self, left: L, right: R) -> (Option<R>, Option<L>) {
        let output = {
            let &mut BiMap {
                ref mut left_data,
                ref mut right_data,
                ref left_hasher,
                ref right_hasher,
            } = self;
            match Self::remove(&left, left_data, right_data, left_hasher, right_hasher) {
                Some((old_left, old_right)) => if old_right == right {
                    (Some(old_right), Some(old_left))
                } else {
                    (
                        Some(old_right),
                        Self::remove(&right, right_data, left_data, right_hasher, left_hasher)
                            .map(|(_key, value)| value),
                    )
                },
                None => (
                    None,
                    Self::remove(&right, right_data, left_data, right_hasher, left_hasher)
                        .map(|(_key, value)| value),
                ),
            }
        };

        let left_ideal_index =
            Self::find_ideal_index(&left, &self.left_hasher, self.left_data.len());
        let right_ideal_index =
            Self::find_ideal_index(&right, &self.right_hasher, self.right_data.len());

        let insert_indexes = {
            let &mut BiMap {
                ref mut left_data,
                ref mut right_data,
                ..
            } = self;
            let left_index = Self::find_insert_index(left_ideal_index, left_data, right_data);
            let right_index = Self::find_insert_index(right_ideal_index, right_data, left_data);

            if let (Some(left_index), Some(right_index)) = (left_index, right_index) {
                Some((left_index, right_index))
            } else {
                None
            }
        };

        if let Some((left_index, right_index)) = insert_indexes {
            Self::mark_as_full(left_ideal_index, left_index, &mut self.left_data);
            Self::mark_as_full(right_ideal_index, right_index, &mut self.right_data);

            self.left_data[left_index].data = Some((left, right_index, left_ideal_index));
            self.right_data[right_index].data = Some((right, left_index, right_ideal_index));
        } else {
            /* resize */
            let capacity = self.left_data.len() * RESIZE_GROWTH_FACTOR;
            let old_left_data = mem::replace(&mut self.left_data, Bucket::empty_vec(capacity));
            let old_right_data = mem::replace(&mut self.right_data, Bucket::empty_vec(capacity));

            iter::once((left, right))
                .chain(BiMapIterator::new(old_left_data, old_right_data))
                .for_each(|(left, right)| {
                    self.insert(left, right);
                });
        }

        output
    }

    /// Removes a key from the key_data section of the hashmap, and removes the value from the
    /// value_data section of the hashmap. Returns the value that is associated with the key, if it
    /// exists.
    fn remove<Q: ?Sized, K, V, KH, VH>(
        key: &Q,
        key_data: &mut [Bucket<K, usize, B>],
        value_data: &mut [Bucket<V, usize, B>],
        key_hasher: &KH,
        value_hasher: &VH,
    ) -> Option<(K, V)>
    where
        Q: Hash + Eq,
        K: Hash + Eq + Borrow<Q>,
        V: Hash,
        KH: BuildHasher,
        VH: BuildHasher,
    {
        let len = key_data.len();
        let index = Self::find_ideal_index(&key, key_hasher, len);

        let neighbourhood = key_data[index].neighbourhood;
        if let Some(offset) = neighbourhood.iter().find(
            |offset| match key_data[(index + offset) % len].data {
                Some((ref canidate_key, ..)) => canidate_key.borrow() == key,
                _ => false,
            },
        ) {
            key_data[index].neighbourhood = neighbourhood & B::zero_at(offset);
            let (key, value_index, _) = key_data[(index + offset) % len].data.take().unwrap();
            let (value, ..) = value_data[value_index].data.take().unwrap();

            let ideal_value_index = Self::find_ideal_index(&value, value_hasher, len);
            let value_offset = (value_index + len - ideal_value_index) % len;

            value_data[ideal_value_index].neighbourhood =
                value_data[ideal_value_index].neighbourhood & B::zero_at(value_offset);

            Some((key, value))
        } else {
            None
        }
    }

    /// Removes a key from the left of the hashmap. Returns the value from the right of the hashmap
    /// that associates with this key, if it exists.
    pub fn remove_left<Q: ?Sized>(&mut self, left: &Q) -> Option<R>
    where
        L: Borrow<Q>,
        Q: Hash + Eq,
    {
        let &mut BiMap {
            ref mut left_data,
            ref mut right_data,
            ref left_hasher,
            ref right_hasher,
        } = self;
        Self::remove(left, left_data, right_data, left_hasher, right_hasher)
            .map(|(_key, value)| value)
    }

    /// Removes a key from the right of the hashmap. Returns the value from the left of the hashmap
    /// that associates with this key, if it exists.
    pub fn remove_right<Q: ?Sized>(&mut self, right: &Q) -> Option<L>
    where
        R: Borrow<Q>,
        Q: Hash + Eq,
    {
        let &mut BiMap {
            ref mut left_data,
            ref mut right_data,
            ref left_hasher,
            ref right_hasher,
        } = self;
        Self::remove(right, right_data, left_data, right_hasher, left_hasher)
            .map(|(_key, value)| value)
    }
}

impl<'a, L, R, LH, RH, B> IntoIterator for &'a BiMap<L, R, LH, RH, B> {
    type Item = (&'a L, &'a R);
    type IntoIter = BiMapRefIterator<'a, L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let &BiMap {
            ref left_data,
            ref right_data,
            ..
        } = self;
        BiMapRefIterator::new(left_data.iter(), &right_data)
    }
}

impl<L, R, LH, RH, B> IntoIterator for BiMap<L, R, LH, RH, B> {
    type Item = (L, R);
    type IntoIter = BiMapIterator<L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let BiMap {
            left_data,
            right_data,
            ..
        } = self;
        BiMapIterator::new(left_data, right_data)
    }
}

#[cfg(test)]
mod test {
    use BiMap;

    #[test]
    fn test_iteration_empty() {
        let map: BiMap<(), ()> = BiMap::new();
        assert_eq!((&map).into_iter().next(), None);
        assert_eq!(map.into_iter().next(), None);
    }
}
