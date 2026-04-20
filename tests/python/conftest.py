"""pytest bootstrap for the Python-bindings integration tests.

Keeps pytest from picking up the Rust `tests/common/` directory as
a Python test target (the `common/` directory is Rust-only, shared
between the Rust integration tests under tests/*.rs).
"""
