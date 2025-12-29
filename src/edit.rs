use std::{any::Any, collections::HashMap, fmt::Debug, io::Cursor, str::FromStr};

use std::collections::HashSet;
use eframe::egui::{CollapsingHeader,  Response,  TextEdit, Ui};
use half::f16;
use sdk::type_map::{ContentLanguage, FieldInfo, TypeInfo, TypeMap};

use crate::save::remap::Remap;
use crate::save::types::{Class, Field, FieldType, FieldValue};
use crate::{rsz::{dump::{RszDump, RszField, RszStruct, get_enum_list, get_enum_val}, rszserde::{DeRsz, DeRszInstance, DeRszRegistry, DeRszType, Enummable, ExternObject, Guid, Mandrake, Nullable, Object, RszDeserializerCtx, RszFieldsValue, RszSerializerCtx, StringU16, Struct, StructData}}, save::SaveFile};
use util::*;

pub type EditableFile = dyn Edit;

#[derive(Debug, Clone)]
pub enum CopyBuffer {
    Null,
    Array(Class),
    Field(Field),
}

//pub type CopyBuffer = Option<Field>;

type C<'a> = RszEditCtx<'a>;
pub trait Edit: Any {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx);
}

pub struct RszEditCtx<'a> {
    pub root: Option<u32>,
    pub field: Option<&'a RszField>,
    //objects: &'a mut Vec<RefCell<RszFieldsValue>>,
    pub objects: &'a mut Vec<RszFieldsValue>,
    pub parent: Option<&'a RszStruct<RszField>>,
    pub id: u64,
    // hash, value, is_array
    pub copy_buffer: &'a mut CopyBuffer,
    pub type_map: &'a TypeMap,
}

impl<'a> RszEditCtx<'a> {
    pub fn new(root: u32, objects: &'a mut Vec<RszFieldsValue>, copy_buffer: &'a mut CopyBuffer, type_map: &'a TypeMap) -> Self {
        Self {
            root: Some(root),
            objects,
            parent: None,
            field: None,
            id: 0,
            copy_buffer,
            type_map: type_map
        }
    }
    pub fn make_new(ctx: &'a mut Self, root: Option<u32>) -> Self {
        Self{
            root: root,
            objects: ctx.objects,
            parent: ctx.parent,
            field: ctx.field,
            id: ctx.id + 1,
            copy_buffer: ctx.copy_buffer,
            type_map: ctx.type_map
        }
    }
}

