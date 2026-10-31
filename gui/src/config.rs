use ree_lib::language::Language;
use ree_save_core::{edit::EditorConfig, save::game::Game};
use serde::{Deserialize, Serialize};

// TODO: add some workspace config stuff
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[cfg(not(target_os = "windows"))]
    #[serde(default)]
    #[serde(rename = "steam_path_linux")]
    pub steam_path: String,
    #[cfg(target_os = "windows")]
    #[serde(default)]
    #[serde(rename = "steam_path_windows")]
    pub steam_path: String,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub game: Game,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    workspace: WorkspaceConfig
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {

}

pub fn load_config(path: &str) -> Config {
    std::fs::read(path)
        .map(|c| {
            serde_json::from_slice(c.as_slice())
                .inspect_err(|e| log::error!("Error: {e}. Could not parse config, using default as fallback"))
                .unwrap_or_default()
        })
        .inspect_err(|e| log::error!("Error: {e}. Could not read config from path {}, using default as fallback", path))
        .unwrap_or_default()
}

pub fn load_config_checked(path: &str) -> anyhow::Result<Config> {
    let data = std::fs::read(path)?;
    let config = serde_json::from_slice(data.as_slice())?;
    Ok(config)
}
