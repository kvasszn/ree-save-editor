use std::path::PathBuf;

use ree_lib::enums::load_enum_map;
use ree_save_core::game_context::{GameData, load_game_configs};


pub fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let game_configs = load_game_configs("game_configs.json")?;

    for (game, config) in &game_configs {
        if let Some(bundle) = &config.bundle {
            log::info!("[{game:?}] Bundle");
            let game_data = GameData::load(config, true);
            let game_data = match game_data {
                Ok(g) => g,
                Err(e) => {
                    log::error!("Could not load game data from config {:?}: {e}", config);
                    continue
                }
            };

            let config = bincode::config::standard();
            let game_data_bincode = bincode::encode_to_vec(&game_data.bundle, config)?;
            std::fs::write(bundle, game_data_bincode)?;
            log::info!("[{game:?}] Packed bundle to {bundle}");
        }

        if let (Some(enums_raw), Some(enums)) = (&config.enums_raw, &config.enums) {
            log::info!("[{game:?}] Enums");
            let enum_map = load_enum_map(&PathBuf::from(enums_raw))?;
            let json = serde_json::to_string(&enum_map)?;
            std::fs::write(enums, json)?;
            log::info!("[{game:?}] Packed enums to {enums}");

        }
    }

    Ok(())
}
