pub mod editor;
use core::f32;
use std::{fs::File, path::PathBuf, sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}};

use eframe::egui::{self, Color32, FontDefinitions, FontFamily, FontSelection, Frame, ScrollArea, TextEdit, TextStyle};
use egui_json_tree::{render::{DefaultRender, RenderContext}, *};
use mhtame::{edit::{Edit, RszEditCtx}, file::{FileReader, StructRW}, rsz::{dump::{RszDump, ENUM_FILE, RSZ_FILE}, rszserde::DeRsz}, save::{types::to_dersz, SaveContext, SaveFile}, user::User};
use rug::az::UnwrappedAs;
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
    user_value: Option<User>,
    dersz: Option<DeRsz>,
    tx: Sender<PathBuf>,
    rx: Receiver<PathBuf>,
}

impl Default for TameApp {
    fn default() -> Self {
        let file_reader = FileReader::new("outputs".into(), None, false, false, true, None);
        let (tx, rx) = mpsc::channel();
        Self {
            current_file_name: None,
            file_name: "".to_string(),
            file_reader,
            steam_id: "".to_string(),
            json_value: None,
            user_value: None,
            dersz: None,
            tx, rx,
        }
    }
}

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

                if ui.button("Open File").clicked() {
                    let tx = self.tx.clone();
                    std::thread::spawn(move || {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            println!("{path:?}");
                            tx.send(path).unwrap();
                        }
                    });
                }

                if let Ok(result) = self.rx.try_recv() {
                    self.current_file_name = Some(result.display().to_string());
                    let reader = File::open(result).unwrap();
                    let user = User::new(reader).unwrap();
                    let result = user.rsz.deserialize_to_dersz().unwrap();
                    self.dersz = Some(result);
                    println!("{}", serde_json::json!(self.dersz));
                    println!("Loaded");
                }

                /*if ui.button("Select File").clicked() {
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
                Some(user)
            } else {
                None
            };
            //self.json_value = json_value;
            self.user_value = json_value;
            }
            }*/
            });
            ui.horizontal(|ui| {
                ui.label("Steam ID:");
                ui.add(TextEdit::singleline(&mut self.steam_id).desired_width(100.0))
            });

            ui.separator();
            ScrollArea::both().auto_shrink(false).max_width(f32::INFINITY).show(ui, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::monospace(14.0));
                if let Some(dersz) = self.dersz.as_mut() {
                    //println!("struct: {:?}", dersz.structs[22]);
                    //println!("{:?}", dersz.roots);
                    for root in &dersz.roots {
                        let val = dersz.structs.get(*root as usize).unwrap();

                        let (hash, mut field_values) = {
                            let val = dersz.structs.get_mut(*root as usize).unwrap();
                            let (hash, field_values) = std::mem::take(&mut *val);
                            (hash, field_values)
                        };
                        let root_type = RszDump::get_struct(hash).unwrap();
                        ui.label(&root_type.name);
                        let mut ctx = RszEditCtx::new(*root, &mut dersz.structs);
                        field_values.edit(ui, &mut ctx);
                        dersz.structs[*root as usize] = (hash, field_values);
                    }
                }
                //let mut fake_structs = Vec::new();
                //let mut ctx = RszEditCtx::new(0, &mut fake_structs);
                //value.edit(ui, &mut ctx);
            })
        });
    }
}
