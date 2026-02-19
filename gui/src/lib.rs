pub mod app;
pub mod file;
pub mod steam;
pub mod tab;
pub mod viewer;

#[cfg(not(target_arch = "wasm32"))]
pub mod code_editor;

use eframe::egui::{self};

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

pub fn save_file_dialog(default_name: &str, data: Vec<u8>) {
    let name = default_name.to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new().set_file_name(&name).save_file() {
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

        let array = js_sys::Array::new();
        let uint8_array = unsafe { js_sys::Uint8Array::view(&data) };
        array.push(&uint8_array);

        let mut options = web_sys::BlobPropertyBag::new();

        options.set_type("application/octet-stream");

        let blob = web_sys::Blob::new_with_blob_sequence_and_options(
            &array,
            &options, 
        ).unwrap();

        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

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
            Box::new(|cc| {
                configure_fonts(&cc.egui_ctx);
                Ok(Box::new(TameApp::new(config)))
            }),
        )
        .await
}

pub fn configure_fonts(ctx: &egui::Context) {
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
