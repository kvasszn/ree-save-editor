use std::{fs::File, path::PathBuf};

use eframe::egui::{self, Color32};

pub struct CodeEditor {
    pub path: Option<String>,
    pub language: String,
    pub code: String,
    pub idx: u64,
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self {
            path: None,
            language: "lua".into(),
            code: "print(\"Hello World\")".into(),
            idx: 0,
        }
    }
}

impl CodeEditor {
    pub fn new_default(idx: u64) -> Self {
        let mut editor = CodeEditor::default();
        editor.idx = idx;
        editor
    }

    pub fn new(path_str: &str, idx: u64) -> Self {
        let path = PathBuf::from(path_str);
        let language = path.extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or("lua".to_string());
        let code = std::fs::read_to_string(path).unwrap_or("".to_string());
        Self {
            path: Some(path_str.to_string()),
            code,
            language,
            idx,
        }
    }

    pub fn reload(&mut self) {
        if let Some(path) = &self.path {
            let code = std::fs::read_to_string(&path);
            if let Ok(code) = code {
                self.code = code;
            } else if let Err(e) = code {
                eprintln!("[ERROR] Error reloading save {e}");
            }
        } else {
            eprintln!("[WARNING] Please save the file before reloading");
        }
    }

    pub fn save(&mut self) {
        let mut path = self.path.clone().map(|p| PathBuf::from(p));
        if path.is_none() {
            let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
            let file_path = rfd::FileDialog::new()
                .set_title("Save Lua Script")
                .add_filter("Lua Script", &["lua"])
                .add_filter("All Files", &["*"])
                .set_directory(cur)
                .save_file();
            path = file_path;
        }
        if let Some(path) = path {
            self.path = Some(path.to_string_lossy().to_string());
            match File::create(path) {
                Ok(mut file) => {
                    use std::io::Write;
                    let _ = file.write_all(self.code.as_bytes());
                }
                Err(e) => {
                    eprintln!("[ERROR] Error Saving file {e}");
                }
            }
        } else {
            eprintln!("[ERROR] No path to save file to");
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), &*ui.style());
        let mut run = false;
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                run = true;
            }
            if ui.button("Save").clicked() {
                self.save();
            }
            if ui.button("Refresh").clicked() {
                self.reload();
            }
            ui.collapsing("Theme", |ui| {
                ui.group(|ui| {
                    theme.ui(ui);
                    theme.clone().store_in_memory(ui.ctx());
                });
            });
        });

        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save();
        }

        ui.label("THIS EDITOR SUCKS YOU PROBABLY SHOULDN'T USE IT IT'S MORE HERE TO JUST SHOW THE CODE");
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                ui.ctx(),
                ui.style(),
                &theme,
                buf.as_str(),
                &self.language,
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };
        let background_color = if theme.is_dark() {
            Color32::from_rgb(30, 30, 30)
                //Color32::from_rgb(40, 44, 52) // One Dark/Atom style
        } else {
            Color32::from_rgb(250, 250, 250)
        };
        let mut add_text_area = |ui: &mut egui::Ui, code: &mut String| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let editor = egui::TextEdit::multiline(code)
                        .font(egui::TextStyle::Monospace) // for cursor height
                        .code_editor()
                        .frame(false)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter)
                        .background_color(background_color);
                    ui.add_sized(ui.available_size(), editor);
                });
        };
        let add_line_numbers = |ui: &mut egui::Ui, code: &String| {
            let line_count = code.lines().count().max(1);
            let mut line_numbers = (1..=line_count)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            ui.add(
                egui::TextEdit::multiline(&mut line_numbers)
                .font(egui::TextStyle::Monospace)
                .interactive(false)
                .frame(false)
                .desired_width(24.0)
                .text_color(egui::Color32::from_gray(100))
            );
        };
        let border_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(60));
        egui::Frame::new()
            .fill(background_color)
            .inner_margin(4.0)
            .stroke(border_stroke)
            .show(ui, |ui| {
                let available_size = ui.available_size();
                ui.allocate_ui_with_layout(
                    available_size,
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                add_line_numbers(ui, &self.code);
                                let (rect, _) = ui.allocate_at_least(egui::vec2(2.0, ui.available_height()), egui::Sense::hover());
                                ui.painter().vline(rect.left(), rect.y_range(), egui::Stroke::new(1.0, egui::Color32::from_gray(45)));
                                add_text_area(ui, &mut self.code);
                            });
                    });
            });

        return run;
    }
}
