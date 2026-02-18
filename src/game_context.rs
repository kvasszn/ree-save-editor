use std::{collections::HashMap};

use crate::{edit::copy::CopyBuffer, save::{game::Game, remap::Remap}, sdk::type_map::{TypeMap}};

#[derive(Debug, Clone, Default)]
pub struct AssetPaths {
    rsz: Option<String>,
    enums: Option<String>,
    msgs: Option<String>,
    mappings: Option<String>,
    strings: Option<String>,
    remap: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
impl AssetPaths {
    pub fn from_game(game: Game) -> Self {
        match game {
            Game::MHWILDS => {
                Self {
                    rsz: Some("assets/mhwilds/rszmhwilds_packed.json".to_string()),
                    enums: Some("assets/mhwilds/enumsmhwilds.json".to_string()),
                    msgs: Some("assets/mhwilds/combined_msgs.json".to_string()),
                    mappings: Some("assets/mhwilds/enums_mappings_mhwilds.json".to_string()),
                    strings: None,
                    remap: Some("assets/mhwilds/remapmhwilds.json".to_string())
                }
            },
            Game::MHST3 => {
                Self {
                    rsz: Some("assets/mhst3/rszmhst3.json".to_string()),
                    enums: Some("assets/mhst3/mhst3_enums.json".to_string()),
                    msgs: None,
                    mappings: None,
                    strings: Some("assets/mhst3/mhst3_strings.txt".to_string()),
                    remap: Some("assets/mhst3/mhst3_remap.json".to_string())
                }
            }
            Game::DD2 => {
                Self {
                    rsz: Some("assets/dd2/rszdd2.json".to_string()),
                    enums: None,
                    msgs: None,
                    mappings: None,
                    strings: None,
                    remap: None,
                }
            }
            Game::PRAGMATA => {
                Self {
                    rsz: Some("assets/pragmata/rszpragmata.json".to_string()),
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

#[derive(Debug, Default)]
pub struct GameCtx {
    pub type_map: TypeMap,
    pub copy_buffer: CopyBuffer,
    pub remaps: HashMap<String, Remap>,
}

impl GameCtx {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(paths: &AssetPaths) -> Self {
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


    #[cfg(target_arch = "wasm32")]
    pub fn start_loading_wasm(tx: std::sync::mpsc::Sender<(Game, GameCtx)>, game: Game, assets: AssetPaths) {
        let tx = tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut type_map = TypeMap::default();
            log::info!("[INFO] loading assets {:?}", &assets);
            if let Some(path) = &assets.rsz {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            type_map.load_rsz_from_data(&data.as_bytes());
                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }
            if let Some(path) = &assets.enums {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            type_map.load_enums_from_data(&data.as_bytes());
                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }

            if let Some(path) = &assets.msgs {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            type_map.load_msgs_from_data(&data.as_bytes());
                        } else {
                            log::error!("idk what happen");

                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }

            if let Some(path) = &assets.mappings {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            type_map.load_enum_mappings_from_data(&data.as_bytes());
                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }

            if let Some(path) = &assets.strings {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            type_map.load_strings_from_data(&data);
                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }


            let mut remaps: HashMap<String, Remap> = HashMap::new();
            if let Some(path) = &assets.remap {
                let path = resolve_url(path);
                match reqwest::get(&path).await {
                    Ok(response) => {
                        if let Ok(data) = response.text().await {
                            remaps = serde_json::from_str(&data).unwrap_or_default();
                        }
                    } 
                    Err(e) => log::error!("[ERROR] Could not load file from {path}: {e}")
                }
            }

            let game_ctx = Self {
                type_map,
                copy_buffer: CopyBuffer::Null,
                remaps,
            };
            let _ = tx.send((game, game_ctx));
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn resolve_url(path: &str) -> String {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let base_uri = document.base_uri().unwrap().unwrap();
    let url = web_sys::Url::new_with_base(path, &base_uri).unwrap();
    url.href()
}
