pub mod app;
pub mod game_context;
pub mod file;
pub mod steam;
pub mod tab;
pub mod viewer;

#[cfg(not(target_arch = "wasm32"))]
pub mod code_editor;

use mhtame::edit::{EditContext, Editable};
use mhtame::file::StructRW;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{Cursor, Read, Seek};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use eframe::egui::{self, ComboBox, ScrollArea, TextEdit, Ui, Vec2};

use crate::steam::*;
use mhtame::save::game::{GAME_OPTIONS, Game};
use mhtame::save::remap::Remap;
use mhtame::sdk::type_map::{ContentLanguage, TypeMap};
use mhtame::{
    edit::copy::CopyBuffer,
    save::{SaveContext, SaveFile},
};

// We need a common config struct that works for both CLI (Clap) and Web
#[derive(Debug, Clone)]
pub struct Config {
    pub file_name: Option<String>,
    pub out_dir: String,
    pub steamid: Option<String>,
    pub rsz_path: Option<String>,
    pub enums_path: Option<String>,
    pub msgs_path: Option<String>,
    pub mappings_path: Option<String>,
    pub remap_path: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub steam_path: String,
}

impl Config {
    #[cfg(not(target_arch = "wasm32"))]
    fn edit_asset_paths(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("rsz_path:");
            //ui.text_edit_singleline(&mut self.rsz_path);
        });
        ui.horizontal(|ui| {
            ui.label("enums_path:");
            //ui.text_edit_singleline(&mut self.enums_path);
        });
        ui.horizontal(|ui| {
            ui.label("msgs_path:");
            //ui.text_edit_singleline(&mut self.msgs_path);
        });
        ui.horizontal(|ui| {
            ui.label("mappings_path:");
            //ui.text_edit_singleline(&mut self.mappings_path);
        });
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_name: None,
            out_dir: "outputs".to_string(),
            steamid: None,
            rsz_path: None,
            enums_path: None,
            msgs_path: None,
            mappings_path: None,
            remap_path: None,
            #[cfg(target_os = "windows")]
            steam_path: "C:\\Program Files (x86)\\Steam".to_string(),
            #[cfg(target_os = "linux")]
            steam_path: shellexpand::full("~/.local/share/Steam")
                .unwrap_or_default()
                .to_string(),
        }
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

impl CurrentFile {
    pub fn write_save_to_buf(&self, key: u64) -> Result<(String, Vec<u8>), Box<dyn Error>> {
        match self {
            CurrentFile::Null | CurrentFile::Path(_) | CurrentFile::FileData { .. } => {
                Err("Can't save unloaded file".into())
            }
            CurrentFile::Loaded { file_name, loaded }
            | CurrentFile::LoadedWeb {
                file_name, loaded, ..
            } => Ok((file_name.clone(), loaded.to_bytes(key)?)),
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
    pending_result: Option<FilePickResult>,
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
                FilePickResult::Wasm { name, .. } => self.text = name.clone(),
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

const LANGUAGE_OPTIONS: [(&'static str, ContentLanguage); 34] = [
    ("Japanese", ContentLanguage::Japanese),
    ("English", ContentLanguage::English),
    ("French", ContentLanguage::French),
    ("Italian", ContentLanguage::Italian),
    ("German", ContentLanguage::German),
    ("Spanish", ContentLanguage::Spanish),
    ("Russian", ContentLanguage::Russian),
    ("Polish", ContentLanguage::Polish),
    ("Dutch", ContentLanguage::Dutch),
    ("Portuguese", ContentLanguage::Portuguese),
    ("Portuguese (Brazil)", ContentLanguage::PortugueseBr),
    ("Korean", ContentLanguage::Korean),
    ("Traditional Chinese", ContentLanguage::TransitionalChinese),
    ("Simplified Chinese", ContentLanguage::SimplelifiedChinese),
    ("Finnish", ContentLanguage::Finnish),
    ("Swedish", ContentLanguage::Swedish),
    ("Danish", ContentLanguage::Danish),
    ("Norwegian", ContentLanguage::Norwegian),
    ("Czech", ContentLanguage::Czech),
    ("Hungarian", ContentLanguage::Hungarian),
    ("Slovak", ContentLanguage::Slovak),
    ("Arabic", ContentLanguage::Arabic),
    ("Turkish", ContentLanguage::Turkish),
    ("Bulgarian", ContentLanguage::Bulgarian),
    ("Greek", ContentLanguage::Greek),
    ("Romanian", ContentLanguage::Romanian),
    ("Thai", ContentLanguage::Thai),
    ("Ukrainian", ContentLanguage::Ukrainian),
    ("Vietnamese", ContentLanguage::Vietnamese),
    ("Indonesian", ContentLanguage::Indonesian),
    ("Fiction", ContentLanguage::Fiction),
    ("Hindi", ContentLanguage::Hindi),
    (
        "Spanish (Latin America)",
        ContentLanguage::LatinAmericanSpanish,
    ),
    ("Unknown", ContentLanguage::Unknown),
];

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/NotoSansCJK-Regular.ttc")).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    ctx.set_fonts(fonts);
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

pub fn save_file_dialog(default_name: &str, data: Vec<u8>) {
    let name = default_name.to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            // 1. Open System Dialog
            if let Some(path) = rfd::FileDialog::new().set_file_name(&name).save_file() {
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
            web_sys::BlobPropertyBag::new().type_("application/octet-stream"),
        )
        .unwrap();

        // 2. Create a URL for that Blob
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

        // 3. Create a hidden <a> tag and click it
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a = document
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();

        a.set_href(&url);
        a.set_download(&name);
        a.click();

        // 4. Clean up
        web_sys::Url::revoke_object_url(&url).ok();
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use crate::app::TameApp;

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    let window = web_sys::window().expect("No window found");
    let document = window.document().expect("No document found");

    if let Some(loader) = document.get_element_by_id("loading_text") {
        loader.remove();
    }

    let web_options = eframe::WebOptions::default();
    let config = Config::default();

    let document = web_sys::window()
        .expect("No window found")
        .document()
        .expect("No document found");

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find canvas with that ID")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "Element is not a canvas")?;

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(TameApp::new(config)))),
        )
        .await
}
