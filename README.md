# Repo for some MH Wilds data generation stuff

Makes use of the RSZ reader, texture codecs, and some file reading stuff from ![https://github.com/wwylele/mhrice](https://github.com/wwylele/mhrice)

Also looked at it to figure out how to read some of the file formats.

Uses praydog's emulation dumper for the rsz files.

## Build

```
git clone https://github.com/kvasszn/mhtame.git
cd mhtame
git submodule update --init
cargo build # you should also use the --release flag since it really helps optimizes images
```

## Usage

By default, the program will dump files based on their extensions and headers.

If you want to use this to dump user files, make sure to get an rsz dump from [https://github.com/alphazolam/RE_RSZ].

*Note:* the one provided there for wilds is not complete


And if you want enums, get an `Enums_Internal.hpp` from RE framework and use the following command to generate a json file.
You can also use the ones in `gen/`
```
python3 gen/enumtojson.py <path/to/Enums_Internal.hpp> enums.json
```

You can then tell the program which ones to use by setting the environment variables:
```
RSZ_FILE=<path/to/rszdump.json>
ENUM_FILE=<path/to/enums.json>
```
(I personally just prefix the command with this or use `export`)

### Single File
Make sure that the `-r` directory + the file directory of the file combine to the real file location
```
./target/<release or debug>/mhwsgen -r <path/to/game/native> -o <output/directory> -f <path/to/file>
```

### Multi File
Note: the root directory prefix gets removed from the file path when saving
```
./target/<release or debug>/mhwsgen -r <path/to/game/native> -o <output/directory> -l <path/to/list of files to process>
```
