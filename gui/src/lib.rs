use std::collections::HashSet;
use std::error::Error;
use std::io::{Cursor, Read, Seek};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use mhtame::edit::{EditContext, Editable};
use mhtame::file::StructRW;

use eframe::egui::{self, ScrollArea, TextEdit, Ui, Vec2};

use mhtame::{
    edit::{CopyBuffer},
    save::{SaveContext, SaveFile},
};
use sdk::type_map::TypeMap;

const RSZ_JSON: &str = include_str!("../../assets/rszmhwilds_packed.json");
const ENUMS_JSON: &str = include_str!("../../assets/enumsmhwilds.json");

// We need a common config struct that works for both CLI (Clap) and Web
#[derive(Debug, Clone)]
pub struct Config {
    pub file_name: Option<String>,
    pub out_dir: String,
    pub steamid: Option<String>,
    pub rsz_path: String,
    pub enums_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_name: None,
            out_dir: "outputs".to_string(),
            steamid: None,
            rsz_path: "assets/rszmhwilds_packed.json".to_string(),
            enums_path: "assets/enumsmhwilds.json".to_string(),
        }
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

impl CurrentFile {
    pub fn write_save_data(&self, key: u64) -> Result<(String, Vec<u8>), Box<dyn Error>> {
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

impl<const FOLDER: bool> FilePicker<FOLDER> {
    pub fn new(label: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        
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

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label(format!("{}:", &self.label));
        ui.add(TextEdit::singleline(&mut self.text));
        if ui.button("Browse").clicked() {
            self.spawn_dialog();
        }
    }

    fn spawn_dialog(&self) {
        let tx = self.tx.clone();
        let title = self.label.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new().set_title(&title);
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

pub struct TameApp {
    steam_id: Option<u64>,
    steam_id_text: String,
    input_file: String,
    current_file: CurrentFile,

    show_popup: bool,
    popup_msg: String,
    copy_buffer: CopyBuffer,
    type_map: TypeMap,
    search_buffer: String,
    search_fields: HashSet<(u32, u32)>,
    search_range: core::ops::Range<usize>,
    max_search_depth: u32,
    search_time: Option<Duration>,

    // THIS IS SO DIsGUSTING AHHHHHHHHHHHH
    // I need to just put this in something like struct FilePicker
    // that has some #[cfg] shit
    input_file_picker: FilePicker<false>,
    #[cfg(not(target_arch = "wasm32"))]
    output_picker: FilePicker<true>,
}

impl TameApp{
    pub fn new(config: Config, _cc: &eframe::CreationContext<'_>) -> Self {
        //#[cfg(not(target_arch = "wasm32"))]
        /*let type_map = TypeMap::load_from_file(&config.rsz_path, &config.enums_path)
            .unwrap_or_else(|_| {
                let type_map = TypeMap::parse_str(RSZ_JSON, ENUMS_JSON).expect("Could not load type map");
                type_map
            });*/
        #[cfg(not(target_arch = "wasm32"))]
        /*let type_map = TypeMap::parse_bincode(include_bytes!("../../assets/types.bin"))
            .unwrap_or_else(|_| {
                let type_map = TypeMap::parse_str(RSZ_JSON, ENUMS_JSON).expect("Could not load type map");
                type_map
            });*/
        let type_map = TypeMap::parse_str(RSZ_JSON, ENUMS_JSON).expect("Could not load type map");

        #[cfg(target_arch = "wasm32")]
        let type_map = TypeMap::parse_str(RSZ_JSON, ENUMS_JSON).expect("Could not load type map");



        let mut steam_id_u64 = config.steamid.as_ref().and_then(|s| {
            if let Some(hex) = s.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                u64::from_str_radix(s, 10).ok()
            }
        });

        #[cfg(target_arch = "wasm32")]
        if steam_id_u64.is_none() {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(saved_str)) = storage.get_item("mhtame_steam_id") {
                        // Parse the saved string
                        if let Ok(val) = u64::from_str_radix(&saved_str, 10) {
                            steam_id_u64 = Some(val);
                        }
                    }
                }
            }
        }
        let steam_id_text = steam_id_u64.map(|x| x.to_string()).unwrap_or("".to_string());

        let mut input_file_picker = FilePicker::<false>::new("File");
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(f) = config.file_name {
                use std::path::Path;

                let p = Path::new(&f);
                if std::fs::exists(p).is_ok() {
                    input_file_picker.pending_result = Some(FilePickResult::Native(f.clone()));
                    input_file_picker.text = f.clone();
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let mut output_picker = FilePicker::<true>::new("Output Path");
        #[cfg(not(target_arch = "wasm32"))]
        {
            output_picker.pending_result = Some(FilePickResult::Native("./outputs/saves/".to_string()));
            output_picker.text = "./outputs/saves/".to_string();
        }

        Self {
            steam_id: steam_id_u64,
            steam_id_text,
            current_file: CurrentFile::Null,
            input_file: String::from(""),
            input_file_picker,

            #[cfg(not(target_arch = "wasm32"))]
            output_picker,

            search_buffer: String::default(),
            search_fields: HashSet::new(),
            search_range: 0..usize::MAX,
            max_search_depth: 100,
            search_time: None,

            show_popup: false,
            popup_msg: String::new(),
            copy_buffer: CopyBuffer::Null,
            type_map,
        }
    }

    fn read_save<R: Read + Seek>(&mut self, reader: &mut R) -> Option<SaveFile> {
        if let Some(steamid) = self.steam_id {
            let mut ctx = SaveContext { key: steamid };
            match SaveFile::read(reader, &mut ctx) {
                Ok(save) => {
                    return Some(save)
                },
                Err(e) => {
                    self.popup_msg = format!("Error reading save: {e:?}");
                    self.show_popup = true;
                }
            }
        } else {
            self.popup_msg = format!("Cannot load save without steamid");
            self.show_popup = true;
        }
        None
    }

    pub fn load(&mut self, current_file: CurrentFile) {
        match current_file {
            CurrentFile::Path(path) => {
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
                            }
                        }
                        Err(e) => {
                            self.popup_msg = format!("Failed to open file: {e}");
                            self.show_popup = true;
                        }
                    }
                }
            },
            CurrentFile::FileData { file_name, bytes } => {
                let mut reader = Cursor::new(&bytes);
                let save = self.read_save(&mut reader);
                if let Some(save) = save {
                    self.current_file = CurrentFile::LoadedWeb { 
                        file_name,
                        original_bytes: bytes,
                        loaded: save
                    };
                }
            },
            _ => {
                self.popup_msg = format!("Need a file to load");
                self.show_popup = true;
            }
        }
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
                self.current_file = CurrentFile::Path(self.input_file.clone());
                if self.reload() == false {
                    self.popup_msg = format!("Could not open file {:?}", self.input_file);
                    self.show_popup = true;
                    return false;
                };
            }
        }
        return false;
    }

    fn add_file_area(&mut self, ui: &mut Ui) {
        ScrollArea::both().auto_shrink(false).max_width(f32::INFINITY).show(ui, |ui| {
            match &mut self.current_file {
                CurrentFile::LoadedWeb { loaded, .. } | CurrentFile::Loaded { loaded, .. } => {
                    let mut edit_ctx = EditContext::new(&self.type_map, &self.search_fields, &self.search_range, &mut self.copy_buffer);
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

    fn update_input_file(&mut self) {
        let res = self.input_file_picker.pending_result.take();
        if let Some(res) = res {
            match res {
                FilePickResult::Native(p) => {
                    self.input_file = p.clone();
                    let file = CurrentFile::Path(p);
                    self.load(file);
                }
                FilePickResult::Wasm{name, data} => {
                    self.input_file = name.clone();
                    let file = CurrentFile::FileData{file_name: name, bytes: data};
                    self.load(file);
                }
            }
        }
    }
}


// Thanks Gemini :D
fn parse_query(input: &str) -> (String, core::ops::Range<usize>) {
    let Some(start_bracket) = input.find('[') else {
        return (input.to_string(), 0..usize::MAX);
    };
    
    let Some(end_bracket) = input.find(']') else {
         return (input.to_string(), 0..usize::MAX);
    };

    let name = input[0..start_bracket].to_string();
    let content = &input[start_bracket+1..end_bracket];

    if let Some(colon_pos) = content.find(':') {
        let start_str = &content[0..colon_pos];
        let end_str = &content[colon_pos+1..];

        let start = start_str.parse::<usize>().unwrap_or(0);
        let end = end_str.parse::<usize>().unwrap_or(usize::MAX);
        
        (name, start..end)
    } else {
        if let Ok(idx) = content.parse::<usize>() {
            (name, idx..idx+1)
        } else {
            (name, 0..usize::MAX)
        }
    }
}

pub fn save_file_dialog(default_name: &str, data: Vec<u8>) {
    let name = default_name.to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            // 1. Open System Dialog
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(&name)
                .save_file() 
            {
                // 2. Write to Disk
                if let Err(e) = std::fs::write(&path, &data) {
                    eprintln!("Failed to save file: {}", e);
                } else {
                    println!("File saved to {:?}", path);
                }
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        
        // 1. Create a Blob (File in memory)
        let array = js_sys::Array::new();
        let uint8_array = unsafe { js_sys::Uint8Array::view(&data) };
        array.push(&uint8_array);
        
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
            &array, 
            web_sys::BlobPropertyBag::new().type_("application/octet-stream")
        ).unwrap();

        // 2. Create a URL for that Blob
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
        
        // 3. Create a hidden <a> tag and click it
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a = document.create_element("a").unwrap().dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
        
        a.set_href(&url);
        a.set_download(&name);
        a.click();
        
        // 4. Clean up
        web_sys::Url::revoke_object_url(&url).ok();
    }
}

impl eframe::App for TameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // File Dropping
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped.first() {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &file.path {
                    self.input_file = path.display().to_string();
                    self.input_file_picker.text = self.input_file.clone();
                    let file = CurrentFile::Path(self.input_file.clone());
                    self.load(file);
                }

                #[cfg(target_arch = "wasm32")]
                if let Some(bytes) = &file.bytes {
                    self.input_file = file.name.clone();
                    self.input_file_picker.text = self.input_file.clone();
                    let file = CurrentFile::FileData {
                        file_name: self.input_file.clone(),
                        bytes: bytes.to_vec()
                    };
                    self.load(file);
                }
            }
        }

        self.update_input_file();
        // Main UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.item_spacing *= 1.5;
            ui.style_mut().spacing.button_padding *= 1.5;
            for (_style, font_id) in ui.style_mut().text_styles.iter_mut() {
                font_id.size *= 1.2;
            }

            ui.heading("MH Wilds Save Editor");


            ui.horizontal(|ui| {

                self.input_file_picker.ui(ui);
                self.input_file_picker.update();


                // This is so unbelievably gross and yucky and disgusting
                if ui.button("Save").clicked() {
                    if let Some(steamid) = self.steam_id {
                        match self.current_file.write_save_data(steamid) {
                            Ok((s, b)) => {
                                #[cfg(not(target_arch = "wasm32"))]
                                if let Some(path) = &self.output_picker.pending_result {
                                    if let FilePickResult::Native(path) = &path {
                                        use std::fs::File;
                                        let mut path = PathBuf::from(path);
                                        path.push(s);
                                        log::info!("Saving to {path:?}");
                                        if let Ok(mut file) = File::create(&path) {
                                            use std::io::Write;
                                            let _ = file.write_all(&b);
                                            self.popup_msg = format!("Saved to {:?}", path)
                                        } else {
                                            self.popup_msg = format!("Could not create file {:?}", path)
                                        }
                                    }
                                } else {
                                    //save_file_dialog(&s, b);
                                    self.popup_msg = format!("Please choose an output path");
                                }

                                #[cfg(target_arch = "wasm32")]
                                {
                                    save_file_dialog(&s, b);
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
                        self.input_file_picker.pending_result = Some(FilePickResult::Native(self.input_file_picker.text.clone()));
                    }

                    self.reload();
                }


            });

            ui.horizontal(|ui| {
                ui.label("Steam ID:");
                if ui.add(TextEdit::singleline(&mut self.steam_id_text)).changed() {
                    if let Ok(val) = u64::from_str_radix(&self.steam_id_text, 10) {
                        self.steam_id = Some(val);

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
                        ui.label("Invalid ID");
                    }
                }
            });

            #[cfg(not(target_arch = "wasm32"))]
            ui.horizontal(|ui| {
                self.output_picker.ui(ui);
                self.output_picker.update();
            });

            ui.horizontal(|ui| {
                ui.label("Search: ");
                let resp = ui.add(TextEdit::singleline(&mut self.search_buffer));
                if resp.changed() {
                    let (search_buffer, range) = parse_query(&self.search_buffer);
                    self.search_range = range;
                    self.search_fields.clear();
                    if !search_buffer.is_empty() {
                        let start = web_time::Instant::now();
                        match &self.current_file {
                            CurrentFile::Loaded { loaded, .. } | CurrentFile::LoadedWeb { loaded, .. }  => {
                                for field in &loaded.fields {
                                    if let Some(type_info) = self.type_map.get_by_hash(field.1.hash) {
                                        //let (_ts, fs) = self.type_map.searchv2(type_info, &search_buffer, self.max_search_depth as usize);
                                        let fs = self.type_map.search(type_info, &search_buffer, self.max_search_depth as usize);
                                        self.search_fields.extend(fs);
                                    }
                                }
                            }
                            _ => {();}
                        }
                        self.search_time = Some(start.elapsed());
                    } else {
                        self.search_time = None;
                    }
                }
                if let Some(search_time) = self.search_time {
                    ui.label(format!("{}s", search_time.as_secs_f64()));
                }
            });

            ui.separator();
            self.add_file_area(ui);

            if self.show_popup {
                let popup_id = ui.make_persistent_id("msg_popup");
                egui::Area::new(popup_id)
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                    .show(ctx, |ui| {
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
        });
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast; // <--- Import this for dyn_into()

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    let window = web_sys::window().expect("No window found");
    let document = window.document().expect("No document found");

    // 2. Remove the Loading Spinner
    // We do this immediately before starting the WebRunner
    if let Some(loader) = document.get_element_by_id("loading_text") {
        loader.remove();
    }

    let web_options = eframe::WebOptions::default();
    let config = Config::default();

    // --- NEW: Manually look up the canvas element ---
    let document = web_sys::window()
        .expect("No window found")
        .document()
        .expect("No document found");

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find canvas with that ID")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Element is not a canvas")?;
    // ------------------------------------------------

    eframe::WebRunner::new()
        .start(
            canvas, // <--- Pass the element, NOT the string
            web_options,
            Box::new(|cc| Ok(Box::new(TameApp::new(config, cc)))),
        )
        .await
}
