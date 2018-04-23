# isomorphism
2 directional hashmaps in rust

This crate aims to provide a data structure that can store a 1:1 relation between two different types.
This data structure also provides constant time lookup within this relation - in either direction.

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
