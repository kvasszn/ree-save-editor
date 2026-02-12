use std::{collections::HashMap, fs::File, path::PathBuf, sync::{Arc, RwLock}};

use eframe::egui::Window;
use mlua::{Lua, Thread};

use crate::{bindings::{SaveDataRef}, file::StructRW, save::{SaveContext, SaveFile, game::Game}};

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
            _ => ()
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
            _ => ()
        }

        runner
    }

    pub fn update_dialogs(&mut self, ctx: &eframe::egui::Context) {

        if let Some(thread) = &self.thread {
            match thread.status() {
                mlua::ThreadStatus::Running | mlua::ThreadStatus::Resumable => { },
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
                        let _ = thread.resume::<u64>(d.value); 
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
                        let _ = thread.resume::<String>(d.value.clone()); 
                    }
                }
            });
        }
    }

    pub fn init_globals(self) -> mlua::Result<Self> {
        Self::register_fs_functions(&self.lua)?;
        self.register_ui_functions()?;
        Ok(self)
    }

    pub fn set_save_file_context(&mut self, save_file: &SaveFile) -> std::result::Result<(), Box<dyn std::error::Error>>{
        let save_ref = SaveDataRef {
            root: Arc::new(RwLock::new(save_file.clone())),
            path: vec![]
        };
        self.lua.globals().set("savefile", save_ref)?;
        Ok(())
    }

    pub fn get_save_file_context(&mut self) -> Option<SaveFile>{
        let ud: mlua::AnyUserData = self.lua.globals().get("savefile").ok()?;
        if let Ok(save) = ud.borrow::<SaveDataRef>() {
            let binding = save.root.read().unwrap();
            return Some(binding.clone());
        }
        eprintln!("[ERROR] Could not get save context from lua userdata");
        None
    }

    pub fn register_fs_functions(lua: &Lua) -> mlua::Result<()> {
        let load_save = lua.create_function(move |_, (path, steamid): (String, u64)| {
            println!("[INFO] Loading Save File {path}");
            let path = shellexpand::full(&path).map_err(|e| mlua::Error::RuntimeError(format!("Failed to load save file {e}")))?;
            let mut reader = File::open(path.as_ref())?;
            let save_file = match SaveFile::read(&mut reader, &mut SaveContext{key: steamid, game: Game::MHWILDS, repair: true}){
                Ok(s) => Ok(s),
                Err(e) => Err(mlua::Error::RuntimeError(format!("Failed to load save file {e}")))
            }?;
            let save_ref = SaveDataRef {
                root: Arc::new(RwLock::new(save_file)),
                path: vec![]
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

    pub fn register_ui_functions(&self) -> mlua::Result<()>{
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
                let path = rfd::FileDialog::new()
                    .set_title(&title)
                    .pick_file();

                if let Some(path) = path {
                    let _ = tx.send(path.display().to_string());
                }
            });
            loop {
                if let Ok(result) = rx.try_recv() {
                    return Ok(result)
                }
            }
        })?;

        let open_folder = self.lua.create_function(move |_, title: String| {
            let (tx, rx) = std::sync::mpsc::channel();

            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let path = rfd::FileDialog::new()
                    .set_title(&title)
                    .pick_folder();

                if let Some(path) = path {
                    let _ = tx.send(path.display().to_string());
                }
            });
            loop {
                if let Ok(result) = rx.try_recv() {
                    return Ok(result)
                }
            }
        })?;

        let ui_table = self.lua.create_table()?;
        ui_table.set("open_file", open_file)?;
        ui_table.set("open_folder", open_folder)?;
        ui_table.set("ask_integer", ask_integer)?;
        ui_table.set("ask_string", ask_string)?;
        self.lua.globals().set("ui", ui_table)?;
        println!("[INFO] Registered ui Table");
        Ok(())
    }

    pub fn load_script_from_file(&mut self, file_path: &str) -> std::io::Result<()>{
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script);
        Ok(())
    }

    pub fn load_and_execute_from_file(&mut self, file_path: &str) -> std::result::Result<(), Box<dyn std::error::Error>>{
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script.clone());
        let func = self.lua.load(&script).into_function()?;
        let thread: Thread = self.lua.create_thread(func)?;
        match thread.resume::<()>(()) {
            Ok(_) => {
                println!("Script finished or yielded successfully");
            },
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
