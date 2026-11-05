use std::{path::PathBuf, sync::mpsc::{Receiver, Sender, channel}};

use eframe::egui;

#[derive(Default, Clone, Copy)]
pub enum DialogType {
    #[default]
    File,
    Folder,
    Save,
}

pub struct AsyncFileDialog {
    receiver: Receiver<Option<PathBuf>>,
    sender: Sender<Option<PathBuf>>,
    pub selected_file: Option<PathBuf>,
    pub is_open: bool,

    filter_name: Option<String>,
    filters: Vec<String>,
    dialog_type: DialogType,
    title: String
}

impl Default for AsyncFileDialog {
    fn default() -> Self {
        let (tx, rx) = channel::<Option<PathBuf>>();
        Self {
            receiver: rx,
            sender: tx,
            selected_file: None,
            is_open: false,
            filter_name: None,
            filters: Vec::new(),
            dialog_type: DialogType::File,
            title: "Choose File".to_string(),
        }
    }
}

impl AsyncFileDialog {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn dialog_type(mut self, ty: DialogType) -> Self {
        self.dialog_type = ty;
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.filter_name = Some(name.to_string());
        self.filters = extensions.iter().map(|ext| ext.to_string()).collect();
        self
    }

    pub fn poll(&mut self) {
        if let Ok(file_opt) = self.receiver.try_recv() {
            self.selected_file = file_opt;
            self.is_open = false;
        }
    }

    pub fn open(&mut self, ctx: egui::Context) {
        if self.is_open {
            return;
        }

        self.is_open = true;
        let tx = self.sender.clone();
        let title = self.title.clone();
        let filter_name = self.filter_name.clone();
        let filters = self.filters.clone();
        let dialog_type = self.dialog_type;

        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new()
                .set_title(&title);

            if let Some(filter_name) = &filter_name {
                dialog = dialog.add_filter(filter_name, &filters);
            }

            // TODO: something to set directory path/filename

            let file = match dialog_type {
                DialogType::File => dialog.pick_file(),
                DialogType::Folder => dialog.pick_folder(),
                DialogType::Save => dialog.save_file(),
            };

            let _ = tx.send(file);
            ctx.request_repaint();
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, show_selected: bool) -> bool {
        let mut file_just_received = false;

        if let Ok(file_opt) = self.receiver.try_recv() {
            self.selected_file = file_opt;
            self.is_open = false;
            file_just_received = true;
        }

        ui.add_enabled_ui(!self.is_open, |ui| {
            if ui.button(&self.title).clicked() {
                self.open(ui.ctx().clone());
            }
        });

        if self.is_open {
            ui.spinner();
        } else if let Some(path) = &self.selected_file && show_selected {
            ui.label(format!("Selected: {}", path.display()));
        }

        file_just_received
    }
}
