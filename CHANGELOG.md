# Changelog

## [0.2.0] - 2026-11-05

### Changed

- Wrap lookups past the final virtual node to the first node.
- Use ordered tree range lookup for logarithmic position lookup.
- Return `None` for empty rings and ignore removal of missing nodes.
- Use a length-prefixed node identifier and a fixed-width replica index for
  virtual-node encoding.
- Retain all owners of colliding positions with deterministic tie breaking and
  remove only the requested owner.
- Store one clone per physical node, and update values with duplicate identifiers.
- Node types require `Clone + ToString`; `Debug` is optional.
- Use Rust edition 2021 with a minimum supported Rust version of 1.74.
- Replace Travis CI with GitHub Actions and update repository metadata and examples.

### Compatibility

- Use SipHash-1-3 with fixed keys and a portable virtual-node encoding in place
  of `DefaultHasher`. Placement differs from 0.1; coordinate routing upgrades
  and plan for data migration or cache warming as needed.
- `hash` accepts strings and raw bytes via `AsRef<[u8]>`. Callers using other
  `Hash` types in 0.1 should encode those values explicitly.
- `Ring::new` returns `Result`, with `InvalidReplicaCount` for zero replicas.
  Callers should handle or propagate this result.

### Added

- `get`, `len`, `is_empty`, and `num_replicas` accessors, plus `Clone` and `Debug`
  implementations when supported by node values.
- Test coverage for ring boundaries, collisions, stable placement,
  membership changes, duplicate identifiers, and sampled load distribution.
- Package verification in CI.

## [0.1.4] - 2019-12-03

- Add a compiled documentation example.

## [0.1.3] - 2017-11-23

- Replace the deprecated `SipHasher` with `DefaultHasher`.
