use std::{collections::HashMap, path::{Path, PathBuf}};
use indexmap::IndexMap;
use ree_lib::{assets::bundle::Bundle, enums::{EnumMap, load_enum_map}, rsz::RszMap};
use serde::Deserialize;

use crate::save::{game::Game, remap::{Remap, get_asset_paths}};

pub type GameConfigs = HashMap<Game, GamePaths>;

pub type Schemas = HashMap<String, Vec<(u32, String)>>;

#[derive(Deserialize, Debug, Clone)]
pub struct GamePaths {
    pub rsz: Option<String>,
    pub enums: Option<String>,
    pub enums_raw: Option<String>,
    pub remaps: Option<String>,
    pub bundle: Option<String>,
    #[serde(default, deserialize_with="deserialize_schemas")]
    pub schemas: Schemas,
}

use serde::de::Error;

pub fn deserialize_schemas<'de, D>(deserializer: D) -> Result<Schemas, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // use indexmap to preserve order
    let raw_data: HashMap<String, IndexMap<String, String>> = HashMap::deserialize(deserializer)?;
    let mut parsed_schemas = HashMap::new();

    for (schema_name, fields) in raw_data {
        let mut parsed_fields = Vec::with_capacity(fields.len());

        for (hex_key, class_name) in fields {
            let clean_hex = hex_key.trim_start_matches("0x");
            let hash_u32 = u32::from_str_radix(clean_hex, 16).map_err(|e| {
                D::Error::custom(format!("Failed to parse hex string '{}': {}", hex_key, e))
            })?;
            parsed_fields.push((hash_u32, class_name));
        }

        parsed_schemas.insert(schema_name, parsed_fields);
    }

    Ok(parsed_schemas)
}

pub struct GameData {
    pub rsz: RszMap,
    pub enums: EnumMap,
    pub remaps: HashMap<String, Remap>,
    pub bundle: Bundle
}

impl GameData {
    pub fn load(paths: &GamePaths, force_raw: bool) -> anyhow::Result<Self> {
        let rsz:    RszMap                 = load_json(paths.rsz.as_ref(), "rsz map")?;
        let enums:  EnumMap                = load_json(paths.enums.as_ref(), "enums")?;
        let remaps: HashMap<String, Remap> = load_json(paths.remaps.as_ref(), "remaps")?;

        let bundle = if force_raw {
            load_bundle_raw(&remaps, &rsz)
        } else {
            load_bundle(paths.bundle.as_deref(), &remaps, &rsz)?
        };

        Ok(Self { rsz, enums, remaps, bundle })
    }
}


pub fn load_game_configs(path: &str) -> anyhow::Result<GameConfigs> {
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

fn load_json<T: serde::de::DeserializeOwned + Default>(path: Option<&String>, label: &str) -> anyhow::Result<T> {
    match path {
        Some(p) => Ok(serde_json::from_slice(&std::fs::read(p)?)?),
        None => {
            log::info!("No {} path configured, using default", label);
            Ok(T::default())
        }
    }
}

fn load_enum(path: Option<&String>) -> anyhow::Result<EnumMap> {
    match path {
        Some(p) => Ok(load_enum_map(&PathBuf::from(p))?),
        None => {
            log::info!("No enums path configured, using default");
            Ok(EnumMap::default())
        }
    }
}

fn load_bundle(path: Option<&str>, remaps: &HashMap<String, Remap>, rsz: &RszMap) -> anyhow::Result<Bundle> {
    if let Some(p) = path.map(Path::new).filter(|p| p.exists()) {
        log::info!("Loading bundle from {}", p.display());
        let data = std::fs::read(p)?;
        Ok(bincode::decode_from_slice::<Bundle, _>(&data, bincode::config::standard())?.0)
    } else {
        Ok(load_bundle_raw(remaps, rsz))
    }
}

fn load_bundle_raw(remaps: &HashMap<String, Remap>, rsz: &RszMap) -> Bundle {
    log::info!("Loading assets from raw remap paths");
    let mut bundle = Bundle::default();
    bundle.load_from_paths(get_asset_paths(remaps), rsz);
    bundle
}

