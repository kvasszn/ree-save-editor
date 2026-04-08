#!/bin/sh
VERSION=$1
MODE=$2

if [ -z "$VERSION" ]; then
    echo "Usage: ./build_release.sh <version> [mhwilds|re9|mhst3]"
    exit 1
fi

WINDOWS_PATH="./outputs/${VERSION}${MODE}/windows"
LINUX_PATH="./outputs/${VERSION}${MODE}/linux"
rm -r "./outputs/$VERSION"
echo "Building Linux..."

if [ -z $MODE ]; then
	features = ""
else
	features = "--features ${MODE}"
fi

cargo build -p mhtame-gui --target x86_64-unknown-linux-gnu  --release ${features}
echo "Building Windows..."
cargo xwin build -p mhtame-gui --target x86_64-pc-windows-msvc --release ${features}


if [ -z "$MODE" ] ;then
    cp -r assets $WINDOWS_PATH/
    cp -r assets $LINUX_PATH/

	mkdir -p $LINUX_PATH/scripts
	mkdir -p $WINDOWS_PATH/scripts
	cp scripts/reset_tickets.lua $WINDOWS_PATH/scripts
	cp scripts/reset_tickets.lua $LINUX_PATH/scripts
elif [ "$MODE" == "mhwilds" ] ;then
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
elif [ "$MODE" == "re9" ] ;then
    ASSETS="enums_re9.json rszre9.json remap.json"

	mkdir -p "$WINDOWS_PATH/assets/re9"
	mkdir -p "$LINUX_PATH/assets/re9"
    for file in $ASSETS; do
        echo "Copying $file..."
        cp "assets/re9/$file" "$WINDOWS_PATH/assets/re9/"
        cp "assets/re9/$file" "$LINUX_PATH/assets/re9/"
    done
elif [ "$MODE" == "mhst3" ] ;then
    ASSETS="mhst3_enums.json mhst3_remap.json mhst3_strings.txt rszmhst3.json"

	mkdir -p "$WINDOWS_PATH/assets/mhst3"
	mkdir -p "$LINUX_PATH/assets/mhst3"
    for file in $ASSETS; do
        echo "Copying $file..."
        cp "assets/mhst3/$file" "$WINDOWS_PATH/assets/mhst3/"
        cp "assets/mhst3/$file" "$LINUX_PATH/assets/mhst3/"
    done
fi

cp ./target/x86_64-unknown-linux-gnu/release/mhtame-gui $LINUX_PATH
cp ./target/x86_64-pc-windows-msvc/release/mhtame-gui.exe $WINDOWS_PATH

echo "Zipping Windows release..."
(cd "$WINDOWS_PATH" && zip -r "../save-editor-windows-$VERSION-$MODE.zip" .)
echo "Contents:"
unzip -l "./$WINDOWS_PATH/../save-editor-windows-$VERSION-$MODE.zip"

echo "Zipping Linux release..."
(cd "$LINUX_PATH" && zip -r "../save-editor-linux-$VERSION-$MODE.zip" .)
echo "Contents:"
unzip -l "./$LINUX_PATH/../save-editor-linux-$VERSION-$MODE.zip"
