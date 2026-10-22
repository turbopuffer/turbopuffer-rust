# Changelog

## [0.1.0-alpha.2](https://github.com/turbopuffer/turbopuffer-rust/compare/v0.1.0-alpha.1...v0.1.0-alpha.2) (2026-10-22)


### Features

* Base64-encode upserted vectors on writes ([#13](https://github.com/turbopuffer/turbopuffer-rust/issues/13)) ([452a123](https://github.com/turbopuffer/turbopuffer-rust/commit/452a1236074015d4a7d17a46fb4580d927bbe971))


### Bug Fixes

* Decode v1 encryption in `update_metadata()` responses ([#14](https://github.com/turbopuffer/turbopuffer-rust/issues/14)) ([c90072a](https://github.com/turbopuffer/turbopuffer-rust/commit/c90072a9dee1a8d5304884c1475f47e70bfa9624))


### Chores

* Declare `respond_async` above the imports in `transport.rs` ([#18](https://github.com/turbopuffer/turbopuffer-rust/issues/18)) ([06f9cbc](https://github.com/turbopuffer/turbopuffer-rust/commit/06f9cbc13deeb76e4d9fe1099f1b2d228660cad8))
* Keep releases on the prerelease line ([#16](https://github.com/turbopuffer/turbopuffer-rust/issues/16)) ([86ca139](https://github.com/turbopuffer/turbopuffer-rust/commit/86ca1398f38d988945b086ed7a8c530ee4b19955))
* Move custom code out of the way of generated changes ([#17](https://github.com/turbopuffer/turbopuffer-rust/issues/17)) ([9a2453b](https://github.com/turbopuffer/turbopuffer-rust/commit/9a2453b7b91e763500ef23f57ae5458399678631))

## [0.1.0-alpha.1](https://github.com/turbopuffer/turbopuffer-rust/compare/v0.0.1...v0.1.0-alpha.1) (2026-10-21)


### Features

* Bound document IDs to 0 so they become `u64` in Rust ([16d8a28](https://github.com/turbopuffer/turbopuffer-rust/commit/16d8a289824944e78d191e91061d5b8b43ce902f))
* OpenAPI: Add missing descriptions to prettify docblocks in output SDKs ([b9c4d9c](https://github.com/turbopuffer/turbopuffer-rust/commit/b9c4d9c0aeb15530d5e3a3b4ca43e820db0e52e2))
* OpenAPI: More unsigned ints for Rust to make `u64` ([cade8ff](https://github.com/turbopuffer/turbopuffer-rust/commit/cade8ff8fc4d554f2d3ba59418ad75131201987f))
* Send client-generated request IDs ([#10](https://github.com/turbopuffer/turbopuffer-rust/issues/10)) ([12da071](https://github.com/turbopuffer/turbopuffer-rust/commit/12da07157a708e87987d060289d3aefd74c60839))


### Chores

* regenerate SDK ([613d6a4](https://github.com/turbopuffer/turbopuffer-rust/commit/613d6a4b01c596986b0cddf68d7a51b465f5438d))
* release 0.1.0-alpha.1 ([#11](https://github.com/turbopuffer/turbopuffer-rust/issues/11)) ([31eb0f9](https://github.com/turbopuffer/turbopuffer-rust/commit/31eb0f99bf9eab151d07e4e02793fcaa01423a5c))
