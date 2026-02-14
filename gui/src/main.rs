#[cfg(not(target_arch = "wasm32" ))]
mod native {
    use clap::Parser;
    use eframe::egui;
    use mhtame::rsz::dump::{ENUM_FILE, RSZ_FILE};
    use mhtame_gui::{Config, app::TameApp};

    #[derive(Parser, Debug)]
    #[command(name = "mhtame-gui")]
    #[command(version, about, long_about = None)]
    struct GuiArgs {
        #[arg(short('f'), long)]
        file_name: Option<String>,

        #[arg(short('o'), long, default_value_t = String::from("outputs"))]
        out_dir: String,

        #[arg(long)]
        steamid: Option<String>,

        #[arg(long, default_value_t = String::from("assets/rszmhwilds_packed.json"))]
        rsz_path: String,

        #[arg(long, default_value_t = String::from("assets/enumsmhwilds.json"))]
        enums_path: String,
        
        #[arg(long, default_value_t = String::from("assets/combined_msgs.json"))]
        msgs_path: String,

        #[arg(long, default_value_t = String::from("assets/enum_text_mappings.json"))]
        mappings_path: String,
        #[arg(long, default_value_t = String::from("assets/wilds_remap.json"))]
        remap_path: String,
        #[cfg(target_os = "linux")]
        #[arg(long, default_value_t = shellexpand::full("~/.local/share/Steam/").unwrap_or_default().to_string())]
        steam_path: String,
        #[cfg(target_os = "windows")]
        #[arg(long, default_value_t = String::from("C:\\Program Files (x86)\\Steam"))]
        steam_path: String,
    }


    pub fn main() -> eframe::Result<()> {
        env_logger::init();
        let args = GuiArgs::parse();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_drag_and_drop(true),
            .. Default::default()
        };

        ENUM_FILE.set(args.enums_path.clone()).unwrap();
        RSZ_FILE.set(args.rsz_path.clone()).unwrap();

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

        eframe::run_native("mhtame",
            options,
            Box::new(|_cc| {
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

// This dummy main is required because Cargo still checks the binary target 
// even when compiling for WASM. This prevents the "missing main" error.
#[cfg(target_arch = "wasm32")]
fn main() {
    // This function never actually runs in the browser. 
    // The browser uses the 'start' function in lib.rs instead.
    panic!("This binary cannot be run on WASM. Use the library entry point.");
}
