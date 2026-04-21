# REE Save Editor

Tool for editing save files from many RE Engine games. This repo was previously mhtame, I just renamed it since the scope changed.

Supported Games:

|Game|Steam AppID|Support|
|-|-|-|
|MH Wilds|2246340|✅|
|MH Stories 3|2852190|✅|
|MH Rise|1446780|✅|
|SF6|1364780|✅|
|Pragmata|3357650|✅|
|DD2|2054970|✅|
|RE2|883710|✅|
|RE3|952060|✅|
|RE4|2050650|✅|
|RE7|418370|✅|
|RE8|1196590|✅|
|RE9|1446780|✅|

> [!NOTE]
> I would like to add more games, but these are all the one's I own (except for DD2 and Pragmata, but I had help with those, and MHST3 was the same as the demo).
> Also, only MH Wilds has an actual decent editing interface at the moment, the others are pretty barebones, but I plan on adding a system to make it easier to make a nice interface

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
      --steam-path <STEAM_PATH>        [default: ~/.local/share/Steam/]
  -h, --help                           Print help
  -V, --version                        Print version
```

## Account Transferring
To transfer games between steam accounts, see [Steam Account Transferring](/docs/USAGE.md#steam-account-transfering)
Generally it's as follows
1. Find your steamid64 (or brute force by selecting the brute force option).
1. Load the save file into the editor.
1. Change the steamid to the steamid of the account you want to transfer it to.
1. Save, the output file will be in the "output path" directory (default "outputs"), and will be named the same as the loaded file
1. Move the output file back to the remote/win64_save steam path for whatever game you're transferring for

> [!NOTE]
> Rise and SF6 are different, see [Rise Account Transferring](/docs/USAGE.md#monster-hunter-rise) for details.

### PS5 <-> PC
This tool supports transferring some games between PC and PS5. You have to load the save file, then change the Save Flags to the correct flags for different each system and game. Mapping these requires more research and save files.
|Game|PS5|PC|Notes|
|-|-|-|-|
|MH Wilds|Deflate|Mandarin & Deflate||
|Pragmata|Mandarin|Mandarin|on PS5, use the id 526253637 and change _RegionHash to 3208943443|

## FAQ
1. Missing Save Files: If you are having trouble finding your steam files, try following this [guide](https://gist.github.com/mi5hmash/47f1be53d213be9b00f2c7e0aa151b11)
1. Is this Safe: Generally yes if you know what you are editing. To be extra safe ALWAYS MAKE BACKUPS BEFORE SAVING CHANGES, and FIRST CHECK CHANGES IN OFFLINE MODE ON STEAM. Some things probably shouldn't always be editing, and you have to make sure the values are legal.
1. Idk what to do/there's too much stuff in the editor: Since the editor gives access to the whole save file, there's alot of things that are shown that aren't necessary, and/or are raw game IDS that I have to semi-manually map to something readable. This takes time and research. Currently MH Wilds is the most useable with this context. I plan on adding better support for more games though, but it will take time.

## Alternatives
You can also use repos by [mi5hmash](https://github.com/mi5hmash/) such as [MandarinJuice](https://github.com/mi5hmash/MandarinJuice) for resigning/transferring unsupported games.

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
