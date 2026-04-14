use mhtame::{game_context::AssetPaths, save::{game::Game, remap::Remap}, sdk::{asset::Assets, type_map::TypeMap}}; // Import from your lib
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Asset Repacker...");

    let paths = AssetPaths::from_game(Game::MHWILDS);

    // 1. Initialize your TypeMap and Remaps just like you currently do
    let mut type_map = TypeMap::default();
    if let Some(path) = &paths.rsz {
        type_map.load_rsz_from_path(path)?;
    }
    if let Some(path) = &paths.enums {
        type_map.load_enums_from_path(path)?;
    }
    if let Some(path) = &paths.msgs {
        type_map.load_msg_from_path(path)?;
    }
    if let Some(path) = &paths.mappings {
        type_map.load_enum_mappings_from_path(path)?;
    }
    if let Some(path) = &paths.strings {
        type_map.load_strings_from_path(path)?;
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
    assets.load_by_remaps(&remaps, &type_map)?;

    let output_path = "assets/mhwilds/packed_assets.bc";
    assets.bake_to_file(output_path)?;

    println!("Successfully packed assets to {}!", output_path);
    Ok(())
}
