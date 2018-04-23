//! Bidirectional hashmaps!
//! This crate aims to provide a data structure that can take store a 1:1 relation between two
//! different types, and provide constant time lookup within this relation.
//!
//! Unlike a regular hashmap, which provides lookups from "keys" to "values", the two directional
//! hashmap provides lookups from "left keys" to "right keys" and from "right keys" to "left keys".
//! The difference between a "value" in a hashmap and a "right key" in a `BiMap` is that the right
//! key must be hashable and comparable, and that duplicate right keys cannot exist within the
//! bimap, even if they have different left keys mapping to them.

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(feature = "serde")]
extern crate serde;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod bitfield;
mod bucket;
mod builder;
mod iterator;

use bitfield::{BitField, DefaultBitField};
use bucket::Bucket;
pub use builder::BiMapBuilder;
pub use iterator::{IntoIter, Iter};

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::fmt::{self, Debug};
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter::{self, Extend, FromIterator};
use std::mem;

pub(crate) const DEFAULT_HASH_MAP_SIZE: usize = 32;
const RESIZE_GROWTH_FACTOR: usize = 2;

// left as a fraction to avoid floating point multiplication and division where it isn't needed
pub(crate) const MAX_LOAD_FACTOR: f32 = 1.1;

/// The two way hashmap itself. See the crate level documentation for more information. Uses
/// hopscotch hashing internally.
///
/// L and R are the left and right types being mapped to eachother. LH and RH are the hash builders
/// used to hash the left keys and right keys. B is the bitfield used to store neighbourhoods.
pub struct BiMap<L, R, LH = RandomState, RH = RandomState, B = DefaultBitField> {
    /// The number of pairs inside the map
    len: usize,
    /// All of the left keys, and the locations of their pairs within the right_data array.
    left_data: Box<[Bucket<L, usize, B>]>,
    /// All of the right keys, and the locations of their pairs within the left_data array.
    right_data: Box<[Bucket<R, usize, B>]>,
    /// Used to generate hash values for the left keys
    left_hasher: LH,
    /// Used to generate hash values for the right keys
    right_hasher: RH,
}

impl<L, R> Default for BiMap<L, R> {
    fn default() -> Self {
        BiMapBuilder::new().finish()
    }
}

impl<L, R> BiMap<L, R> {
    /// Creates a new empty BiMap.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let map: BiMap<u64, char> = BiMap::new();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }
}

impl<L, R, LH, RH, B> BiMap<L, R, LH, RH, B> {
    /// Returns a lower bound on the number of elements that this hashmap can hold without needing
    /// to be resized.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let map: BiMap<String, String> = BiMap::new();
    /// let capacity = map.capacity();
    /// assert!(capacity >= 0);
    /// ```
    pub fn capacity(&self) -> usize {
        (self.left_data.len() as f32 / MAX_LOAD_FACTOR).floor() as usize
    }

