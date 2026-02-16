use std::collections::HashMap;

use mhtame::{edit::copy::CopyBuffer, save::{game::Game, remap::Remap}, sdk::type_map::TypeMap};

use crate::Config;

#[derive(Clone, Default)]
pub struct AssetPaths {
    rsz: Option<String>,
    enums: Option<String>,
    msgs: Option<String>,
    mappings: Option<String>,
    strings: Option<String>,
    remap: Option<String>,
}

impl AssetPaths {
    pub fn from_game(game: Game) -> Self {
        match game {
            Game::MHWILDS => {
                Self {
                    rsz: Some("./assets/mhwilds/rszmhwilds_packed.json".to_string()),
                    enums: Some("./assets/mhwilds/enumsmhwilds.json".to_string()),
                    msgs: Some("./assets/mhwilds/combined_msgs.json".to_string()),
                    mappings: Some("./assets/mhwilds/enums_mappings_mhwilds.json".to_string()),
                    strings: None,
                    remap: Some("./assets/mhwilds/remapmhwilds.json".to_string())
                }
            },
            Game::MHST3 => {
                Self {
                    rsz: Some("./assets/mhst3/rszmhst3.json".to_string()),
                    enums: Some("./assets/mhst3/mhst3_enums.json".to_string()),
                    msgs: None,
                    mappings: None,
                    strings: Some("./assets/mhst3/mhst3_strings.txt".to_string()),
                    remap: Some("./assets/mhst3/mhst3_remap.json".to_string())
                }
            }
            Game::DD2 => {
                Self {
                    rsz: Some("./assets/dd2/rszdd2.json".to_string()),
                    enums: None,
                    msgs: None,
                    mappings: None,
                    strings: None,
                    remap: None,
                }
            }
            Game::PRAGMATA => {
                Self {
                    rsz: Some("./assets/pragmata/rszpragmata.json".to_string()),
                    enums: None,
                    msgs: None,
                    mappings: None,
                    strings: None,
                    remap: None,
                }
            }
            //_ => Self::default(),
        }
    }
}

#[derive(Debug)]
pub struct GameCtx {
    pub type_map: TypeMap,
    pub copy_buffer: CopyBuffer,
    pub remaps: HashMap<String, Remap>,
}

impl GameCtx {
    pub fn new(paths: &AssetPaths) -> Self {
       #[cfg(not(target_arch = "wasm32"))]
        let mut type_map = TypeMap::default();
        if let Some(path) = &paths.rsz {
            type_map.load_rsz_from_path(path);
        }
        if let Some(path) = &paths.enums {
            type_map.load_enums_from_path(path);
        }
        if let Some(path) = &paths.msgs {
            type_map.load_msg_from_path(path);
        }
        if let Some(path) = &paths.mappings {
            type_map.load_enum_mappings_from_path(path);
        }
        if let Some(path) = &paths.strings {
            type_map.load_strings_from_path(path);
        }

       #[cfg(target_arch = "wasm32")]
        let type_map = {
            use mhtame::rsz::dump::decompress;
            const RSZ_JSON: &[u8] = include_bytes!("../../assets/rszmhwilds_packed.json.gz");
            const ENUMS_JSON: &[u8] = include_bytes!("../../assets/enumsmhwilds.json.gz");
            const MSGS_JSON: &[u8] = include_bytes!("../../assets/combined_msgs.json.gz");
            const ENUM_MAPPINGS_JSON: &str = include_str!("../../assets/enum_text_mappings.json");
            const MHST3_STRINGS: &str = include_str!("../../assets/mhst3_strings.txt");
            let rsz = decompress(RSZ_JSON);
            let enums = decompress(ENUMS_JSON);
            let msgs = decompress(MSGS_JSON);
            let mut res = TypeMap::parse_str(&rsz, &enums)
                .expect("Could not load type map")
                .load_msg(&msgs, ENUM_MAPPINGS_JSON);
            match res.load_string_map_from_str(&MHST3_STRINGS) {
                Err(e) => eprintln!("[ERROR] Could not load MHST3 Strings"),
                _ => ()
            };
            res
        };

        #[cfg(target_arch = "wasm32")]
        let remaps: HashMap<String, Remap> =
            serde_json::from_str(include_str!("../../assets/wilds_remap.json")).unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        let remaps = if let Some(remap_path) = &paths.remap {
            let data = std::fs::read_to_string(remap_path);
            data.map(|data| {
                let remaps: HashMap<String, Remap> =
                    serde_json::from_str(&data).unwrap_or_default();
                remaps
            })
            .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            type_map,
            copy_buffer: CopyBuffer::Null,
            remaps,
        }
    }
}
