#!/bin/bash
set -e

APP_NAME="oxide"
REPO="oxide-cli/oxide"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)     OS="linux";;
    Darwin*)    OS="macos";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

case "$ARCH" in
    x86_64)     ARCH="x86_64";;
    arm64|aarch64) ARCH="aarch64";;
    *)          echo "Unsupported architecture: $ARCH"; exit 1;;
esac

LATEST_VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "Failed to fetch latest version"
    exit 1
fi

echo "Installing $APP_NAME $LATEST_VERSION..."

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION/${APP_NAME}-${OS}-${ARCH}.tar.gz"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

echo "Downloading from $DOWNLOAD_URL..."
curl -sL "$DOWNLOAD_URL" | tar xz

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
mv "$APP_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$APP_NAME"

cd -
rm -rf "$TMP_DIR"

echo "✓ $APP_NAME installed successfully to $INSTALL_DIR/$APP_NAME"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
