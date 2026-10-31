use std::{collections::{HashMap}, sync::{Arc, RwLock, mpsc::{Sender}}};

use eframe::egui::{Ui};
use egui_dock::tab_viewer::OnCloseResponse;

use ree_save_core::{
    edit::copy::CopyBuffer, game_context::GameData, save::game::Game
};

use crate::{config::Config, tab::{self, TabType}};

pub struct Viewer<'a> {
    pub game_contexts: &'a Arc<RwLock<HashMap<Game, GameData>>>,
    pub config: &'a Config,
    pub load_request: &'a Sender<Game>,
    pub copy_buffer: &'a mut CopyBuffer
}

impl<'a> egui_dock::TabViewer for Viewer<'a> {
    type Tab = tab::Tab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.push_id(tab.id, |ui| {
            match &mut tab.tab {
                TabType::SaveFile(save_file) => {
                    let game_contexts = self.game_contexts.read().unwrap();
                    if !game_contexts.contains_key(&save_file.game)  {
                        self.load_request.send(save_file.game).ok();
                    }
                    save_file.ui(ui, self.config, &game_contexts, self.copy_buffer);
                }
            }
        });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        let title: String = match &tab.tab {
            TabType::SaveFile(s) => {
                use std::path::PathBuf;

                let file_str = match &s.file_picker.selected_file {
                    Some(path) => {
                        let count = path.iter().count();
                        let last_two: PathBuf = path.iter().skip(count.saturating_sub(2)).collect();
                        last_two.display().to_string()
                    }
                    None => "(empty)".to_string(),
                };

                format!("{:?}|{}", s.game, file_str).into()
            },
        };
        title.into()
    }

    fn context_menu(
        &mut self,
        ui: &mut Ui,
        tab: &mut Self::Tab,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        ui.label(self.title(tab));
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> egui_dock::tab_viewer::OnCloseResponse {
        OnCloseResponse::Close
    }
}
