# Isomorphism
Fast, 2 way hashmaps in rust.

[![Build Status](https://travis-ci.org/ashfordneil/isomorphism.svg?branch=develop)](https://travis-ci.org/ashfordneil/isomorphism) ![Crates.io](https://img.shields.io/crates/v/isomorphism.svg)

This crate provides a data structure that stores a 1:1 relation between two different types.
A normal, one-way hashmap has keys and values, and a fast lookup from keys to values.
This isomorphism `BiMap` structure has left-keys and right-keys, and a fast lookup from left-keys to right keys _or_ from right-keys to left keys.


## Example use

```rust
extern crate isomorphism;

use isomorphism::BiMap;

fn main() {
    let mut map = BiMap::new();
    map.insert("Hello", "World");

    assert_eq!(map.get_left("Hello"), Some(&"World"));
    assert_eq!(map.get_right("World"), Some(&"Hello"));
}
```

## Comparisons to a naive 2 way hashmap
Two way hashmaps are nothing new, and a naive implementation of one is quite simple.
You simply have two hashmaps, one from left-keys to right-keys, and the other doing reverse lookups.
To ensure consistency, however, typically the data is inserted into each of these hashmaps as pointers to actual data stored elsewhere.
This implementation has some potential disadvantages.
Firstly it requires a form of garbage collection (unless you're doing manual memory management).
Secondly it means that you can't do lookups within the hashmap without dereferencing pointers that could cause unwanted cache misses.

As an alternative to this, the isomorphism `BiMap` makes the following guarantees about how it stores and accesses data.

- Each left-key is stored in one single contiguous block of memory, and each right-key is stored in another contiguous block of memory.
- Each key is only stored in one location in memory within the map, meaning that there is no extra overhead in reference counting or pointer dereferencing to access an element.
- Hash collisions within the `BiMap` are handled using hopscotch hashing, which guarantees that the number of (sequential) buckets accessed in order to do a hash lookup has a deterministic upper bound (that can be configured by library users).

## Disclaimers
Firstly, the API is not 100% stable yet.
It is modelled very closely on the `std::collections::HashMap` API, and I currently see no reason to change it before publishing 1.0.
However, until more people have a chance to use this and give feedback I don't want to lock anything in, so I'm not going to promise a semver-enforced stable API until that's happened.

Secondly, while I hope performance of this hashmap should be better than a naive attempt, I haven't benchmarked it yet.
I have very limited experience with writing or analysing benchmarks, and it looks like finding all of the conditions under which the performance of the `BiMap` are important could be a lot of work.
When possible, I plan to produce benchmarks for this and compare them to that of a naive two way hashmap, but until then the performance gains this library _should_ provide are purely theoretical.
