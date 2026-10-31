use std::{collections::HashMap, error::Error, io::Write, ops::Range, path::{Path, PathBuf}};

use eframe::egui::{ComboBox, DragValue, Ui};
use ree_lib::{assets::bundle::Bundle, context::EngineContext, enums::EnumMap, rsz::RszMap};
use ree_save_core::{edit::{EditContext, Editable, copy::CopyBuffer}, game_context::GameData, save::{SaveFile, SaveOptions, game::Game, json}};
use serde::Serialize;
use uuid::Uuid;

use crate::{config::Config, dialog::{AsyncFileDialog, DialogType}, steam::{Steam}};

pub struct Tab {
    pub tab: TabType,
    pub id: Uuid,
    pub name: Option<String>,
}

pub enum TabType {
    SaveFile(SaveFileView)
}

impl From<SaveFileView> for Tab {
    fn from(value: SaveFileView) -> Self {
        Self {
            tab: TabType::SaveFile(value),
            id: Uuid::new_v4(),
            name: None
        }
    }
}

pub struct SaveFileView {
    save_file: Option<SaveFile>,
    pub game: Game,
    steam: Steam,
    last_error: Option<Box<dyn Error>>,

    pub file_picker: AsyncFileDialog,
    save_as_picker: AsyncFileDialog,
    json_save_picker: AsyncFileDialog,

    // Save Options
    brute_force: bool,
    brute_force_range: Range<u64>,
    curve_index: Option<usize>,
    dump: bool,
}

