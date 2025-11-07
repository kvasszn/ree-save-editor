pub mod editor;
use core::f32;
use std::{fs::File, path::PathBuf};

use eframe::egui::{self, Color32, FontDefinitions, FontFamily, FontSelection, Frame, ScrollArea, TextEdit, TextStyle};
use egui_json_tree::{render::{DefaultRender, RenderContext}, *};
use mhtame::{file::{FileReader, StructRW}, rsz::dump::{ENUM_FILE, RSZ_FILE}, save::{types::to_dersz, SaveContext, SaveFile}, user::User};
use serde_json::json;

use crate::editor::Editor;
pub fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
        .. Default::default()
    };
    ENUM_FILE.set("enums.json".to_string()).unwrap();
    RSZ_FILE.set("rszmhwilds_unpacked_structs.json".to_string()).unwrap();
    eframe::run_native("mhtame",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::<TameApp>::default())
        }),
    )
}

pub struct TameApp {
    file_name: String,
    current_file_name: Option<String>,
    steam_id: String,
    file_reader: FileReader,
    json_value: Option<serde_json::Value>,
    user_value: Option<User>

}

impl Default for TameApp {
    fn default() -> Self {
        let file_reader = FileReader::new("outputs".into(), None, false, false, true, None);
        Self {
            current_file_name: None,
            file_name: "".to_string(),
            file_reader,
            steam_id: "".to_string(),
            json_value: None,
            user_value: None,
        }
    }

}

fn setup_custom_fonts(ctx: &egui::Context) {
    //let mut fonts = egui::FontDefinitions::default();
    
    // Load your custom font
    /*fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("path/to/your/font.ttf")),
    );
    
    // Set it as the proportional font (used for most text)
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());*/
    
    // Optionally set it as the monospace font too
    /*fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "my_font".to_owned());*/
    
    // Apply the fonts to the context
    //ctx.set_fonts(fonts);
}


#[derive(Clone)]
struct MyPayload { pub path: String }

/*impl eframe::App for TameApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("mhtame");
            let f = Frame::default();

            let file_select = ui.add(TextEdit::singleline(&mut self.file_name));
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if !dropped_files.is_empty() {
                let file = &dropped_files[0];
                if let Some(path) = &file.path {
                    self.current_file_name = Some(path.display().to_string());
                } else {
                    self.current_file_name = Some(file.name.clone());
                }
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("File Drop Example");

                ui.label("Drag and drop a file anywhere on the window!");

                if let Some(file) = &self.current_file_name {
                    ui.separator();
                    ui.label(format!("Last dropped file: {file}"));
                }
            });

        });
    }
}*/

impl eframe::App for TameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {


        // this does not work on wayland :(
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let df = &dropped_files[0];
            if let Some(path) = &df.path {
                self.current_file_name = Some(path.display().to_string());
                println!("Dropped file path: {}", path.display());
            } else {
                self.current_file_name = Some(df.name.clone());
                println!("Dropped file name: {}", df.name);
            }
        }

        if let Some(file) = &self.current_file_name {
            self.file_name = file.clone()
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("mhtame");
            ui.horizontal(|ui| {
                ui.label("path:");
                ui.add(TextEdit::singleline(&mut self.file_name).desired_width(400.0));
                if ui.button("Select File").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.current_file_name = Some(path.display().to_string());
                        let json_value = if let Some(file) = &self.current_file_name {
                            /*let steamid = if let Some(hex) = self.steam_id.strip_prefix("0x") {
                                u64::from_str_radix(hex, 16)
                            } else {
                                u64::from_str_radix(&self.steam_id, 10)
                            }.unwrap();
                            //Mandarin::sanity_check(&file_path);
                            let mut reader = File::open(file).unwrap();
                            let save = SaveFile::read(&mut reader, &mut SaveContext{key: steamid}).unwrap();
                            //let save = SaveFile::from_file(&file)?;
                            let dersz = to_dersz(save.data).unwrap();
                            //println!("{:?}, {:?}", dersz.structs.len(), dersz.roots);
                            let json = json!(&dersz);
                            Some(json)*/
                            let reader = File::open(file).unwrap();
                            let user = User::new(reader).unwrap();
                            //let json = json!(user);
                            Some(user)
                        } else {
                            None
                        };
                        //self.json_value = json_value;
                        self.user_value = json_value;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Steam ID:");
                ui.add(TextEdit::singleline(&mut self.steam_id).desired_width(100.0))
            });

            ui.separator();
            ScrollArea::both().auto_shrink(false).max_width(f32::INFINITY).show(ui, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::monospace(14.0));
                if let Some(value) = &mut self.user_value {
                    let mut dersz = value.rsz.deserialize_to_dersz().unwrap();
                    Editor::show(ui, &mut dersz);
                }

                if let Some(value) = &mut self.json_value {
                    JsonTree::new("file", value).show(ui);
                }
            })
        });
    }
}


