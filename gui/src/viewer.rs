use std::{collections::{HashMap, HashSet}, sync::mpsc::{self, Receiver, Sender}};

use eframe::egui::{Ui};
use egui_dock::tab_viewer::OnCloseResponse;

#[cfg(not(target_arch = "wasm32"))]
use mhtame::bindings::runner::ScriptRunner;

use mhtame::{
    save::game::Game, sdk::type_map::ContentLanguage
};

use crate::{Config, game_context::{AssetPaths, GameCtx}, tab::{self, TabType}};

#[derive(Debug)]
pub struct Viewer {
    pub num_tabs: u64,
    pub config: Config,
    pub default_language: ContentLanguage,

    #[cfg(not(target_arch = "wasm32"))]
    pub script_runner: ScriptRunner,
    pub reload: bool,
    game_contexts: HashMap<Game, GameCtx>,
    pub loading_games: HashSet<Game>,
    pub rx: Receiver<(Game, GameCtx)>, 
    pub tx: Sender<(Game, GameCtx)>,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            config,
            num_tabs: 0,
            default_language: ContentLanguage::English,
            #[cfg(not(target_arch = "wasm32"))]
            script_runner: ScriptRunner::new(),
            reload: true,
            game_contexts: HashMap::new(),
            loading_games: HashSet::new(),
            rx, tx
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_script(&mut self, path: &str) {
        let res = self.script_runner.load_and_execute_from_file(path);
        println!("[INFO] run_script {res:?}");
    }

    pub fn update(&mut self, ctx: &eframe::egui::Context) {
        if self.reload {
            self.reload();
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.script_runner.update_dialogs(ctx);
    }

    pub fn reload(&mut self) {
        println!("[INFO] Reloaded");
        self.game_contexts = HashMap::new();
        self.reload = false;
    }
}

impl egui_dock::TabViewer for Viewer {
    type Tab = tab::Tab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        #[cfg(target_arch = "wasm32")]
        if !self.loading_games.is_empty() {
            ui.label("Loading Assets...");
            while let Ok((game, ctx)) = self.rx.try_recv() {
                log::info!("[INFO] Received game context for {game:?}");
                self.game_contexts.insert(game, ctx);
                self.loading_games.remove(&game);
            }
        }
        ui.push_id(tab.idx, |ui| {
            match &mut tab.tab {
                TabType::SaveFileView(save_file) => {
                    save_file.language = self.default_language;
                    let game = save_file.game;
                    #[cfg(target_arch = "wasm32")]
                    if !self.game_contexts.contains_key(&game) && !self.loading_games.contains(&game) {
                        let assets = AssetPaths::from_game(game);
                        GameCtx::start_loading_wasm(self.tx.clone(), game, assets);
                        self.loading_games.insert(game);
                    }
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    self.game_contexts.entry(game).or_insert_with(|| {
                        let asset_paths = AssetPaths::from_game(game);
                        GameCtx::new(&asset_paths)
                    });
                    save_file.ui(ui, &mut self.game_contexts)
                },
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
