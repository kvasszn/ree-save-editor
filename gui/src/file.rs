use std::collections::HashMap;
use std::path::PathBuf;
use std::{collections::HashSet, error::Error, io::{Cursor, Read, Seek}, sync::mpsc::{Receiver, Sender}, time::Duration};

use eframe::egui::{self, Align2, Order};
use eframe::{self, egui::{ComboBox, ScrollArea, TextEdit, Ui}};
use mhtame::game_context::GameCtx;
use mhtame::save::corrupt::CorruptSaveReader;
use mhtame::sdk::type_map::{TypeMap};
use mhtame::{edit::{EditContext, Editable}, file::StructRW, save::{SaveContext, SaveFile, game::{GAME_OPTIONS, Game}}, sdk::type_map::ContentLanguage};


use crate::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::steam::UserAccount;

pub struct FileView {
    pub idx: u64,
    pub input_file_picker: FilePicker<false>,
    pub output_path_picker: FilePicker<true>,
    pub current_file: CurrentFile,
    pub language: ContentLanguage,
    pub game: Game,
    pub search: Search,
    pub steam: Steam,
    popup_msg: String,
    show_popup: bool,
    repair: bool,
    brute_force: bool,
}

impl FileView {
    pub fn new(config: &Config, idx: u64, language: ContentLanguage) -> Self {
        let mut output_path_picker = FilePicker::<true>::new("Output Path");
        output_path_picker.text = "./outputs".to_string();
        let game = if cfg!(feature = "mhwilds") {
            Game::MHWILDS
        } else if cfg!(feature = "re9") {
            Game::RE9
        } else if cfg!(feature = "mhst3") {
            Game::MHST3
        } else {
            Game::MHWILDS
        };
        Self {
            idx,
            input_file_picker: FilePicker::<false>::new("File"),
            output_path_picker,
            current_file: CurrentFile::Null,
            language,
            game,
            search: Search::default(),
            steam: Steam::new(config),
            show_popup: false,
            popup_msg: "".to_string(),
            repair: false,
            brute_force: false,
        }
    }


    pub fn from_data(config: &Config, file_name: String, bytes: Vec<u8>, idx: u64, language: ContentLanguage) -> Self {
        let mut file = FileView::new(config, idx, language);
        let mut input_file_picker = FilePicker::<false>::new("File");
        input_file_picker.text = file_name.clone();
        file.input_file_picker.text = file_name.clone();
        file.current_file = CurrentFile::FileData{file_name, bytes};
        file
    }

    pub fn from_path(config: &Config, file_path: String, idx: u64, language: ContentLanguage) -> Self {
        let mut file = FileView::new(config, idx, language);
        let mut input_file_picker = FilePicker::<false>::new("File");
        input_file_picker.text = file_path.clone();
        file.input_file_picker.text = file_path.clone();
        file.current_file = CurrentFile::Path(file_path);
        file
    }

    // TODO: make this a macro
    /*fn _get_edit_context<'a>(&'a mut self, game_ctx: &'a mut GameCtx) -> EditContext<'a> {
        let edit_ctx = EditContext::new(
            &game_ctx.type_map,
            &self.search.fields,
            &self.search.leaf_nodes,
            &self.search.range,
            &mut game_ctx.copy_buffer,
            self.language,
            &game_ctx.remaps
        );
        edit_ctx
    }*/

    pub fn update_file_path(&mut self, path: String) {
        self.input_file_picker.set_path(&path);
        self.current_file = CurrentFile::Path(path);
    }

    fn update_input_file(&mut self, game: &mut GameCtx) {
        let res = self.input_file_picker.pending_result.take();
        if let Some(res) = res {
            match res {
                FilePickResult::Native(p) => {
                    let file = CurrentFile::Path(p);
                    self.load(file, game);
                }
                FilePickResult::Wasm { name, data } => {
                    let file = CurrentFile::FileData {
                        file_name: name,
                        bytes: data,
                    };
                    self.load(file, game);
                }
            }
        }
    }

