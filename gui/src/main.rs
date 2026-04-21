#[cfg(not(target_arch = "wasm32" ))]
mod native {
    use clap::Parser;
    use eframe::egui;
    use ree_save_editor::{Config, app::TameApp, configure_fonts};

    #[derive(Parser, Debug)]
    #[command(name = "ree-save-editor")]
    #[command(version, about, long_about = None)]
    struct GuiArgs {
        #[arg(short('f'), long)]
        file_name: Option<String>,

        #[arg(short('o'), long, default_value_t = String::from("outputs"))]
        out_dir: String,

        #[arg(long)]
        steamid: Option<String>,

        #[arg(long)]
        rsz_path: Option<String>,
        #[arg(long)]
        enums_path: Option<String>,
        #[arg(long)]
        msgs_path: Option<String>,
        #[arg(long)]
        mappings_path: Option<String>,
        #[arg(long)]
        remap_path: Option<String>,

        #[cfg(target_os = "linux")]
        #[arg(long, default_value_t = shellexpand::full("~/.local/share/Steam/").unwrap_or_default().to_string())]
        steam_path: String,
        #[cfg(target_os = "windows")]
        #[arg(long, default_value_t = String::from("C:\\Program Files (x86)\\Steam"))]
        steam_path: String,
    }


    pub fn main() -> eframe::Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("info")
        ).init();
        let args = GuiArgs::parse();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
            .. Default::default()
        };

        let config = Config { 
            file_name: args.file_name,
            out_dir: args.out_dir,
            steamid: args.steamid,
            rsz_path: args.rsz_path,
            enums_path: args.enums_path,
            msgs_path: args.msgs_path,
            mappings_path: args.mappings_path,
            steam_path: args.steam_path,
            remap_path: args.remap_path,
        };

        eframe::run_native("ree-save-editor",
            options,
            Box::new(|_cc| {
                configure_fonts(&_cc.egui_ctx);
                egui_extras::install_image_loaders(&_cc.egui_ctx);
                //Ok(Box::new(TameApp::new(config, _cc)))
                Ok(Box::new(TameApp::new(config)))
            }),
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    native::main()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("This binary cannot be run on WASM. Use the library entry point.");
}
