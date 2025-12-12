use core::f32;
use std::{fs::{File, create_dir_all}, path::PathBuf, sync::mpsc::{self, Receiver, Sender}};

use eframe::egui::{self,  ScrollArea, TextEdit, Vec2};
use mhtame::{edit::{Edit, RszEditCtx}, file::{FileReader, StructRW}, rsz::{dump::{ENUM_FILE, RSZ_FILE}, rszserde::DeRsz}, save::{SaveContext, SaveFile}};
use clap::{Parser};

#[derive(Parser, Debug)]
#[command(name = "mhtame-gui")]
#[command(version, about, long_about = None)]
pub struct GuiArgs {
    #[arg(short('f'), long)]
    file_name: Option<String>,

    #[arg(short('o'), long, default_value_t = String::from("outputs"))]
    out_dir: String,

    #[arg(long)]
    steamid: Option<String>,

    #[arg(long, default_value_t = String::from("rszmhwilds_packed.json"))]
    rsz_path: String,

    #[arg(long, default_value_t = String::from("enums.json"))]
    enums_path: String,
}

pub fn main() -> eframe::Result<()> {
    env_logger::init();
    let args = GuiArgs::parse();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
        .. Default::default()
    };
    ENUM_FILE.set(args.enums_path.clone()).unwrap();
    RSZ_FILE.set(args.rsz_path.clone()).unwrap();
    eframe::run_native("mhtame",
        options,
        Box::new(|_cc| {
            Ok(Box::new(TameApp::new(args)))
        }),
    )
}

pub struct TameApp {
    file_name: String,
    current_file_name: Option<String>,
    steam_id: Option<u64>,
    file_reader: FileReader,
    dersz: Option<DeRsz>,
    tx: Sender<PathBuf>,
    rx: Receiver<PathBuf>,
    tx_output: Sender<PathBuf>,
    rx_output: Receiver<PathBuf>,
    updated: bool,
    //current_file: Option<Box<dyn Edit>>,
    current_file: Option<SaveFile>,
    output_path: PathBuf,
    show_popup: bool,
    popup_msg: String,
}

impl TameApp {
    pub fn new(args: GuiArgs) -> Self {
        let file_reader = FileReader::new(args.out_dir.into(), None, false, false, true, args.steamid.clone());
        let (tx, rx) = mpsc::channel();
        let steamid = if let Some(x) = args.steamid {
            let u = if let Some(hex) = x.clone().strip_prefix("0x") {
                u64::from_str_radix(hex, 16).ok()
            } else {
                u64::from_str_radix(&x, 10).ok()
            };
            u
        } else { None };
        let (tx_output, rx_output) = mpsc::channel();
        Self {
            current_file_name: args.file_name.clone(),
            file_name: args.file_name.unwrap_or_default(),
            file_reader,
            steam_id: steamid,
            dersz: None,
            tx, rx,
            tx_output, rx_output,
            updated: false,
            current_file: None,
            output_path: PathBuf::from("./outputs/saves/"),
            show_popup: false,
            popup_msg: String::from("")
        }

    }
}

impl Default for TameApp {
    fn default() -> Self {
        let file_reader = FileReader::new("outputs".into(), None, false, false, true, None);
        let (tx, rx) = mpsc::channel();
        let (tx_output, rx_output) = mpsc::channel();
        Self {
            current_file_name: None,
            file_name: "".to_string(),
            file_reader,
            steam_id: None,
            dersz: None,
            tx, rx,
            tx_output, rx_output,
            updated: false,
            current_file: None,
            output_path: PathBuf::from("./outputs/saves/"),
            show_popup: false,
            popup_msg: String::from("")
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
                self.updated = false;
                println!("Dropped file path: {}", path.display());
            } else {
                self.current_file_name = Some(df.name.clone());
                self.updated = false;
                println!("Dropped file name: {}", df.name);
            }
        }

        if let Some(file) = &self.current_file_name {
            self.file_name = file.clone()
        };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().spacing.item_spacing *= 1.5;
            ui.style_mut().spacing.button_padding *= 1.5;
            for (_style, font_id) in ui.style_mut().text_styles.iter_mut() {
                font_id.size *= 1.2;
            }
            ui.heading("MH Wilds Save Editor");
            ui.horizontal(|ui| {
                ui.label("File:");
                ui.add(TextEdit::singleline(&mut self.file_name).desired_width(400.0));

                if ui.button("Open File").clicked() {
                    let tx = self.tx.clone();
                    std::thread::spawn(move || {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            tx.send(path).unwrap();
                        }
                    });
                }

                if ui.button("Save").clicked() {
                    if let Some(steamid) = self.steam_id {
                        if let Some(save_file) = &self.current_file {
                            if let Some(path) = &self.current_file_name {
                                let path = PathBuf::from(path);
                                if let Some(file_name) = path.file_name() {
                                    let mut path = self.output_path.clone();
                                    if create_dir_all(&path).is_ok() {
                                        path.push(file_name);
                                        self.popup_msg = match save_file.save(&path, steamid) {
                                            Ok(()) => format!("Saved to {path:?}"),
                                            Err(e) => format!("Failed to save file {e}")
                                        };
                                        self.show_popup = true;
                                    } else {
                                        self.popup_msg = String::from("Could not create directory for file");
                                        self.show_popup = true;
                                    };
                                } 
                            }
                        }
                    } else {
                        self.popup_msg = String::from("Need a steamid to save");
                        self.show_popup = true;
                    }
                }

                if ui.button("Refresh").clicked() {
                    self.updated = false;
                }

                if !self.updated {
                    if let Some(path) = &self.current_file_name {
                        if let Ok(mut reader) = File::open(path) {
                            if let Some(steamid) = self.steam_id {
                                if let Ok(save) = SaveFile::read(&mut reader, &mut SaveContext { key: steamid }) {
                                    self.current_file = Some(save);
                                }
                                self.updated = true;
                            }
                        } else {
                            self.popup_msg = String::from("Failed to open path to file {path}");
                            self.show_popup = true;
                        }
                    }
                }

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
                                        self.show_popup= false;
                                    }
                                });
                            });
                        });
                }
                if ui.input(|i| i.pointer.any_pressed()) {
                    self.show_popup = false;
                }

                if let Ok(result) = self.rx.try_recv() {
                    self.current_file_name = Some(result.display().to_string());
                    self.updated = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Steam ID:");
                let mut s = if let Some(steamid) = self.steam_id {
                    steamid.to_string()
                } else {"".to_string()};
                ui.add(TextEdit::singleline(&mut s));
                self.steam_id = u64::from_str_radix(&s, 10).ok();
            });
            ui.horizontal(|ui| {
                ui.label("Output Path:");
                let mut s = self.output_path.to_string_lossy().to_string();
                ui.add(TextEdit::singleline(&mut s));
                self.output_path = PathBuf::from(s);

                if ui.button("Choose Output Path").clicked() {
                    let tx = self.tx_output.clone();
                    std::thread::spawn(move || {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            tx.send(path).unwrap();
                        }
                    });
                }

                if let Ok(result) = self.rx_output.try_recv() {
                    self.output_path = result;
                }
            });

            ui.separator();
            ScrollArea::both().auto_shrink(false).max_width(f32::INFINITY).show(ui, |ui| {
                ui.style_mut().override_font_id = Some(egui::FontId::monospace(14.0));
                if let Some(cur_file) = self.current_file.as_mut() {
                    let mut fake_structs = Vec::new();
                    let mut ctx = RszEditCtx::new(0, &mut fake_structs);
                    cur_file.edit(ui, &mut ctx);
                }
            })
        });
    }
}
