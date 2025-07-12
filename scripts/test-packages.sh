#!/bin/bash
set -euo pipefail

# Test script to verify package configurations
# Usage: ./scripts/test-packages.sh

echo "=== Testing Package Configurations ==="

# Test if cargo-deb config is valid
echo "Testing DEB configuration..."
if cargo deb --help &>/dev/null; then
    echo "✓ cargo-deb is available"
else
    echo "✗ cargo-deb not found (run: cargo install cargo-deb)"
fi

# Test if cargo-generate-rpm config is valid  
echo "Testing RPM configuration..."
if cargo generate-rpm --help &>/dev/null; then
    echo "✓ cargo-generate-rpm is available"
else
    echo "✗ cargo-generate-rpm not found (run: cargo install cargo-generate-rpm)"
fi

# Test if required files exist
echo "Testing required files..."
required_files=(
    "systemd/linux-bt-proxy.service"
    "README.rst"
    "LICENSE"
    "Cargo.toml"
)

for file in "${required_files[@]}"; do
    if [[ -f "$file" ]]; then
        echo "✓ $file exists"
    else
        echo "✗ $file missing"
    fi
done

# Check if release binary would be built
if [[ -f "target/release/linux_bt_proxy" ]]; then
    echo "✓ Release binary exists"
else
    echo "⚠ Release binary not found (run: cargo build --release)"
fi

echo ""
echo "Package metadata:"
echo "Name: $(grep '^name = ' Cargo.toml | sed 's/name = "\(.*\)"/\1/')"
echo "Version: $(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')"
echo "Description: $(grep '^description = ' Cargo.toml | sed 's/description = "\(.*\)"/\1/')"
echo "License: $(grep '^license = ' Cargo.toml | sed 's/license = "\(.*\)"/\1/')"

echo ""
echo "Ready to build packages with: ./scripts/build-packages.sh"