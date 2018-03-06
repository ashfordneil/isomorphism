extern crate bimap;
#[macro_use]
extern crate quickcheck;

use std::collections::HashSet;

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
    fn get_after_insert(inputs: Vec<(usize, char)>, a: usize, b: char) -> bool {
        let mut map = BiMap::new();

        for (a, b) in inputs {
            map.insert(a, b);
        }

        map.insert(a, b);

        map.get_left(&a) == Some(&b) && map.get_right(&b) == Some(&a)
    }
}

quickcheck! {
    fn get_before_insert(inputs: Vec<(usize, char)>, a: usize, b: char) -> TestResult {
        let mut map = BiMap::new();

        map.insert(a, b);

        if inputs.iter().any(|&(input_a, input_b)| a == input_a || b == input_b) {
            TestResult::discard()
        } else {
            for (a, b) in inputs {
                map.insert(a, b);
            }

            TestResult::from_bool(map.get_left(&a) == Some(&b) && map.get_right(&b) == Some(&a))
        }
    }
}

quickcheck! {
    fn insert(inputs: Vec<(usize, char)>) -> bool {
        let mut map = BiMap::new();

        inputs
            .into_iter()
            .all(|(a, b)| {
                let old_b = map.get_left(&a).map(|&x| x);
                let old_a = map.get_right(&b).map(|&x| x);

                map.insert(a, b) == (old_b, old_a)
            })
    }
}

quickcheck! {
    fn iterate_by_ref(inputs: Vec<(usize, char)>) -> bool {
        let mut map = BiMap::new();

        for (a, b) in inputs {
            map.insert(a, b);
        }

        for (a, b) in &map {
            if map.get_left(a) != Some(b) || map.get_right(b) != Some(a) {
                println!("Failure");
                println!("{:?}", map);
                println!("left: expected {0} => {1:?}, got {0} => {2:?}", a, Some(b), map.get_left(a));
                println!("right: expected {0} => {1:?}, got {0} => {2:?}", b, Some(a), map.get_right(b));
                return false;
            }
        }

        true
    }
}

quickcheck! {
    fn iterate(inputs: Vec<(usize, char)>) -> bool {
        let mut map = BiMap::new();

        for (a, b) in inputs {
            map.insert(a, b);
        }

        let mut refs: Vec<_> = (&map).into_iter().map(|(&a, &b)| (a, b)).collect();
        let mut vals: Vec<_> = map.into_iter().collect();

        refs.sort();
        vals.sort();

        refs == vals
    }
}
