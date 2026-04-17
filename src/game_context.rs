use std::{collections::HashMap};

use crate::{edit::copy::CopyBuffer, save::{game::Game, remap::Remap}, sdk::{asset::Assets, type_map::TypeMap}};

#[derive(Debug, Clone, Default)]
pub struct AssetPaths {
    pub rsz: Option<String>,
    pub enums: Option<String>,
    pub msgs: Option<String>,
    pub mappings: Option<String>,
    pub strings: Option<String>,
    pub remap: Option<String>,
    pub packed_assets: Option<String>,
}

impl AssetPaths {
    pub fn from_game(game: Game) -> Self {
        let p = |s: &str| Some(s.to_string());

        #[cfg(target_arch = "wasm32")]
        let gz = |s: &str| Some(format!("{s}.gz"));
        #[cfg(not(target_arch = "wasm32"))]
        let gz = |s: &str| Some(s.to_string());

        match game {
            Game::MHWILDS => Self {
                rsz: gz("assets/mhwilds/rszmhwilds_packed.json"),
                enums: gz("assets/mhwilds/enumsmhwilds.json"),
                msgs: gz("assets/mhwilds/combined_msgs.json"),
                mappings: p("assets/mhwilds/enums_mappings_mhwilds.json"),
                remap: p("assets/mhwilds/remapmhwilds.json"),
                packed_assets: p("assets/mhwilds/packed_assets.bc"),
                ..Default::default() // Automatically fills the rest with `None`!
            },
            Game::MHST3 => Self {
                rsz: p("assets/mhst3/rszmhst3.json"),
                enums: p("assets/mhst3/mhst3_enums.json"),
                strings: p("assets/mhst3/mhst3_strings.txt"),
                remap: p("assets/mhst3/mhst3_remap.json"),
                ..Default::default()
            },
            Game::RE9 => Self {
                rsz: p("assets/re9/rszre9.json"),
                enums: p("assets/re9/enums_re9.json"),
                strings: p("assets/re9/strings.txt"),
                remap: p("assets/re9/remap.json"),
                ..Default::default()
            },
            Game::DD2 => Self {
                rsz: p("assets/dd2/rszdd2.json"),
                enums: p("assets/dd2/enumsdd2.json"),
                ..Default::default()
            },
            Game::PRAGMATA => Self {
                rsz: p("assets/pragmata/rszpragmata.json"),
                enums: p("assets/pragmata/enumspragmata.json"),
                strings: p("assets/pragmata/strings_pragmata.txt"),
                ..Default::default()
            },
            Game::MHRISE => Self {
                rsz: p("assets/mhrise/rszmhrise.json"),
                ..Default::default()
            },
            Game::SF6 => Self {
                rsz: p("assets/sf6/rszsf6.json"),
                ..Default::default()
            },
        }}
}

#[derive(Debug, Default)]
pub struct GameCtx {
    pub type_map: TypeMap,
    pub copy_buffer: CopyBuffer,
    pub remaps: HashMap<String, Remap>,
    pub assets: Assets
}

impl GameCtx {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(paths: &AssetPaths) -> Self {
        let mut type_map = TypeMap::default();
        if let Some(path) = &paths.rsz {
            let _ = type_map.load_rsz_from_path(path)
                .inspect_err(|e| log::error!("[ERROR] Could not load rsz from {path}: {e}"));
        }
        if let Some(path) = &paths.enums {
            let _ = type_map.load_enums_from_path(path)
                .inspect_err(|e| log::error!("[ERROR] Could not load enums from {path}: {e}"));
        }
        if let Some(path) = &paths.msgs {
            let _ = type_map.load_msg_from_path(path)
                .inspect_err(|e| log::error!("[ERROR] Could not load msgs from {path}: {e}"));
        }
        if let Some(path) = &paths.mappings {
            let _ = type_map.load_enum_mappings_from_path(path)
                .inspect_err(|e| log::error!("[ERROR] Could not load mappigns from {path}: {e}"));
        }
        if let Some(path) = &paths.strings {
            let _ = type_map.load_strings_from_path(path)
                .inspect_err(|e| log::error!("[ERROR] Could not load strings from {path}: {e}"));
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

        let mut assets = Assets::default();
        if let Some(packed_assets_path) = &paths.packed_assets {
            println!("[INFO] Loading assets from bitcode binary");
            let a = Assets::load_baked(&packed_assets_path);
            match a {
                Ok(a) => assets = a,
                Err(e) => eprintln!("[ERROR] Loading assets {e}")
            }
        } else {
            let res = assets.load_by_remaps(&remaps, &type_map);
            println!("Loading asset res {res:?}");
        };
        //let res = assets.load_by_remaps(&remaps, &type_map);

        Self {
            type_map,
            copy_buffer: CopyBuffer::Null,
            remaps,
            assets
        }
    }


    #[cfg(target_arch = "wasm32")]
    pub fn start_loading_wasm(tx: std::sync::mpsc::Sender<(Game, GameCtx)>, game: Game, assets: AssetPaths) {
        let tx = tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut type_map = TypeMap::default();
            log::info!("[INFO] loading assets {:?}", &assets);
            if let Some(path) = &assets.rsz {
                let _ = crate::load_from_url(path, |e| type_map.types = e).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load rsz from url {path}: {e}"));
            }
            if let Some(path) = &assets.enums {
                let _ = crate::load_from_url(path, |e| type_map.enums= e).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load enums from url {path}: {e}"));
            }
            if let Some(path) = &assets.msgs {
                let _ = crate::load_from_url(path, |e| type_map.msgs = e).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load msgs from url {path}: {e}"));
            }
            if let Some(path) = &assets.mappings {
                let _ = crate::load_from_url(path, |e| type_map.enum_mappings = e).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load mappings from url {path}: {e}"));
            }

            if let Some(path) = &assets.strings {
                let _ = crate::with_str_loaded_from_url(path, |data| {
                    type_map.load_strings_from_data(data)?;
                    Ok(())
                }).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load strings from url {path}: {e}"));
            }

            let mut remaps: HashMap<String, Remap> = HashMap::new();
            if let Some(path) = &assets.remap {
                let _ = crate::with_str_loaded_from_url(path, |data| {
                    remaps = serde_json::from_str(data)?;
                    Ok(())
                }).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load remaps from url {path}: {e}"));
            }

            let mut game_assets = Assets::default();
            if let Some(path) = &assets.packed_assets {
                let _ = crate::with_str_loaded_from_url(path, |data| {
                    println!("[INFO] Loading assets from bitcode binary");
                    let a = Assets::load_baked_bytes(data.as_bytes());
                    match a {
                        Ok(a) => game_assets = a,
                        Err(e) => eprintln!("[ERROR] Loading assets {e}")
                    }
                    Ok(())
                }).await
                    .inspect_err(|e| log::error!("[ERROR] Could not load strings from url {path}: {e}"));
            } else {
                let res = game_assets.load_by_remaps(&remaps, &type_map);
                println!("Loading asset res {res:?}");
            }

            let game_ctx = Self {
                type_map,
                copy_buffer: CopyBuffer::Null,
                remaps,
                assets: game_assets,
            };
            let _ = tx.send((game, game_ctx));
        });
    }
}
