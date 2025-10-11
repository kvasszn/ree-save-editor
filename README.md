# MH Tame
This repo is mainly for dumping RE engine files into readable json. It also does converts textures to png and fonts. and I'm working on recreating some files from json.

Files that it currently supports are `user, tex, msg, pog, poglst, oft`.
Some basic support exists for `scn` files, but it's rough, RSZ works for it though.

Makes use of texture codecs, and file reading stuff from ![https://github.com/wwylele/mhrice](https://github.com/wwylele/mhrice)

Also looked at it to figure out how to read some of the file formats.

Uses praydog's emulation dumper for the rsz files.

## Build

```
git clone https://github.com/kvasszn/mhtame.git
cd mhtame
git submodule update --init
cargo build --release # the release flag optimizes speeds alot
```

## Usage

By default, the program will dump or recreates files based on their extensions and headers. It can only recreate `User` files from json, but this is still WIP and some things don't work.

The rsz dump is already in this repo, but if you want one you can get one at [https://github.com/dtlnor/RE_RSZ], or dump it yourself.

And if you want enums, get an `Enums_Internal.hpp` from RE framework and use the following command to generate a json file.
You can also use the already generated ones.
```
python gen/enumtojson.py <path/to/Enums_Internal.hpp> enums.json
```

You can then tell the program which ones to use by setting the environment variables: (not recommended anymore, just use flags, --rsz and --enums)
These two files should be in the repo by default, so unless you know what you're doing, you probably don't have to mess with it.
```
RSZ_FILE=<path/to/rszdump.json>
ENUM_FILE=<path/to/enums.json>
```

### Single File
Make sure that the `-r` directory + the file directory of the file combine to the real file location
```
./target/<release or debug>/mhtame -r <path/to/game/native> -o <output/directory> -f <path/to/file>
```
To specify enums and rsz files, use:
```
./target/<release or debug>/mhtame -r <path/to/game/native> -o <output/directory> -f <path/to/file> --rsz <pathtorsz> --enums <pathtoenums>
```

### Multi File
Note: the root directory prefix gets removed from the file path when saving
```
./target/<release or debug>/mhtame -r <path/to/game/native> -o <output/directory> -l <path/to/list of files to process>
```

## Dumping Save Files
It's important to use the unpacked structs version of the rsz dump, otherwise the file doesnt get read properly.
```
./target/release/mhtame -f <path/to/savefile> --rsz rszmhwilds_unpacked_structs.json --steamid <your steam id>
```
For help getting your steamid: https://help.steampowered.com/en/faqs/view/2816-BE67-5B69-0FEC
If you dont pass one in, the program will try to brute force the key, but i'm pretty sure this is like completely broken atm. Still need to figure some stuff out for it.


## Recreating Files
As mentioned, this is still WIP, but if the program sees a file like `ItemData.user.3.json` (either in a list or single file), it will try and recreate `ItemData.user.3` from the json data.
