use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

use sdk::type_map::TypeMap; 

fn main() {
    /*println!("cargo:rerun-if-env-changed=PACK_FILES");
    let do_pack_sdk = std::env::var("PACK_FILES").unwrap_or_default();
    let should_pack = do_pack_sdk == "1" || do_pack_sdk.eq_ignore_ascii_case("true");
    if should_pack {
        println!("cargo:rerun-if-changed=assets/rszmhwilds.json");
        println!("cargo:rerun-if-changed=assets/enumsmhwilds.json");

        let rsz_file = File::open("./assets/rszmhwilds.json") 
            .expect("Failed to open RSZ JSON schema from build script");
        let enums_file = File::open("./assets/enumsmhwilds.json") 
            .expect("Failed to open Enum JSON schema from build script");

        let map = TypeMap::from_reader(BufReader::new(rsz_file), BufReader::new(enums_file))
            .expect("Failed to parse/clean JSON");

        let dest_path = "./assets/types.bin";
        println!("Building TypeMap to {:?}", dest_path);
        map.to_bincode(dest_path).unwrap();
    }*/
}

