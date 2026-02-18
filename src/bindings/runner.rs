use std::{
    collections::HashMap,
    fs::File,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use eframe::egui::Window;
use mlua::{Lua, Thread};

use crate::{
    bindings::{SaveDataRef}, file::StructRW, game_context::GameCtx, save::{SaveContext, SaveFile, corrupt::CorruptSaveReader, game::Game}, sdk::type_map::{self, TypeMap}
};

#[derive(Debug)]
pub struct PendingDialog<T> {
    pub title: String,
    pub is_open: bool,
    pub value: T,
}
impl<T: Default> PendingDialog<T> {
    pub fn new() -> Self {
        Self {
            title: String::default(),
            is_open: false,
            value: T::default(),
        }
    }
}

#[derive(Debug)]
pub struct ScriptRunner {
    pub lua: Lua,
    pub thread: Option<Thread>,
    scripts: HashMap<String, String>,
    ask_integer_dialog: Arc<RwLock<PendingDialog<u64>>>,
    ask_string_dialog: Arc<RwLock<PendingDialog<String>>>,
}

impl ScriptRunner {
    pub fn new() -> Self {
        let lua = Lua::new();

        match Self::register_fs_functions(&lua) {
            Err(e) => eprintln!("[ERROR] Failed to register fs table {e}"),
            _ => (),
        }

        match Self::register_save_file_functions(&lua) {
            Err(e) => eprintln!("[ERROR] Failed to register SaveFile table {e}"),
            _ => (),
        }

        match Self::register_game_table(&lua) {
            Err(e) => eprintln!("[ERROR] Failed to register game table {e}"),
            _ => (),
        }

        let runner = Self {
            lua,
            scripts: HashMap::new(),
            thread: None,
            ask_integer_dialog: Arc::new(RwLock::new(PendingDialog::new())),
            ask_string_dialog: Arc::new(RwLock::new(PendingDialog::new())),
        };

        match runner.register_ui_functions() {
            Err(e) => eprintln!("[ERROR] Failed to register ui table {e}"),
            _ => (),
        }

        runner
    }

    pub fn update_dialogs(&mut self, ctx: &eframe::egui::Context) {
        if let Some(thread) = &self.thread {
            match thread.status() {
                mlua::ThreadStatus::Running | mlua::ThreadStatus::Resumable => {}
                mlua::ThreadStatus::Finished => {
                    println!("[LUA INFO] Script completed");
                    self.thread = None;
                }
                mlua::ThreadStatus::Error => {
                    eprintln!("[LUA ERROR] Thread Error idk what happened oops");
                    self.thread = None;
                }
            }
        }
        let mut d = self.ask_integer_dialog.write().unwrap();
        if d.is_open {
            Window::new(&d.title).show(ctx, |ui| {
                let mut buffer = d.value.to_string();
                if ui.text_edit_singleline(&mut buffer).changed() {
                    if let Ok(value) = buffer.parse::<u64>() {
                        d.value = value;
                    }
                }
                if ui.button("Confirm").clicked() {
                    d.is_open = false;

                    if let Some(thread) = self.thread.take() {
                        match thread.resume::<()>(d.value) {
                            Ok(_) => {
                                if thread.status() == mlua::ThreadStatus::Resumable {
                                    self.thread = Some(thread);
                                }
                            }
                            Err(e) => {
                                eprintln!("[LUA ERROR] Script crashed after integer input: {:?}", e)
                            }
                        }
                    }
                }
            });
        }

        let mut d = self.ask_string_dialog.write().unwrap();
        if d.is_open {
            Window::new(&d.title).show(ctx, |ui| {
                let mut buffer = d.value.to_string();
                if ui.text_edit_singleline(&mut buffer).changed() {
                    d.value = buffer;
                }

                if ui.button("Confirm").clicked() {
                    d.is_open = false;

                    if let Some(thread) = self.thread.take() {
                        match thread.resume::<()>(d.value.clone()) {
                            Ok(_) => {
                                if thread.status() == mlua::ThreadStatus::Resumable {
                                    self.thread = Some(thread);
                                }
                            }
                            Err(e) => {
                                eprintln!("[LUA ERROR] Script crashed after string input: {:?}", e)
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn init_globals(self) -> mlua::Result<Self> {
        Self::register_fs_functions(&self.lua)?;
        Self::register_save_file_functions(&self.lua)?;
        self.register_ui_functions()?;
        Ok(self)
    }

    pub fn set_save_file_context(
        &mut self,
        save_file: &SaveFile,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let save_ref = SaveDataRef {
            root: Arc::new(RwLock::new(save_file.clone())),
            path: vec![],
        };
        self.lua.globals().set("savefile", save_ref)?;
        Ok(())
    }

    pub fn get_save_file_context(&mut self) -> Option<SaveFile> {
        let ud: mlua::AnyUserData = self.lua.globals().get("savefile").ok()?;
        if let Ok(save) = ud.borrow::<SaveDataRef>() {
            let binding = save.root.read().unwrap();
            return Some(binding.clone());
        }
        eprintln!("[ERROR] Could not get save context from lua userdata");
        None
    }

    pub fn register_game_table(lua: &Lua) -> mlua::Result<()> {
        let game_table = lua.create_table()?;
        game_table.set("MHWILDS", "MHWILDS")?;
        game_table.set("DD2", "DD2")?;
        game_table.set("PRAGMATA", "PRAGMATA")?;
        game_table.set("MHST3", "MHST3")?;
        lua.globals().set("game", game_table)?;
        println!("[INFO] register game table");
        Ok(())
    }

    pub fn set_typemap(&mut self, type_map: TypeMap) {
        self.lua.set_app_data(type_map);
    }

    pub fn register_save_file_functions(lua: &Lua) -> mlua::Result<()> {
        let scan_classes =
            lua.create_function(move |lua, (path, steamid, game): (String, u64, String)| {
                println!("[INFO] Loading Save File {path}");
                let path = shellexpand::full(&path).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to load save file {e}"))
                })?;
                let game = Game::from_string(&game).unwrap_or_else(|| {
                    eprintln!("[LUA ERROR] Unknown Game {game}, defaulting to MHWILDS");
                    Game::MHWILDS
                });

                let game_contexts = lua.app_data_ref::<Arc<RwLock<HashMap<Game, GameCtx>>>>().ok_or_else(|| {
                    mlua::Error::RuntimeError("TypeMap not loaded in Lua context".into())
                })?;
                let binding = game_contexts.read().unwrap();
                let type_map = &binding.get(&game).ok_or_else(||{
                    mlua::Error::RuntimeError(format!("Game Context not loaded for game {game:?}"))
                })?.type_map;

                let mut reader = File::open(path.as_ref())?;
                let data = SaveFile::read_data(
                    &mut reader,
                    SaveContext {
                        key: steamid,
                        game: game,
                        repair: true,
                    },
                )
                .map_err(|e| {
                    mlua::Error::RuntimeError(format!("Could not Read Save Data Bytes {e}"))
                })?;

                let mut reader = Cursor::new(data);
                let mut corrupted_reader = CorruptSaveReader::new(&type_map, game);
                let save_file = corrupted_reader.read_with_scan(&mut reader);
                let save_ref = SaveDataRef {
                    root: Arc::new(RwLock::new(save_file)),
                    path: vec![],
                };
                println!("[INFO] Loaded Save File");
                Ok(save_ref)
            })?;

        let scan_missing =
            lua.create_function(move |lua, (path, steamid, game): (String, u64, String)| {
                println!("[INFO] Loading Save File {path}");
                let path = shellexpand::full(&path).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to load save file {e}"))
                })?;
                let game = Game::from_string(&game).unwrap_or_else(|| {
                    eprintln!("[LUA ERROR] Unknown Game {game}, defaulting to MHWILDS");
                    Game::MHWILDS
                });

                let game_contexts = lua.app_data_ref::<Arc<RwLock<HashMap<Game, GameCtx>>>>().ok_or_else(|| {
                    mlua::Error::RuntimeError("TypeMap not loaded in Lua context".into())
                })?;
                let binding = game_contexts.read().unwrap();
                let type_map = &binding.get(&game).ok_or_else(||{
                    mlua::Error::RuntimeError(format!("Game Context not loaded for game {game:?}"))
                })?.type_map;

                let mut reader = File::open(path.as_ref())?;
                let data = SaveFile::read_data(
                    &mut reader,
                    SaveContext {
                        key: steamid,
                        game: game,
                        repair: true,
                    },
                )
                .map_err(|e| {
                    mlua::Error::RuntimeError(format!("Could not Read Save Data Bytes {e}"))
                })?;

                let mut reader = Cursor::new(data);
                let mut corrupted_reader = CorruptSaveReader::new(&*type_map, game);
                let save_file = corrupted_reader.read_missing(&mut reader);
                let save_ref = SaveDataRef {
                    root: Arc::new(RwLock::new(save_file)),
                    path: vec![],
                };
                println!("[INFO] Loaded Save File");
                Ok(save_ref)
            })?;

        let save_file_table = lua.create_table()?;
        save_file_table.set("scan_classes", scan_classes)?;
        save_file_table.set("scan_missing", scan_missing)?;
        lua.globals().set("SaveFile", save_file_table)?;
        println!("[INFO] Registered SaveFile Table");
        Ok(())
    }

    pub fn register_fs_functions(lua: &Lua) -> mlua::Result<()> {
        let load_save =
            lua.create_function(move |_, (path, steamid, game): (String, u64, String)| {
                println!("[INFO] Loading Save File {path}");
                let path = shellexpand::full(&path).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to load save file {e}"))
                })?;
                let game = Game::from_string(&game).unwrap_or_else(|| {
                    eprintln!("[LUA ERROR] Unknown Game {game}, defaulting to MHWILDS");
                    Game::MHWILDS
                });
                let mut reader = File::open(path.as_ref())?;
                let save_file = match SaveFile::read(
                    &mut reader,
                    &mut SaveContext {
                        key: steamid,
                        game: game,
                        repair: true,
                    },
                ) {
                    Ok(s) => Ok(s),
                    Err(e) => Err(mlua::Error::RuntimeError(format!(
                        "Failed to load save file {e}"
                    ))),
                }?;
                let save_ref = SaveDataRef {
                    root: Arc::new(RwLock::new(save_file)),
                    path: vec![],
                };
                println!("[INFO] Loaded Save File");
                Ok(save_ref)
            })?;


        let fs_table = lua.create_table()?;
        fs_table.set("load_save", load_save)?;
        lua.globals().set("fs", fs_table)?;
        println!("[INFO] Registered fs Table");
        Ok(())
    }

    pub fn register_ui_functions(&self) -> mlua::Result<()> {
        let dialog = self.ask_string_dialog.clone();
        let ask_string = self.lua.create_async_function(move |lua, title: String| {
            let dialog = dialog.clone();
            async move {
                {
                    let mut d = dialog.write().unwrap();
                    d.title = title;
                    d.is_open = true;
                }
                let res = lua.yield_with::<String>(mlua::MultiValue::new()).await?;
                Ok(res)
            }
        })?;
        let dialog = self.ask_integer_dialog.clone();
        let ask_integer = self.lua.create_async_function(move |lua, title: String| {
            let dialog = dialog.clone();
            async move {
                {
                    let mut d = dialog.write().unwrap();
                    d.title = title;
                    d.is_open = true;
                }
                let res = lua.yield_with::<u64>(mlua::MultiValue::new()).await?;
                Ok(res)
            }
        })?;
        let open_file = self.lua.create_function(move |_, title: String| {
            let (tx, rx) = std::sync::mpsc::channel();

            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
                let path = rfd::FileDialog::new()
                    .set_title(&title)
                    .set_directory(cur)
                    .pick_file();

                if let Some(path) = path {
                    let _ = tx.send(path.display().to_string());
                }
            });
            loop {
                if let Ok(result) = rx.try_recv() {
                    return Ok(result);
                }
            }
        })?;

        let save_file = self.lua.create_function(move |_, title: String| {
            let (tx, rx) = std::sync::mpsc::channel();

            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
                let path = rfd::FileDialog::new()
                    .set_title(&title)
                    .set_directory(cur)
                    .save_file();

                if let Some(path) = path {
                    let _ = tx.send(path.display().to_string());
                }
            });
            loop {
                if let Ok(result) = rx.try_recv() {
                    return Ok(result);
                }
            }
        })?;

        let open_folder = self.lua.create_function(move |_, title: String| {
            let (tx, rx) = std::sync::mpsc::channel();

            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let cur = std::env::current_dir().unwrap_or(PathBuf::from("~"));
                let path = rfd::FileDialog::new()
                    .set_directory(cur)
                    .set_title(&title)
                    .pick_folder();

                if let Some(path) = path {
                    let _ = tx.send(path.display().to_string());
                }
            });
            loop {
                if let Ok(result) = rx.try_recv() {
                    return Ok(result);
                }
            }
        })?;

        let ui_table = self.lua.create_table()?;
        ui_table.set("open_file", open_file)?;
        ui_table.set("save_file", save_file)?;
        ui_table.set("open_folder", open_folder)?;
        ui_table.set("ask_integer", ask_integer)?;
        ui_table.set("ask_string", ask_string)?;
        self.lua.globals().set("ui", ui_table)?;
        println!("[INFO] Registered ui Table");
        Ok(())
    }

    pub fn load_script_from_file(&mut self, file_path: &str) -> std::io::Result<()> {
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script);
        Ok(())
    }

    pub fn load_and_execute_from_file(
        &mut self,
        file_path: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script.clone());
        let func = self.lua.load(&script).into_function()?;
        let thread: Thread = self.lua.create_thread(func)?;

        match thread.resume::<()>(()) {
            Ok(_) => {
                println!("Script finished or yielded successfully");
            }
            Err(e) => {
                eprintln!("Script Error: {:?}", e);
                return Err(Box::new(e));
            }
        }

        if thread.status() == mlua::ThreadStatus::Resumable {
            println!("[INFO] Script paused (yielded), waiting for UI...");
            self.thread = Some(thread);
        } else {
            self.thread = None;
        }
        Ok(())
    }
}
