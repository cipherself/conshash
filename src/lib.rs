//! #Example
//!
//! ```Rust
//! extern crate conshash;
//!
//! use std::collections::hash_map::DefaultHasher;
//!
//! #[derive(Clone, Debug)]
//! struct TestNode {
//!     host_name: &'static str,
//!     ip_address: &'static str,
//!     port: u32,
//! }
//!
//! impl ToString for TestNode {
//!     fn to_string(&self) -> String {
//!         format!("{}{}", self.ip_address.to_string(), self.port.to_string())
//!     }
//! }
//!
//! let mut hash_ring = Ring::new(5);
//!
//! let test_node = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
//! hash_ring.add_node(&test_node);
//! hash_ring.remove_node(&test_node);
//! hash_ring.add_node(&test_node);
//! let x = hash_ring.get_node(hash(&format!("{}{}", test_node.to_string(), 0.to_string())));
//! // x is the node in the form of an Option<T> where T: Clone + ToString + Debug
//! ```


use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::clone::Clone;
use std::fmt::Debug;
use std::string::ToString;
use std::collections::BTreeMap;


pub fn hash<T: Hash>(value: &T) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

pub struct Ring <T: Clone + ToString + Debug> {
    num_replicas: usize,
    ring: BTreeMap<u64, T>,
}


impl <T> Ring<T> where T: Clone + ToString + Debug {
    pub fn new(num_replicas: usize) -> Ring<T> {
        Ring {
            num_replicas: num_replicas,
            ring: BTreeMap::new(),
        }
    }

    pub fn add_nodes(&mut self, nodes: &[T]) {
        if !nodes.is_empty() {
            for node in nodes.iter() { self.add_node(node); }
        }
    }

    pub fn remove_nodes(&mut self, nodes: &[T]) {
        if !nodes.is_empty() {
            for node in nodes.iter() { self.remove_node(node); }
        }
    }

    pub fn add_node(&mut self, node: &T) {
        for i in 0..self.num_replicas {
            let key = hash(&format!("{}{}", node.to_string(), i.to_string()));
            self.ring.insert(key, node.clone());
        }
    }

    pub fn remove_node(&mut self, node: &T) {
        assert!(!self.ring.is_empty());

        for i in 0..self.num_replicas {
            let key = hash(&format!("{}{}", node.to_string(), i.to_string()));
            self.ring.remove(&key);
        }
    }

    pub fn get_node(&self, key: u64) -> Option<&T> {
        assert!(!self.ring.is_empty());
        let mut keys = self.ring.keys();
        keys.find(|k| *k >= &key)
            .and_then(|k| self.ring.get(k))
            .or(keys.nth(0).and_then(|x| self.ring.get(x)))
    }
}


#[cfg (test)]
mod tests {

    use super::*;
    use std::string::ToString;

    #[derive(Clone, Debug)]
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

    #[test]
    fn test_add_node(){
        let mut hash_ring = Ring::new(3);
        assert_eq!(hash_ring.num_replicas, 3);

        let test_node = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
        hash_ring.add_node(&test_node);

    }

    #[test]
    fn test_remove_node(){
        let mut hash_ring = Ring::new(3);
        assert_eq!(hash_ring.num_replicas, 3);

        let test_node = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
        hash_ring.add_node(&test_node);
        hash_ring.remove_node(&test_node);
    }

    #[test]
    fn test_get_node(){
        let mut hash_ring = Ring::new(3);
        assert_eq!(hash_ring.num_replicas, 3);

        let test_node = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
        hash_ring.add_node(&test_node);
        let my_node = hash_ring.get_node(hash(&test_node.to_string()));

        assert_eq!(my_node.unwrap().host_name, test_node.host_name);
        assert_eq!(my_node.unwrap().ip_address, test_node.ip_address);
        assert_eq!(my_node.unwrap().port, test_node.port);
    }

    #[test]
    fn test_add_nodes(){
        let mut hash_ring = Ring::new(3);
        assert_eq!(hash_ring.num_replicas, 3);

        let test_node1 = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
        let test_node2 = TestNode{host_name: "Inferno", ip_address: "10.0.1.1", port: 666};
        let test_node3 = TestNode{host_name: "Klimt", ip_address: "127.0.0.1", port: 1};

        let v = vec![test_node1.clone(), test_node2.clone(), test_node3.clone()];
        hash_ring.add_nodes(&v);

        let node1 = hash_ring.get_node(hash(&format!("{}{}", test_node1.to_string(), 0.to_string())));
        let node2 = hash_ring.get_node(hash(&format!("{}{}", test_node2.to_string(), 0.to_string())));
        let node3 = hash_ring.get_node(hash(&format!("{}{}", test_node3.to_string(), 0.to_string())));

        assert_eq!(node1.unwrap().host_name, test_node1.host_name);
        assert_eq!(node1.unwrap().ip_address, test_node1.ip_address);
        assert_eq!(node1.unwrap().port, test_node1.port);

        assert_eq!(node2.unwrap().host_name, test_node2.host_name);
        assert_eq!(node2.unwrap().ip_address, test_node2.ip_address);
        assert_eq!(node2.unwrap().port, test_node2.port);

        assert_eq!(node3.unwrap().host_name, test_node3.host_name);
        assert_eq!(node3.unwrap().ip_address, test_node3.ip_address);
        assert_eq!(node3.unwrap().port, test_node3.port);
    }

    #[test]
    fn test_remove_nodes(){
        let mut hash_ring = Ring::new(3);
        assert_eq!(hash_ring.num_replicas, 3);

        let test_node1 = TestNode{host_name: "Skynet", ip_address: "192.168.1.1", port: 42};
        let test_node2 = TestNode{host_name: "Inferno", ip_address: "10.0.1.1", port: 666};
        let test_node3 = TestNode{host_name: "Klimt", ip_address: "127.0.0.1", port: 1};

        let v = vec![test_node1.clone(), test_node2.clone(), test_node3.clone()];
        hash_ring.add_nodes(&v);
        hash_ring.remove_nodes(&v);

        assert!(hash_ring.ring.is_empty());
    }
}
