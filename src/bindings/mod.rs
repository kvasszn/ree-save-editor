pub mod runner;
pub mod save;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use mlua::prelude::*;

use crate::{
    bindings::save::set_fieldvalue_from_lua,
    game_context::GameCtx,
    save::{
        SaveFile, SaveOptions, game::Game, types::{Array, Class, Field, FieldValue}
    },
    sdk::type_map,
};

#[derive(Clone, Debug)]
pub enum RefPath {
    FieldName(String),
    FieldHash(u32),
    Index(usize),
}

#[derive(Clone, Debug)]
pub enum DataRoot {
    Class(Class),
    Array(Array),
}
impl LuaUserData for DataRoot {}

#[derive(Clone, Debug)]
pub struct DataRef {
    pub root: Arc<RwLock<DataRoot>>,
    pub path: Vec<RefPath>,
}

impl DataRef {
    #[allow(unused)]
    fn traverse<'b, 'a: 'b>(&'a self) -> Option<FieldValue> {
        println!("traversing: {:?}", self.path);
        match &*self.root.write().unwrap() {
            DataRoot::Class(c) => {
                let iter = &mut self.path.iter();
                if let RefPath::FieldName(first_segment) = iter.next()? {
                    let mut current = c.get_value(first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value(*i),
                                _ => None,
                            };
                        }
                    }
                    return current.cloned();
                } else {
                    return None;
                }
            }
            DataRoot::Array(c) => {
                let iter = &mut self.path.iter();
                if let RefPath::Index(first_segment) = iter.next()? {
                    let mut current = c.get_value(*first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value(*i),
                                _ => None,
                            };
                        }
                    }
                    return current.cloned();
                } else {
                    return None;
                }
            }
        }
    }

    fn with_target_mut<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&mut FieldValue) -> LuaResult<R>,
    {
        let iter = &mut self.path.iter();
        let binding = &mut *self.root.write().unwrap();
        let target = match binding {
            DataRoot::Class(c) => {
                if let RefPath::FieldName(first_segment) = iter
                    .next()
                    .ok_or(LuaError::RuntimeError("Empty Path".to_string()))?
                {
                    let mut current = c.get_value_mut(first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => {
                                    c.get_value_mut(&k)
                                }
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value_mut(*i),
                                _ => None,
                            };
                        }
                    }
                    current
                } else {
                    None
                }
            }
            DataRoot::Array(c) => {
                if let RefPath::Index(first_segment) = iter
                    .next()
                    .ok_or(LuaError::RuntimeError("Empty Path".to_string()))?
                {
                    let mut current = c.get_value_mut(*first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => {
                                    c.get_value_mut(&k)
                                }
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value_mut(*i),
                                _ => None,
                            };
                        }
                    }
                    current
                } else {
                    None
                }
            }
        };

        let cur_unwrapped = target.ok_or(LuaError::RuntimeError(
            "Could not evaluate path".to_string(),
        ))?;
        return func(cur_unwrapped);
    }
}
impl LuaUserData for DataRef {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            let index = match key {
                LuaValue::String(s) => Some(RefPath::FieldName(s.to_str()?.to_string())),
                LuaValue::Integer(i) => Some(RefPath::Index((i - 1) as usize)), // Lua 1-based to Rust 0-based
                _ => None,
            };