    /// Returns the number of pairs inside this hashmap. Each remove will decrement this count.
    /// Each insert will increment this count, but may then also decrement it by one or two if the
    /// keys being inserted already existed and were associated with other pairs.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// assert_eq!(0, map.len());
    ///
    /// map.insert("Hello", "World");
    /// map.insert("Hashmaps", "Are cool");
    /// assert_eq!(2, map.len());
    ///
    /// // this removes the ("Hello", "World") pair and the ("Hashmaps", "Are cool") pair, leaving
    /// // only the ("Hello", "Are cool") pair behind.
    /// map.insert("Hello", "Are cool");
    /// assert_eq!(1, map.len());
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the bimap contains no pairs.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// assert!(map.is_empty());
    ///
    /// map.insert("Hello", "World");
    /// assert!(!map.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// An iterator visiting all key-value pairs in an arbitrary order. The iterator element is
    /// type (&'a L, &'a R).
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// map.insert("Hello", "World");
    /// map.insert("Hashmaps", "Are cool");
    ///
    /// for (&left, &right) in map.iter() {
    ///     println!("{} {}", left, right);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<L, R, B> {
        self.into_iter()
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
    /// check the invariants of this hashmap, panicking if they are not met
    fn invariants(&self) {
        // check lengths
        assert_eq!(self.left_data.len(), self.right_data.len());
        let len = self.left_data.len();

        // check ideal indexes are stored correctly (in the bucket and its ideal bucket's bitfield)
        self.left_data
            .iter()
            .enumerate()
            .filter_map(|(i, bucket)| bucket.data.as_ref().map(|bucket| (i, bucket)))
            .for_each(|(i, &(ref key, _value, ideal))| {
                assert_eq!(Self::find_ideal_index(key, &self.left_hasher, len), ideal);
                assert!(
                    (self.left_data[ideal].neighbourhood | B::zero_at((len + i - ideal) % len))
                        .full()
                );
            });
        self.right_data
            .iter()
            .enumerate()
            .filter_map(|(i, bucket)| bucket.data.as_ref().map(|bucket| (i, bucket)))
            .for_each(|(i, &(ref key, _value, ideal))| {
                assert_eq!(Self::find_ideal_index(key, &self.right_hasher, len), ideal);
                assert!(
                    (self.right_data[ideal].neighbourhood | B::zero_at((len + i - ideal) % len))
                        .full()
                );
            });

        // check matches exist
        self.left_data
            .iter()
            .enumerate()
            .filter_map(|(i, bucket)| bucket.data.as_ref().map(|bucket| (i, bucket)))
            .for_each(|(i, &(ref _key, value, _ideal))| {
                let &(_, pair_value, _) = self.right_data[value].data.as_ref().unwrap();
                assert_eq!(pair_value, i);
            });
        self.right_data
            .iter()
            .enumerate()
            .filter_map(|(i, bucket)| bucket.data.as_ref().map(|bucket| (i, bucket)))
            .for_each(|(i, &(ref _key, value, _ideal))| {
                let &(_, pair_value, _) = self.left_data[value].data.as_ref().unwrap();
                assert_eq!(pair_value, i);
            });

        // check length reporting holds
        assert_eq!(
            self.left_data
                .iter()
                .filter(|bucket| bucket.data.is_some())
                .count(),
            self.len()
        );
        assert_eq!(
            self.right_data
                .iter()
                .filter(|bucket| bucket.data.is_some())
                .count(),
            self.len()
        );
    }

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

    /// Finds the bitflield associated with an ideal hash index in a hashmap array, and mark a
    /// given index as empty.
    fn mark_as_empty<K>(ideal_index: usize, actual_index: usize, data: &mut [Bucket<K, usize, B>]) {
        let offset = (data.len() + actual_index - ideal_index) % data.len();
        data[ideal_index].neighbourhood = data[ideal_index].neighbourhood & B::zero_at(offset);
    }

