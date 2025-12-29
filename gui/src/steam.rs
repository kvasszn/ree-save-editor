use std::{error::Error, path::{Path, PathBuf}};

use egui::ahash::HashMap;
use serde::{self,Deserialize};

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

pub fn get_wilds_save_files(path: &Path, steamid64: u64) -> Vec<PathBuf> {
    let mut res = Vec::new();
    let mut save_path = path
        .join("userdata")
        .join((steamid64 & 0xffffffff).to_string())
        .join("2246340/remote/win64_save/data00-1.bin");
    //println!("System Save path: {}", &save_path.display());
    if let Ok(true) = std::fs::exists(&save_path) {
        res.push(save_path.clone());
    }
    save_path.set_file_name("data001Slot.bin");
    //println!("User Data Save path: {}", &save_path.display());
    if let Ok(true) = std::fs::exists(&save_path) {
        res.push(save_path.clone());
    }
    //println!("{res:?}");
    res
}