/*pub fn add_copyable_field(ui: &mut Ui, item: &mut Box<dyn DeRszInstance>, field: &RszField, ctx: &mut C) {
    let label = format!("{}: {}", &field.name, &field.original_type);
    let id = ui.make_persistent_id(ctx.id);
    let state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let mut toggle = false;
    let mut header_response = state.show_header(ui, |ui| {
            let label_response = ui.add(Label::new(label).sense(Sense::click()).selectable(false));
            if label_response.clicked() {
                toggle = true;
            }
            if let Some(item) = item.as_any().downcast_ref::<Class>() {
                if ui.button("Copy").clicked() {
                    let hash = murmur3(&field.original_type, 0xffffffff);
                        *ctx.copy_buffer = CopyBuffer::Class(hash, item.clone());
                }
            }
            else if let Some(item) = item.as_any().downcast_ref::<Array>() {
                if ui.button("Copy").clicked() {
                    let hash = murmur3(&field.original_type, 0xffffffff);
                        *ctx.copy_buffer = CopyBuffer::Array(hash, item.clone());
            }
            if let CopyBuffer::Class(copied_hash, copied_item) = ctx.copy_buffer {
                let hash = murmur3(&field.original_type, 0xffffffff);
                if *copied_hash == hash  {
                    if ui.button("Paste").clicked() {
                        *item = Box::new(copied_item.clone());
                        *ctx.copy_buffer = CopyBuffer::Null
                    }
                }
            } else if let CopyBuffer::Array(copied_hash, copied_item) = ctx.copy_buffer {
                let hash = murmur3(&field.original_type, 0xffffffff);
                if *copied_hash == hash  {
                    if ui.button("Paste").clicked() {
                        *item = Box::new(copied_item.clone());
                        *ctx.copy_buffer = CopyBuffer::Null
                    }
                }
            }
            label_response
        });
    if toggle {
        header_response.toggle();
    }
    header_response.body(|ui| {
            item.edit(ui, ctx);
        });
}

pub fn add_copyable_element(ui: &mut Ui, item: &mut Box<dyn DeRszInstance>, index: u64, ctx: &mut C) {
    let label = format!("{index}: ");
    let id = ui.make_persistent_id(ctx.id);
    let state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let mut toggle = false;
    let mut header_response = state.show_header(ui, |ui| {
            let label_response = ui.add(Label::new(label).sense(Sense::click()).selectable(false));
            if label_response.clicked() {
                toggle = true;
            }
            if let Some(field) = ctx.field {
            if let Some(item) = item.as_any().downcast_ref::<Class>() {
                if ui.button("Copy").clicked() {
                    let hash = murmur3(&field.original_type, 0xffffffff);
                        *ctx.copy_buffer = CopyBuffer::Class(hash, item.clone());
                }
            }
            else if let Some(item) = item.as_any().downcast_ref::<Array>() {
                if ui.button("Copy").clicked() {
                    let hash = murmur3(&field.original_type, 0xffffffff);
                        *ctx.copy_buffer = CopyBuffer::Array(hash, item.clone());
                }
            }
            // TODO: add a check for class/array here
            if let CopyBuffer::Class(copied_hash, copied_item) = ctx.copy_buffer {
                let hash = murmur3(&field.original_type, 0xffffffff);
                if *copied_hash == hash  {
                    if ui.button("Paste").clicked() {
                        *item = Box::new(copied_item.clone());
                        *ctx.copy_buffer = CopyBuffer::Null
                    }
                }
            } else if let CopyBuffer::Array(copied_hash, copied_item) = ctx.copy_buffer {
                let hash = murmur3(&field.original_type, 0xffffffff);
                if *copied_hash == hash  {
                    if ui.button("Paste").clicked() {
                        *item = Box::new(copied_item.clone());
                        *ctx.copy_buffer = CopyBuffer::Null
                    }
                }
            }
            }
            label_response
    });
    if toggle {
        header_response.toggle();
    }
    header_response.body(|ui| {
        item.edit(ui, ctx);
    });
}*/

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
            fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut C) {
                ui.add(eframe::egui::DragValue::new(self).speed(0.0).range(<$ty>::MIN..=<$ty>::MAX));
            }
        }
    };
}

impl<'a> Edit for Vec<Box<dyn DeRszInstance>> {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        for (_i, _item) in self.iter_mut().enumerate() {
            ctx.id += 1;
            let mut _new_ctx = RszEditCtx {
                root: None,
                field: ctx.field,
                objects: ctx.objects,
                parent: ctx.parent,
                id: ctx.id,
                copy_buffer: ctx.copy_buffer,
                type_map: ctx.type_map,
            };
            //add_copyable_element(ui, item, i as u64, &mut new_ctx);
        };
    }
}

