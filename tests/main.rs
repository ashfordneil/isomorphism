extern crate bimap;
#[macro_use]
extern crate quickcheck;

use std::collections::{HashSet, HashMap};

use bimap::BiMap;

use quickcheck::TestResult;

quickcheck! {
    fn test_capacity(cap: usize) -> bool {
        BiMap::<(), ()>::with_capacity(cap).capacity() >= cap
    }

    fn remove_from_empty(a: usize, b: char) -> bool {
        let mut map: BiMap<usize, char> = BiMap::new();
        map.remove_left(&a) == None && map.remove_right(&b) == None
    }

    fn insert_unique(inputs: Vec<(usize, char)>) -> TestResult {
        let mut map = BiMap::new();
        let mut left = HashSet::<usize>::new();
        let mut right = HashSet::<char>::new();

        for (a, b) in inputs {
            if left.contains(&a) || right.contains(&b) {
                return TestResult::discard();
            }

            left.insert(a);
            right.insert(b);

            if map.insert(a, b) != (None, None) {
                return TestResult::failed();
            }
        }

        TestResult::passed()
    }
}