            if this.path.is_empty() {
                if let Some(ref index) = index {
                    let binding = &mut *this.root.write().unwrap();
                    let target = match (binding, &index) {
                        (DataRoot::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (DataRoot::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None,
                    };
                    if let Some(val) = target {
                        match val {
                            FieldValue::Class(_) | FieldValue::Array(_) | FieldValue::Struct(_) => {
                                let mut new_path = this.path.clone();
                                new_path.push(index.clone());
                                return Ok(DataRef {
                                    root: this.root.clone(),
                                    path: new_path,
                                }
                                .into_lua(lua)?);
                            }
                            _ => return val.clone().into_lua(lua),
                        }
                    }
                }
            }

            this.with_target_mut(|value| {
                if let Some(index) = index {
                    let target = match (value, &index) {
                        (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (FieldValue::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None,
                    };
                    if let Some(val) = target {
                        match val {
                            FieldValue::Class(_) | FieldValue::Array(_) | FieldValue::Struct(_) => {
                                let mut new_path = this.path.clone();
                                new_path.push(index);
                                return Ok(DataRef {
                                    root: this.root.clone(),
                                    path: new_path,
                                }
                                .into_lua(lua)?);
                            }
                            _ => return val.clone().into_lua(lua),
                        }
                    }
                }
                Ok(LuaValue::Nil)
            })
        });

        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |lua, this, (key, value): (LuaValue, LuaValue)| {
                let segment = match key {
                    LuaValue::String(s) => RefPath::FieldName(s.to_str()?.to_string()),
                    LuaValue::Integer(i) => RefPath::Index((i - 1) as usize),
                    _ => return Ok(()),
                };

                let mut self_cloned_val: Option<FieldValue> = None;

                if let LuaValue::UserData(ref ud) = value {
                    if let Ok(rhs_ref) = ud.borrow::<DataRef>() {
                        let root_read = this.root.read().unwrap();
                        if let Some(source_fv) =
                            save::get_target_from_root(&*root_read, &rhs_ref.path)
                        {
                            self_cloned_val = Some(source_fv.clone());
                        }
                    }
                }

                let mut root_write = this.root.write().unwrap();
                let mut cursor: Option<&mut FieldValue> = None;

                if !this.path.is_empty() {
                    match (&mut *root_write, &this.path[0]) {
                        (DataRoot::Class(c), RefPath::FieldName(k)) => cursor = c.get_value_mut(k),
                        (DataRoot::Array(a), RefPath::Index(i)) => cursor = a.values.get_mut(*i),
                        _ => {}
                    };

                    for p in this.path.iter().skip(1) {
                        if let Some(curr) = cursor {
                            cursor = match (curr, p) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                                _ => None,
                            }
                        }
                    }
                }

                let target_slot = if let Some(parent) = cursor {
                    match (parent, &segment) {
                        (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (FieldValue::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None,
                    }
                } else {
                    match (&mut *root_write, &segment) {
                        (DataRoot::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (DataRoot::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None,
                    }
                };

                if let Some(slot) = target_slot {
                    if let Some(val) = self_cloned_val {
                        match (slot, val) {
                            (FieldValue::Class(lhs), FieldValue::Class(rhs)) => *lhs = rhs,
                            (FieldValue::Array(lhs), FieldValue::Array(rhs)) => *lhs = rhs,
                            _ => {
                                return Err(LuaError::RuntimeError(
                                    "Type mismatch in self-copy".into(),
                                ));
                            }
                        }
                    } else {
                        save::set_fieldvalue_from_lua(lua, slot, value)?;
                    }
                }

                Ok(())
            },
        );
        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            let path_str = this
                .path
                .iter()
                .map(|p| match p {
                    RefPath::FieldName(k) => format!(".{}", k),
                    RefPath::FieldHash(k) => format!(".{}", k),
                    RefPath::Index(i) => format!("[{}]", i + 1),
                })
                .collect::<String>();
            lua.create_string(&format!("Ref(root{})", path_str))
        });
    }
}

#[derive(Clone, Debug)]
pub struct SaveDataRef {
    pub root: Arc<RwLock<SaveFile>>,
    pub path: Vec<RefPath>,
}

impl SaveDataRef {
    #[allow(unused)]
    fn get_value_with_path(&self, last_path: &RefPath) -> Option<FieldValue> {
        let mut target = self.get_value()?;
        target = match (target, last_path) {
            (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(&name).cloned(),
            (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index).cloned(),
            _ => None,
        }?;
        Some(target)
    }

    fn next_field<'a>(class: &'a Class, path: &'a RefPath) -> Option<&'a FieldValue> {
        let next = match path {
            RefPath::FieldName(name) => class.get_value(name),
            RefPath::Index(i) => class.get_index_value(*i),
            _ => None,
        };
        next
    }

    fn next_field_mut<'a>(class: &'a mut Class, path: &'a RefPath) -> Option<&'a mut FieldValue> {
        let next = match path {
            RefPath::FieldName(name) => class.get_value_mut(name),
            RefPath::Index(i) => class.get_index_value_mut(*i),
            _ => None,
        };
        next
    }

    fn next_value<'a>(value: &'a FieldValue, path: &'a RefPath) -> Option<&'a FieldValue> {
        let next = match (value, path) {
            (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(name),
            (FieldValue::Class(class), RefPath::Index(i)) => class.get_index_value(*i),
            (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
            _ => None,
        };
        next
    }

    fn next_value_mut<'a>(
        value: &'a mut FieldValue,
        path: &'a RefPath,
    ) -> Option<&'a mut FieldValue> {
        let next = match (value, path) {
            (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value_mut(name),
            (FieldValue::Class(class), RefPath::Index(i)) => class.get_index_value_mut(*i),
            (FieldValue::Array(array), RefPath::Index(index)) => array.get_value_mut(*index),
            _ => None,
        };
        next
    }

    fn get_value(&self) -> Option<FieldValue> {
        let mut path = self.path.iter();
        let initial_class_index = path.next()?;
        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| x.1.clone()),
            _ => None,
        }?;

        let Some(initial_index) = path.next() else {
            return Some(FieldValue::Class(Box::new(initial_class)));
        };
        let mut target = Self::next_field(&initial_class, initial_index)?;
        while let Some(path_ref) = path.next() {
            target = Self::next_value(target, path_ref)?;
        }
        Some(target.clone())
    }
    fn with_initial_class<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&Class) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on root ref"
        )))?;
        func(initial_class)
    }

    fn with_target_class<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&Class) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on root ref"
        )))?;
        let Some(initial_index) = path.next() else {
            return func(initial_class);
        };

        let mut target =
            Self::next_field(initial_class, initial_index).ok_or(LuaError::RuntimeError(
                format!("Could not find PathRef {initial_index:?} on initial_class"),
            ))?;
        while let Some(path_ref) = path.next() {
            target = Self::next_value(target, path_ref).ok_or(LuaError::RuntimeError(format!(
                "Could not find PathRef {path_ref:?} on root ref"
            )))?;
        }
        let target = target.as_class().ok_or(LuaError::RuntimeError(format!(
            "Final ref is not a class {:?}",
            self.path
        )))?;
        func(target)
    }

    fn with_target<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&FieldValue) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on 2nd root ref"
        )))?;

        let initial_index = path.next().ok_or(LuaError::RuntimeError(
            "Second path segment empty on savefile ref".to_string(),
        ))?;

        let mut target =
            Self::next_field(initial_class, initial_index).ok_or(LuaError::RuntimeError(
                format!("Could not find PathRef {initial_index:?} on root ref"),
            ))?;

        while let Some(path_ref) = path.next() {
            target = Self::next_value(target, path_ref).ok_or(LuaError::RuntimeError(format!(
                "Could not find PathRef {path_ref:?} on root ref"
            )))?;
        }
        func(target)
    }

    fn with_target_mut<F, R>(&mut self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&mut FieldValue) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let mut root = self.root.write().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get_mut(*i).map(|x| &mut x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on 2nd root ref"
        )))?;

        let initial_index = path.next().ok_or(LuaError::RuntimeError(
            "Second path segment empty on savefile ref".to_string(),
        ))?;

        let mut target =
            Self::next_field_mut(initial_class, initial_index).ok_or(LuaError::RuntimeError(
                format!("Could not find PathRef {initial_index:?} on root ref"),
            ))?;

        while let Some(path_ref) = path.next() {
            target = Self::next_value_mut(target, path_ref).ok_or(LuaError::RuntimeError(
                format!("Could not find PathRef {path_ref:?} on root ref"),
            ))?;
        }
        func(target)
    }

    fn get_parent_class_hash(&self) -> LuaResult<u32> {
        let mut path = self.path.iter().peekable();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on root ref"
        )))?;

        let Some(initial_index) = path.next() else {
            return Ok(initial_class.hash);
        };

        let mut target = match initial_index {
            RefPath::FieldName(name) => initial_class.get_value(name),
            RefPath::Index(i) => initial_class.get_index_value(*i),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find PathRef {initial_index:?} on 2nd root ref"
        )))?;

        let mut last_class = Some(initial_class);
        while let Some(path_ref) = path.next() {
            target = match (target, path_ref) {
                (FieldValue::Class(class), RefPath::FieldName(name)) => {
                    last_class = Some(&*class);
                    class.get_value(name)
                }
                (FieldValue::Class(class), RefPath::Index(i)) => {
                    last_class = Some(&*class);
                    class.get_index_value(*i)
                }
                (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
                _ => None,
            }
            .ok_or(LuaError::RuntimeError(format!(
                "Could not find PathRef {path_ref:?} on root ref"
            )))?;
        }
        match last_class {
            Some(class) => Ok(class.hash),
            _ => Err(LuaError::RuntimeError(format!(
                "No parent class in path {:?}",
                self.path
            ))),
        }
    }

    fn get_field_hash(&self) -> LuaResult<u32> {
        let mut path = self.path.iter();
        let initial_class_index = path.next().ok_or(LuaError::RuntimeError(
            "First path segment empty on savefile ref".to_string(),
        ))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find Index {initial_class_index:?} on root ref"
        )))?;

        let initial_index = path.next().ok_or(LuaError::RuntimeError(
            "Second path segment empty on savefile ref".to_string(),
        ))?;

        let mut last_field = None;
        let mut target = match initial_index {
            RefPath::FieldName(name) => {
                last_field = initial_class.get_field(name);
                initial_class.get_value(name)
            }
            RefPath::Index(i) => {
                last_field = initial_class.get_index(*i);
                initial_class.get_index_value(*i)
            }
            _ => None,
        }
        .ok_or(LuaError::RuntimeError(format!(
            "Could not find PathRef {initial_index:?} on 2nd root ref from {:08x}",
            initial_class.hash
        )))?;

        while let Some(path_ref) = path.next() {
            target = match (target, path_ref) {
                (FieldValue::Class(class), RefPath::FieldName(name)) => {
                    last_field = class.get_field(name);
                    class.get_value(name)
                }
                (FieldValue::Class(class), RefPath::Index(i)) => {
                    last_field = class.get_index(*i);
                    class.get_index_value(*i)
                }
                (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
                _ => None,
            }
            .ok_or(LuaError::RuntimeError(format!(
                "Could not find PathRef {path_ref:?} on root ref"
            )))?;
        }
        last_field
            .map(|f| f.hash)
            .ok_or(LuaError::RuntimeError(format!(
                "Could not find any fields in the path {:?}",
                self.path
            )))
    }
}

