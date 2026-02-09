use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{Chunk, Lua};

use crate::{bindings::{DataRef, DataRoot}, save::SaveFile};

pub struct ScriptRunner {
    lua_state: Lua,
    scripts: HashMap<String, String>
}

impl ScriptRunner {
    pub fn new() -> Self {
        Self {
            lua_state: Lua::new(),
            scripts: HashMap::new()
        }
    }

    pub fn set_save_file_context(&mut self, save_file: &SaveFile) -> std::result::Result<(), Box<dyn std::error::Error>>{
        let data: Rc<RefCell<DataRoot>> = Rc::new(DataRoot::Class(save_file.fields[0].1.clone()).into());
        let root_ref = DataRef { root: data, path: vec![] };
        self.lua_state.globals().set("savefile", root_ref)?;
        Ok(())
    }

    pub fn get_save_file_context(&mut self) -> Option<SaveFile>{
        //let save_file = self.lua_state.globals().get::<DataRef>("savefile");
        None
    }

    pub fn load_script_from_file(&mut self, file_path: &str) -> std::io::Result<()>{
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script);
        Ok(())
    }
    
    pub fn load_and_execute_from_file(&mut self, file_path: &str) -> std::result::Result<(), Box<dyn std::error::Error>>{
        let script = std::fs::read_to_string(file_path)?;
        self.scripts.insert(file_path.to_string(), script.clone());
        self.lua_state.load(script).exec()?;
        Ok(())
    }

    pub fn execute_script(&mut self, name: &str) -> std::result::Result<(), Box<dyn std::error::Error>>{
        let res = self.scripts.get(name).map(|x| {
            self.lua_state.load(x).exec()
        }).unwrap_or(Ok(()));
        Ok(res?)
    }

}
