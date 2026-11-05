use conshash::{hash, InvalidReplicaCount, Ring};
use std::fmt;

#[test]
fn key_hash_protocol_vectors() {
    for (key, expected) in [
        ("", 0xd1fb_a762_150c_532c),
        ("a", 0x4074_48d2_b89b_1813),
        ("hello", 0xe2e7_7b41_cb4e_1f9e),
        ("café", 0xf01c_fd3b_cd0a_4e24),
        ("customer:42", 0x13d5_b21c_01d9_bdd1),
    ] {
        assert_eq!(hash(key), expected);
        assert_eq!(hash(key.as_bytes()), expected);
        assert_eq!(hash(String::from(key)), expected);
    }
}

#[test]
fn empty_ring_and_removing_missing_nodes_are_safe() {
    let mut ring = Ring::new(3).unwrap();
    assert_eq!(ring.num_replicas(), 3);
    assert!(ring.is_empty());
    assert_eq!(ring.get_node(0), None);
    assert_eq!(ring.get_node(u64::MAX), None);
    assert_eq!(ring.get("key"), None);
    ring.remove_nodes(&["missing"]);
    ring.add_nodes(&[]);
    ring.add_node(&"a");
    ring.remove_node(&"missing");
    assert_eq!(ring.len(), 1);
    ring.remove_nodes(&["a", "a"]);
    assert!(ring.is_empty());
    assert_eq!(ring.get("key"), None);
}

#[test]
fn zero_replicas_are_rejected() {
    assert_eq!(Ring::<String>::new(0).unwrap_err(), InvalidReplicaCount);
}

#[test]
fn a_single_node_owns_every_key_including_wraparound() {
    let mut ring = Ring::new(1).unwrap();
    ring.add_node(&"a");
    for key in [0, 1, u64::MAX / 2, u64::MAX] {
        assert_eq!(ring.get_node(key), Some(&"a"));
    }
}

#[derive(Clone)]
struct Node {
    id: &'static str,
    metadata: u32,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id)
    }
}

#[test]
fn duplicate_identifiers_update_metadata_without_adding_nodes() {
    let mut ring = Ring::new(32).unwrap();
    ring.add_node(&Node {
        id: "a",
        metadata: 1,
    });
    ring.add_node(&Node {
        id: "a",
        metadata: 2,
    });
    assert_eq!(ring.len(), 1);
    assert_eq!(ring.get("key").unwrap().metadata, 2);
    ring.remove_node(&Node {
        id: "a",
        metadata: 999,
    });
    assert!(ring.is_empty());
}

fn sample_keys() -> impl Iterator<Item = u64> {
    (0..20_000_u64).map(|i| hash(i.to_le_bytes()))
}

#[test]
fn insertion_order_does_not_change_placement() {
    let mut first = Ring::new(128).unwrap();
    first.add_nodes(&["a", "b", "c", "d"]);
    let mut second = Ring::new(128).unwrap();
    second.add_nodes(&["d", "b", "a", "c"]);
    for key in sample_keys() {
        assert_eq!(first.get_node(key), second.get_node(key));
    }
}

#[test]
fn adding_a_node_only_moves_keys_to_that_node() {
    let mut before = Ring::new(128).unwrap();
    before.add_nodes(&["a", "b", "c"]);
    let mut after = before.clone();
    after.add_node(&"d");
    let mut moved = 0;
    for key in sample_keys() {
        let old = before.get_node(key).unwrap();
        let new = after.get_node(key).unwrap();
        if old != new {
            assert_eq!(new, &"d");
            moved += 1;
        }
    }
    assert!(moved > 0);
}

#[test]
fn removing_a_node_only_moves_keys_previously_owned_by_it() {
    let mut before = Ring::new(128).unwrap();
    before.add_nodes(&["a", "b", "c", "d"]);
    let mut after = before.clone();
    after.remove_node(&"b");
    let mut moved = 0;
    for key in sample_keys() {
        let old = before.get_node(key).unwrap();
        let new = after.get_node(key).unwrap();
        assert_ne!(new, &"b");
        if old != new {
            assert_eq!(old, &"b");
            moved += 1;
        }
    }
    assert!(moved > 0);
    after.add_node(&"b");
    for key in sample_keys() {
        assert_eq!(before.get_node(key), after.get_node(key));
    }
}

#[test]
fn duplicate_add_and_missing_remove_do_not_change_placement() {
    let mut before = Ring::new(128).unwrap();
    before.add_nodes(&["a", "b", "c"]);
    let mut after = before.clone();
    after.add_node(&"b");
    after.remove_node(&"missing");
    assert_eq!(after.len(), 3);
    for key in sample_keys() {
        assert_eq!(before.get_node(key), after.get_node(key));
    }
}

#[test]
fn shared_prefix_node_identifiers_support_independent_removal() {
    let mut ring = Ring::new(11).unwrap();
    ring.add_nodes(&["node", "node1", "other"]);
    ring.remove_node(&"node");
    let mut expected = Ring::new(11).unwrap();
    expected.add_nodes(&["node1", "other"]);
    for key in sample_keys() {
        assert_eq!(ring.get_node(key), expected.get_node(key));
    }
}

#[test]
fn sampled_distribution_has_no_gross_imbalance() {
    let mut ring = Ring::new(128).unwrap();
    ring.add_nodes(&["a", "b", "c", "d"]);
    let mut counts = [0; 4];
    for key in sample_keys() {
        let node = ring.get_node(key).unwrap();
        counts[usize::from(node.as_bytes()[0] - b'a')] += 1;
    }
    for count in counts {
        assert!((3_000..7_000).contains(&count), "counts: {counts:?}");
    }
}

#[test]
fn get_hashes_keys_before_lookup() {
    let mut ring = Ring::new(128).unwrap();
    ring.add_nodes(&["a", "b"]);
    assert_eq!(ring.get("key"), ring.get_node(hash("key")));
}
