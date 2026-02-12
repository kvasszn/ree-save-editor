use std::{
    collections::HashMap,
};

use eframe::egui::{Ui};
use egui_dock::tab_viewer::OnCloseResponse;
use mhtame::{
    bindings::runner::ScriptRunner,
    edit::copy::CopyBuffer,
    save::remap::Remap,
    sdk::type_map::{ContentLanguage, TypeMap},
};

use crate::{Config, file::FileView, tab::{self, TabType}};

#[derive(Debug)]
pub struct Viewer {
    pub num_tabs: u64,
    game_ctx: GameCtx,
    pub config: Config,
    pub default_language: ContentLanguage,
    pub script_runner: ScriptRunner,
    pub reload: bool,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let game_ctx = GameCtx::new(&config);
        let script_runner = ScriptRunner::new();
        Self {
            game_ctx,
            config,
            num_tabs: 0,
            default_language: ContentLanguage::English,
            script_runner: script_runner,
            reload: true,
        }
    }

    pub fn run_script(&mut self, path: &str) {
        let res = self.script_runner.load_and_execute_from_file(path);
        println!("[INFO] run_script {res:?}");
    }

    pub fn update(&mut self, ctx: &eframe::egui::Context) {
        self.script_runner.update_dialogs(ctx);
    }
}

#[derive(Debug)]
pub struct GameCtx {
    pub type_map: TypeMap,
    pub copy_buffer: CopyBuffer,
    pub remaps: HashMap<String, Remap>,
}

impl GameCtx {
    pub fn new(config: &Config) -> Self {
        let type_map = TypeMap::load_with_msgs(
            &config.rsz_path,
            &config.enums_path,
            &config.msgs_path,
            &config.mappings_path,
        )
        .expect(
            "Could not load assets for the editor, make sure your assets are in the right place",
        );
        let type_map = type_map
            .load_string_map("assets/mhst3_strings.txt")
            .expect("Error loading mhst3_strings.txt map");
        let remaps = {
            let data = std::fs::read_to_string("./assets/wilds_remap.json");
            data.map(|data| {
                let remaps: HashMap<String, Remap> =
                    serde_json::from_str(&data).unwrap_or_default();
                remaps
            })
            .unwrap_or_default()
        };
        Self {
            type_map,
            copy_buffer: CopyBuffer::Null,
            remaps,
        }
    }
}

impl egui_dock::TabViewer for Viewer {
    type Tab = tab::Tab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.push_id(tab.idx, |ui| {
            match &mut tab.tab {
                TabType::SaveFileView(save_file) => save_file.ui(ui, &mut self.game_ctx),
                TabType::Script(script) => {
                    if script.ui(ui) {
                        if script.path.is_none() {
                            script.save();
                        }
                        if let Some(path) = &script.path {
                            self.run_script(&path);
                        } else {
                            eprintln!("[WARNING] Could not run script, idk where it is");
                        }
                    }
                },
            }
        });
    }

    fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
        let title: String = match &tab.tab {
            TabType::SaveFileView(_) => format!("File #{}", tab.idx).into(),
            TabType::Script(script) => script.path.clone().unwrap_or(format!("File #{}", tab.idx).into())
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
        self.num_tabs -= 1;
        OnCloseResponse::Close
    }
}