    pub fn show_popup(&mut self, ui: &mut Ui) {
        if self.show_popup {
            let popup_id = ui.make_persistent_id((self.idx, "msg_popup"));
            eframe::egui::Area::new(popup_id)
                .order(Order::Foreground)
                .anchor(Align2::CENTER_CENTER, eframe::egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.add_space(10.0);
                        ui.label(&self.popup_msg);
                        ui.vertical_centered(|ui| {
                            if ui.button("Close").clicked() {
                                self.show_popup = false;
                            }
                        });
                    });
                });
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, game_contexts: &mut HashMap<Game, GameCtx>) {
        self.show_popup(ui);

        let mut game_ctx = game_contexts.get_mut(&self.game);

        if let Some(game_ctx) = game_ctx.as_deref_mut() {
            self.update_input_file(game_ctx);
        }

        ui.horizontal(|ui| {
            self.input_file_picker.ui(ui);
            self.input_file_picker.update();
            self.output_path_picker.update();
            if ui.button("Save").clicked() {
                if let Some(steamid) = self.steam.steam_id {
                    match self.current_file.write_save_to_buf(steamid) {
                        Ok((file_path, data)) => {
                            // quite disgusting but oh well
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let output_path = self.output_path_picker.text.clone();
                                use std::{fs::File};
                                let mut path = PathBuf::from(output_path.clone());
                                let _ = std::fs::create_dir_all(&path);
                                let in_file_path = PathBuf::from(&file_path);
                                let file_name = in_file_path.file_name()
                                    .map(|x| x.to_string_lossy().to_string())
                                    .unwrap_or("data.bin".to_string());
                                path.push(file_name);
                                log::info!("Saving to {path:?}");
                                match File::create(&path) {
                                    Ok(mut file) => {
                                        // idk wtf im doing here uh
                                        use std::io::Write;
                                        let _ = file.write_all(&data);
                                        let path = output_path.clone();
                                        let path = shellexpand::full(&path).unwrap_or(output_path.clone().into());
                                        let path = std::fs::canonicalize(path.to_string()).unwrap_or(output_path.clone().into());
                                        self.set_popup(format!("Saved to {:?}", path));
                                    }
                                    Err(e) => {
                                        self.set_popup(format!("Could not create file {:?}: error: {e}", path));

                                    }
                                }
                            }

                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::save_file_dialog(&file_path, data);
                                self.set_popup(format!("Saved/Downloaded"));
                            }

                        }
                        Err(e) => {
                            self.set_popup(format!("Error saving {}", e));
                        }
                    }
                } else {
                    self.set_popup(format!("Need a steamid to save"));
                }
            }

            if ui.button("Refresh / Load").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.input_file_picker.pending_result =
                        Some(FilePickResult::Native(self.input_file_picker.text.clone()));
                }
                if let Some(game_ctx) = game_ctx.as_deref_mut() {
                    self.update_input_file(game_ctx);
                    //self.reload(game_ctx);
                }
            }

            ui.label("Game Profile");
            let changed_game = ComboBox::from_id_salt("GameProfilePicker")
                .selected_text(GAME_OPTIONS[self.game as usize].0)
                .show_ui(ui, |ui| {
                    for option in GAME_OPTIONS {
                        ui.selectable_value(&mut self.game, option.1, option.0);
                }
                }).response.changed();
            if let Some(game_ctx) = game_ctx.as_deref_mut() {
                if changed_game {
                    self.reload(game_ctx);
                }
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        ui.horizontal(|ui| {
            self.output_path_picker.ui(ui);
        });

        ui.horizontal(|ui| {
            if let Some(path) = self.steam.found_file(ui, self.game) {
                self.update_file_path(path);
                if let Some(game_ctx) = game_ctx.as_deref_mut() {
                    self.reload(game_ctx);
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            self.steam.edit_steam_path(ui);
        });

        if let Some(game_ctx) = game_ctx.as_deref_mut() {
            self.search.ui(ui, &self.current_file, &game_ctx.type_map);
        }

        ui.horizontal(|ui| {
            ui.label("Try Repairing Corruption (WIP)");
            ui.checkbox(&mut self.repair, "");
            ui.label("Brute Force SteamID");
            ui.checkbox(&mut self.brute_force, "");
        });

        if let Some(game_ctx) = game_ctx {
            ScrollArea::both().auto_shrink(false).max_width(f32::INFINITY).show(ui, |ui| {
                ui.style_mut().spacing.item_spacing *= 1.5;
                ui.style_mut().spacing.button_padding *= 1.5;
                for (_style, font_id) in ui.style_mut().text_styles.iter_mut() {
                    font_id.size *= 1.2;
                }
                match &mut self.current_file {
                    CurrentFile::LoadedWeb { loaded, .. } | CurrentFile::Loaded { loaded, .. } => {
                        let mut edit_ctx = EditContext::new(
                            &game_ctx.type_map,
                            &self.search.fields,
                            &self.search.leaf_nodes,
                            &self.search.range,
                            &mut game_ctx.copy_buffer,
                            self.language,
                            &game_ctx.remaps,
                            &game_ctx.assets
                        );
                        loaded.edit(ui, &mut edit_ctx);
                    }
                    _ => {
                        ui.label("No File Loaded.");
                        #[cfg(target_arch = "wasm32")]
                        ui.label("Drag and Drop or use the file dialog");
                        #[cfg(target_os = "windows")]
                        ui.label("Drag and Drop or use the file dialog");
                    }
                }
            });
        }
    }

    fn set_popup(&mut self, msg: String) {
        log::info!("popup msg set: {}", msg);
        self.popup_msg = msg;
        self.show_popup = true;
    }

    fn read_save<R: Read + Seek>(&mut self, reader: &mut R, game_ctx: &mut GameCtx) -> Option<SaveFile> {
        if self.steam.steam_id.is_some() || self.brute_force {
            let mut save_ctx = SaveContext{
                key: if self.brute_force {
                    None
                } else {self.steam.steam_id},
                game: self.game,
            };

            if self.repair {
                let data = SaveFile::read_data(reader, &mut save_ctx).ok()?;
                let mut reader = Cursor::new(data);
                let mut corrupted_reader = CorruptSaveReader::new(&game_ctx.type_map, self.game);
                //let save_file = corrupted_reader.read_n_objects(&mut reader, "app.savedata.cUserSaveParam", 3);
                let save_file = corrupted_reader.read_missing(&mut reader, &[(0xdbe3f199, "app.savedata.cUserSaveData"), (0x85e904c1, "via.storage.saveService.SaveFileDetail")]);
                if self.brute_force {
                    self.steam.steam_id = save_ctx.key; 
                    if let Some(steam_id) = save_ctx.key { self.steam.steam_id_text = steam_id.to_string(); }
                }
                return Some(save_file)
            }
            match SaveFile::read(reader, &mut save_ctx) {
                Ok(save) => {
                    if self.brute_force {
                        self.steam.steam_id = save_ctx.key; 
                        if let Some(steam_id) = save_ctx.key { self.steam.steam_id_text = steam_id.to_string(); }
                    }
                    return Some(save)
                },
                Err(e) => {
                    self.set_popup(format!("Error reading save: {e:?}"));
                }
            }
            if self.brute_force {
                self.steam.steam_id = save_ctx.key; 
                if let Some(steam_id) = save_ctx.key { self.steam.steam_id_text = steam_id.to_string(); }
            }
        } else {
            self.set_popup(format!("Cannot load save without steamid, or brute force"));
        }
        None
    }

    pub fn load(&mut self, current_file: CurrentFile, game_ctx: &mut GameCtx) {
        match current_file {
            CurrentFile::Path(path) => {
                let expanded =
                    shellexpand::full(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));
                let path = PathBuf::from(expanded.as_ref());
                if path.exists() {
                    match std::fs::File::open(&path) {
                        Ok(mut reader) => {
                            let save = self.read_save(&mut reader, game_ctx);
                            if let Some(save) = save {
                                self.current_file = CurrentFile::Loaded {
                                    file_name: path.display().to_string(),
                                    loaded: save,
                                };
                            }
                        }
                        Err(e) => {
                            self.set_popup(format!("Failed to open file: {e}"));
                        }
                    }
                }
            }
            CurrentFile::FileData { file_name, bytes } => {
                let mut reader = Cursor::new(&bytes);

                let save = self.read_save(&mut reader, game_ctx);
                if let Some(save) = save {
                    self.current_file = CurrentFile::LoadedWeb {
                        file_name,
                        original_bytes: bytes,
                        loaded: save,
                    };
                } else {
                    self.current_file = CurrentFile::FileData {
                        file_name,
                        bytes: bytes,
                    };
                }
            }
            _ => {
                self.set_popup(format!("Need a file to load"));
            }
        }
    }

    pub fn reload(&mut self, game_ctx: &mut GameCtx) -> bool {
        let current_file = std::mem::replace(&mut self.current_file, CurrentFile::Null);
        match current_file {
            CurrentFile::Path(path) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let expanded = shellexpand::full(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));
                    let path = PathBuf::from(expanded.as_ref());
                    if path.exists() {
                        match std::fs::File::open(&path) {
                            Ok(mut reader) => {
                                let save = self.read_save(&mut reader, game_ctx);
                                if let Some(save) = save {
                                    self.current_file = CurrentFile::Loaded {
                                        file_name: path.display().to_string(),
                                        loaded: save 
                                    };
                                    return true;
                                }
                            }
                            Err(e) => {
                                eprintln!("[ERROR] opening file {e:?}");
                                //self.set_popup(format!("Failed to open file: {e}"));
                                return true;
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                log::error!("Cannot load as path {path} on web, there should be no way to get here");
            },
            CurrentFile::FileData { file_name, bytes } => {
                let mut reader = Cursor::new(&bytes);
                let save = self.read_save(&mut reader, game_ctx);
                #[cfg(target_arch = "wasm32")]
                if let Some(save) = save {
                    self.current_file = CurrentFile::LoadedWeb {
                        file_name,
                        original_bytes: bytes,
                        loaded: save,
                    };
                    return true;
                } else {
                    self.current_file = CurrentFile::FileData {
                        file_name,
                        bytes: bytes,
                    };
                }

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(save) = save {
                    self.current_file = CurrentFile::Loaded {
                        file_name,
                        loaded: save,
                    };
                    return true;
                } else {
                    self.current_file = CurrentFile::FileData {
                        file_name,
                        bytes: bytes,
                    };
                }
            },
            CurrentFile::LoadedWeb { file_name, original_bytes, .. } => {
                let mut reader = Cursor::new(&original_bytes);
                let save = self.read_save(&mut reader, game_ctx);
                if let Some(save) = save {
                    self.current_file = CurrentFile::LoadedWeb {
                        file_name: file_name.to_string(),
                        original_bytes,
                        loaded: save 
                    };
                    return true;
                }
            },
            CurrentFile::Loaded { .. } | CurrentFile::Null => {
                self.current_file = CurrentFile::Path(self.input_file_picker.text.clone());
                if self.reload(game_ctx) == false {
                    //self.set_popup(format!("Could not open file {:?}", self.input_file_picker.text));
                    return false;
                };
            }
        }
        return false;
    }
}

#[derive(Debug, Clone)]
pub struct Search {
    buffer: String,
    fields: HashSet<(u32, u32)>,
    leaf_nodes: HashSet<(u32, u32)>,
    range: core::ops::Range<usize>,
    max_depth: u32,
    time: Option<Duration>,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            range: 0..usize::MAX,
            max_depth: 100,
            time: None,
            buffer: String::default(),
            fields: HashSet::new(),
            leaf_nodes: HashSet::new(),
        }
    }
}

