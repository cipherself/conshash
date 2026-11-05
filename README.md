# conshash

[![CI](https://github.com/cipherself/conshash/actions/workflows/ci.yml/badge.svg)](https://github.com/cipherself/conshash/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/conshash.svg)](https://crates.io/crates/conshash)
[![docs.rs](https://docs.rs/conshash/badge.svg)](https://docs.rs/conshash)

A small Rust library for consistent hashing with virtual nodes, deterministic
placement, and logarithmic ring-position lookup.

## Usage

Add the dependency to `Cargo.toml`:

```toml
[dependencies]
conshash = "0.2"
```

```rust
use conshash::{hash, Ring};

let mut ring = Ring::new(128)?;
ring.add_nodes(&["cache-a", "cache-b", "cache-c"]);

let owner = ring.get("customer:42").copied();
assert!(owner.is_some());
assert_eq!(ring.get_node(hash("customer:42")).copied(), owner);

ring.remove_node(&"cache-b");
assert_ne!(ring.get("customer:42"), Some(&"cache-b"));
assert_eq!(ring.len(), 2);
Ok::<(), conshash::InvalidReplicaCount>(())
```

Nodes can be any `Clone + ToString` type. For custom types, implement `Display`
using a unique, stable node identifier. Each node value is cloned once; replicas
store identifiers. `Debug` is optional. Adding an existing identifier replaces its
value without adding replicas. Removing a missing node is a no-op. Empty rings
return `None`. Construction returns `Result<Ring<T>, InvalidReplicaCount>`; zero
replicas return an error.

`Ring::new(128)` creates 128 virtual positions per physical node. Increasing this
count generally improves balance while using more memory and update work. Measure
your own node and key distribution; equal replica counts do not guarantee equal
loads. Lookups find the next clockwise position and wrap around at the end.
Colliding positions retain all node identifiers and choose the lexicographically
smallest identifier, independently of insertion order.

## Stable placement and migration from 0.1

Keys use SipHash-1-3 with fixed keys `(0, 0)` over raw bytes. Strings use UTF-8;
encode numeric or structured keys explicitly, for example `hash(42_u64.to_le_bytes())`.
The `hash` input bound is `AsRef<[u8]>`; callers migrating from other `Hash`
types in 0.1 should provide an explicit byte encoding.

Virtual nodes hash the following concatenated bytes with the same algorithm:

1. The domain marker `b"conshash-v1\0"`.
2. The identifier's UTF-8 byte length as a little-endian `u64`.
3. The identifier's UTF-8 bytes.
4. The zero-based replica index as a little-endian `u64`.

All participants must agree on identifiers, key encoding, replica count, and this
placement protocol. Fixed public keys provide reproducibility, not cryptographic
authentication or resistance to adversarial key selection.

Version 0.2 uses a different placement protocol from 0.1. When upgrading,
coordinate routing changes across participants and plan for data migration or
cache warming. Recompute hashes stored by 0.1 from the original keys.

`Ring::new` returns `Result`; use `?` to propagate construction errors.
`Ring::new(0)` returns `Err(InvalidReplicaCount)`.
Empty-ring lookups return `None`, and removals are a no-op.

## Development

Rust 1.74 or newer, edition 2021. There is one runtime dependency: `siphasher`.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo publish --dry-run --locked --registry crates-io
```

## License

[MIT](LICENSE)
