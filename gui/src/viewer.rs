use std::collections::HashMap;

use eframe::egui::Ui;
use egui_dock::tab_viewer::OnCloseResponse;
use mhtame::{edit::copy::CopyBuffer, save::remap::Remap, sdk::type_map::{ContentLanguage, TypeMap}};

use crate::{Config, file::FileView};

#[derive(Debug)]
pub struct Viewer {
    pub num_tabs: u64,
    game_ctx: GameCtx,
    pub config: Config,
    pub default_language: ContentLanguage
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let game_ctx = GameCtx::new(&config);
        Self {
            game_ctx,
            config,
            num_tabs: 0,
            default_language: ContentLanguage::English
        }
    }
}

#[derive(Debug)]
pub struct GameCtx {
    pub type_map: TypeMap,
    pub copy_buffer: CopyBuffer,
    pub remaps: HashMap<String, Remap>
}

impl GameCtx {
    pub fn new(config: &Config) -> Self {
        let type_map = TypeMap::load_with_msgs(&config.rsz_path, &config.enums_path, &config.msgs_path, &config.mappings_path)
            .expect("Could not load assets for the editor, make sure your assets are in the right place");
        let remaps = {
            let data = std::fs::read_to_string("./assets/wilds_remap.json");
            data.map(|data| {
                let remaps: HashMap<String, Remap> = serde_json::from_str(&data).unwrap_or_default();
                remaps
            }).unwrap_or_default()
        };
        println!("{remaps:?}");
        Self {
            type_map,
            copy_buffer: CopyBuffer::Null,
            remaps
        }
    }
}

impl egui_dock::TabViewer for Viewer {
    type Tab = FileView;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.push_id(tab.idx, |ui| {
            tab.ui(ui, &mut self.game_ctx);
        });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        format!("Save File #{}", tab.idx).into()
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
        self.num_tabs -= 1;
        OnCloseResponse::Close
    }
}
