#!/bin/sh
VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: ./build_release.sh <version>"
    exit 1
fi

WINDOWS_PATH="./outputs/$VERSION/windows"
LINUX_PATH="./outputs/$VERSION/linux"
echo "Building Linux..."
cargo build -p mhtame-gui --target x86_64-unknown-linux-gnu  --release 
echo "Building Windows..."
cargo xwin build -p mhtame-gui --target x86_64-pc-windows-msvc --release

ASSETS="combined_msgs.json empty_user_save.bin enums_mappings_mhwilds.json enumsmhwilds.json remapmhwilds.json rszmhwilds_packed.json"

mkdir -p "$WINDOWS_PATH/assets/mhwilds"
mkdir -p "$LINUX_PATH/assets/mhwilds"

for file in $ASSETS; do
    echo "Copying $file..."
    cp "assets/mhwilds/$file" "$WINDOWS_PATH/assets/mhwilds/"
    cp "assets/mhwilds/$file" "$LINUX_PATH/assets/mhwilds/"
done

mkdir -p $LINUX_PATH/scripts
mkdir -p $WINDOWS_PATH/scripts
cp scripts/reset_tickets.lua $WINDOWS_PATH/scripts
cp scripts/reset_tickets.lua $LINUX_PATH/scripts

cp ./target/x86_64-unknown-linux-gnu/release/mhtame-gui $LINUX_PATH
cp ./target/x86_64-pc-windows-msvc/release/mhtame-gui.exe $WINDOWS_PATH

echo "Zipping Windows release..."
(cd "$WINDOWS_PATH" && zip -r "../save-editor-windows-$VERSION.zip" .)
echo "Contents:"
unzip -l "./outputs/$VERSION/save-editor-windows-$VERSION.zip"

echo "Zipping Linux release..."
(cd "$LINUX_PATH" && zip -r "../save-editor-linux-$VERSION.zip" .)
echo "Contents:"
unzip -l "./outputs/$VERSION/save-editor-linux-$VERSION.zip"
