use std::{collections::HashMap, error::Error, path::{Path, PathBuf}};

use eframe::egui::{ComboBox, TextEdit, Ui};
use ree_save_core::save::game::Game;
use serde::{self, Deserialize};

#[derive(Deserialize, Debug, Clone)]
pub struct UserAccountRaw {
    #[serde(rename = "AccountName")]
    account_name: String,
    #[serde(rename = "PersonaName")]
    persona_name: String,
    #[serde(rename = "MostRecent")]
    most_recent: Option<bool>,
    #[serde(rename = "Timestamp")]
    time_stamp: u32,
}

#[derive(Debug, Clone)]
pub struct UserAccount {
    pub steam_id: u64,
    pub account_name: String,
    pub persona_name: String,
    pub most_recent: bool,
    pub time_stamp: u32,
}

pub fn parse_accounts(path: &Path) -> Result<Vec<UserAccount>, Box<dyn Error>> {
    let data = std::fs::read_to_string(path)?;
    let users: HashMap<u64, UserAccountRaw> = keyvalues_serde::from_str(&data)?;
    let users = users.into_iter().map(|(k, v)| {
        UserAccount {
            steam_id: k,
            account_name: v.account_name,
            persona_name: v.persona_name,
            most_recent: v.most_recent.unwrap_or(false),
            time_stamp: v.time_stamp,
        }
    }).collect::<Vec<_>>();
    Ok(users)
}

pub fn get_save_files(path: &Path, steamid64: u64, game: Game) -> Vec<PathBuf> {
    let mut res = Vec::new();
    let save_path = path
        .join("userdata")
        .join((steamid64 & 0xffffffff).to_string())
        .join(game.get_appid().to_string())
        .join("remote/win64_save/");
    let paths = std::fs::read_dir(&save_path);
    if let Ok(paths) = paths {
        for path in paths {
            if let Ok(path) = path {
                let path = path.path();
                if let Some(ext) = path.extension() {
                    if ext == "bin" {
                        res.push(path.clone());
                    }
                }
            }
        }
    }
    res.sort();
    res
}

pub struct Steam {
    pub steam_id: Option<u64>,
    steam_id_text: String,
    #[cfg(not(target_arch = "wasm32"))]
    steam_path: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    users: Vec<UserAccount>,
    #[cfg(not(target_arch = "wasm32"))]
    selected_user_name: Option<String>,
}

impl Steam {
    pub fn new(id: Option<u64>, steam_path: String) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let steam_path = PathBuf::from(&steam_path);

        #[cfg(not(target_arch = "wasm32"))]
        let users = {
            use crate::steam;

            let mut path = steam_path.clone();
            path.push("config/loginusers.vdf");
            println!("Searching for users in steam path {}", steam_path.display());
            let users = steam::parse_accounts(&path).unwrap_or_default();
            println!("found {users:?}");
            users
        };

        let steam_id = id;
        let steam_id_text = steam_id.map(|x| x.to_string())
            .unwrap_or("".to_string());
        Self {
            steam_id,
            steam_id_text,
            #[cfg(not(target_arch = "wasm32"))]
            steam_path,
            #[cfg(not(target_arch = "wasm32"))]
            users,
            #[cfg(not(target_arch = "wasm32"))]
            selected_user_name: None,
        }
    }

    pub fn set_id(&mut self, id: Option<u64>) {
        self.steam_id = id;
        self.steam_id_text = id.map(|v| v.to_string()).unwrap_or_default();
    }

    pub fn edit_steam_id(&mut self, ui: &mut Ui) {
        ui.label("Steam ID:");
        if ui
            .add(TextEdit::singleline(&mut self.steam_id_text))
            .changed()
        {
            if let Ok(val) = u64::from_str_radix(&self.steam_id_text, 10) {
                self.steam_id = Some(val);
                {
                    self.selected_user_name = None;
                }
                // TODO: do this natively too with a config?
            } else {
                self.steam_id = None;
            }
        }
    }

    pub fn select_user(&mut self, ui: &mut Ui) {
        if !self.users.is_empty() {
            ui.label("Users");
            let label = self
                .selected_user_name
                .clone()
                .unwrap_or("Select User".to_string());
            ComboBox::from_id_salt("users_select")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for account in &self.users {
                        if ui.selectable_label(false, &account.persona_name).clicked() {
                            self.steam_id_text = account.steam_id.to_string();
                            self.selected_user_name = Some(account.persona_name.clone());
                            self.steam_id = Some(account.steam_id);
                        }
                    }
                });
        }
    }

    pub fn edit_steam_path(&mut self, ui: &mut Ui) -> bool {
        ui.label("Steam Path: ");
        let mut buf = self.steam_path.display().to_string();
        let res = ui.text_edit_singleline(&mut buf).changed();
        self.steam_path = PathBuf::from(buf);
        res
    }

    pub fn found_file(&mut self, ui: &mut Ui, _game: Game) -> Option<String> {
        let mut res = None;

        if !self.users.is_empty() && let Some(steam_id) = self.steam_id {
            let save_files = get_save_files(&self.steam_path, steam_id, _game);

            if !save_files.is_empty() {
                ComboBox::from_id_salt("save_select")
                    .selected_text("Select Save File")
                    .show_ui(ui, |ui| {
                        for file in &save_files {
                            let val = file.to_string_lossy().to_string();
                            if ui.selectable_label(false, &val).clicked() {
                                res = Some(val);
                            }
                        }
                    });
            }
        }

        res
    }
}

