use std::{fmt::Debug, str::FromStr};

use eframe::egui::{CollapsingHeader, TextEdit, Ui};
use serde::Serialize;

use crate::{rsz::{dump::{get_enum_list, get_enum_val, RszDump, RszField, RszStruct}, rszserde::{DeRsz, DeRszInstance, Enummable, ExternObject, Guid, Nullable, Object, RszFieldsValue, Struct, StructData}}, save::types::{Array, Class}, user::User};

type C<'a> = RszEditCtx<'a>;
pub trait Edit {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut C);
}

pub struct RszEditCtx<'a> {
    root: Option<u32>,
    field: Option<&'a RszField>,
    //objects: &'a mut Vec<RefCell<RszFieldsValue>>,
    objects: &'a mut Vec<RszFieldsValue>,
    parent: Option<&'a RszStruct<RszField>>,
    id: u64,
}

impl<'a> RszEditCtx<'a> {
    pub fn new(root: u32, objects: &'a mut Vec<RszFieldsValue>) -> Self {
        Self {
            root: Some(root),
            objects,
            parent: None,
            field: None,
            id: 0
        }
    }
}

// edit with no needed ctx
#[macro_export]
macro_rules! edit {
    ($ident:ident) => {
        ident.edit(ui, ctx);
    };
}

macro_rules! derive_edit_num {
    ($ty:ty) => {
        impl Edit for $ty {
            fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut C) {
                ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(<$ty>::MIN..=<$ty>::MAX));
            }
        }
    };
}

impl<'a> Edit for Vec<Box<dyn DeRszInstance>> {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        //println!("before: {:?}", self);
        for (i, item) in self.iter_mut().enumerate() {
            ctx.id += 1;
            let mut new_ctx = RszEditCtx {
                root: None,
                field: ctx.field,
                objects: ctx.objects,
                parent: ctx.parent,
                id: ctx.id,
            };
            //println!("{:?}", item);
            CollapsingHeader::new(format!("{i}:"))
                .id_salt(ctx.id)
                .show(ui, |ui| {
                    item.edit(ui, &mut new_ctx);
                });
            //println!("after {:?}", item);
        };
        //println!("after: {:?}", self);
    }
}

impl<'a, T: 'static + Edit + Debug + Clone> Edit for Vec<T> {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        for (i, item) in self.iter_mut().enumerate() {
            ctx.id += 1;
            let mut new_ctx = RszEditCtx {
                root: None,
                field: ctx.field,
                objects: ctx.objects,
                parent: ctx.parent,
                id: ctx.id,
            };
            CollapsingHeader::new(format!("{i}:"))
                .id_salt(ctx.id)
                .show(ui, |ui| {
                    item.edit(ui, &mut new_ctx);
                });
        };
    }
}

impl<'a, T: 'static + Edit + Debug +  Clone, const N: usize> Edit for [T; N] {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        for (i, item) in self.iter_mut().enumerate() {
            ctx.id += 1;
            let mut new_ctx = RszEditCtx {
                root: None,
                objects: ctx.objects,
                field: ctx.field,
                parent: ctx.parent,
                id: ctx.id,
            };
            CollapsingHeader::new(format!("{i}:"))
                .id_salt(ctx.id)
                .show(ui, |ui| {
                    item.edit(ui, &mut new_ctx);
                });
        };
    }
}

