# REE Save Editor

Tool for editing save files from many RE Engine games. This repo was previously mhtame, I just renamed it since the scope changed.

Supported Games:

|Game|Steam AppID|Support|
|-|-|-|
|MH Wilds|2246340|✅ |
|MH Stories 3|2852190|✅ WIP|
|RE9|1446780|✅ WIP|
|MH Rise|1446780|✅ WIP|
|SF6|1364780|✅ WIP|
|Pragmata|3357650|✅ WIP|
|DD2|2054970|✅ WIP|

I plan on doing RE Remakes, RE7, RE8 and more upon request.

## Build
```
git clone https://github.com/kvasszn/ree-save-editor.git
cd ree-save-editor
cargo build --release -p ree-save-editor
```

## Usage
Run just run the exe. For a more in-depth usage guide, see the [docs](/docs/USAGE.md)
```
Usage: ree-save-editor [OPTIONS]

Options:
  -f, --file-name <FILE_NAME>
  -o, --out-dir <OUT_DIR>              [default: outputs]
      --steamid <STEAMID>
      --rsz-path <RSZ_PATH>
      --enums-path <ENUMS_PATH>
      --msgs-path <MSGS_PATH>
      --mappings-path <MAPPINGS_PATH>
      --remap-path <REMAP_PATH>
      --steam-path <STEAM_PATH>        [default: /home/nikola/.local/share/Steam/]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Previous REPO (file dumping/mhtame)
File dumping should still work. It can convert some files to json for readability and data mining.

Files that it currently supports are `user, tex, msg, pog, poglst, oft` and `wilds saves`.
Some basic support exists for `scn` files, but it's rough, RSZ works for it though.

Makes use of texture codecs, and file reading stuff from [https://github.com/wwylele/mhrice](https://github.com/wwylele/mhrice)

Also looked at it to figure out how to read some of the file formats.

Uses praydog's emulation dumper for the rsz files.

### Build
```
cargo build --release --bin ree-dump
```

### Usage
```
Usage: ree-dump [OPTIONS]

Options:
  -f, --file-name <FILE_NAME>
  -r, --root-dir <ROOT_DIR>
  -l, --list <LIST>
  -p, --preserve
  -d, --dump-rsz
  -s, --dump-sdk
  -o, --out-dir <OUT_DIR>      [default: outputs]
      --rsz <RSZ>              [default: assets/mhwilds/rszmhwilds.json]
      --enums <ENUMS>          [default: assets/mhwilds/enumsmhwilds.json]
      --save-file <SAVE_FILE>
      --steamid <STEAMID>
  -h, --help                   Print help
  -V, --version                Print version
```
