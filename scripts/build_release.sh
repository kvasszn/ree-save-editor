#!/bin/sh
VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: ./build_release.sh <version>"
    exit 1
fi

WINDOWS_PATH="./outputs/$VERSION/windows"
LINUX_PATH="./outputs/$VERSION/linux"
echo "Building Linux..."
cargo build --release --target x86_64-unknown-linux-gnu
echo "Building Windows..."
cargo build --release --target x86_64-pc-windows-gnu
mkdir -p $WINDOWS_PATH
mkdir -p $LINUX_PATH
cp target/x86_64-pc-windows-gnu/release/{mhtame.exe,mhtame-gui.exe} $WINDOWS_PATH
cp target/x86_64-unknown-linux-gnu/release/{mhtame,mhtame-gui} $LINUX_PATH
cp rszmhwilds.json rszmhwilds_packed.json enums.json $WINDOWS_PATH
cp rszmhwilds.json rszmhwilds_packed.json enums.json $LINUX_PATH

echo "Zipping Windows release..."
(cd "$WINDOWS_PATH" && zip -r "../mhtame-windows-$VERSION.zip" .)
echo "Contents:"
unzip -l "./outputs/$VERSION/mhtame-windows-$VERSION.zip"

echo "Zipping Linux release..."
(cd "$LINUX_PATH" && zip -r "../mhtame-linux-$VERSION.zip" .)
echo "Contents:"
unzip -l "./outputs/$VERSION/mhtame-linux-$VERSION.zip"