    /// Inserts a given key into its data bucket. As this may do reshuffling, it requires a
    /// reference to the value data buckets also. Returns, if it was possible to insert the value,
    /// the index to which it was inserted. If it was not possible to do the insert, returns the
    /// key that was going to be inserted. If this function returns successfully, it is guaranteed
    /// that the key is located at the index specified, but its matching value is not set to
    /// anything meaningful. This is the callers responsibility.
    fn insert_one_sided<K: Hash, V, H: BuildHasher>(
        key: K,
        key_data: &mut [Bucket<K, usize, B>],
        value_data: &mut [Bucket<V, usize, B>],
        hasher: &H,
    ) -> Result<usize, K> {
        let len = key_data.len();
        let ideal_index = Self::find_ideal_index(&key, hasher, len);

        if key_data[ideal_index].neighbourhood.full() {
            return Err(key);
        }

        let nearest = key_data[ideal_index..]
            .iter()
            .chain(key_data[..ideal_index].iter())
            .enumerate()
            .find(|&(_, bucket)| bucket.data.is_none())
            .map(|(offset, _)| offset);
        if let Some(offset) = nearest {
            // is this free space within the neighbourhood?
            if offset < B::size() {
                // insert and we're done
                let index = (offset + ideal_index) % len;
                Self::mark_as_full(ideal_index, index, key_data);
                key_data[index].data = Some((key, usize::max_value(), ideal_index));
                Ok(index)
            } else {
                // need to make room -> find a space, boot the old thing out to make room, insert,
                // repeat
                let max_offset = (ideal_index + B::size()) % len;
                let nearest = (0..)
                    .map(|i| (len + max_offset - i) % len)
                    .take(B::size())
                    .skip(1)
                    .find(|&i| {
                        let &(_, _, ideal) = key_data[i].data.as_ref().unwrap();
                        ideal > ideal_index || ideal < max_offset
                    });
                if let Some(index) = nearest {
                    // we've found a spot to insert into
                    let (new_key, new_value, new_ideal) = key_data[index].data.take().unwrap();
                    key_data[index].data = Some((key, usize::max_value(), ideal_index));
                    match Self::insert_one_sided(new_key, key_data, value_data, hasher) {
                        Ok(new_key_index) => {
                            // the replacement worked
                            {
                                let &mut (_, ref mut paired_key_index, _) =
                                    value_data[new_value].data.as_mut().unwrap();
                                *paired_key_index = new_key_index;
                                let &mut (_, ref mut paired_value_index, _) =
                                    key_data[new_key_index].data.as_mut().unwrap();
                                *paired_value_index = new_value;
                            }
                            Self::mark_as_empty(new_ideal, index, key_data);
                            Self::mark_as_full(new_ideal, new_key_index, key_data);

                            // finish our insert and return
                            Self::mark_as_full(ideal_index, index, key_data);
                            Ok(index)
                        }
                        Err(new_key) => {
                            // the replacement failed - undo our insert
                            let (key, _, _) = key_data[index].data.take().unwrap();
                            key_data[index].data = Some((new_key, new_value, new_ideal));
                            Err(key)
                        }
                    }
                } else {
                    // no spot can be inserted into
                    Err(key)
                }
            }
        } else {
            // there is no free space
            Err(key)
        }
    }

