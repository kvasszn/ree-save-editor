pub mod save;
pub mod runner;

use std::{cell::RefCell, fs::File, io::Write, path::PathBuf, rc::Rc, sync::{Arc, RwLock}};

use eframe::egui::TextBuffer;
use mlua::{MaybeSend, prelude::*};

use crate::{bindings::save::set_fieldvalue_from_lua, save::{SaveFile, types::{Array, Class, FieldValue}}, sdk::type_map::murmur3};

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
    fn traverse<'b, 'a: 'b> (&'a self) -> Option<FieldValue> {
        println!("traversing: {:?}", self.path);
        match &*self.root.write().unwrap() {
            DataRoot::Class(c) => {
                let iter = &mut self.path.iter();
                if let RefPath::FieldName(first_segment) = iter.next()? {
                    let mut current = c.get_value(first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current  {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value(*i),
                                _ => None
                            };
                        }
                    }
                    return current.cloned()
                } else {
                    return None
                }
            }
            DataRoot::Array(c) => {
                let iter = &mut self.path.iter();
                if let RefPath::Index(first_segment) = iter.next()? {
                    let mut current = c.get_value(*first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current  {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value(*i),
                                _ => None
                            };
                        }
                    }
                    return current.cloned()
                } else {
                    return None
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
                if let RefPath::FieldName(first_segment) = iter.next().ok_or(LuaError::RuntimeError("Empty Path".to_string()))? {
                    let mut current = c.get_value_mut(first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current  {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value_mut(*i),
                                _ => None
                            };
                        }
                    }
                    current
                } else {
                    None
                }
            }
            DataRoot::Array(c) => {
                if let RefPath::Index(first_segment) = iter.next().ok_or(LuaError::RuntimeError("Empty Path".to_string()))? {
                    let mut current = c.get_value_mut(*first_segment);
                    while let Some(path_segment) = iter.next() {
                        if let Some(cur) = current  {
                            current = match (cur, &path_segment) {
                                (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(&k),
                                (FieldValue::Array(a), RefPath::Index(i)) => a.get_value_mut(*i),
                                _ => None
                            };
                        }
                    }
                    current
                } else {
                    None
                }
            }
        };

        let cur_unwrapped = target.ok_or(LuaError::RuntimeError("Could not evaluate path".to_string()))?;
        return func(cur_unwrapped)
    }
}
impl LuaUserData for DataRef {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            let index = match key {
                LuaValue::String(s) => Some(RefPath::FieldName(s.to_str()?.to_string())),
                LuaValue::Integer(i) => Some(RefPath::Index((i - 1) as usize)), // Lua 1-based to Rust 0-based
                _ => None
            };

