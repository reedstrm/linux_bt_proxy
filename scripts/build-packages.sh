#!/bin/bash
set -euo pipefail

# Build script for creating DEB, RPM, and tarball packages
# Usage: ./scripts/build-packages.sh [--skip-build]

SKIP_BUILD=false
if [[ "${1:-}" == "--skip-build" ]]; then
    SKIP_BUILD=true
fi

echo "=== Linux Bluetooth Proxy Package Builder ==="

# Get version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Building packages for version $VERSION"

# Create output directory
mkdir -p dist

# Build release binary (unless skipped)
if [[ "$SKIP_BUILD" == "false" ]]; then
    echo "Building release binary..."
    cargo build --release
else
    echo "Skipping build (using existing binary)"
fi

# Check if binary exists
if [[ ! -f "target/release/linux_bt_proxy" ]]; then
    echo "Error: Release binary not found. Run 'cargo build --release' first."
    exit 1
fi

# Build DEB package
echo "Building DEB package..."
if ! command -v cargo-deb &> /dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb
fi
cargo deb --output dist/

# Build RPM package
echo "Building RPM package..."
if ! command -v cargo-generate-rpm &> /dev/null; then
    echo "Installing cargo-generate-rpm..."
    cargo install cargo-generate-rpm
fi
cargo generate-rpm --output dist/

# Create tarball for Arch users
echo "Building tarball..."
TARBALL_NAME="linux-bt-proxy-${VERSION}-x86_64-unknown-linux-gnu"
TARBALL_DIR="dist/${TARBALL_NAME}"

# Create temporary directory structure
mkdir -p "$TARBALL_DIR"/{usr/bin,lib/systemd/system,usr/share/doc/linux-bt-proxy}

# Copy files
cp target/release/linux_bt_proxy "$TARBALL_DIR/usr/bin/"
cp systemd/linux-bt-proxy.service "$TARBALL_DIR/lib/systemd/system/"
cp README.rst "$TARBALL_DIR/usr/share/doc/linux-bt-proxy/"
cp LICENSE "$TARBALL_DIR/usr/share/doc/linux-bt-proxy/"

# Create install script for tarball
cat > "$TARBALL_DIR/install.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

echo "Installing Linux Bluetooth Proxy..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)" 
   exit 1
fi

# Copy files
cp usr/bin/linux_bt_proxy /usr/bin/
chmod 755 /usr/bin/linux_bt_proxy

cp lib/systemd/system/linux-bt-proxy.service /lib/systemd/system/
chmod 644 /lib/systemd/system/linux-bt-proxy.service

# Copy documentation
mkdir -p /usr/share/doc/linux-bt-proxy
cp usr/share/doc/linux-bt-proxy/* /usr/share/doc/linux-bt-proxy/

# Reload systemd
systemctl daemon-reload

echo "Installation complete!"
echo "To start the service: sudo systemctl start linux-bt-proxy"
echo "To enable at boot: sudo systemctl enable linux-bt-proxy"
EOF

chmod +x "$TARBALL_DIR/install.sh"

# Create uninstall script
cat > "$TARBALL_DIR/uninstall.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

echo "Uninstalling Linux Bluetooth Proxy..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)" 
   exit 1
fi

# Stop and disable service
systemctl stop linux-bt-proxy 2>/dev/null || true
systemctl disable linux-bt-proxy 2>/dev/null || true

# Remove files
rm -f /usr/bin/linux_bt_proxy
rm -f /lib/systemd/system/linux-bt-proxy.service
rm -rf /usr/share/doc/linux-bt-proxy

# Reload systemd
systemctl daemon-reload

echo "Uninstallation complete!"
EOF

chmod +x "$TARBALL_DIR/uninstall.sh"

# Create tarball
cd dist
tar -czf "${TARBALL_NAME}.tar.gz" "$TARBALL_NAME"
rm -rf "$TARBALL_NAME"
cd ..

echo "=== Package build complete ==="
echo "Generated packages in dist/:"
ls -la dist/
echo ""
echo "DEB: Install with 'sudo dpkg -i dist/*.deb'"
echo "RPM: Install with 'sudo rpm -i dist/*.rpm' or 'sudo dnf install dist/*.rpm'"
echo "Tarball: Extract and run 'sudo ./install.sh'"