    /// Inserts an (L, R) pair into the hashmap. Returned is a (R, L) tuple of options. The
    /// `Option<R>` is the value that was previously associated with the inserted L (or lack
    /// thereof), and vice versa for the `Option<L>`.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    ///
    /// // neither "Hello" nor 5 were previously mapped to anything, so nothing is returned.
    /// assert_eq!((None, None), map.insert("Hello", 5));
    ///
    /// // "Hello" was previously mapped to 5, so remapping it to 7 means that the 5 got evicted
    /// // from the hashmap and is therefore returned. 7 was not previously mapped to anything,
    /// // though.
    /// assert_eq!((Some(5), None), map.insert("Hello", 7));
    ///
    /// // Note now that inserting "Hello" with a new right value means that 5 no longer exists in
    /// // the hashmap.
    /// assert_eq!(None, map.get_right(&5));
    /// ```
    pub fn insert(&mut self, left: L, right: R) -> (Option<R>, Option<L>) {
        self.invariants();

        let output = {
            let &mut BiMap {
                ref mut len,
                ref mut left_data,
                ref mut right_data,
                ref left_hasher,
                ref right_hasher,
            } = self;
            match Self::remove(&left, left_data, right_data, left_hasher, right_hasher, len) {
                Some((old_left, old_right)) => if old_right == right {
                    (Some(old_right), Some(old_left))
                } else {
                    (
                        Some(old_right),
                        Self::remove(
                            &right,
                            right_data,
                            left_data,
                            right_hasher,
                            left_hasher,
                            len,
                        ).map(|(_key, value)| value),
                    )
                },
                None => (
                    None,
                    Self::remove(
                        &right,
                        right_data,
                        left_data,
                        right_hasher,
                        left_hasher,
                        len,
                    ).map(|(_key, value)| value),
                ),
            }
        };

        self.invariants();

        // attempt to insert, hold onto the keys if it fails
        let failure: Option<(L, R)> = if MAX_LOAD_FACTOR * self.len as f32
            >= self.left_data.len() as f32
        {
            Some((left, right))
        } else {
            let &mut BiMap {
                ref mut left_data,
                ref mut right_data,
                ref left_hasher,
                ref right_hasher,
                ..
            } = self;
            match Self::insert_one_sided(left, left_data, right_data, left_hasher) {
                Ok(left_index) => {
                    match Self::insert_one_sided(right, right_data, left_data, right_hasher) {
                        Ok(right_index) => {
                            let &mut (_, ref mut paired_right_index, _) =
                                left_data[left_index].data.as_mut().unwrap();
                            *paired_right_index = right_index;

                            let &mut (_, ref mut paired_left_index, _) =
                                right_data[right_index].data.as_mut().unwrap();
                            *paired_left_index = left_index;
                            None
                        }
                        Err(right) => {
                            let (left, _, left_ideal) = left_data[left_index].data.take().unwrap();
                            Self::mark_as_empty(left_ideal, left_index, left_data);
                            Some((left, right))
                        }
                    }
                }
                Err(left) => Some((left, right)),
            }
        };

        if failure.is_none() {
            self.len += 1;
        }

        self.invariants();

        if let Some((left, right)) = failure {
            // resize, as we were unable to insert
            self.len = 0;
            let capacity = self.left_data.len() * RESIZE_GROWTH_FACTOR;
            let old_left_data = mem::replace(&mut self.left_data, Bucket::empty_vec(capacity));
            let old_right_data = mem::replace(&mut self.right_data, Bucket::empty_vec(capacity));

            iter::once((left, right))
                .chain(IntoIter::new(old_left_data, old_right_data))
                .for_each(|(left, right)| {
                    self.insert(left, right);
                });
        }

        self.invariants();

        output
    }

