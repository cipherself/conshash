//! Consistent hashing with stable key placement and virtual nodes.
//!
//! ```
//! use conshash::{hash, Ring};
//!
//! let mut ring = Ring::new(128)?;
//! ring.add_nodes(&["cache-a", "cache-b", "cache-c"]);
//!
//! let owner = ring.get_node(hash("customer:42")).copied();
//! assert!(owner.is_some());
//! ring.remove_node(&"cache-b");
//! assert_ne!(ring.get("customer:42"), Some(&"cache-b"));
//! # Ok::<(), conshash::InvalidReplicaCount>(())
//! ```
//!
//! Nodes are identified by their [`ToString`] representation. Give each distinct
//! node a unique, stable identifier; implementing [`std::fmt::Display`] is the
//! idiomatic way to provide it. Adding the same identifier updates its stored
//! value without changing its positions. Removing a missing node is a no-op.
//!
//! # Placement compatibility
//!
//! All participants must use the same replica count, node identifiers, key bytes,
//! and placement protocol. Version 0.2 uses SipHash-1-3 with keys `(0, 0)`, hashing
//! keys as raw bytes. Virtual nodes hash `b"conshash-v1\0"`, the UTF-8 identifier's
//! byte length as a little-endian `u64`, its bytes, and the zero-based replica
//! index as a little-endian `u64`, in that order. This encoding does not depend on
//! Rust's [`std::hash::Hash`] implementations or the platform's pointer width.
//! These rules are part of this version's compatibility contract.
//!
//! Placement differs from 0.1: migrating requires coordinated routing changes
//! and may require moving data or warming caches. Hashes use public fixed keys
//! for reproducibility; they are not cryptographic authentication or protection
//! against adversarial key selection.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use siphasher::sip::SipHasher13;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hasher;

/// Returned when constructing a ring with zero virtual replicas per node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidReplicaCount;

impl std::fmt::Display for InvalidReplicaCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("num_replicas must be greater than zero")
    }
}

impl std::error::Error for InvalidReplicaCount {}

/// Hash raw key bytes with SipHash-1-3 and fixed keys `(0, 0)`.
///
/// Strings are hashed as UTF-8 bytes. Encode other types explicitly (for example,
/// `hash(42_u64.to_le_bytes())`) for portable placement.
#[must_use]
pub fn hash(value: impl AsRef<[u8]>) -> u64 {
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    hasher.write(value.as_ref());
    hasher.finish()
}

fn node_position(id: &str, replica: usize) -> u64 {
    let mut hasher = SipHasher13::new_with_keys(0, 0);
    hasher.write(b"conshash-v1\0");
    hasher.write(&(id.len() as u64).to_le_bytes());
    hasher.write(id.as_bytes());
    hasher.write(&(replica as u64).to_le_bytes());
    hasher.finish()
}

/// A consistent hash ring with a fixed number of virtual nodes per physical node.
///
/// Node values are cloned once on insertion. Lookup takes `O(log V)` time for
/// `V` distinct virtual-node positions, plus node-identifier lookup. Colliding
/// positions retain all owners, choosing the lexicographically smallest UTF-8
/// identifier, independently of insertion order.
#[derive(Clone, Debug)]
pub struct Ring<T> {
    num_replicas: usize,
    nodes: BTreeMap<String, T>,
    ring: BTreeMap<u64, BTreeSet<String>>,
}

impl<T> Ring<T> {
    /// Create an empty ring with `num_replicas` virtual nodes per node.
    ///
    /// More replicas generally improve balance at the cost of memory and update
    /// work. 128 is a reasonable starting point; measure your actual workload.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidReplicaCount`] if `num_replicas` is zero.
    pub fn new(num_replicas: usize) -> Result<Self, InvalidReplicaCount> {
        if num_replicas == 0 {
            return Err(InvalidReplicaCount);
        }
        Ok(Self {
            num_replicas,
            nodes: BTreeMap::new(),
            ring: BTreeMap::new(),
        })
    }

    /// Return the number of physical nodes, excluding virtual replicas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Return whether the ring has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Return the configured number of virtual replicas per node.
    #[must_use]
    pub fn num_replicas(&self) -> usize {
        self.num_replicas
    }