impl Search {
    pub fn ui(&mut self, ui: &mut Ui, current_file: &CurrentFile, type_map: &TypeMap) {
        ui.horizontal(|ui| {
            ui.label("Search: ");
            if ui.text_edit_singleline(&mut self.buffer).changed() {
                let (search_buffer, range) = Self::parse_query(&self.buffer);
                self.range = range;
                self.fields.clear();
                self.leaf_nodes.clear();
                if !self.buffer.is_empty() {
                    let start = web_time::Instant::now();
                    match &current_file {
                        CurrentFile::Loaded { loaded, .. }
                        | CurrentFile::LoadedWeb { loaded, .. } => {
                            for field in &loaded.fields {
                                if let Some(type_info) = type_map.get_by_hash(field.1.hash)
                                {
                                    let fs = type_map.search(
                                        type_info,
                                        &search_buffer,
                                        self.max_depth as usize,
                                    );
                                    self.fields.extend(fs.0);
                                    self.leaf_nodes.extend(fs.1);
                                }
                            }
                        }
                        _ => {
                            ();
                        }
                    }
                    self.time = Some(start.elapsed());
                } else {
                    self.time = None;
                }
            }
            if let Some(t) = &self.time {
                ui.label(format!("{} ms", t.as_millis()));
            }
        });
    }
    fn parse_query(input: &str) -> (String, core::ops::Range<usize>) {
        let Some(start_bracket) = input.find('[') else {
            return (input.to_string(), 0..usize::MAX);
        };

        let Some(end_bracket) = input.find(']') else {
            return (input.to_string(), 0..usize::MAX);
        };

        let name = input[0..start_bracket].to_string();
        let content = &input[start_bracket + 1..end_bracket];

        if let Some(colon_pos) = content.find(':') {
            let start_str = &content[0..colon_pos];
            let end_str = &content[colon_pos + 1..];

            let start = start_str.parse::<usize>().unwrap_or(0);
            let end = end_str.parse::<usize>().unwrap_or(usize::MAX);

            (name, start..end)
        } else {
            if let Ok(idx) = content.parse::<usize>() {
                (name, idx..idx + 1)
            } else {
                (name, 0..usize::MAX)
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Steam {
    steam_id: Option<u64>,
    steam_id_text: String,
    #[cfg(not(target_arch = "wasm32"))]
    steam_path: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    users: Vec<UserAccount>,
    #[cfg(not(target_arch = "wasm32"))]
    selected_user_name: Option<String>,
}

impl Steam {
    pub fn new(config: &Config) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let steam_path = PathBuf::from(&config.steam_path);

        #[cfg(not(target_arch = "wasm32"))]
        let users = {
            use crate::steam;

            let mut path = steam_path.clone();
            path.push("config/loginusers.vdf");
            println!("Searching for users in steam path {}", steam_path.display());
            let users = steam::parse_accounts(&path)
                .unwrap_or_default();
            println!("found {users:?}");
            users
        };

        let steam_id = config.steamid.as_ref().and_then(|s| {
            if let Some(hex) = s.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                u64::from_str_radix(s, 10).ok()
            }
        });

        #[cfg(target_arch = "wasm32")]
        let steam_id = {
            let mut steam_id = steam_id;
            if steam_id.is_none() {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        if let Ok(Some(saved_str)) = storage.get_item("mhtame_steam_id") {
                            // Parse the saved string
                            if let Ok(val) = u64::from_str_radix(&saved_str, 10) {
                                steam_id = Some(val);
                            }
                        }
                    }
                }
            }
            steam_id
        };

        let steam_id_text = steam_id.map(|x| x.to_string()).unwrap_or("".to_string());
        Self {
            steam_id,
            steam_id_text,
            #[cfg(not(target_arch = "wasm32"))]
            steam_path,
            #[cfg(not(target_arch = "wasm32"))]
            users,
            #[cfg(not(target_arch = "wasm32"))]
            selected_user_name: None
        }
    }