    /// Looks up a key in the key_data section of the hashap, and if it exists returns it from the
    /// value_data section of the hashap. Returns the value that is associated with the key, if it
    /// exists.
    fn get<'a, Q: ?Sized, K, V, KH>(
        key: &Q,
        key_data: &[Bucket<K, usize, B>],
        value_data: &'a [Bucket<V, usize, B>],
        key_hasher: &KH,
    ) -> Option<&'a V>
    where
        Q: Hash + Eq,
        K: Hash + Eq + Borrow<Q>,
        KH: BuildHasher,
    {
        let len = key_data.len();
        let ideal = Self::find_ideal_index(&key, key_hasher, len);

        let neighbourhood = key_data[ideal].neighbourhood;
        neighbourhood
            .iter()
            .filter_map(|offset| key_data[(ideal + offset) % len].data.as_ref())
            .filter(|&&(ref candidate_key, ..)| candidate_key.borrow() == key)
            .filter_map(|&(_, pair_index, _)| value_data[pair_index].data.as_ref())
            .map(|&(ref value, ..)| value)
            .next()
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
        map_len: &mut usize,
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
        if let Some(offset) = neighbourhood.iter().find(|offset| {
            match key_data[(index + offset) % len].data {
                Some((ref candidate_key, ..)) => candidate_key.borrow() == key,
                _ => false,
            }
        }) {
            key_data[index].neighbourhood = neighbourhood & B::zero_at(offset);
            let (key, value_index, _) = key_data[(index + offset) % len].data.take().unwrap();
            let (value, ..) = value_data[value_index].data.take().unwrap();

            let ideal_value_index = Self::find_ideal_index(&value, value_hasher, len);
            let value_offset = (value_index + len - ideal_value_index) % len;

            value_data[ideal_value_index].neighbourhood =
                value_data[ideal_value_index].neighbourhood & B::zero_at(value_offset);

            *map_len -= 1;

            Some((key, value))
        } else {
            None
        }
    }

    /// Gets a key from the left of the hashmap. Returns the value from the right of the hashmap
    /// that associates with this key, if it exists.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// assert_eq!(None, map.get_left("Hello"));
    ///
    /// map.insert("Hello", 5);
    /// assert_eq!(Some(&5), map.get_left("Hello"));
    /// ```
    pub fn get_left<'a, Q: ?Sized>(&'a self, left: &Q) -> Option<&'a R>
    where
        L: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.invariants();
        let &BiMap {
            ref left_data,
            ref right_data,
            ref left_hasher,
            ..
        } = self;
        Self::get(left, left_data, right_data, left_hasher)
    }

    /// Gets a key from the right of the hashmap. Returns the value from the left of the hashmap
    /// that associates with this key, if it exists.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// assert_eq!(None, map.get_right(&5));
    ///
    /// map.insert("Hello", 5);
    /// assert_eq!(Some(&"Hello"), map.get_right(&5));
    /// ```
    pub fn get_right<'a, Q: ?Sized>(&'a self, right: &Q) -> Option<&'a L>
    where
        R: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.invariants();
        let &BiMap {
            ref right_data,
            ref left_data,
            ref right_hasher,
            ..
        } = self;
        Self::get(right, right_data, left_data, right_hasher)
    }

    /// Removes a key from the left of the hashmap. Returns the value from the right of the hashmap
    /// that was associated with this key, if it existed. Will remove both the left and right sides
    /// of the pair, if it exists, meaning that `get_right` will no longer work for the value
    /// associated with the key that is removed.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// map.insert("Hello", 5);
    ///
    /// assert_eq!(Some(&5), map.get_left("Hello"));
    /// assert_eq!(Some(&"Hello"), map.get_right(&5));
    ///
    /// let old = map.remove_left("Hello");
    /// assert_eq!(Some(5), old);
    ///
    /// assert_eq!(None, map.get_left("Hello"));
    /// assert_eq!(None, map.get_right(&5));
    /// ```
    pub fn remove_left<Q: ?Sized>(&mut self, left: &Q) -> Option<R>
    where
        L: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.invariants();
        let &mut BiMap {
            ref mut len,
            ref mut left_data,
            ref mut right_data,
            ref left_hasher,
            ref right_hasher,
        } = self;
        Self::remove(left, left_data, right_data, left_hasher, right_hasher, len)
            .map(|(_key, value)| value)
    }

    /// Removes a key from the right of the hashmap. Returns the value from the left of the hashmap
    /// that was associated with this key, if it existed. Will remove both the left and right sides
    /// of the pair, if it exists, meaning that `get_left` will no longer work for the value
    /// associated with the key that is removed.
    ///
    /// ```
    /// # use isomorphism::BiMap;
    /// let mut map = BiMap::new();
    /// map.insert("Hello", 5);
    ///
    /// assert_eq!(Some(&5), map.get_left("Hello"));
    /// assert_eq!(Some(&"Hello"), map.get_right(&5));
    ///
    /// let old = map.remove_right(&5);
    /// assert_eq!(Some("Hello"), old);
    ///
    /// assert_eq!(None, map.get_left("Hello"));
    /// assert_eq!(None, map.get_right(&5));
    /// ```
    pub fn remove_right<Q: ?Sized>(&mut self, right: &Q) -> Option<L>
    where
        R: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.invariants();
        let &mut BiMap {
            ref mut len,
            ref mut left_data,
            ref mut right_data,
            ref left_hasher,
            ref right_hasher,
        } = self;
        Self::remove(right, right_data, left_data, right_hasher, left_hasher, len)
            .map(|(_key, value)| value)
    }
}

