use std::{collections::HashMap};
use std::path::PathBuf;
use std::{
    error::Error,
    io::{Cursor},
    sync::mpsc::{Receiver, Sender},
};
use eframe::egui::{self, Align2, Order};
use eframe::{
    self,
    egui::{ComboBox, TextEdit, Ui},
};
use ree_lib::{game_context::{GameCtx}, save::SaveFlags};
use ree_lib::save::SaveOptions;
use ree_lib::save::corrupt::CorruptSaveReader;
use ree_lib::{
    save::{
        SaveFile,
        game::{Game},
    },
    sdk::type_map::ContentLanguage,
};

use crate::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::steam::UserAccount;

pub struct SaveMover {
    pub idx: u64,
    pub steam_folder_picker: FilePicker<true>,
    pub switch_folder_picker: FilePicker<true>,
    pub output_path_picker: FilePicker<true>,
    pub current_file1: CurrentFile,
    pub current_file2: CurrentFile,
    pub language: ContentLanguage,
    pub game: Game,
    pub steam: Steam,
    popup_msg: String,
    show_popup: bool,
    show_alert: bool,
    transfer_type: TransferType,
    repair: bool,
    brute_force: bool,
    curve_index: Option<usize>,
    brute_force_options: (u64, u64),
    dump: bool
}

impl SaveMover {
    pub fn new(config: &Config, idx: u64, language: ContentLanguage) -> Self {
        let mut output_path_picker = FilePicker::<true>::new("Output Path");
        output_path_picker.text = "./outputs".to_string();
        let game = Game::MHRISE;
        Self {
            idx,
            steam_folder_picker: FilePicker::<true>::new("Steam save path"),
            switch_folder_picker: FilePicker::<true>::new("Switch save path"),
            output_path_picker,
            current_file1: CurrentFile::Null,
            current_file2: CurrentFile::Null,
            language,
            game,
            steam: Steam::new(config),
            show_popup: false,
            show_alert: false,
            transfer_type: TransferType::SW2ST,
            popup_msg: "".to_string(),
            repair: false,
            brute_force: false,
            curve_index: None,
            brute_force_options: (0x0110000100000000, 0xffffffff),
            dump: false
        }
    }