    /// Find the owner of an already hashed key, wrapping at the end of the ring.
    ///
    /// Returns `None` only when the ring is empty. Use [`hash`] to create the
    /// hash, or [`Self::get`] to hash a key and look up its owner in one step.
    #[must_use]
    pub fn get_node(&self, key: u64) -> Option<&T> {
        let (_, owners) = self
            .ring
            .range(key..)
            .next()
            .or_else(|| self.ring.first_key_value())?;
        let id = owners.first()?;
        self.nodes.get(id)
    }

    /// Hash raw key bytes and return their owner, or `None` for an empty ring.
    #[must_use]
    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&T> {
        self.get_node(hash(key))
    }
}

impl<T: Clone + ToString> Ring<T> {
    /// Add nodes, updating stored values for identifiers already present.
    pub fn add_nodes(&mut self, nodes: &[T]) {
        for node in nodes {
            self.add_node(node);
        }
    }

    /// Remove nodes by identifier. Missing identifiers are ignored.
    pub fn remove_nodes(&mut self, nodes: &[T]) {
        for node in nodes {
            self.remove_node(node);
        }
    }

    /// Add a node identified by `node.to_string()`.
    ///
    /// An existing identifier's value is replaced without changing placement.
    pub fn add_node(&mut self, node: &T) {
        let id = node.to_string();
        if self.nodes.insert(id.clone(), node.clone()).is_some() {
            return;
        }
        for replica in 0..self.num_replicas {
            self.ring
                .entry(node_position(&id, replica))
                .or_default()
                .insert(id.clone());
        }
    }

    /// Remove a node identified by `node.to_string()`.
    ///
    /// This is a no-op for a missing identifier, including on an empty ring.
    pub fn remove_node(&mut self, node: &T) {
        let id = node.to_string();
        if self.nodes.remove(&id).is_none() {
            return;
        }
        for replica in 0..self.num_replicas {
            let position = node_position(&id, replica);
            if let std::collections::btree_map::Entry::Occupied(mut entry) =
                self.ring.entry(position)
            {
                entry.get_mut().remove(&id);
                if entry.get().is_empty() {
                    entry.remove();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_node_protocol_vectors() {
        for (id, replica, expected) in [
            ("", 0, 0x57d9_412b_5e1b_0b91),
            ("", 10, 0xc3e4_1dcb_9f22_5ca4),
            ("1", 0, 0x88d0_6a60_33de_4903),
            ("a", 0, 0xd027_d987_44ee_7454),
            ("a", 127, 0xe6ad_04fc_b393_c10d),
            ("node", 10, 0x5579_ebc6_49e5_e587),
            ("node1", 0, 0x847c_b7fc_3447_f010),
            ("café", 1, 0xa76c_1a77_40ee_5f59),
        ] {
            assert_eq!(node_position(id, replica), expected);
        }
    }

    #[test]
    fn exact_positions_gaps_and_wraparound() {
        let ring = Ring {
            num_replicas: 1,
            nodes: BTreeMap::from([("a".into(), "a"), ("b".into(), "b")]),
            ring: BTreeMap::from([
                (10, BTreeSet::from(["a".into()])),
                (20, BTreeSet::from(["b".into()])),
            ]),
        };
        for (key, expected) in [
            (0, "a"),
            (10, "a"),
            (11, "b"),
            (20, "b"),
            (21, "a"),
            (u64::MAX, "a"),
        ] {
            assert_eq!(ring.get_node(key), Some(&expected));
        }
    }

    #[test]
    fn collisions_have_deterministic_ownership_and_preserve_other_owners_on_removal() {
        // Force a collision at a real replica position without needing to find
        // two natural 64-bit collisions.
        for ids in [["a", "b"], ["b", "a"]] {
            let mut ring = Ring::new(1).unwrap();
            ring.add_nodes(&ids);
            let position = node_position("a", 0);
            let owners = ring.ring.get_mut(&position).unwrap();
            owners.clear();
            for id in ids {
                owners.insert(id.into());
            }
            assert_eq!(ring.get_node(position), Some(&"a"));
            ring.remove_node(&"a");
            assert_eq!(ring.get_node(position), Some(&"b"));
        }
    }
}