impl SaveFileView {
    pub fn new(config: &Config) -> Self {
        Self {
            save_file: None,
            game: config.game,
            steam: Steam::new(config.id, config.steam_path.clone()),
            last_error: None,
            brute_force: false,
            brute_force_range: (0x0110000100000000u64..0x01100001ffffffffu64),
            curve_index: None,
            file_picker: AsyncFileDialog::new().filter("bin", &["bin", "*"])
                .title("Browse"),
            json_save_picker: AsyncFileDialog::new().filter("json", &["json", "*"])
                .title("Save As").dialog_type(DialogType::Save),
            save_as_picker: AsyncFileDialog::new().filter("bin", &["bin", "*"])
                .title("Save As").dialog_type(DialogType::Save),
            dump: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, config: &Config, game_contexts: &HashMap<Game, GameData>, copy_buffer: &mut CopyBuffer) {
        let game_context = game_contexts.get(&self.game);

        ui.horizontal(|ui| {
            if let Some(path) = &self.file_picker.selected_file {
                ui.label(format!("File: {}", path.display()));
            } else {
                ui.label("Click Browse to select a file");
            }
        });

        ui.horizontal(|ui| {
            if self.file_picker.ui(ui, false) {
                self.load_save(game_context);
            };

            if ui.button("Load").clicked() {
                self.load_save(game_context);
            }

            if ui.button("Save").clicked() {
                self.save_file(self.file_picker.selected_file.clone(), config);
            }

            if self.save_as_picker.ui(ui, false) {
                self.save_file(self.save_as_picker.selected_file.clone(), config);
                self.save_as_picker.selected_file = None;
            } 

            ui.style_mut().wrap_mode = Some(eframe::egui::TextWrapMode::Extend);
            ui.label("Game Profile");
            ComboBox::from_id_salt("select_game")
                .selected_text(self.game.to_string())
                .show_ui(ui, |ui| {
                    use strum::IntoEnumIterator;
                    for option in Game::iter() {
                        ui.selectable_value(
                            &mut self.game,
                            option,
                            option.to_string(),
                        );
                    }
                });

            if ui.button("Save Json").clicked() {
                self.save_json(game_context, self.file_picker.selected_file.clone(), config);
            }

            if ui.button("Save Json As").clicked() {
                self.save_json(game_context, self.json_save_picker.selected_file.clone(), config);
            }
        });

        ui.horizontal(|ui| {
            self.steam.edit_steam_id(ui);
            self.steam.select_user(ui);
            if let Some(path) = self.steam.found_file(ui, self.game) {
                self.file_picker.selected_file = Some(PathBuf::from(path));
                self.load_save(game_context);
            }
            self.steam.edit_steam_path(ui);
        });

        ui.horizontal(|ui| {
            if let Some(val) = self.curve_index.as_mut() {
                ui.label("Citrus Curve Index");
                ui.add(DragValue::new(val).speed(1.0));
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.brute_force, "Brute Force SteamID");
            ui.checkbox(&mut self.dump, "Dump Bytes");
        });

        if self.brute_force {
            ui.horizontal(|ui| {
                ui.label("Start:");
                ui.add(DragValue::new(&mut self.brute_force_range.start).speed(1.0).hexadecimal(8, true, false));
                ui.label("End:");
                ui.add(DragValue::new(&mut self.brute_force_range.end).speed(1.0).hexadecimal(8, true, false));
            });
        }

        if let Some(last_error) = &self.last_error {
            ui.label(format!("Error: {}", last_error));
        }

        ui.separator();

        if let Some(save_file) = &mut self.save_file {
            let empty_rsz_map = RszMap::default();
            let empty_assets = Bundle::default();
            let empty_enums = EnumMap::default();
            let empty_remaps = HashMap::new();

            // verbose but whatever who cares
            let rsz_map = if let Some(game_context) = game_context {
                &game_context.rsz
            } else {
                &empty_rsz_map
            };

            let assets = if let Some(game_context) = game_context {
                &game_context.bundle
            } else {
                &empty_assets
            };

            let enums = if let Some(game_context) = game_context {
                &game_context.enums
            } else {
                &empty_enums
            };

            let remaps = if let Some(game_context) = game_context {
                &game_context.remaps
            } else {
                &empty_remaps
            };

            let engine = EngineContext::new(config.language, rsz_map, assets, enums);
            let mut query_cache = HashMap::new();
            let mut path = Vec::with_capacity(50);
            let mut ctx = EditContext {
                engine_context: &engine,
                config: &config.editor,
                remaps,
                path: &mut path,
                query_cache: &mut query_cache,
                copy_buffer,
            };
            save_file.ui(ui, &mut ctx);
        }
    }

    fn load_save(&mut self, game_context: Option<&GameData>) {
        let Some(path) = &self.file_picker.selected_file else {
            self.last_error = Some("No save file to load".into());
            return;
        };

        if path.exists() {
            match std::fs::read(path) {
                Ok(data) => {
                    let mut options = SaveOptions::new(self.game);

                    if let Some(id) = self.steam.steam_id {
                        options = options.id(id);
                    }
                    if self.brute_force {
                        options = options.brute_force(self.brute_force_range.start, self.brute_force_range.end - self.brute_force_range.start);
                    }
                    if let Some(curve_index) = self.curve_index {
                        options = options.curve_index(curve_index);
                    }
                    if self.dump {
                        options = options.debug_dump();
                    }

                    let save_file = SaveFile::read_save(data, &mut options);
                    match save_file {
                        Ok(save_file) => self.save_file = Some(save_file),
                        Err(e) => self.last_error = Some(Box::new(e)),
                    }

                    if self.brute_force {
                        self.steam.set_id(options.id);
                    }
                    self.curve_index = options.curve_index;
                }
                Err(e) => {
                    self.last_error = Some(Box::new(e));
                },
            }

        }
    }

    fn save_file(&mut self, path: Option<PathBuf>, config: &Config) {
        log::info!("Saving file to {path:?}");
        let Some(path) = path else {
            self.last_error = Some("No save location to save to".into());
            return;
        };

        // TODO: Is this bugged af rn
        if path.exists() && !path.is_file() {
            self.last_error = Some(format!("{path:?} is not a file").into());
            return;
        }

        if self.save_file.is_some() {
            self.backup_loaded_save(&path, config);
        }
        if let Some(save_file) = &self.save_file {
            self.file_picker.selected_file = Some(path.clone());
            // backup saves if the file being written to already exists
            match std::fs::File::create(&path) {
                Ok(mut f) => {
                    let mut options = SaveOptions::new(self.game);
                    if let Some(id) = self.steam.steam_id {
                        options = options.id(id);
                    }
                    if let Some(curve_index) = self.curve_index {
                        options = options.curve_index(curve_index);
                    }

                    let data = save_file.write_save(&options);
                    match data {
                        Ok(data) => {
                            match f.write_all(&data) {
                                Ok(_) => log::info!("Save File to {path:?}"),
                                Err(e) => self.last_error = Some(Box::new(e)),
                            }
                        }
                        Err(e) => self.last_error = Some(Box::new(e)),
                    }
                }
                Err(e) => {
                    self.last_error = Some(Box::new(e));
                },
            }

        }
    }

    fn save_json(&mut self, game_context: Option<&GameData>, path: Option<PathBuf>, config: &Config) {
        log::info!("Saving file to {path:?}");
        let Some(mut path) = path else {
            self.last_error = Some("No save location to save to".into());
            return;
        };
        path.set_extension("json");

        if let Some(save_file) = &self.save_file {
            let empty_rsz_map = RszMap::default();
            let empty_assets = Bundle::default();
            let empty_enums = EnumMap::default();

            // verbose but whatever who cares
            let rsz_map = if let Some(game_context) = game_context {
                &game_context.rsz
            } else {
                &empty_rsz_map
            };

            let assets = if let Some(game_context) = game_context {
                &game_context.bundle
            } else {
                &empty_assets
            };

            let enums = if let Some(game_context) = game_context {
                &game_context.enums
            } else {
                &empty_enums
            };

            let engine = EngineContext::new(config.language, rsz_map, assets, enums);

            let view = json::SaveFileView {
                file: save_file,
                ctx: &engine
            };
            match std::fs::File::create(&path) {
                Ok(mut f) => {
                    let json = serde_json::to_string_pretty(&view);
                    match json {
                        Ok(json) => {
                            match f.write_all(json.as_bytes()) {
                                Ok(_) => log::info!("Save Json File to {path:?}"),
                                Err(e) => self.last_error = Some(Box::new(e)),
                            }
                        }
                        Err(e) => self.last_error = Some(Box::new(e)),
                    }
                }
                Err(e) => {
                    self.last_error = Some(Box::new(e));
                },
            }

        }
    }

    pub fn backup_loaded_save(&mut self, path: &Path, config: &Config) {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let backup_name = format!("{}_{}.{}.bak", file_stem, timestamp, ext);
        let backup_path = path.with_file_name(backup_name);

        if let Err(e) = std::fs::copy(path, &backup_path) {
            self.last_error = Some(format!("Failed to create backup: {}", e).into());
            return;
        }

        log::info!("Created backup at: {}", backup_path.display());
    }
}