impl<'a, T: 'static + Edit + Debug + Clone> Edit for Vec<T> {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut RszEditCtx) {
        ui.vertical(|ui| {
            for (i, item) in self.iter_mut().enumerate() {
                ctx.id += 1;
                let mut new_ctx = RszEditCtx {
                    root: None,
                    field: ctx.field,
                    objects: ctx.objects,
                    parent: ctx.parent,
                    id: ctx.id,
                    copy_buffer: ctx.copy_buffer,
                    type_map: ctx.type_map,
                };
                ui.horizontal(|ui| {
                    ui.label(format!("{i}"));
                    item.edit(ui, &mut new_ctx);
                });
            };
        });
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
                copy_buffer: ctx.copy_buffer,
                type_map: ctx.type_map,
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
                    ctx.id += 1;
                    let mut new_ctx = RszEditCtx {
                        root: None,
                        field: Some(&field),
                        objects: ctx.objects,
                        parent: Some(struct_desc),
                        id: ctx.id,
                        copy_buffer: ctx.copy_buffer,
                        type_map: ctx.type_map,
                    };
                    //add_copyable_field(ui, obj, field, &mut new_ctx);
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
                    ctx.id += 1;
                    let mut new_ctx = RszEditCtx {
                        root: None,
                        field: Some(&field),
                        objects: ctx.objects,
                        parent: Some(struct_desc),
                        id: ctx.id,
                        copy_buffer: ctx.copy_buffer,
                        type_map: ctx.type_map,
                    };
                    //add_copyable_field(ui, obj, field, &mut new_ctx);
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
                    copy_buffer: ctx.copy_buffer,
                    type_map: ctx.type_map,
                };
                if let Some(_obj) = item.as_any().downcast_ref::<Object>() {
                    ui.horizontal(|ui| {
                        //ui.label(&field_info.name);
                        //add_copyable_field(ui, item, field_info, &mut new_ctx);
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
        Edit::edit(&mut self.path, ui, ctx);
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
                        // probably better to pregenerate the enums into a single file that
                        // that maps String -> Vec<String>
                        let mut options: Vec<&String> = map.iter().filter_map(|x| if x.0.parse::<Self>().is_ok() {
                            Some(x.1)
                        } else {None}).collect();
                        options.sort();

                        eframe::egui::ComboBox::from_id_salt(ctx.id)
                            .selected_text(&enum_str_val)
                            .show_ui(ui, |ui| {
                                for option in options {
                                    ui.selectable_value(&mut enum_str_val, option.to_string(), option);
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

impl Edit for half::f16 {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut RszEditCtx) {
        let mut s = self.to_string();
        ui.add(TextEdit::singleline(&mut s));
        *self = f16::from_str(&s).unwrap_or_default();
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
        if let Some(x) = self {
            x.edit(ui, ctx);
        } else {
            ui.label("[None]");
        }
    }
}

impl Edit for Nullable {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        Edit::edit(&mut self.has_value, ui, ctx);
        self.value.edit(ui, ctx);
    }
}

impl Edit for StructData {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        let original_type = &ctx.field.unwrap().original_type;
        let hash = *RszDump::name_map().get(&ctx.field.unwrap().original_type).unwrap();
        let data = Box::new(Cursor::new(self.0.clone()));
        let fake_extern = HashMap::new();
        let fake_types = Vec::new();
        let cur_hash = vec![hash];
        let field = vec![ctx.field.unwrap()];
        let mut registry = DeRszRegistry::new();
        registry.init();
        let t = ctx.field.unwrap().r#type.clone();
        let dersz_fn = registry.get(t.as_str());
        let dersz_fn_obj = registry.get(original_type.as_str());

        let mut registry = DeRszRegistry::new();
        registry.init();
        let roots = vec![];
        let mut de_ctx = RszDeserializerCtx {
            roots: &roots,
            registry: registry.into(),
            data,
            extern_slots: &fake_extern,
            type_descriptors: &fake_types,
            cur_hash,
            field,
        };

        if let Ok(dersz_fn) = dersz_fn_obj {
            let mut x: Box<dyn DeRszInstance> = dersz_fn(&mut de_ctx).unwrap();
            x.edit(ui, ctx);
            let mut data = Cursor::new(Vec::<u8>::new());
            let mut ser_ctx = RszSerializerCtx {
                data: &mut data,
                base_addr: 0
            };
            x.to_bytes(&mut ser_ctx).unwrap();
            self.0 = data.into_inner();
        } else if let Ok(dersz_fn) = dersz_fn {
            let mut x: Box<dyn DeRszInstance> = dersz_fn(&mut de_ctx).unwrap();
            x.edit(ui, ctx);
            let mut data = Cursor::new(Vec::<u8>::new());
            let mut ser_ctx = RszSerializerCtx {
                data: &mut data,
                base_addr: 0
            };
            x.to_bytes(&mut ser_ctx).unwrap();
            self.0 = data.into_inner();
        } else {
            let mut s = Struct::from_bytes(&mut de_ctx).unwrap();
            s.edit(ui, ctx);
            let mut data = Cursor::new(Vec::<u8>::new());
            let mut ser_ctx = RszSerializerCtx {
                data: &mut data,
                base_addr: 0
            };
            s.to_bytes(&mut ser_ctx).unwrap();
            self.0 = data.into_inner();
        }
    }
}

impl Edit for String {
    fn edit(&mut self, ui: &mut Ui, _ctx: &mut C) {
        ui.add(TextEdit::singleline(self));
    }
}

impl Edit for Struct {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut C) {
        let struct_desc = RszDump::get_struct(self.hash).unwrap();
        for (_i, field) in struct_desc.fields.iter().enumerate() {
            let field_hash = murmur3(&field.name, 0xffffffff);
            if let Some(field_value) = self.values.get_mut(_i) {
                ctx.id += 1;
                let mut new_ctx = RszEditCtx {
                    root: None,
                    field: Some(&field),
                    objects: ctx.objects,
                    parent: Some(struct_desc),
                    id: ctx.id,
                    copy_buffer: ctx.copy_buffer,
                    type_map: ctx.type_map,
                };
                match field.r#type.as_str() {
                    "Object" | "Struct" => {
                        //add_copyable_field(ui, field_value, field, &mut new_ctx);
                    }
                    _ => {
                        if field.array {
                            //add_copyable_field(ui, field_value, field, &mut new_ctx);
                        }
                        else {
                            ui.horizontal(|ui| {
                                ui.label(format!("  {}", &field.name));
                                field_value.edit(ui, &mut new_ctx);
                            });
                        }
                    }

                }
            } else {
                println!("Missing field: {}, {:08x} in struct {}", &field.name, field_hash, struct_desc.name);
            }
        }
    }
}

impl Editable for SaveFile {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        for field in &mut self.fields {
            let child_id = ui.make_persistent_id(field.0);
            let mut child_ctx = EditContext {
                copy_buffer: ctx.copy_buffer,
                id: child_id.value(),
                ..*ctx
            };
            CollapsingHeader::new(format!("{:08x}, {}", field.0, child_id.value()))
                .id_salt(child_id)
                .default_open(true)
                .show(ui, |ui| {
                    field.1.edit(ui, &mut child_ctx);
                });
        }
        EditResponse::default()
    }
}

