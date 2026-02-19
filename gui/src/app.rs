#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use eframe::{
    self,
    egui::{Align, CentralPanel, Layout, MenuBar, TopBottomPanel},
};
use egui_dock::{DockArea, DockState};
use mhtame::{sdk::type_map::ContentLanguage};

#[cfg(not(target_arch = "wasm32"))]
use crate::code_editor::CodeEditor;

use crate::{Config, file::{FilePicker, FileView}, tab::Tab, viewer::Viewer};

pub struct TameApp {
    viewer: Viewer,
    tree: DockState<Tab>,
    file_opener: FilePicker<false>,
}

impl TameApp {
    pub fn new(config: Config) -> Self {
        let file_view = FileView::new(&config, 0, ContentLanguage::English);
        let tab = Tab::from(file_view);
        let mut viewer = Viewer::new(config);
        let dock_state = DockState::new(vec![tab]);
        viewer.num_tabs += 1;
        Self {
            viewer,
            tree: dock_state,
            file_opener: FilePicker::new("Open"),
        }
    }
    fn add_tab(&mut self, tab: Tab) {
        if self.viewer.num_tabs == 0 {
            self.viewer.num_tabs += 1;
            self.tree = DockState::new(vec![tab])
        } else {
            self.viewer.num_tabs += 1;
            let surface = self.tree.main_surface_mut();
            surface.push_to_focused_leaf(tab);
        }
    }
}

impl eframe::App for TameApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        if let Some(file_pick_res) = self.file_opener.take() {
            use crate::file::FilePickResult;