    pub fn edit_steam_id(&mut self, ui: &mut Ui) {
        ui.label("Steam ID:");
        if ui.add(TextEdit::singleline(&mut self.steam_id_text)).changed() {
            if let Ok(val) = u64::from_str_radix(&self.steam_id_text, 10) {
                self.steam_id = Some(val);
                #[cfg(not(target_arch = "wasm32"))]
                { self.selected_user_name = None; }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        if let Ok(Some(storage)) = window.local_storage() {
                            // We save the raw string "7656..."
                            let _ = storage.set_item("mhtame_steam_id", &self.steam_id_text);
                        }
                    }
                }
                // TODO: do this natively too with a config?
            } else {
                self.steam_id = None; 
            }

        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn select_user(&mut self, ui: &mut Ui) {
        if !self.users.is_empty() {
            ui.label("Users");
            let label = self.selected_user_name.clone().unwrap_or("Select User".to_string());
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn edit_steam_path(&mut self, ui: &mut Ui) -> bool{
        ui.label("Steam Path: ");
        let mut buf = self.steam_path.display().to_string();
        let res = ui.text_edit_singleline(&mut buf).changed();
        self.steam_path = PathBuf::from(buf);
        res
    }

    pub fn found_file(&mut self, ui: &mut Ui, _game: Game) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        let mut res = None;
        #[cfg(target_arch = "wasm32")]
        let res = None;
        self.edit_steam_id(ui);

        #[cfg(not(target_arch = "wasm32"))]
        self.select_user(ui);

        #[cfg(not(target_arch = "wasm32"))]
        if !self.users.is_empty() {
            if let Some(steam_id) = self.steam_id {
                use crate::steam::get_save_files;

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
        }
        return res;
    }
}
#[derive(Debug)]
pub enum CurrentFile {
    Null,
    Path(String),
    FileData { file_name: String, bytes: Vec<u8> },
    Loaded { file_name: String, loaded: SaveFile },
    LoadedWeb { file_name: String, original_bytes: Vec<u8>, loaded: SaveFile },
}

#[derive(Debug, Clone)]
pub enum FilePickResult {
    Native(String),
    Wasm { name: String, data: Vec<u8> },
}

// could make name a const generic
pub struct FilePicker<const FOLDER: bool> {
    tx: Sender<FilePickResult>,
    rx: Receiver<FilePickResult>,
    label: String,
    text: String,
    pending_result: Option<FilePickResult>
}

impl CurrentFile {
    pub fn get_file_name(&self) -> Option<&str> {
        match self {
            CurrentFile::Null | CurrentFile::FileData {..} => None,
            CurrentFile::Path(path) => Some(path.as_str()),
            CurrentFile::Loaded { file_name, ..} | CurrentFile::LoadedWeb { file_name, .. } => Some(file_name.as_str())
        }
    }
    pub fn write_save_to_buf(&self, key: u64) -> Result<(String, Vec<u8>), Box<dyn Error>> {
        match self {
            CurrentFile::Null | CurrentFile::Path(_)| CurrentFile::FileData {..} => {
                Err("Can't save unloaded file".into())
            }
            CurrentFile::Loaded { file_name, loaded } | CurrentFile::LoadedWeb { file_name, loaded, .. } => {
                Ok((file_name.clone(), loaded.to_bytes(key)?))
            }
        }
    }
}

impl<const FOLDER: bool> FilePicker<FOLDER> {
    pub fn new(label: &str) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            label: label.to_string(),
            text: String::new(),
            tx,
            rx,
            pending_result: None,
        }
    }

