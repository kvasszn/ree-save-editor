use std::{cell::RefCell, rc::Rc};

use eframe::{self, egui::{Align, CentralPanel, DragValue, Layout, MenuBar, TopBottomPanel, Window}};
use egui_dock::{DockArea, DockState};
use mhtame::{bindings::runner::ScriptRunner, sdk::type_map::ContentLanguage};

use crate::{Config, file::FilePicker, file::FileView, viewer::Viewer};

pub struct TameApp {
    viewer: Viewer,
    tree: DockState<FileView>,
    file_opener: FilePicker<false>,
}

impl TameApp {
    pub fn new(config: Config) -> Self {
        let file_view = FileView::new(&config, 0, ContentLanguage::English);
        let mut viewer = Viewer::new(config);
        let dock_state = DockState::new(vec![file_view]);
        viewer.num_tabs += 1;
        Self {
            viewer,
            tree: dock_state,
            file_opener: FilePicker::new("Open"),
        }
    }
    fn add_tab(&mut self, file_view: FileView) {
        println!("adding {}", file_view.idx);
        if self.viewer.num_tabs == 0 {
            self.viewer.num_tabs += 1;
            self.tree = DockState::new(vec![file_view])
        } else {
            self.viewer.num_tabs += 1;
            let surface = self.tree.main_surface_mut();
            surface.push_to_focused_leaf(file_view);
        }
    }
}

impl eframe::App for TameApp {

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if self.viewer.reload {
            //let _ = self.viewer.register_ui_functions();
            //let _ = self.viewer.script_runner.register_io_functions();
            self.viewer.reload = false;
        }
        if let Some(path) = self.file_opener.take() {
            let file_view = FileView::from_path(&self.viewer.config, path, self.viewer.num_tabs, self.viewer.default_language);
            self.add_tab(file_view);
        }

        self.viewer.update(ctx);

        TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("MH Wilds Save Editor");

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/kvasszn/mhtame");
                    ui.separator();
                    ui.label("v0.1.5");
                });
            });

            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.file_opener.spawn_dialog();
                    }
                    if ui.button("Empty File").clicked() {
                        let file_view = FileView::new(&self.viewer.config, self.viewer.num_tabs, self.viewer.default_language);
                        self.add_tab(file_view);
                    }
                    if ui.button("Run Script").clicked() {
                        self.viewer.run_script("scripts/load_saves.lua");

                    }
                });
                ui.menu_button(LANGUAGE_OPTIONS[self.viewer.default_language as usize].0, |ui| {
                    for option in LANGUAGE_OPTIONS {
                        ui.selectable_value(&mut self.viewer.default_language, option.1, option.0);
                    }
                });
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
    ("Spanish (Latin America)", ContentLanguage::LatinAmericanSpanish),
    ("Unknown", ContentLanguage::Unknown),
    ];
