use clap::Parser;
use eframe::egui;
use ree_save_editor::{app::App, config::{Config, load_config}, configure_fonts};

#[derive(Parser, Debug)]
#[command(name = "ree-save-editor")]
#[command(version, about, long_about = None)]
struct GuiArgs {
    #[arg(short, long, default_value_t = String::from("./config.json"))]
    config: String,
    #[arg(long)]
    id: Option<u64>,
    #[cfg(target_os = "linux")]
    #[arg(long, default_value_t = shellexpand::full("~/.local/share/Steam/").unwrap_or_default().to_string())]
    steam_path: String,
    #[cfg(target_os = "windows")]
    #[arg(long, default_value_t = String::from("C:\\Program Files (x86)\\Steam"))]
    steam_path: String,
}


pub fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,egui=info")
    ).init();

    let args = GuiArgs::parse();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
        .. Default::default()
    };

    let mut config: Config = load_config(&args.config);

    config.id = args.id;
    config.steam_path = args.steam_path;

    eframe::run_native("ree-save-editor",
        options,
        Box::new(|_cc| {
            configure_fonts(&_cc.egui_ctx);
            egui_extras::install_image_loaders(&_cc.egui_ctx);
            //Ok(Box::new(TameApp::new(config, _cc)))
            Ok(Box::new(App::new(args.config, config)))
        }),
    )
}