macro_rules! register_struct_accessors {
    ($methods:ident, $($name:ident, $type:ty, $size:expr),*) => {
        $(
            // Read Method
            $methods.add_method(concat!("read_", stringify!($name)), |_, this, offset: usize| {
                this.with_target(|target| {
                    if let FieldValue::Struct(s) = target {
                        let bytes = s.data.get(offset..offset + $size)
                            .ok_or_else(|| mlua::Error::RuntimeError(
                                    format!("Out of bounds on read_{} at offset {}", stringify!($name), offset)
                            ))?;
                        Ok(<$type>::from_le_bytes(bytes.try_into().unwrap()))
                    } else {
                        Err(mlua::Error::RuntimeError(format!("Path is not a Struct: {:?}", target)))
                    }
                })
            });

            // Write Method
            $methods.add_method_mut(concat!("write_", stringify!($name)), |_, this, (value, offset): ($type, usize)| {
                this.with_target_mut(|target| {
                    if let FieldValue::Struct(s) = target {
                        let bytes = s.data.get_mut(offset..offset + $size)
                            .ok_or_else(|| mlua::Error::RuntimeError(
                                    format!("Out of bounds on write_{} at offset {}", stringify!($name), offset)
                            ))?;
                        bytes.copy_from_slice(&value.to_le_bytes());
                        Ok(())
                    } else {
                        Err(mlua::Error::RuntimeError(format!("Path is not a Struct: {:?}", target)))
                    }
                })
            });
            )*
    };
}

