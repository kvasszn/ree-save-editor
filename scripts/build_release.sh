#!/bin/sh
set -e

VERSION=$1
MODE=$2

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version> [mhwilds|re9|mhst3|mhrise|sf6|pragmata|dd2]"
    exit 1
fi

BASE_OUT="./outputs/${VERSION}${MODE}"
WINDOWS_PATH="${BASE_OUT}/windows"
LINUX_PATH="${BASE_OUT}/linux"

if [ -z "$MODE" ]; then
    FEATURES=""
else
    FEATURES="--features ${MODE}"
fi

rm -rf "$BASE_OUT"

echo "Building Linux..."
cargo build -p ree-save-editor --target x86_64-unknown-linux-gnu --release $FEATURES

echo "Building Windows..."
cargo xwin build -p ree-save-editor --target x86_64-pc-windows-msvc --release $FEATURES

mkdir -p "$WINDOWS_PATH"
mkdir -p "$LINUX_PATH"

COPY_SCRIPTS=false

if [ -z "$MODE" ]; then
    echo "Copying default assets..."
    rsync -aq --exclude='*.user.3' --exclude='*.msg.23' --exclude='*raw_enums/*' assets/ "$WINDOWS_PATH/assets/"
    rsync -aq --exclude='*.user.3' --exclude='*.msg.23' --exclude='*raw_enums/*' assets/ "$LINUX_PATH/assets/"
    COPY_SCRIPTS=true
else
    case "$MODE" in
        "mhwilds")
            ASSETS="combined_msgs.json empty_user_save.bin enums_mappings_mhwilds.json enumsmhwilds.json remapmhwilds.json rszmhwilds_packed.json packed_assets.bc"
            COPY_SCRIPTS=true
            ;;
        "re9")
            ASSETS="enums_re9.json rszre9.json remap.json"
            ;;
        "mhst3")
            ASSETS="mhst3_enums.json mhst3_remap.json mhst3_strings.txt rszmhst3.json"
            ;;
        "mhrise")
            ASSETS="rszmhrise.json"
            ;;
        "sf6")
            ASSETS="rszsf6.json"
            ;;
        "pragmata")
            ASSETS="rszpragmata.json enumspragmata.json strings_pragmata.txt"
            ;;
        "dd2")
            ASSETS="rszdd2.json enumsdd2.json"
            ;;
        *)
            echo "Unknown mode: $MODE"
            exit 1
            ;;
    esac

    ASSET_DIR="assets/$MODE"
    mkdir -p "$WINDOWS_PATH/$ASSET_DIR"
    mkdir -p "$LINUX_PATH/$ASSET_DIR"

    for file in $ASSETS; do
        echo "Copying $file..."
        cp "$ASSET_DIR/$file" "$WINDOWS_PATH/$ASSET_DIR/"
        cp "$ASSET_DIR/$file" "$LINUX_PATH/$ASSET_DIR/"
    done
fi

if [ "$COPY_SCRIPTS" = true ]; then
    mkdir -p "$LINUX_PATH/scripts" "$WINDOWS_PATH/scripts"
    cp scripts/reset_tickets.lua "$WINDOWS_PATH/scripts/"
    cp scripts/reset_tickets.lua "$LINUX_PATH/scripts/"
fi

cp ./target/x86_64-unknown-linux-gnu/release/ree-save-editor "$LINUX_PATH/"
cp ./target/x86_64-pc-windows-msvc/release/ree-save-editor.exe "$WINDOWS_PATH/"

ZIP_WIN="ree-save-editor-windows-$VERSION$MODE.zip"
ZIP_LINUX="ree-save-editor-linux-$VERSION$MODE.zip"

echo "Zipping Windows release..."
(cd "$WINDOWS_PATH" && zip -rq "../$ZIP_WIN" .)
unzip -l "$BASE_OUT/$ZIP_WIN"

echo "Zipping Linux release..."
(cd "$LINUX_PATH" && zip -rq "../$ZIP_LINUX" .)
unzip -l "$BASE_OUT/$ZIP_LINUX"
