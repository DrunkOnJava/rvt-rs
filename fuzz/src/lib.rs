//! Empty lib so that this scaffold parses as a Cargo crate before
//! any fuzz targets (SEC-15..23) are added. `cargo-fuzz` itself
//! does not require a `lib.rs` — it uses `[[bin]]` entries — but
//! `cargo metadata` refuses to parse a manifest with zero targets,
//! so we ship this placeholder until the first fuzz target lands.
//! Once `fuzz_targets/<first>.rs` exists and a matching `[[bin]]`
//! stanza is added to `Cargo.toml`, this file can be deleted.