impl LuaUserData for SaveDataRef {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        register_struct_accessors!(
            methods, u8, u8, 1, u16, u16, 2, u32, u32, 4, u64, u64, 8, i8, i8, 1, i16, i16, 2, i32,
            i32, 4, i64, i64, 8, f32, f32, 4, f64, f64, 8
        );
        methods.add_method("save", |_, this, (path, steamid): (String, u64)| {
            let path_og = path.clone();
            let path_expanded = shellexpand::full(&path).map_err(|e| {
                mlua::Error::RuntimeError(format!("Failed to shell expand path {e}"))
            })?;
            let path = PathBuf::from(path_expanded.as_ref());

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    mlua::Error::RuntimeError(format!("Failed to create directories: {e}"))
                })?;
            }

            let save_file = this.root.read().unwrap();
            let save_opts = SaveOptions::new(save_file.game)
                .id(steamid);
            save_file
                .save(&path, &save_opts)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to save file: {e}")))?;
            Ok(path_og)
        });

        methods.add_method("field_hash_indexed", |_, this, index: u64| {
            let mut new_path = this.path.clone();
            new_path.push(RefPath::Index(index as usize - 1));
            SaveDataRef {
                root: this.root.clone(),
                path: new_path,
            }
            .get_field_hash()
        });
        methods.add_method("field_hash", |_, this, name: String| {
            let mut new_path = this.path.clone();
            new_path.push(RefPath::FieldName(name));
            SaveDataRef {
                root: this.root.clone(),
                path: new_path,
            }
            .get_field_hash()
        });
        methods.add_method("field_name", |lua, this, (index, game): (u64, String)| {
            let mut new_path = this.path.clone();
            new_path.push(RefPath::Index(index as usize - 1));
            let field_hash = SaveDataRef {
                root: this.root.clone(),
                path: new_path,
            }
            .get_field_hash()?;
            let class_hash = this.get_parent_class_hash()?;
            let game = Game::from_string(&game).unwrap_or_else(|| {
                eprintln!("[LUA ERROR] Unknown Game {game}, defaulting to MHWILDS");
                Game::MHWILDS
            });
            let game_contexts = {
                let app_data_guard = lua
                    .app_data_ref::<Arc<RwLock<HashMap<Game, GameCtx>>>>()
                    .ok_or_else(|| {
                        mlua::Error::RuntimeError("TypeMap not loaded in Lua context".into())
                    })?;
                Arc::clone(&*app_data_guard)
            };
            let binding = game_contexts.read().unwrap();
            let type_map = &binding
                .get(&game)
                .ok_or_else(|| {
                    mlua::Error::RuntimeError(format!("Game Context not loaded for game {game:?}"))
                })?
                .type_map;
            let type_info = type_map
                .get_by_hash(class_hash)
                .ok_or(mlua::Error::RuntimeError(format!(
                    "Class does not exist in type map {class_hash:010x}"
                )))?;
            let field_info = type_info
                .get_by_hash(field_hash)
                .ok_or(mlua::Error::RuntimeError(format!(
                    "Class does contain field with hash {field_hash:010x}"
                )))?;
            Ok(field_info.name.clone())
        });
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            let index = match key {
                LuaValue::String(s) => Some(RefPath::FieldName(s.to_str()?.to_string())),
                LuaValue::Integer(i) => Some(RefPath::Index((i - 1) as usize)),
                _ => None,
            };

            if let Some(index) = index {
                if this.path.is_empty() {
                    let root = this.root.read().unwrap();
                    let initial_class = match index {
                        RefPath::Index(i) => root.fields.get(i),
                        _ => None,
                    };
                    if initial_class.is_some() {
                        let mut new_path = this.path.clone();
                        new_path.push(index.clone());
                        return Ok(SaveDataRef {
                            root: this.root.clone(),
                            path: new_path,
                        }
                        .into_lua(lua)?);
                    }
                }
                if this.path.len() == 1 {
                    return this.with_initial_class(|class| {
                        let target =
                            Self::next_field(class, &index).ok_or(LuaError::RuntimeError(
                                format!("Could not find next field {index:?} on root ref"),
                            ))?;
                        match target {
                            FieldValue::Class(_) | FieldValue::Array(_) | FieldValue::Struct(_) => {
                                let mut new_path = this.path.clone();
                                new_path.push(index.clone());
                                return Ok(SaveDataRef {
                                    root: this.root.clone(),
                                    path: new_path,
                                }
                                .into_lua(lua)?);
                            }
                            _ => {
                                let res = target.clone().into_lua(lua);
                                //println!("path={:?}, {target:?}, {res:?}", this.path);
                                return res;
                            }
                        }
                    });
                }

                return this.with_target(|target| {
                    let target = match (target, &index) {
                        (FieldValue::Class(class), RefPath::FieldName(name)) => {
                            class.get_value(name)
                        }
                        (FieldValue::Class(class), RefPath::Index(i)) => class.get_index_value(*i),
                        (FieldValue::Array(array), RefPath::Index(index)) => {
                            array.get_value(*index)
                        }
                        _ => None,
                    }
                    .ok_or(LuaError::RuntimeError(format!(
                        "Could not find PathRef {index:?} on root ref"
                    )))?;

                    match target {
                        FieldValue::Class(_) | FieldValue::Array(_) | FieldValue::Struct(_) => {
                            let mut new_path = this.path.clone();
                            new_path.push(index.clone());
                            return Ok(SaveDataRef {
                                root: this.root.clone(),
                                path: new_path,
                            }
                            .into_lua(lua)?);
                        }
                        _ => {
                            let res = target.clone().into_lua(lua);
                            //println!("path={:?}, {target:?}, {res:?}", this.path);
                            return res;
                        }
                    }
                });
            }
            return Ok(LuaValue::Nil);
        });

        methods.add_meta_method(LuaMetaMethod::Len, |_, this, ()| {
            if this.path.is_empty() {
                let root = this.root.read().unwrap();
                return Ok(root.fields.len());
            }
            if this.path.len() == 1 {
                let root = this.root.read().unwrap();
                let index = &this.path[0];
                let initial_class = match index {
                    RefPath::Index(i) => root.fields.get(*i),
                    _ => None,
                };
                if let Some(initial_class) = initial_class {
                    return Ok(initial_class.1.fields.len());
                } else {
                    return Err(LuaError::RuntimeError(format!(
                        "Could not find first class with path {:?}",
                        this.path
                    )));
                }
            }

            return this.with_target(|target| {
                if let FieldValue::Array(array) = target {
                    return Ok(array.values.len());
                } else if let FieldValue::Class(class) = target {
                    return Ok(class.fields.len());
                } else if let FieldValue::Struct(s) = target {
                    return Ok(s.data.len());
                };
                Err(LuaError::RuntimeError(format!(
                    "Could not find PathRef {:?} on root ref",
                    this.path
                )))
            });
        });

        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |lua, this, (key, value): (LuaValue, LuaValue)| {
                let index = match key {
                    LuaValue::String(s) => RefPath::FieldName(s.to_str()?.to_string()),
                    LuaValue::Integer(i) => RefPath::Index((i - 1) as usize),
                    _ => return Ok(()),
                };
                let mut new_path = this.path.clone();
                new_path.push(index);
                let mut new_this = SaveDataRef {
                    root: this.root.clone(),
                    path: new_path,
                };

                new_this.with_target_mut(|lhs| set_fieldvalue_from_lua(lua, lhs, value))?;
                Ok(())
            },
        );
        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            let path_str = this
                .path
                .iter()
                .map(|p| match p {
                    RefPath::FieldName(k) => format!(".{}", k),
                    RefPath::FieldHash(k) => format!(".{}", k),
                    RefPath::Index(i) => format!("[{}]", i + 1),
                })
                .collect::<String>();
            lua.create_string(&format!("Ref(root{})", path_str))
        });
    }
}