    pub fn update(&mut self) {
        if let Ok(result) = self.rx.try_recv() {
            match &result {
                FilePickResult::Native(p) => self.text = p.clone(),
                FilePickResult::Wasm{ name, .. } => self.text = name.clone(),
            }
            self.pending_result = Some(result);
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.text = path.to_string();
    }

    pub fn get(&mut self) -> Option<String> {
        self.update();
        if let Some(result) = &self.pending_result {
            return Some(match result {
                FilePickResult::Native(p) => p.clone(),
                FilePickResult::Wasm{ name, .. } => name.clone(),
            })
        }
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn take(&mut self) -> Option<String> {
        let res = self.get();
        if res.is_some() {
            self.pending_result = None
        }
        res
    }

    #[cfg(target_arch = "wasm32")]
    pub fn take(&mut self) -> Option<FilePickResult> {
        let res = self.get();
        if res.is_some() {
            let res = self.pending_result.take();
            return res;
        }
        None
    }

    pub fn button_open(&mut self, ui: &mut Ui) {
        if ui.button("Open").clicked() {
            self.spawn_dialog();
        }
    }
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label(format!("{}:", &self.label));
        ui.add(TextEdit::singleline(&mut self.text));
        if ui.button("Browse").clicked() {
            self.spawn_dialog();
        }
    }

    pub fn spawn_dialog(&self) {
        let tx = self.tx.clone();
        let title = self.label.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
            let dialog = rfd::FileDialog::new()
                .set_directory(cur)
                .set_title(&title);
            let result = if FOLDER {
                dialog.pick_folder()
            } else {
                dialog.pick_file()
            };
            if let Some(path) = result {
                let _ = tx.send(FilePickResult::Native(path.display().to_string()));
            }
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let dialog = rfd::AsyncFileDialog::new().set_title(&title);
            let file_handle = dialog.pick_file().await;
            if let Some(file) = file_handle {
                let name = file.file_name();
                let data = file.read().await;
                let _ = tx.send(FilePickResult::Wasm { name, data });
            }
        });
    }
}