    pub fn edit_save(&mut self){
        let save1 = match &mut self.current_file1 {
            CurrentFile::Loaded { loaded, .. } => loaded,
            _ => return,
        };
        let save2 = match &mut self.current_file2 {
            CurrentFile::Loaded { loaded, .. } => loaded,
            _ => return,
        };
        let fields1 = &mut save1.fields;
        let fields2 = &save2.fields;

        for field in fields1{
            let found2: Option<usize> = fields2.iter().position(|(id, _class)| *id == field.0);
            if let Some(i) = found2{
                *field = fields2[i].clone(); // .clone() is required since Class isn't Copy
                println!("{:08X} has been edited", field.0);
            }
            else {
                println!("{:08X} is Steam-only field", field.0);
            }

        }
    }
    pub fn move_save(&mut self, game_ctx: &mut GameCtx, _type: TransferType){
        let st_p= self.steam_folder_picker.text.clone();
        let sw_p= self.switch_folder_picker.text.clone();

        let from_path = if _type == TransferType::SW2ST {sw_p.clone()} else {st_p.clone()};
        let to_path = if _type == TransferType::SW2ST {st_p.clone()} else {sw_p.clone()};
        let Ok(paths) = std::fs::read_dir(from_path) else {
            return;
        };
        let Ok(output_paths) = std::fs::read_dir(self.output_path_picker.text.clone()) else {
            return;
        };
        for entry in output_paths.flatten() {
            if let Err(e) = std::fs::remove_file(&entry.path()){
                eprintln!("Failed to delete file: {}", e);
            }
        }
        for entry in paths.flatten() {
            let path = entry.path();
            let Some(file_name) = path.file_name() else {
                return;
            };
            let file_name = file_name.to_string_lossy().to_string();
            if file_name == "data00-1.bin"{
                let save_from = CurrentFile::Path(path.to_string_lossy().to_string());
                self.load(save_from, game_ctx, 2);
                let save_to_path = format!("{}/{}", to_path, file_name); 
                let save_to = CurrentFile::Path(save_to_path);
                self.load(save_to, game_ctx, 1);
                self.edit_save();
                self.save_as("".to_string());
            }
            else if self.is_data_slot(file_name.clone()){
                let save_from = CurrentFile::Path(path.to_string_lossy().to_string());
                self.load(save_from, game_ctx, 2);
                let save_to_path = format!("{}/{}", to_path, "data001Slot.bin"); 
                let save_to = CurrentFile::Path(save_to_path);
                self.load(save_to, game_ctx, 1);
                self.edit_save();
                self.save_as(file_name);
            }
            else if file_name.starts_with("SS"){
                let save_from = CurrentFile::Path(path.to_string_lossy().to_string());
                self.load(save_from, game_ctx, 1);

                let save = match &mut self.current_file1 {
                    CurrentFile::Loaded { loaded, .. } => loaded,
                    _ => return, // Exit early if not loaded
                };
                if _type == TransferType::SW2ST{
                    save.flags.set(SaveFlags::HAS_ID, true);
                }
                if _type == TransferType::ST2SW{
                    save.flags.set(SaveFlags::HAS_ID, false);
                }
                self.save_as("".to_string());
            }
        }
        self.set_popup(format!("Save transfer finished successfully"));
    }
    pub fn is_data_slot(&mut self, file_name: String) -> bool {
        if file_name.starts_with("data") && file_name.ends_with("Slot.bin"){
            return true;
        }
        return false;
    }
    pub fn save_as(&mut self, filename: String){
        let mut save_options = SaveOptions::new(self.game);
        if let Some(id) = self.steam.steam_id {
            save_options = save_options.id(id);
        }
        if let Some(curve_index) = self.curve_index {
            save_options = save_options.curve_index(curve_index);
        }

        match self
            .current_file1
            .write_save(&save_options)
        {
            Ok((file_path, data)) => {
                // quite disgusting but oh well
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let output_path = self.output_path_picker.text.clone();
                    use std::fs::File;
                    let mut path = PathBuf::from(output_path.clone());
                    let _ = std::fs::create_dir_all(&path);
                    let in_file_path = PathBuf::from(&file_path);
                    let mut file_name = in_file_path
                        .file_name()
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or("data.bin".to_string());
                    if filename != "" {
                        file_name = filename;
                    } 
                    path.push(file_name);
                    match File::create(&path) {
                        Ok(mut file) => {
                            // idk wtf im doing here uh
                            use std::io::Write;
                            let _ = file.write_all(&data);
                            let path = path.clone().to_string_lossy().to_string();
                            let path = std::fs::canonicalize(path.to_string())
                                .unwrap_or(PathBuf::from(path.to_string()));
                            log::info!("Saved to {}", path.to_string_lossy().to_string());
                        }
                        Err(e) => {
                            self.set_popup(format!(
                                "Could not create file {:?}: error: {e}",
                                path
                            ));
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
                self.set_popup(format!("Error saving file {}", e));
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
    pub fn show_alert(&mut self, ui: &mut Ui, game_ctx: &mut GameCtx) {
        if self.show_alert {
            let popup_id = ui.make_persistent_id((self.idx, "alert"));
            eframe::egui::Area::new(popup_id)
                .order(Order::Foreground)
                .anchor(Align2::CENTER_CENTER, eframe::egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(320.0);

                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label("All files in the output folder will be deleted. Is that okay?");
                            ui.add_space(15.0);

                            ui.horizontal(|ui| {
                                let total_width = ui.available_width();
                                let buttons_width = 120.0;
                                ui.add_space((total_width - buttons_width) / 2.0);

                                if ui.button("Ok").clicked() {
                                    self.show_alert = false;
                                    self.move_save(game_ctx, self.transfer_type.clone());
                                }
                                if ui.button("Cancel").clicked() {
                                    self.show_alert = false;
                                }
                            });
                            ui.add_space(5.0);
                        });
                    });
            });
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, game_contexts: &mut HashMap<Game, GameCtx>) {
        self.show_popup(ui);
        let mut game_ctx = game_contexts.get_mut(&self.game);
        if let Some(game_ctx) = game_ctx.as_deref_mut() {
            self.show_alert(ui, game_ctx);
        }
        self.output_path_picker.update();
        self.steam_folder_picker.update();
        self.switch_folder_picker.update();
  

        #[cfg(not(target_arch = "wasm32"))]
        ui.horizontal(|ui| {
            self.output_path_picker.ui(ui);
        });

        ui.horizontal(|ui| {
            if let Some(_) = self.steam.found_file(ui, self.game) {
                if let Some(game_ctx) = game_ctx.as_deref_mut() {
                    self.reload(game_ctx);
                }
            };
            #[cfg(not(target_arch = "wasm32"))]
            self.steam.edit_steam_path(ui);
        });

        ui.horizontal(|ui| {
            self.steam_folder_picker.ui(ui);
        });
        ui.horizontal(|ui| {
            self.switch_folder_picker.ui(ui);
        });
        ui.horizontal(|ui| {
            if ui.add_sized(egui::vec2(150.0, 150.0), egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("Switch2Steam.png")).fit_to_exact_size(egui::vec2(150.0, 50.0)), 
                ""
            )).clicked() {
                self.transfer_type = TransferType::SW2ST;
                self.show_alert = true;
            }
            if ui.add_sized(egui::vec2(150.0, 150.0), egui::Button::image_and_text(
                egui::Image::new(egui::include_image!("Steam2Switch.png")).fit_to_exact_size(egui::vec2(150.0, 50.0)), 
                ""
            )).clicked() {
                self.transfer_type = TransferType::ST2SW;
                self.show_alert = true;
            }
        });
        
        
    }

    fn set_popup(&mut self, msg: String) {
        log::info!("popup msg set: {}", msg);
        self.popup_msg = msg;
        self.show_popup = true;
    }

    fn read_save(&mut self, data: Vec<u8>, game_ctx: &mut GameCtx) -> Option<SaveFile> {
        let mut save_options = SaveOptions::new(self.game);
        if self.brute_force {
            save_options = save_options.brute_force(self.brute_force_options.0, self.brute_force_options.1);
        }
        if let Some(id) = self.steam.steam_id {
            save_options = save_options.id(id);
        }
        if let Some(curve_index) = self.curve_index {
            save_options = save_options.curve_index(curve_index);
        }
        if self.dump {
            save_options = save_options.debug_dump();
        }

        if self.repair {
            let data: Vec<u8> = SaveFile::process_bytes_to_stream(data, &mut save_options).ok()?.0;
            let mut reader = Cursor::new(data);
            let mut corrupted_reader = CorruptSaveReader::new(&game_ctx.type_map, self.game);
            //let save_file = corrupted_reader.read_n_objects(&mut reader, "app.savedata.cUserSaveParam", 3);
            let save_file = corrupted_reader.read_missing(
                &mut reader,
                &[
                    (0xdbe3f199, "app.savedata.cUserSaveData"),
                    (0x85e904c1, "via.storage.saveService.SaveFileDetail"),
                ],
            );
            if self.brute_force {
                self.steam.steam_id = save_options.id;
                if let Some(steam_id) = save_options.id{
                    self.steam.steam_id_text = steam_id.to_string();
                }
            }
            return Some(save_file);
        }
        match SaveFile::read_save(data, &mut save_options) {
            Ok(save) => {
                self.update_from_save_options(&save_options);
                return Some(save);
            }
            Err(e) => {
                self.set_popup(format!("Error reading save: {e}"));
            }
        }
        self.update_from_save_options(&save_options);

        None
    }

    fn update_from_save_options(&mut self, opts: &SaveOptions) {
        if self.brute_force {
            self.steam.steam_id = opts.id;
            if let Some(steam_id) = opts.id {
                self.steam.steam_id_text = steam_id.to_string();
            }
        }
        self.curve_index = opts.curve_index;
    }

    pub fn load(&mut self, current_file: CurrentFile, game_ctx: &mut GameCtx, idx : u8) {
        match current_file {
            CurrentFile::Path(path) => {
                let expanded =
                    shellexpand::full(&path).unwrap_or(std::borrow::Cow::Borrowed(&path));
                let path = PathBuf::from(expanded.as_ref());
                if path.exists() {
                    match std::fs::read(&path) {
                        Ok(data) => {
                            let save = self.read_save(data, game_ctx);
                            if let Some(save) = save {
                                if idx == 1 {
                                    self.current_file1 = CurrentFile::Loaded {
                                        file_name: path.display().to_string(),
                                        loaded: save,
                                    };
                                }
                                else if idx == 2{
                                    self.current_file2 = CurrentFile::Loaded {
                                        file_name: path.display().to_string(),
                                        loaded: save,
                                    };
                                }
                            }

                        }
                        Err(e) => {
                            self.set_popup(format!("Failed to open file: {e}"));
                        }
                    }
                }
                else {
                    println!("Path doesn't exist! : {}", path.to_string_lossy().to_string());
                }
            }
            CurrentFile::FileData { file_name, bytes } => {
                // clone is fine here, i think give the reader the whole buffer is just better
                let save = self.read_save(bytes.clone(), game_ctx);
                if let Some(save) = save {
                    self.current_file1 = CurrentFile::LoadedWeb {
                        file_name,
                        original_bytes: bytes,
                        loaded: save,
                    };
                } else {
                    self.current_file1 = CurrentFile::FileData {
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
        let current_file1 = std::mem::replace(&mut self.current_file1, CurrentFile::Null);
        match current_file1 {
            CurrentFile::Path(path) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let expanded = shellexpand::full(&path)
                        .unwrap_or(std::borrow::Cow::Borrowed(&path));
                    let path = PathBuf::from(expanded.as_ref());
                    if path.exists() {
                        match std::fs::read(&path) {
                            Ok(data) => {
                                let save = self.read_save(data, game_ctx);
                                if let Some(save) = save {
                                    self.current_file1 = CurrentFile::Loaded {
                                        file_name: path.display().to_string(),
                                        loaded: save,
                                    };
                                    return true;
                                }
                            }
                            Err(e) => {
                                self.set_popup(format!("Failed to open file: {e}"));
                                return true;
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                log::error!(
                    "Cannot load as path {path} on web, there should be no way to get here"
                );
            }
            CurrentFile::FileData { file_name, bytes } => {
                let save = self.read_save(bytes.clone(), game_ctx);
                #[cfg(target_arch = "wasm32")]
                if let Some(save) = save {
                    self.current_file1 = CurrentFile::LoadedWeb {
                        file_name,
                        original_bytes: bytes,
                        loaded: save,
                    };
                    return true;
                } else {
                    self.current_file1 = CurrentFile::FileData {
                        file_name,
                        bytes: bytes,
                    };
                }

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(save) = save {
                    self.current_file1 = CurrentFile::Loaded {
                        file_name,
                        loaded: save,
                    };
                    return true;
                } else {
                    self.current_file1 = CurrentFile::FileData {
                        file_name,
                        bytes: bytes,
                    };
                }
            }
            CurrentFile::LoadedWeb {
                file_name,
                original_bytes,
                ..
            } => {
                let save = self.read_save(original_bytes.clone(), game_ctx);
                if let Some(save) = save {
                    self.current_file1 = CurrentFile::LoadedWeb {
                        file_name: file_name.to_string(),
                        original_bytes,
                        loaded: save,
                    };
                    return true;
                }
            }
            CurrentFile::Loaded { .. } | CurrentFile::Null => {

            }
        }
        return false;
    }
}
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TransferType {
    SW2ST, //Switch to Steam
    ST2SW // Steam to Switch
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
            let users = steam::parse_accounts(&path).unwrap_or_default();
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
                        if let Ok(Some(saved_str)) = storage.get_item("ree_save_editor_steam_id") {
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
            selected_user_name: None,
        }
    }

    pub fn edit_steam_id(&mut self, ui: &mut Ui) {
        ui.label("Steam ID:");
        if ui
            .add(TextEdit::singleline(&mut self.steam_id_text))
            .changed()
        {
            if let Ok(val) = u64::from_str_radix(&self.steam_id_text, 10) {
                self.steam_id = Some(val);
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.selected_user_name = None;
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        if let Ok(Some(storage)) = window.local_storage() {
                            // We save the raw string "7656..."
                            let _ = storage.set_item("ree_save_editor_steam_id", &self.steam_id_text);
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn edit_steam_path(&mut self, ui: &mut Ui) -> bool {
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
    FileData {
        file_name: String,
        bytes: Vec<u8>,
    },
    Loaded {
        file_name: String,
        loaded: SaveFile,
    },
    LoadedWeb {
        file_name: String,
        original_bytes: Vec<u8>,
        loaded: SaveFile,
    },
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
    pending_result: Option<FilePickResult>,
}

impl CurrentFile {
    pub fn get_file_name(&self) -> Option<&str> {
        match self {
            CurrentFile::Null | CurrentFile::FileData { .. } => None,
            CurrentFile::Path(path) => Some(path.as_str()),
            CurrentFile::Loaded { file_name, .. } | CurrentFile::LoadedWeb { file_name, .. } => {
                Some(file_name.as_str())
            }
        }
    }

    pub fn write_save(
        &self,
        options: &SaveOptions
    ) -> Result<(String, Vec<u8>), Box<dyn Error>> {
        match self {
            CurrentFile::Null | CurrentFile::Path(_) | CurrentFile::FileData { .. } => {
                Err("Can't save unloaded file".into())
            }
            CurrentFile::Loaded { file_name, loaded }
            | CurrentFile::LoadedWeb {
                file_name, loaded, ..
            } => Ok((file_name.clone(), loaded.write_save(options)?)),
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
                FilePickResult::Wasm { name, .. } => self.text = name.clone(),
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
                FilePickResult::Wasm { name, .. } => name.clone(),
            });
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
            let dialog = rfd::FileDialog::new().set_directory(cur).set_title(&title);
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
