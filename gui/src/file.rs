#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::{collections::HashSet, error::Error, io::{Cursor, Read, Seek}, sync::mpsc::{Receiver, Sender}, time::Duration};

use eframe::{self, egui::{ComboBox, Context, Id, ScrollArea, TextEdit, Ui, Window}};
use mhtame::{bindings::runner::ScriptRunner, edit::{EditContext, Editable}, file::StructRW, save::{SaveContext, SaveFile, game::{GAME_OPTIONS, Game}}, sdk::type_map::ContentLanguage};

use crate::{Config, steam::{UserAccount, get_wilds_save_files}, viewer::GameCtx};

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
}

impl FileView {
    pub fn new(config: &Config, idx: u64, language: ContentLanguage) -> Self {
        Self {
            //id: ui.make_persistent_id(format!("file_view_{idx}")),
            idx,
            input_file_picker: FilePicker::<false>::new("File"),
            output_path_picker: FilePicker::new("Output Path"),
            current_file: CurrentFile::Null,
            language,
            game: Game::MHWILDS,
            search: Search::default(),
            steam: Steam::new(config),
            show_popup: false,
            popup_msg: "".to_string(),
            repair: false,
        }
    }

    pub fn from_path(config: &Config, file_path: String, idx: u64, language: ContentLanguage) -> Self {
        let mut input_file_picker = FilePicker::<false>::new("File");
        input_file_picker.text = file_path.clone();
        let current_file = CurrentFile::Path(file_path);
        Self {
            idx,
            input_file_picker,
            output_path_picker: FilePicker::new("Output Path"),
            current_file,
            language,
            game: Game::MHWILDS,
            search: Search::default(),
            steam: Steam::new(config),
            show_popup: false,
            popup_msg: "".to_string(),
            repair: false,

        }
    }

    // TODO: make this a macro
    fn get_edit_context<'a>(&'a mut self, game_ctx: &'a mut GameCtx) -> EditContext<'a> {
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
    }

    pub fn update_file_path(&mut self, path: String) {
        self.input_file_picker.set_path(&path);
        self.current_file = CurrentFile::Path(path);

    }

