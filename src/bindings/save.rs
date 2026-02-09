use std::{cell::RefCell, rc::Rc};
use mlua::prelude::*;

use crate::{bindings::{DataRef, DataRoot, RefPath}, save::types::{Array, Class, Field, FieldValue, Struct}, sdk::StringU16};

impl IntoLua for FieldValue {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            FieldValue::Boolean(b) => Ok(LuaValue::Boolean(b)),
            FieldValue::S8(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::U8(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::S16(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::U16(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::S32(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::U32(v) => Ok(LuaValue::Integer(v as i64)),
            FieldValue::S64(v) => Ok(LuaValue::Integer(v)),
            FieldValue::U64(v) => Ok(LuaValue::Number(v as f64)),
            FieldValue::F32(v) => Ok(LuaValue::Number(v as f64)),
            FieldValue::F64(v) => Ok(LuaValue::Number(v)),

            FieldValue::String(s) => {
                let string = String::from_utf16_lossy(&s.0);
                Ok(LuaValue::String(lua.create_string(&string)?))
            },

            FieldValue::Class(c) => {
                lua.create_userdata(*c)?.into_lua(lua)
            },
            FieldValue::Array(a) => {
                lua.create_userdata(*a)?.into_lua(lua)
            },
            FieldValue::Enum(v) => Ok(LuaValue::Integer(v as i64)),
            _ => Ok(LuaValue::Nil),
        }
    }
}

pub fn get_target_from_root<'a>(root: &'a DataRoot, path: &'a [RefPath]) -> Option<&'a FieldValue> {
    if path.is_empty() { return None; }

    // 1. Step One: DataRoot -> FieldValue
    let mut cursor = match (root, &path[0]) {
        (DataRoot::Class(c), RefPath::FieldName(k)) => c.get_value(k)?,
        (DataRoot::Array(a), RefPath::Index(i)) => a.values.get(*i)?,
        _ => return None, // Type mismatch (e.g. indexing Class with Int)
    };

    // 2. Step Two+: FieldValue -> FieldValue
    for segment in &path[1..] {
        cursor = match (cursor, segment) {
            (FieldValue::Class(c), RefPath::FieldName(k)) => c.get_value(k)?,
            (FieldValue::Array(a), RefPath::Index(i)) => a.values.get(*i)?,
            _ => return None,
        };
    }

    Some(cursor)
}

pub fn set_fieldvalue_from_lua(_lua: &Lua, lhs: &mut FieldValue, value: LuaValue) -> LuaResult<()> {
    match lhs {
        FieldValue::Boolean(v) => value.as_boolean().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::U8(v) => value.as_u32().inspect(|x| *v = *x as u8).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::U16(v) => value.as_u32().inspect(|x| *v = *x as u16).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::U32(v) => value.as_u32().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::U64(v) => value.as_u64().inspect(|x| *v = *x as u64).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::S8(v) => value.as_i32().inspect(|x| *v = *x as i8).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::S16(v) => value.as_i32().inspect(|x| *v = *x as i16).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::S32(v) => value.as_i32().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::S64(v) => value.as_i64().inspect(|x| *v = *x as i64).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::F32(v) => value.as_f32().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::F64(v) => value.as_f64().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::C8(v) => value.as_u32().inspect(|x| *v = *x as u8).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::C16(v) => value.as_u32().inspect(|x| *v = *x as u16).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::String(v) => {
            match value {
                LuaValue::String(s) => {
                    let data = s.as_bytes().iter().map(|b| *b as u16).collect();
                    *v = Box::new(StringU16::new(data));
                    Ok(())
                }
                LuaValue::UserData(ud) => {
                    let rhs = ud.borrow::<StringU16>()?.clone();
                    *v = Box::new(rhs);
                    Ok(())
                }
                _ => Err(LuaError::FromLuaConversionError { from: "Something", to: "FieldValue::String".to_string(), message: None })
            }
        }
        FieldValue::Class(v) => {
            if let LuaValue::UserData(ud) = value {
                println!("class: {ud:?}");
                let incoming = ud.borrow::<DataRef>()?;
                let root = incoming.root.borrow();
                let inc_class = get_target_from_root(&root, &incoming.path)
                    .ok_or(LuaError::RuntimeError("".to_string())).unwrap();
                if let FieldValue::Class(c) = inc_class {
                    *v = c.clone();
                } else {
                    return Err(LuaError::RuntimeError("Data Ref did not evaluate to a Class FieldValue".to_string()));
                }
                Ok(())
            } else {
                Err(LuaError::FromLuaConversionError { from: "non-userdata", to: "Class".to_string(), message: None })
            }
        }
        FieldValue::Array(v) => {
            if let LuaValue::UserData(ud) = value {
                println!("arr: {ud:?}");
                let incoming = ud.borrow::<DataRef>()?;
                let inc = incoming.traverse().ok_or(LuaError::UserDataTypeMismatch)?;
                if let FieldValue::Array(a) = inc {
                    *v = a;
                } else {
                    return Err(LuaError::RuntimeError("Data Ref did not evaluate to a Class FieldValue".to_string()));
                }
                Ok(())
            } else {
                Err(LuaError::FromLuaConversionError { from: "non-userdata", to: "Class".to_string(), message: None })
            }
        }
        FieldValue::Struct(v) => {
            if let LuaValue::UserData(ud) = value {
                let rhs = ud.borrow::<Struct>()?.clone();
                *v = Box::new(rhs);
                Ok(())
            } else {
                Err(LuaError::FromLuaConversionError { from: "non-userdata", to: "Class".to_string(), message: None })
            }
        }
        FieldValue::Enum(v) => value.as_i32().inspect(|x| *v = *x).map(|_| ()).ok_or(LuaError::UserDataTypeMismatch),
        FieldValue::Unknown => Ok(()),
        //_ => Err(LuaError::RuntimeError("Unimplemented FieldValue".to_string()))
    }
}

impl IntoLua for Field {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        self.value.into_lua(lua)
    }
}

impl LuaUserData for Class {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, key: String| {
            let root: Rc<RefCell<DataRoot>> = Rc::new(DataRoot::Class(this.clone()).into());
            if this.get_value(&key).is_some() {
                let path = vec![RefPath::FieldName(key)];
                return Ok(DataRef { root, path })
            }
            return Err(LuaError::UserDataTypeMismatch)
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |lua, this, (key, value): (String, LuaValue)| {
            let Some(lhs) = this.get_value_mut(&key) else {
                return Err(LuaError::FromLuaConversionError { from: "Something", to: "Anything".to_string(), message: Some("Could not find field".to_string()) })
            };
            set_fieldvalue_from_lua(lua, lhs, value)
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            lua.create_string(&format!("Class(hash={:08x}, fields={})", this.hash, this.num_fields))
        });
    }
}

impl LuaUserData for Array {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, index: usize| {
            if index == 0 { return Ok(LuaValue::Nil); }
            let rust_index = index - 1;
            if let Some(val) = this.get_value(rust_index) {
                return val.clone().into_lua(lua);
            }
            Ok(LuaValue::Nil)
        });

        methods.add_meta_method(LuaMetaMethod::Len, |_, this, ()| {
            Ok(this.values.len())
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |lua, this, (index, value): (usize, LuaValue)| {
            if index == 0 { 
                return Err(LuaError::RuntimeError(format!("lua indeces start at 1")))
            }
            let rust_index = index - 1;
            if let Some(val) = this.get_value_mut(rust_index) {
                return set_fieldvalue_from_lua(lua, val, value);
            }
            Err(LuaError::RuntimeError(format!("Index {index}, out of bounds, len={}", this.values.len())))
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |lua, this, ()| {
            lua.create_string(&format!("Array(len={}, type={:?})", this.values.len(), this.member_type))
        });
    }
}