impl Edit for Object {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut RszEditCtx) {
        let idx = ctx.root.unwrap_or(self.idx) as usize;
        let (hash, mut field_values) = {
            let val = ctx.objects.get_mut(idx).unwrap();
            let (hash, field_values) = std::mem::take(&mut *val);
            (hash, field_values)
        };
        //println!("{:?}", field_values);
        let struct_desc = RszDump::get_struct(hash).unwrap();
        if struct_desc.name.ends_with("_Serializable") {
            struct_desc.fields.iter().enumerate().for_each(|(i, field)| {
                let obj = &mut field_values[i];
                if i == 0 {
                    if let Some(enummable) = obj.as_any().downcast_ref::<i32>() {
                        if let Some(enum_str_val) = enummable.get_enum_name(&struct_desc.name) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", &field.name));
                                ui.label(enum_str_val);
                            });
                        }
                    }
                    else if let Some(enummable) = obj.as_any().downcast_ref::<u32>() {
                        if let Some(enum_str_val) = enummable.get_enum_name(&struct_desc.name) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", &field.name));
                                ui.label(enum_str_val);
                            });
                        }
                    }
                    else if let Some(enummable) = obj.as_any().downcast_ref::<u64>() {
                        if let Some(enum_str_val) = enummable.get_enum_name(&struct_desc.name) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", &field.name));
                                ui.label(enum_str_val);
                            });
                        }
                    }
                } else {
                    let mut new_ctx = RszEditCtx {
                        root: None,
                        field: Some(&field),
                        objects: ctx.objects,
                        parent: Some(struct_desc),
                        id: ctx.id,
                    };
                    ctx.id += 1;
                    CollapsingHeader::new(format!("{}: {}", &field.name, &field.original_type))
                        .id_salt(ctx.id)
                        .show(ui, |ui| {
                            obj.edit(ui, &mut new_ctx);
                        });
                }
            });
        }

        // Enumerable Param
        else if let Some(types) = ctx.parent.and_then(|parent| parent.name.strip_prefix("app.cEnumerableParam`2<")) {
            let types = types.strip_suffix(">").unwrap().split(",").collect::<Vec<&str>>();
            struct_desc.fields.iter().enumerate().for_each(|(i, field)| {
                let obj = &mut field_values[i];
                if field.name.contains("EnumValue") {
                    if let Some(enummable) = obj.as_any().downcast_ref::<i32>() {
                        if let Some(enum_str_val) = enummable.get_enum_name(&types[0]) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", &field.name));
                                ui.label(enum_str_val);
                            });
                        }
                    }
                    else if let Some(enummable) = obj.as_any().downcast_ref::<u32>() {
                        if let Some(enum_str_val) = enummable.get_enum_name(&types[0]) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}", &field.name));
                                ui.label(enum_str_val);
                            });
                        }
                    }
                } else {
                    let mut new_ctx = RszEditCtx {
                        root: None,
                        field: Some(&field),
                        objects: ctx.objects,
                        parent: Some(struct_desc),
                        id: ctx.id,
                    };
                    ctx.id += 1;
                    CollapsingHeader::new(format!("{}: {}", &field.name, &field.original_type))
                        .id_salt(ctx.id)
                        .show(ui, |ui| {
                            obj.edit(ui, &mut new_ctx);
                        });
                }
            });
        } else {
            let mut i = 0;
            for item in &mut field_values {
                ctx.id += 1;
                let field_info = &struct_desc.fields[i];
                let mut new_ctx = RszEditCtx {
                    root: None,
                    field: Some(&field_info),
                    objects: ctx.objects,
                    parent: Some(struct_desc),
                    id: ctx.id,
                };
                if let Some(obj) = item.as_any().downcast_ref::<Object>() {
                    ui.horizontal(|ui| {
                        //ui.label(&field_info.name);
                        CollapsingHeader::new(format!("{}: {}", &field_info.name, &field_info.original_type))
                            .id_salt(ctx.id)
                            .show(ui, |ui| {
                                item.edit(ui, &mut new_ctx);
                            });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(format!("  {}", &field_info.name));
                        item.edit(ui, &mut new_ctx);
                    });
                }
                i += 1;
            }
        }
        //println!("{field_values:?}");
        ctx.objects[idx] = (hash, field_values);
    }
}
impl Edit for ExternObject {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}
///AAAAAAAAAAAAAHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH AIDK
impl Edit for i32 {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        match ctx.field {
            Some(field) => {
                let tmp = field.original_type.replace("[]", "");
                let enum_str_val = self.get_enum_name(&tmp);
                if let Some(mut enum_str_val) = enum_str_val {
                    if let Some(map) = get_enum_list(&tmp) {
                        //ui.label(enum_str_val.to_string());
                        println!("{:?}", map);
                        eframe::egui::ComboBox::from_label("")
                            .selected_text(&enum_str_val)
                            .show_ui(ui, |ui| {
                                for (key, val) in map.iter() {
                                    if let Ok(_) = key.parse::<Self>() {
                                    }
                                    ui.selectable_value(&mut enum_str_val, val.to_string(), val);
                                }
                            });
                        *self = get_enum_val(&tmp, &enum_str_val).unwrap() as Self;
                    } else {
                        // shouldnt really ever get here?
                        ui.add(TextEdit::singleline(&mut enum_str_val));
                        panic!();
                    }
                } else {
                    ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
                }
            },
            None => {
                ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
            }
        }
    }
}
impl Edit for u32 {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        match ctx.field {
            Some(field) => {
                let tmp = field.original_type.replace("[]", "");
                let enum_str_val = self.get_enum_name(&tmp);
                if let Some(mut enum_str_val) = enum_str_val {
                    if let Some(map) = get_enum_list(&tmp) {
                        //ui.label(enum_str_val.to_string());
                        eframe::egui::ComboBox::from_label("")
                            .selected_text(&enum_str_val)
                            .show_ui(ui, |ui| {
                                for (key, val) in map.iter() {
                                    if let Ok(_) = key.parse::<Self>() {
                                        ui.selectable_value(&mut enum_str_val, val.to_string(), val);
                                    }
                                }
                            });
                        *self = get_enum_val(&tmp, &enum_str_val).unwrap() as Self;
                    } else {
                        // shouldnt really ever get here?
                        ui.add(TextEdit::singleline(&mut enum_str_val));
                        panic!();
                    }
                } else {
                    ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
                }
            },
            None => {
                ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
            }
        }
    }
}
impl Edit for u64 {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        match ctx.field {
            Some(field) => {
                let tmp = field.original_type.replace("[]", "");
                let enum_str_val = self.get_enum_name(&tmp);
                if let Some(mut enum_str_val) = enum_str_val {
                    if let Some(map) = get_enum_list(&tmp) {
                        //ui.label(enum_str_val.to_string());
                        eframe::egui::ComboBox::from_label("")
                            .selected_text(&enum_str_val)
                            .show_ui(ui, |ui| {
                                for (key, val) in map.iter() {
                                    if let Ok(_) = key.parse::<i128>() {
                                        ui.selectable_value(&mut enum_str_val, val.to_string(), val);
                                    }
                                }
                            });
                        *self = get_enum_val(&tmp, &enum_str_val).unwrap() as Self;
                    } else {
                        // shouldnt really ever get here?
                        ui.add(TextEdit::singleline(&mut enum_str_val));
                        panic!();
                    }
                } else {
                    ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
                }
            },
            None => {
                ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(Self::MIN..=Self::MAX));
            }
        }
    }
}

