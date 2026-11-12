#!/bin/bash

# Format
cargo fmt --all -- --check

# Clippy
cargo clippy --all-targets --locked -- -D warnings

# Test
cargo test --all-targets --locked

# Build 
cargo build --locked