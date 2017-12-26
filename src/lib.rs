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

/// The two way hashmap itself. See the module level documentation for more information.
///
/// L and R are the left and right types being mapped to eachother. LH and RH are the hash builders
/// used to hash the left keys and right keys. B is the bitfield used to store neighbourhoods.
#[derive(Debug)]
pub struct BiMap<L, R, LH, RH, B> {
    /// All of the left keys, and the locations of their pairs within the right_data array.
    left_data: Box<[Bucket<L, usize, B>]>,
    /// All of the right keys, and the locations of their pairs within the left_data array.
    right_data: Box<[Bucket<R, usize, B>]>,
    /// Used to generate hash values for the left keys
    left_hasher: LH,
    /// Used to generate hash values for the right keys
    right_hasher: RH,
}

impl <L, R> BiMap<L, R, RandomState, RandomState, DefaultBitField> {
    /// Create a new BiMap with a given capacity. It is guaranteed that at least capacity elements
    /// can be inserted before the map needs to be resized.
    pub fn with_capacity(capacity: usize) -> Self {
        BiMap {
            left_data: Bucket::empty_vec(capacity),
            right_data: Bucket::empty_vec(capacity),
            left_hasher: Default::default(),
            right_hasher: Default::default(),
        }
    }
}