            if this.path.is_empty() {
                if let Some(ref index) = index {
                    let binding = &mut *this.root.write().unwrap();
                    let target = match (binding, &index) {
                        (DataRoot::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (DataRoot::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None
                    };
                    if let Some(val) = target {
                        match val {
                            FieldValue::Class(_) | FieldValue::Array(_) => {
                                let mut new_path = this.path.clone();
                                new_path.push(index.clone());
                                return Ok(DataRef { root: this.root.clone(), path: new_path }.into_lua(lua)?);
                            },
                            _ => return val.clone().into_lua(lua)
                        }
                    }
                }
            }

            this.with_target_mut(|value| {
                if let Some(index) = index {
                    let target = match (value, &index) {
                        (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                        (FieldValue::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                        _ => None
                    };
                    if let Some(val) = target {
                        match val {
                            FieldValue::Class(_) | FieldValue::Array(_) => {
                                let mut new_path = this.path.clone();
                                new_path.push(index);
                                return Ok(DataRef { root: this.root.clone(), path: new_path }.into_lua(lua)?);
                            },
                            _ => return val.clone().into_lua(lua)
                        }
                    }
                }
                Ok(LuaValue::Nil)
            })
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |lua, this, (key, value): (LuaValue, LuaValue)| {
            let segment = match key {
                LuaValue::String(s) => RefPath::FieldName(s.to_str()?.to_string()),
                LuaValue::Integer(i) => RefPath::Index((i - 1) as usize),
                _ => return Ok(()),
            };

            let mut self_cloned_val: Option<FieldValue> = None;

            if let LuaValue::UserData(ref ud) = value {
                if let Ok(rhs_ref) = ud.borrow::<DataRef>() {
                    let root_read = this.root.read().unwrap();
                    if let Some(source_fv) = save::get_target_from_root(&*root_read, &rhs_ref.path) {
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
                            _ => None
                        }
                    }
                }
            }

            let target_slot = if let Some(parent) = cursor {
                match (parent, &segment) {
                    (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                    (FieldValue::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                    _ => None
                }
            } else {
                match (&mut *root_write, &segment) {
                    (DataRoot::Class(c), RefPath::FieldName(k)) => c.get_value_mut(k),
                    (DataRoot::Array(a), RefPath::Index(i)) => a.values.get_mut(*i),
                    _ => None
                }
            };

            if let Some(slot) = target_slot {
                if let Some(val) = self_cloned_val {
                    match (slot, val) {
                        (FieldValue::Class(lhs), FieldValue::Class(rhs)) => *lhs = rhs,
                        (FieldValue::Array(lhs), FieldValue::Array(rhs)) => *lhs = rhs,
                        _ => return Err(LuaError::RuntimeError("Type mismatch in self-copy".into())),
                    }
                } else {
                    save::set_fieldvalue_from_lua(lua, slot, value)?;
                }
            }

            Ok(())
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            let path_str = this.path.iter().map(|p| match p {
                RefPath::FieldName(k) => format!(".{}", k),
                RefPath::FieldHash(k) => format!(".{}", k),
                RefPath::Index(i) => format!("[{}]", i + 1),
            }).collect::<String>();
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

    fn get_value_with_path(&self, last_path: &RefPath) -> Option<FieldValue> {
        let mut target = self.get_value()?;
        target = match (target, last_path) {
            (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(&name).cloned(),
            (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index).cloned(),
            _ => None
        }?;
        Some(target)
    }

    fn get_value(&self) -> Option<FieldValue> {
        let mut path = self.path.iter();
        let initial_class_index = path.next()?;
        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| x.1.clone()),
            _ => None
        }?;

        let Some(initial_index) = path.next() else {
            return Some(FieldValue::Class(Box::new(initial_class)))
        };
        let mut target = match initial_index {
            RefPath::FieldName(name) => initial_class.get_value(name),
            _ => None,
        }?;

        while let Some(path_ref) = path.next() {
            target = match (target, path_ref) {
                (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(name),
                (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
                _ => None
            }?;
        }
        Some(target.clone())
    }


    fn with_initial_class<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&Class) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next()
            .ok_or(LuaError::RuntimeError("First path segment empty on savefile ref".to_string()))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None
        }.ok_or(LuaError::RuntimeError(format!("Could not find Index {initial_class_index:?} on root ref")))?;

        let Some(initial_index) = path.next() else {
            return func(initial_class)
        };

        /*let mut target = match initial_index {
          RefPath::FieldName(name) => initial_class.get_value(name),
          _ => None,
          }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {initial_index:?} on root ref")))?;

          while let Some(path_ref) = path.next() {
          target = match (target, path_ref) {
          (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(name),
          (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
          _ => None
          }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {path_ref:?} on root ref")))?;
          }

          if let FieldValue::Class(class) = target {
          return func(class)
          };*/
        Err(LuaError::RuntimeError(format!("Final Ref is not a Class")))
    }

    fn with_target<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&FieldValue) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next()
            .ok_or(LuaError::RuntimeError("First path segment empty on savefile ref".to_string()))?;

        let root = self.root.read().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get(*i).map(|x| &x.1),
            _ => None
        }.ok_or(LuaError::RuntimeError(format!("Could not find Index {initial_class_index:?} on root ref")))?;

        let initial_index = path.next()
            .ok_or(LuaError::RuntimeError("Second path segment empty on savefile ref".to_string()))?;

        let mut target = match initial_index {
            RefPath::FieldName(name) => initial_class.get_value(name),
            _ => None,
        }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {initial_index:?} on root ref")))?;

        while let Some(path_ref) = path.next() {
            target = match (target, path_ref) {
                (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(name),
                (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
                _ => None
            }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {path_ref:?} on root ref")))?;
        }
        func(target)
    }

    fn with_target_mut<F, R>(&self, func: F) -> LuaResult<R>
    where
        F: FnOnce(&mut FieldValue) -> LuaResult<R>,
    {
        let mut path = self.path.iter();
        let initial_class_index = path.next()
            .ok_or(LuaError::RuntimeError("First path segment empty on savefile ref".to_string()))?;

        let mut root = self.root.write().unwrap();
        let initial_class = match initial_class_index {
            RefPath::Index(i) => root.fields.get_mut(*i).map(|x| &mut x.1),
            _ => None
        }.ok_or(LuaError::RuntimeError(format!("Could not find Index {initial_class_index:?} on root ref")))?;

        let initial_index = path.next()
            .ok_or(LuaError::RuntimeError("Second path segment empty on savefile ref".to_string()))?;

        let mut target = match initial_index {
            RefPath::FieldName(name) => initial_class.get_value_mut(name),
            _ => None,
        }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {initial_index:?} on root ref")))?;

        while let Some(path_ref) = path.next() {
            target = match (target, path_ref) {
                (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value_mut(name),
                (FieldValue::Array(array), RefPath::Index(index)) => array.get_value_mut(*index),
                _ => None
            }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {path_ref:?} on root ref")))?;
        }
        func(target)
    }
}

impl LuaUserData for SaveDataRef {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("save", |lua, this, (path, steamid): (String, u64)| {
            let path_og = path.clone();
            let path_expanded = shellexpand::full(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to shell expand path {e}")))?;
            let path = PathBuf::from(path_expanded.as_ref());

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to create directories: {e}")))?;
            }

            let save_file = this.root.read().unwrap();
            save_file.save(&path, steamid)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to save file: {e}")))?;
            Ok(path_og)
        });
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            let index = match key {
                LuaValue::String(s) => Some(RefPath::FieldName(s.to_str()?.to_string())),
                LuaValue::Integer(i) => Some(RefPath::Index((i - 1) as usize)),
                _ => None
            };

            if let Some(index) = index {
                if this.path.is_empty() {
                    let root = this.root.read().unwrap();
                    let initial_class = match index {
                        RefPath::Index(i) => root.fields.get(i),
                        _ => None
                    };
                    if initial_class.is_some() {
                        let mut new_path = this.path.clone();
                        new_path.push(index.clone());
                        return Ok(SaveDataRef { root: this.root.clone(), path: new_path }.into_lua(lua)?);
                    }
                }
                if this.path.len() == 1 {
                    let mut new_path = this.path.clone();
                    new_path.push(index.clone());
                    return Ok(SaveDataRef { root: this.root.clone(), path: new_path }.into_lua(lua)?);
                }

                return this.with_target(|target| {
                    let target = match (target, &index) {
                        (FieldValue::Class(class), RefPath::FieldName(name)) => class.get_value(name),
                        (FieldValue::Array(array), RefPath::Index(index)) => array.get_value(*index),
                        _ => None
                    }.ok_or(LuaError::RuntimeError(format!("Could not find PathRef {index:?} on root ref")))?;

                    match target {
                        FieldValue::Class(_) | FieldValue::Array(_) => {

                            let mut new_path = this.path.clone();
                            new_path.push(index.clone());
                            return Ok(SaveDataRef { root: this.root.clone(), path: new_path }.into_lua(lua)?);
                        },
                        _ => return target.clone().into_lua(lua)
                    }
                })
            }
            return Ok(LuaValue::Nil)
        });

        methods.add_meta_method(LuaMetaMethod::Len, |_, this, ()| {
            return this.with_target(|target| {
                if let FieldValue::Array(array) = target {
                    return Ok(array.values.len())
                } else if let FieldValue::Class(class) = target {
                    return Ok(class.fields.len())
                };
                Err(LuaError::RuntimeError(format!("Could not find PathRef {:?} on root ref", this.path)))
            });
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |lua, this, (key, value): (LuaValue, LuaValue)| {
            let index = match key {
                LuaValue::String(s) => RefPath::FieldName(s.to_str()?.to_string()),
                LuaValue::Integer(i) => RefPath::Index((i - 1) as usize),
                _ => return Ok(()),
            };
            let mut new_path = this.path.clone();
            new_path.push(index);
            let new_this = SaveDataRef {
                root: this.root.clone(),
                path: new_path
            };

            new_this.with_target_mut(|lhs| {
                set_fieldvalue_from_lua(lua, lhs, value)
            })?;
            Ok(())
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            let path_str = this.path.iter().map(|p| match p {
                RefPath::FieldName(k) => format!(".{}", k),
                RefPath::FieldHash(k) => format!(".{}", k),
                RefPath::Index(i) => format!("{}", i + 1),
            }).collect::<String>();
            lua.create_string(&format!("Ref(root[{}])", path_str))
        });
    }
}