impl Edit for DeRsz {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut C) {
        for root in &self.roots {
            //let (root_hash, root_struct) = &dersz.structs[*root as usize];
            //let val = dersz.structs.get_mut(idx).ok_or(RszError::InvalidRszObjectIndex(self.idx, self.hash))?;
            let (hash, mut field_values) = {
                let val = self.structs.get_mut(*root as usize).unwrap();
                let (hash, field_values) = std::mem::take(&mut *val);
                (hash, field_values)
            };
            let _root_type = RszDump::get_struct(hash).unwrap();
            let mut ctx = RszEditCtx::new(*root, &mut self.structs, _ctx.copy_buffer, _ctx.type_map);
            field_values.edit(ui, &mut ctx);
        }
    }
}

impl Edit for Guid {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut C) {
        let mut disp = uuid::Uuid::from_bytes_le(self.0).to_string();
        ui.add(TextEdit::singleline(&mut disp).clip_text(false));
        if let Ok(edited) = uuid::Uuid::from_str(&disp) {
            self.0 = edited.to_bytes_le();
        } else {
            println!("Invalid Value for Guid");
        }
    }
}
impl Edit for StringU16 {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut C) {
        let mut disp = String::from_utf16_lossy(&self.0);
        ui.add(TextEdit::singleline(&mut disp).clip_text(false));
        let encoded: Vec<u16> = disp.encode_utf16().collect();
        *self = StringU16(encoded)
    }
}

