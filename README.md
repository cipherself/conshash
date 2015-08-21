# conshash

[![Build Status](https://travis-ci.org/skeuomorf/conshash.svg?branch=master)](https://travis-ci.org/skeuomorf/conshash)

A library to do consistent hashing in Rust.

## Crate
Get the crate at [conshash](https://crates.io/crates/conshash)

## Example

 ```Rust
extern crate conshash;

use std::hash::{hash, SipHasher};

#[derive(Clone , Debug)]
struct TestNode {
    host_name: &'static str,
    ip_address: &'static str,
    port: u32,
}

impl ToString for TestNode {
    fn to_string(&self) -> String {
        format!("{}{}", self.ip_address.to_string(), self.port.to_string())
    }
}

let mut hash_ring = Ring::new(5);

let test_node = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
hash_ring.add_node(&test_node);
hash_ring.remove_node(&test_node);
hash_ring.add_node(&test_node);
let x = hash_ring.get_node(hash::<_, SipHasher>(&format!("{}{}", test_node.to_string(), 0.to_string())));
// x is the node in the form of an Option<T> where T: Clone + ToString + Debug
```

## License
[MIT](./LICENSE)
