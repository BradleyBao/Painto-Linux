#!/bin/bash

# Ask user to input version number
read -p "Enter the version number (e.g., 1.0.0): " version

# Check if this script is ran from project root
if [ ! -f "Cargo.toml" ]; then
    echo "Error: This script must be run from the project root directory."
    exit 1
fi

# Run Check Scripts
./scripts/check.sh

# Build 
cargo build --release --locked

ldd target/release/painto-linux

# Tar the file into releases/
tar -czf releases/painto-linux-${version}.tar.gz \
    -C target/release painto-linux

echo "Release package created: releases/painto-linux-${version}.tar.gz"