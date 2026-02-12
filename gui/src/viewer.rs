use std::{
    collections::HashMap,
};

use eframe::egui::{Ui};
use egui_dock::tab_viewer::OnCloseResponse;

#[cfg(not(target_arch = "wasm32"))]
use mhtame::bindings::runner::ScriptRunner;

use mhtame::{
    edit::copy::CopyBuffer,
    save::remap::Remap,
    sdk::type_map::{ContentLanguage, TypeMap},
};

use crate::{Config, tab::{self, TabType}};

#[derive(Debug)]
pub struct Viewer {
    pub num_tabs: u64,
    game_ctx: GameCtx,
    pub config: Config,
    pub default_language: ContentLanguage,

    #[cfg(not(target_arch = "wasm32"))]
    pub script_runner: ScriptRunner,
    pub reload: bool,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let game_ctx = GameCtx::new(&config);
        Self {
            game_ctx,
            config,
            num_tabs: 0,
            default_language: ContentLanguage::English,
            #[cfg(not(target_arch = "wasm32"))]
            script_runner: ScriptRunner::new(),
            reload: true,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_script(&mut self, path: &str) {
        let res = self.script_runner.load_and_execute_from_file(path);
        println!("[INFO] run_script {res:?}");
    }

    pub fn update(&mut self, ctx: &eframe::egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
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
       #[cfg(not(target_arch = "wasm32"))]
        let type_map = TypeMap::load_with_msgs(
            &config.rsz_path,
            &config.enums_path,
            &config.msgs_path,
            &config.mappings_path,
        )
        .expect("Could not load assets for the editor, make sure your assets are in the right place")
        .load_string_map("assets/mhst3_strings.txt")
        .expect("Error loading mhst3_strings.txt map");
       #[cfg(target_arch = "wasm32")]
        let type_map = {
            use mhtame::rsz::dump::decompress;
            const RSZ_JSON: &[u8] = include_bytes!("../../assets/rszmhwilds_packed.json.gz");
            const ENUMS_JSON: &[u8] = include_bytes!("../../assets/enumsmhwilds.json.gz");
            const MSGS_JSON: &[u8] = include_bytes!("../../assets/combined_msgs.json.gz");
            const ENUM_MAPPINGS_JSON: &str = include_str!("../../assets/enum_text_mappings.json");
            const MHST3_STRINGS: &str = include_str!("../../assets/mhst3_strings.txt");
            let rsz = decompress(RSZ_JSON);
            let enums = decompress(ENUMS_JSON);
            let msgs = decompress(MSGS_JSON);
            let mut res = TypeMap::parse_str(&rsz, &enums)
                .expect("Could not load type map")
                .load_msg(&msgs, ENUM_MAPPINGS_JSON);
            match res.load_string_map_from_str(&MHST3_STRINGS) {
                Err(e) => eprintln!("[ERROR] Could not load MHST3 Strings"),
                _ => ()
            };
            res
        };

        #[cfg(target_arch = "wasm32")]
        let remaps: HashMap<String, Remap> =
            serde_json::from_str(include_str!("../../assets/wilds_remap.json")).unwrap();
        #[cfg(not(target_arch = "wasm32"))]
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
                #[cfg(not(target_arch = "wasm32"))]
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
            #[cfg(not(target_arch = "wasm32"))]
            TabType::Script(script) => script.path.clone().unwrap_or(format!("File #{}", tab.idx).into()),
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