impl<L, R, LH, RH, B> PartialEq for BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq,
    R: Hash + Eq,
    LH: BuildHasher,
    RH: BuildHasher,
    B: BitField,
{
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().all(|(left, right)| {
            other.get_left(left).map_or(false, |r| *right == *r)
                && other.get_right(right).map_or(false, |l| *left == *l)
        })
    }
}

impl<L, R, LH, RH, B> Eq for BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq,
    R: Hash + Eq,
    LH: BuildHasher,
    RH: BuildHasher,
    B: BitField,
{
}

impl<L, R, LH, RH, B> Debug for BiMap<L, R, LH, RH, B>
where
    L: Debug,
    R: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a, L, R, LH, RH, B> IntoIterator for &'a BiMap<L, R, LH, RH, B> {
    type Item = (&'a L, &'a R);
    type IntoIter = Iter<'a, L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let &BiMap {
            ref left_data,
            ref right_data,
            ..
        } = self;
        Iter::new(left_data.iter(), right_data)
    }
}

impl<L, R, LH, RH, B> IntoIterator for BiMap<L, R, LH, RH, B> {
    type Item = (L, R);
    type IntoIter = IntoIter<L, R, B>;

    fn into_iter(self) -> Self::IntoIter {
        let BiMap {
            left_data,
            right_data,
            ..
        } = self;
        IntoIter::new(left_data, right_data)
    }
}

impl<L, R, LH, RH, B> FromIterator<(L, R)> for BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq,
    R: Hash + Eq,
    LH: BuildHasher + Default,
    RH: BuildHasher + Default,
    B: BitField,
{
    fn from_iter<T: IntoIterator<Item = (L, R)>>(iter: T) -> Self {
        let mut output = BiMapBuilder::new()
            .left_hasher(Default::default())
            .right_hasher(Default::default())
            .bitfield::<B>()
            .finish();
        output.extend(iter);
        output
    }
}

impl<L, R, LH, RH, B> Extend<(L, R)> for BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq,
    R: Hash + Eq,
    LH: BuildHasher,
    RH: BuildHasher,
    B: BitField,
{
    fn extend<T: IntoIterator<Item = (L, R)>>(&mut self, iter: T) {
        for (left, right) in iter {
            self.insert(left, right);
        }
    }
}

#[cfg(feature = "serde")]
impl<L, R, LH, RH, B> Serialize for BiMap<L, R, LH, RH, B>
where
    L: Serialize,
    R: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.len))?;
        for (ref left, ref right) in self.iter() {
            seq.serialize_element(&(left, right))?;
        }

        seq.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, L, R, LH, RH, B> Deserialize<'de> for BiMap<L, R, LH, RH, B>
where
    L: Hash + Eq + Deserialize<'de>,
    R: Hash + Eq + Deserialize<'de>,
    LH: BuildHasher + Default,
    RH: BuildHasher + Default,
    B: BitField,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use std::fmt;
        use std::marker::PhantomData;

        use serde::de::{MapAccess, Visitor};

        struct MapVisitor<L, R, LH, RH, B> {
            marker: PhantomData<BiMap<L, R, LH, RH, B>>,
        }

        impl<'de, L, R, LH, RH, B> Visitor<'de> for MapVisitor<L, R, LH, RH, B>
        where
            L: Hash + Eq + Deserialize<'de>,
            R: Hash + Eq + Deserialize<'de>,
            LH: BuildHasher + Default,
            RH: BuildHasher + Default,
            B: BitField,
        {
            type Value = BiMap<L, R, LH, RH, B>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let builder = BiMapBuilder::new()
                    .left_hasher(Default::default())
                    .right_hasher(Default::default())
                    .bitfield::<B>();
                let mut output = if let Some(size) = map.size_hint() {
                    builder.capacity(size).finish()
                } else {
                    builder.finish()
                };

                while let Some((left, right)) = map.next_entry()? {
                    output.insert(left, right);
                }

                Ok(output)
            }
        }

        let visitor = MapVisitor {
            marker: PhantomData,
        };
        deserializer.deserialize_map(visitor)
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
