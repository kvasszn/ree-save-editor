pub mod save;
pub mod runner;

use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;

use crate::{save::types::{Array, Class, FieldValue}};

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
    pub root: Rc<RefCell<DataRoot>>,
    pub path: Vec<RefPath>,
}


impl DataRef {
    fn traverse<'b, 'a: 'b> (&'a self) -> Option<FieldValue> {
        println!("traversing: {:?}", self.path);
        match &*self.root.borrow_mut() {
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
        let binding = &mut *self.root.borrow_mut();
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
                    let binding = &mut *this.root.borrow_mut();
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
                    if Rc::ptr_eq(&this.root, &rhs_ref.root) {
                        let root_read = this.root.borrow();
                        if let Some(source_fv) = save::get_target_from_root(&*root_read, &rhs_ref.path) {
                            self_cloned_val = Some(source_fv.clone());
                        }
                    }
                }
            }

            let mut root_write = this.root.borrow_mut();
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