impl Edit for Mandrake {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, ctx: &mut C) {
        if let Some(mut real_val) = self.get() {
            ui.horizontal(|ui| {
                ui.label("  real_value");
                Edit::edit(&mut real_val, ui, ctx);
            });
            self.set(real_val);
        }
        ui.horizontal(|ui| {
            ui.label("  v");
            ui.label(format!("{}", self.v));
        });
        ui.horizontal(|ui| {
            ui.label("  m");
            ui.label(format!("{}", self.m));
        });

    }

}

#[derive(Debug)]
pub struct EditContext<'a> {
    pub type_map: &'a TypeMap,
    pub search_paths: &'a HashSet<(u32, u32)>,
    pub search_leaf_nodes: &'a HashSet<(u32, u32)>,
    pub search_found_leaf: bool,
    pub search_range: &'a core::ops::Range<usize>,
    pub parent_hash: u64,
    pub parent_type: Option<&'a TypeInfo>,
    pub cur_type: Option<&'a TypeInfo>,
    pub field_info: Option<&'a FieldInfo>,
    pub array_index: Option<usize>,
    pub id: u64,
    pub depth: usize,
    pub copy_buffer: &'a mut CopyBuffer,
    pub language: ContentLanguage,
    pub remaps: &'a HashMap<String, Remap>,
}

impl<'a> EditContext<'a> {
    pub fn new(type_map: &'a TypeMap, search_paths: &'a HashSet<(u32, u32)>, search_leaf_nodes: &'a HashSet<(u32, u32)>, search_range: &'a core::ops::Range<usize>, copy_buffer: &'a mut CopyBuffer, language: ContentLanguage, remaps: &'a HashMap<String, Remap>) -> Self {
        Self {
            type_map,
            search_paths,
            search_leaf_nodes,
            search_found_leaf: false,
            search_range,
            parent_type: None,
            parent_hash: 0,
            cur_type: None,
            field_info: None,
            array_index: None,
            id: 0,
            depth: 0,
            copy_buffer,
            language,
            remaps,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EditResponse {
    pub found_search: bool,
    pub changed: bool,
}


impl Default for EditResponse {
    fn default() -> Self {
        Self {
            found_search: true,
            changed: false,
        }
    }
}

impl From<Response> for EditResponse {
    fn from(value: Response) -> Self {
        if value.changed() {
            EditResponse::change()
        } else {
            EditResponse::default()
        }
    }
}

impl EditResponse {
    pub fn change() -> EditResponse {
        let mut resp = EditResponse::default();
        resp.changed = true;
        resp
    }
}

pub trait Editable {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse;
}

macro_rules! derive_editable_num {
    // Match a comma-separated list of types
    ($( $ty:ty ),*) => {
        // Repeat the implementation block for each type captured ($ty)
        $(
            impl Editable for $ty {
                fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut EditContext) -> EditResponse {
                    let response = ui.add(
                        eframe::egui::DragValue::new(self)
                            .speed(1.0)
                            .range(<$ty>::MIN..=<$ty>::MAX)
                    );
                    
                    if response.changed() {
                        EditResponse::change()
                    } else {
                        EditResponse::default()
                    }
                }
            }
        )*
    };
}

derive_editable_num!(i8, i16, i32, i64, u8, u16, u32, u64);
derive_editable_num!(f32, f64);

impl Editable for bool {
    fn edit(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut EditContext) -> EditResponse {
        let response = ui.checkbox(self, "");
        if response.changed() {
            EditResponse::change()
        } else {
            EditResponse::default()
        }
    }
}