            if let FilePickResult::Wasm {name, data} = file_pick_res {
                log::info!("Loading Save File {name}");
                let file_view = FileView::from_data(
                    &self.viewer.config,
                    name, data,
                    self.viewer.num_tabs,
                    self.viewer.default_language,
                );
                let tab = Tab::from(file_view);
                self.add_tab(tab);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = self.file_opener.take() {
            #[cfg(not(target_arch = "wasm32"))]
            if path.ends_with("lua") {
                log::info!("Loading Lua Script From {path}");
                let tab = Tab::load_script(&path, self.viewer.num_tabs);
                self.add_tab(tab);
            } else {
                log::info!("Loading Save File From {path}");
                let file_view = FileView::from_path(
                    &self.viewer.config,
                    path,
                    self.viewer.num_tabs,
                    self.viewer.default_language,
                );
                let tab = Tab::from(file_view);
                self.add_tab(tab);
            }
        }

        self.viewer.update(ctx);

        TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("MH Wilds Save Editor");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/kvasszn/mhtame");
                    ui.separator();
                });
            });

            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.file_opener.spawn_dialog();
                    }
                    if ui.button("Empty Save File").clicked() {
                        let file_view = FileView::new(
                            &self.viewer.config,
                            self.viewer.num_tabs,
                            self.viewer.default_language,
                        );
                        self.add_tab(Tab::from(file_view));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("New Script").clicked() {
                        self.add_tab(Tab::from(CodeEditor::new_default(self.viewer.num_tabs)));
                    } 
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Open Script").clicked() {
                        let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
                        let file_path = rfd::FileDialog::new()
                            .set_title("Select Lua Script")
                            .add_filter("Lua Script", &["lua"])
                            .add_filter("All Files", &["*"])
                            .set_directory(cur)
                            .pick_file();
                        // ahh whatever who cares
                        if let Some(file_path) = file_path {
                            let file_path = file_path.to_str();
                            if let Some(file_path) = file_path {
                                self.add_tab(Tab::from(CodeEditor::new(&file_path, self.viewer.num_tabs)));
                            }
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    ui.menu_button("Run Script", |ui| {
                        if let Ok(paths) = std::fs::read_dir("./scripts/") {
                            for path in paths {
                                if let Ok(path) = path {
                                    let path = path.path().display().to_string();
                                    if path.ends_with("lua") {
                                        if ui.button(&path).clicked() {
                                            self.add_tab(Tab::from(CodeEditor::new(&path, self.viewer.num_tabs)));
                                            self.viewer.run_script(&path);
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
                ui.menu_button("Options", |ui| {
                    ui.menu_button(
                        format!("Language ({})", LANGUAGE_OPTIONS[self.viewer.default_language as usize].0),
                        |ui| {
                            for option in LANGUAGE_OPTIONS.iter().filter(|x| INGAME_LANGUAGES.contains(&x.1)) {
                                ui.selectable_value(
                                    &mut self.viewer.default_language,
                                    option.1,
                                    option.0,
                                );
                            }
                        },
                    );
                });
                if ui.button("Reload").clicked() {
                    println!("[INFO] Requested Full Reload");
                    self.viewer.reload = true;
                }
            });
        });

        //let style = ctx.style();
        CentralPanel::default()
            //.frame(egui::Frame::central_panel(style)).inner_margin(0.))
            .show(ctx, |ui| {
                DockArea::new(&mut self.tree)
                    .show_close_buttons(true)
                    .tab_context_menus(true)
                    .draggable_tabs(true)
                    .show_tab_name_on_hover(true)
                    .show_leaf_close_all_buttons(true)
                    .show_secondary_button_hint(true)
                    .secondary_button_context_menu(true)
                    .secondary_button_on_modifier(true)
                    .show_inside(ui, &mut self.viewer);
                });
    }
}

const INGAME_LANGUAGES: [ContentLanguage; 15] = [
    ContentLanguage::Japanese,
    ContentLanguage::English,
    ContentLanguage::French,
    ContentLanguage::German,
    ContentLanguage::Italian,
    ContentLanguage::Spanish,
    ContentLanguage::Russian,
    ContentLanguage::Polish,
    ContentLanguage::PortugueseBr,
    ContentLanguage::Korean,
    ContentLanguage::TransitionalChinese,
    ContentLanguage::SimplelifiedChinese,
    ContentLanguage::Arabic,
    ContentLanguage::Thai,
    ContentLanguage::LatinAmericanSpanish,
];

const LANGUAGE_OPTIONS: [(&'static str, ContentLanguage); 34] = [
    ("Japanese", ContentLanguage::Japanese),
    ("English", ContentLanguage::English),
    ("French", ContentLanguage::French),
    ("Italian", ContentLanguage::Italian),
    ("German", ContentLanguage::German),
    ("Spanish", ContentLanguage::Spanish),
    ("Russian", ContentLanguage::Russian),
    ("Polish", ContentLanguage::Polish),
    ("Dutch", ContentLanguage::Dutch),
    ("Portuguese", ContentLanguage::Portuguese),
    ("Portuguese (Brazil)", ContentLanguage::PortugueseBr),
    ("Korean", ContentLanguage::Korean),
    ("Traditional Chinese", ContentLanguage::TransitionalChinese),
    ("Simplified Chinese", ContentLanguage::SimplelifiedChinese),
    ("Finnish", ContentLanguage::Finnish),
    ("Swedish", ContentLanguage::Swedish),
    ("Danish", ContentLanguage::Danish),
    ("Norwegian", ContentLanguage::Norwegian),
    ("Czech", ContentLanguage::Czech),
    ("Hungarian", ContentLanguage::Hungarian),
    ("Slovak", ContentLanguage::Slovak),
    ("Arabic", ContentLanguage::Arabic),
    ("Turkish", ContentLanguage::Turkish),
    ("Bulgarian", ContentLanguage::Bulgarian),
    ("Greek", ContentLanguage::Greek),
    ("Romanian", ContentLanguage::Romanian),
    ("Thai", ContentLanguage::Thai),
    ("Ukrainian", ContentLanguage::Ukrainian),
    ("Vietnamese", ContentLanguage::Vietnamese),
    ("Indonesian", ContentLanguage::Indonesian),
    ("Fiction", ContentLanguage::Fiction),
    ("Hindi", ContentLanguage::Hindi),
    (
        "Spanish (Latin America)",
        ContentLanguage::LatinAmericanSpanish,
    ),
    ("Unknown", ContentLanguage::Unknown),
    ];
