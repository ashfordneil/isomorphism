extern crate bimap;
#[macro_use]
extern crate quickcheck;

use std::collections::{HashMap, HashSet};

use bimap::BiMap;

use quickcheck::TestResult;

quickcheck! {
    fn test_capacity(cap: usize) -> bool {
        BiMap::<(), ()>::with_capacity(cap).capacity() >= cap
    }
}

quickcheck! {
    fn remove_from_empty(a: usize, b: char) -> bool {
        let mut map: BiMap<usize, char> = BiMap::new();
        map.remove_left(&a) == None && map.remove_right(&b) == None
    }
}

quickcheck! {
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

quickcheck! {
    fn insert(inputs: Vec<(usize, char)>) -> bool {
        let mut map = BiMap::new();
        let mut left = HashMap::new();
        let mut right = HashMap::new();

        inputs
            .into_iter()
            .all(|(a, b)| {
                let old_b = left.insert(a, b);
                let old_a = right.insert(b, a);

                if let Some(ref b) = old_b {
                    right.remove(b);
                }

                if let Some(ref a) = old_a {
                    left.remove(a);
                }

                map.insert(a, b) == (old_b, old_a)
            })
    }
}
