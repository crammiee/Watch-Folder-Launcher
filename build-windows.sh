#!/usr/bin/env bash
set -e

TARGET="x86_64-pc-windows-gnu"
OUT="dist"

echo "==> Checking dependencies..."

if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "==> Adding Rust target $TARGET..."
    rustup target add "$TARGET"
fi

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
    echo "==> Installing mingw-w64..."
    sudo pacman -S --noconfirm mingw-w64-gcc
fi

echo "==> Building release..."
cargo build --release --target "$TARGET"

echo "==> Copying to $OUT/..."
mkdir -p "$OUT"
cp "target/$TARGET/release/ame-watcher.exe" "$OUT/"
cp config.json "$OUT/"

echo ""
echo "Done. Files ready in $OUT/:"
ls -lh "$OUT/"