derive_edit_num!(i8);
derive_edit_num!(i16);
derive_edit_num!(i64);
derive_edit_num!(u8);
derive_edit_num!(u16);
derive_edit_num!(f32);
derive_edit_num!(f64);
impl Edit for bool {
    fn edit(&mut self, ui: &mut Ui, _ctx: &mut C) {
        ui.checkbox(self, "");
    }
}

impl Edit for Option<Box<dyn DeRszInstance>> {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}

impl Edit for Nullable {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}

impl Edit for StructData {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}

impl Edit for String {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        ui.add(TextEdit::singleline(self));
    }
}

impl Edit for Struct {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}

impl Edit for Array {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}
impl Edit for Class {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        todo!()
    }
}

impl Edit for DeRsz {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut C) {
        for root in &self.roots {
            //let (root_hash, root_struct) = &dersz.structs[*root as usize];
            //let val = dersz.structs.get_mut(idx).ok_or(RszError::InvalidRszObjectIndex(self.idx, self.hash))?;

            let (hash, mut field_values) = {
                let val = self.structs.get_mut(*root as usize).unwrap();
                let (hash, field_values) = std::mem::take(&mut *val);
                (hash, field_values)
            };
            let root_type = RszDump::get_struct(hash).unwrap();
            let mut ctx = RszEditCtx::new(*root, &mut self.structs);
            field_values.edit(ui, &mut ctx);
        }
    }
}

impl Edit for Guid {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut C) {
        let mut disp = uuid::Uuid::from_bytes_le(self.0).to_string();
        ui.add(TextEdit::singleline(&mut disp));
        if let Ok(edited) = uuid::Uuid::from_str(&disp) {
            self.0 = edited.to_bytes_le();
        } else {
            println!("Invalid Value for Guid");
        }
    }
}