    pub fn ui(&mut self, ui: &mut Ui, game_ctx: &mut GameCtx) {
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
                            let path = self.output_path_picker.text.clone();
                            use std::{fs::File};
                            let mut path = PathBuf::from(path);
                            let _ = std::fs::create_dir_all(&path);
                            let in_file_path = PathBuf::from(&file_path);
                            let file_name = in_file_path.file_name()
                                .map(|x| x.to_string_lossy().to_string())
                                .unwrap_or("data.bin".to_string());
                            path.push(file_name);
                            log::info!("Saving to {path:?}");
                            match File::create(&path) {
                                Ok(mut file) => {
                                    use std::io::Write;
                                    let _ = file.write_all(&data);
                                    self.popup_msg = format!("Saved to {:?}", path)
                                }
                                Err(e) => {
                                    self.popup_msg = format!("Could not create file {:?}: error: {e}", path)

                                }
                            }

                            #[cfg(target_arch = "wasm32")]
                            {
                                save_file_dialog(&file_path, data);
                                self.popup_msg = "Saved/Downloaded".to_string();
                            }

                        }
                        Err(e) => {
                            self.popup_msg = format!("Error saving {}", e)
                        }
                    }
                    self.show_popup = true;
                } else {
                    self.popup_msg = String::from("Need a steamid to save");
                    self.show_popup = true;
                }
            }

            if ui.button("Refresh / Load").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.input_file_picker.pending_result =
                        Some(FilePickResult::Native(self.input_file_picker.text.clone()));
                }
                self.reload();
            }

            ui.label("Game Profile");
            ComboBox::from_id_salt("GameProfilePicker")
                .selected_text(GAME_OPTIONS[self.game as usize].0)
                .show_ui(ui, |ui| {
                    for option in GAME_OPTIONS {
                        ui.selectable_value(&mut self.game, option.1, option.0);
                    }
                });


        });
        ui.horizontal(|ui| {
            self.output_path_picker.ui(ui);
        });
        ui.horizontal(|ui| {
            if let Some(path) = self.steam.found_file(ui) {
                self.update_file_path(path);
                self.reload();
            };
        });
        ui.horizontal(|ui| {
            ui.label("Try Repairing Corruption (WIP)");
            ui.checkbox(&mut self.repair, "");
            /*if ui.button("Run Script").clicked() {
                match &mut self.current_file {
                    CurrentFile::Loaded { loaded, .. } => {
                        let mut script_runner = ScriptRunner::new();
                        let res = script_runner.set_save_file_context(loaded);
                        match res {
                            Ok(_) => {
                                let res = script_runner.load_and_execute_from_file("scripts/foo.lua");
                                println!("[INFO] lua res: {res:?}");
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Script Runner: {e}")
                            }
                        }
                        let res = script_runner.get_save_file_context();
                        if let Some(res) = res {
                            *loaded = res;
                        }
                    }
                    _ => {}
                }
            }*/
        });

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
                        &game_ctx.remaps
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

    fn set_popup(&mut self, msg: String) {
        self.popup_msg = msg;
        self.show_popup = true;
    }

    fn read_save<R: Read + Seek>(&mut self, reader: &mut R) -> Option<SaveFile> {
        if let Some(steamid) = self.steam.steam_id {
            let mut ctx = SaveContext { key: steamid , game: self.game , repair: false };
            match SaveFile::read(reader, &mut ctx) {
                Ok(save) => {
                    return Some(save)
                },
                Err(e) => {
                    self.set_popup(format!("Error reading save: {e:?}"));
                }
            }
        } else {
            self.set_popup(format!("Cannot load save without steamid"));
        }
        None
    }

    pub fn reload(&mut self) -> bool {
        let current_file = std::mem::replace(&mut self.current_file, CurrentFile::Null);
        match current_file {
            CurrentFile::Path(path) => {
                // do this on wasm too?
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let expanded = shellexpand::full(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));
                    let path = PathBuf::from(expanded.as_ref());
                    if path.exists() {
                        match std::fs::File::open(&path) {
                            Ok(mut reader) => {
                                let save = self.read_save(&mut reader);
                                if let Some(save) = save {
                                    self.current_file = CurrentFile::Loaded {
                                        file_name: path.display().to_string(),
                                        loaded: save 
                                    };
                                    return true;
                                }
                            }
                            Err(e) => {
                                self.popup_msg = format!("Failed to open file: {e}");
                                self.show_popup = true;
                                return true;
                            }
                        }
                    }
                }
            },
            CurrentFile::FileData { file_name, bytes } => {
                let mut reader = Cursor::new(bytes);
                let save = self.read_save(&mut reader);
                if let Some(save) = save {
                    self.current_file = CurrentFile::Loaded {
                        file_name: file_name.to_string(),
                        loaded: save 
                    };
                    return true;
                }
            },
            CurrentFile::LoadedWeb { file_name, original_bytes, .. } => {
                let mut reader = Cursor::new(&original_bytes);
                let save = self.read_save(&mut reader);
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
                if self.reload() == false {
                    self.popup_msg = format!("Could not open file {:?}", self.input_file_picker.text);
                    self.show_popup = true;
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
            steam_path,
            users,
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

    pub fn select_user(&mut self, ui: &mut Ui) {
        #[cfg(not(target_arch = "wasm32"))]
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

    pub fn found_file(&mut self, ui: &mut Ui) -> Option<String> {
        let mut res = None;
        self.edit_steam_id(ui);
        self.select_user(ui);
        if !self.users.is_empty() {
            if let Some(steam_id) = self.steam_id {
                let save_files = get_wilds_save_files(&self.steam_path, steam_id);
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

    pub fn take(&mut self) -> Option<String> {
        let res = self.get();
        if res.is_some() {
            self.pending_result = None
        }
        res
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
