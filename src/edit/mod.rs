pub mod copy;

use std::{collections::HashMap, io::Cursor, sync::Arc, time::Instant};

use anyhow::Result;
use eframe::egui::{self, CollapsingHeader, Frame, InnerResponse, Layout, ScrollArea, TextEdit, Ui, collapsing_header::CollapsingState};
use serde::{Deserialize, Serialize};
use ree_lib::{context::EngineContext, rsz::{self, FieldInfo, RszMap, TypeInfo, Value, deserializer::RszDeserializer}, types::{Mandrake, StringU16, Vec2, Vec3, Vec4, Color}};

use crate::{edit::{copy::CopyBuffer}, save::{SaveFile, SaveFlags, remap::Remap, types::{Array, Class, EnumValue, Field, FieldValue, Struct}}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathNode {
    Class(u32),
    Field(u32),
    Index(usize),
}

pub enum Action {
    CopyObject(Class),
    CopyArray(Array),
    PasteObject(Class),
    PasteArray(Array),
    ModifyReference
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ArrayConfig {
    pub right_margin: f32,
    pub target_row_height: f32,
    pub item_spacing: f32,
    pub max_inf_height: f32,
    pub available_height_delta: f32,
    pub min_max_height: f32,
    pub height_clamp: f32,
    pub max_rows: f32,
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EditorConfig {
    pub array: ArrayConfig
}

impl Default for ArrayConfig {
    fn default() -> Self {
        Self {
            right_margin: 5.0,
            target_row_height: 22.0,
            item_spacing: 6.0,
            max_inf_height: 1000.0,
            available_height_delta: 5.0,
            min_max_height: 500.0,
            height_clamp: 40.0,
            max_rows: 50.0,
        }
    }
}

pub struct EditContext<'a> {
    pub engine_context: &'a EngineContext<'a>,
    pub remaps: &'a HashMap<String, Remap>,
    pub config: &'a EditorConfig,
    pub path: &'a mut Vec<PathNode>,
    pub query_cache: &'a mut HashMap<(String, String), Value>,
    pub copy_buffer: &'a mut CopyBuffer
}

impl<'a> EditContext<'a> {
    pub fn draw_remapped_dropdown(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut FieldValue,
        remap_key: &str,
        id_salt: impl std::hash::Hash + Copy,
    ) -> bool {
        let mut changed = false;

        let current_preview = self.remap_format(remap_key, value)
            .unwrap_or_else(|| format!("{:?}", value));

        let enum_def = match self.engine_context.enums.get(remap_key) {
            Some(def) => def,
            None => {
                return false;
            }
        };

        let cache_id = ui.make_persistent_id(("dropdown_cache", remap_key));
        let search_id = ui.make_persistent_id(("dropdown_search", remap_key, id_salt));
        let mut search_text = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
        //let start = Instant::now();
        let cached_options: Arc<Vec<(FieldValue, String)>> = ui.data_mut(|d| {
            let cached = d.get_temp::<Arc<Vec<(FieldValue, String)>>>(cache_id);
            match cached {
                Some(opts) => opts,
                None => {
                    let mut new_opts = Vec::with_capacity(enum_def.name_to_value.len());

                    for (enum_str, enum_val) in enum_def.name_to_value.iter() {
                        let enum_val_u64 = enum_val.as_u64();
                        let field_val = match value {
                            FieldValue::U8(_) => FieldValue::U8(enum_val_u64 as u8),
                            FieldValue::U16(_) => FieldValue::U16(enum_val_u64 as u16),
                            FieldValue::U32(_) => FieldValue::U32(enum_val_u64 as u32),
                            FieldValue::U64(_) => FieldValue::U64(enum_val_u64),
                            FieldValue::S8(_) => FieldValue::S8(enum_val_u64 as i8),
                            FieldValue::S16(_) => FieldValue::S16(enum_val_u64 as i16),
                            FieldValue::S32(_) => FieldValue::S32(enum_val_u64 as i32),
                            FieldValue::S64(_) => FieldValue::S64(enum_val_u64 as i64),
                            FieldValue::Enum(EnumValue::E1(_)) => FieldValue::Enum(EnumValue::E1(enum_val_u64 as i8)),
                            FieldValue::Enum(EnumValue::E2(_)) => FieldValue::Enum(EnumValue::E2(enum_val_u64 as i16)),
                            FieldValue::Enum(EnumValue::E4(_)) => FieldValue::Enum(EnumValue::E4(enum_val_u64 as i32)),
                            FieldValue::Enum(EnumValue::E8(_)) => FieldValue::Enum(EnumValue::E8(enum_val_u64 as i64)),
                            _ => {
                                log::warn!("Attempting to create enum dropdown for un-coercable type");
                                continue
                            }
                        };

                        let option_text = self.remap_format(remap_key, &field_val)
                            .unwrap_or_else(|| enum_str.clone());

                        new_opts.push((field_val, option_text));
                    }

                    let arc_opts = Arc::new(new_opts);
                    d.insert_temp(cache_id, arc_opts.clone());
                    arc_opts
                }
            }
        });

        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(current_preview)
            .show_ui(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut search_text);
                });
                ui.separator();
                let search_lower = search_text.to_lowercase();

                for (option_val, option_text) in cached_options.iter() {
                    if !search_lower.is_empty() && !option_text.to_lowercase().contains(&search_lower) {
                        continue;
                    }

                    let is_selected = value.as_any_u64() == option_val.as_any_u64();

                    if ui.selectable_label(is_selected, option_text).clicked() {
                        *value = option_val.clone();
                        changed = true;
                    }
                }
            });
        //log::info!("Enum drop time: {}", start.elapsed().as_millis());
        ui.data_mut(|d| d.insert_temp(search_id, search_text));
        changed
    }
}

pub trait Editable {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext);
}

impl Editable for SaveFile {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        self.flags.ui(ui, ctx);
        for (unk, class) in &mut self.fields {
            let class_label = ctx.engine_context.rsz_map.get_by_hash(class.hash)
                .map(|t| t.name.clone()).unwrap_or(format!("{:08x}", class.hash));
            let label = format!("{:08x}: {}", unk, class_label);
            CollapsingHeader::new(label)
                .show(ui, |ui| {
                    class.ui(ui, ctx);
                });
        }
    }
}

impl Editable for SaveFlags {
    fn ui(&mut self, ui: &mut Ui, _ctx: &mut EditContext) {
        ui.collapsing("Save Flags", |ui| {
            ui.vertical(|ui| {
                let flag_checkbox = |ui: &mut Ui, flags: &mut SaveFlags, flag: SaveFlags, label: &str| {
                    let mut is_on = flags.contains(flag);
                    if ui.checkbox(&mut is_on, label).changed() {
                        flags.set(flag, is_on);
                    }
                };

                flag_checkbox(ui, self, SaveFlags::BLOWFISH, "Blowfish");
                flag_checkbox(ui, self, SaveFlags::HAS_ID, "HasID");
                flag_checkbox(ui, self, SaveFlags::CITRUS, "Citrus");
                flag_checkbox(ui, self, SaveFlags::DEFLATE, "Deflate");
                flag_checkbox(ui, self, SaveFlags::MANDARIN, "Mandarin");
            });
        });
    }
}

impl Editable for Class {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        ctx.path.push(PathNode::Class(self.hash));

        for field in self.fields.iter_mut() {
            field.ui(ui, ctx);
        }

        ctx.path.pop();
    }
}

impl Editable for Array {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        let ArrayConfig { 
            right_margin, 
            target_row_height: target_height, 
            item_spacing: spacing, 
            max_inf_height, 
            available_height_delta, 
            min_max_height, 
            height_clamp, 
            max_rows 
        } = ctx.config.array;

        let parent_class_hash = {
            let path = &ctx.path;
            path.iter().rev().find_map(|node| {
                if let PathNode::Class(hash) = node { Some(*hash) } else { None }
            })
        };

        let parent_field_hash = {
            let path = &ctx.path;
            path.iter().rev().find_map(|node| {
                if let PathNode::Field(hash) = node { Some(*hash) } else { None }
            })
        };

        let get_type_label = || {
            let type_info = parent_class_hash.and_then(|h| ctx.engine_context.rsz_map.get_by_hash(h))?;
            let field_info = type_info.get_field_by_hash(parent_field_hash?)?;
            let remap = ctx.remaps.get(&type_info.name)?;
            let type_label = remap.fields.get(&field_info.name).unwrap_or(&field_info.original_type);
            Some(type_label)
        };

        let mut add_array_value = |value: &mut FieldValue, i: usize, ui: &mut egui::Ui| {
            ui.push_id(i, |ui| {
                ctx.path.push(PathNode::Index(i));
                match value {
                    FieldValue::Class(_) | FieldValue::Array(_) => {
                        let index_label = match value {
                            FieldValue::Class(c) => {
                                let type_info = ctx.engine_context.rsz_map.get_by_hash(c.hash);
                                let label = get_type_label().or_else(|| type_info.map(|t| &t.name));

                                if let Some(type_label) = label {
                                    if let Some(formatted) = ctx.remap_format(&type_label.replace("[]", ""), value) {
                                        format!("{i}: {formatted}")
                                    } else {
                                        format!("{i}: {type_label}")
                                    }
                                } else {
                                    format!("{i}: ")
                                }
                            }
                            FieldValue::Array(_) => {
                                if let Some(type_label) = get_type_label() 
                                    && let Some(formatted) = ctx.remap_format(&type_label.replace("[]", ""), value) 
                                {
                                    format!("{i}: {formatted}")
                                } else {
                                    format!("{i}:")
                                }
                            }
                            _ => unreachable!(),
                        };

                        collapsing_header_with_buttons(
                            ui, ctx, value, &index_label, 
                            |ui, ctx, value| {
                                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                    if ui.button("Copy").clicked() {
                                        *ctx.copy_buffer = CopyBuffer::FieldValue(value.clone())
                                    }
                                    if *ctx.copy_buffer != CopyBuffer::Null && ui.button("Paste").clicked() && let Some(pasted) = ctx.copy_buffer.paste_into(value) {
                                        *value = pasted;
                                    }
                                });
                            }, 
                            |ui, ctx, value| {
                                value.ui(ui, ctx);
                            }
                        );
                    }
                    _ => {
                        let index_label = format!("{}:", i);
                        ui.horizontal(|ui| {
                            ui.label(index_label);
                            if let Some(type_label) = get_type_label() {
                                ctx.draw_remapped_dropdown(ui, value, type_label, i);
                            }
                            value.ui(ui, ctx);
                        });
                    }
                }
                ctx.path.pop();
            });
        };

        ui.scope(|ui| {
            // wtf are these variable names dawg

            ui.style_mut().spacing.item_spacing.y = spacing;
            ui.style_mut().spacing.interact_size.y = target_height;

            let state_id = ui.make_persistent_id("row_heights");
            let mut row_heights = ui.data_mut(|d| d.get_temp::<Vec<f32>>(state_id).unwrap_or_default());

            if row_heights.len() != self.values.len() {
                row_heights.resize(self.values.len(), target_height);
            }

            let (visible_sum, visible_count): (_, u32) = row_heights.iter().enumerate()
                                               //.filter(|(i, _)| ctx.search_range.contains(i))
                                               .fold((0.0, 0), |(acc_h, acc_c), (_, h)| (acc_h + h, acc_c + 1));

            let total_content_height = visible_sum + (visible_count.saturating_sub(1) as f32 * spacing);

            let h = ui.available_height();
            let max_height = if h.is_infinite() { max_inf_height } else { (h - available_height_delta).max(min_max_height) };

            let explicit_max_height = (target_height + spacing) * max_rows;
            let view_height = total_content_height.clamp(height_clamp, max_height.min(explicit_max_height));

            let max_width = (ui.available_width() - right_margin).max(1.0);

            Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .min_scrolled_height(view_height)
                        .max_width(max_width)
                        .show(ui, |ui| {
                            let clip_rect = ui.clip_rect();
                            let mut current_y = ui.cursor().min.y;
                            let last_visible_index = self.values.len().saturating_sub(1);
                            let mut rendered_count = 0;
                            for (i, value) in self.values.iter_mut().enumerate() {
                                let cached_h = row_heights[i];
                                let add_spacing = if i < last_visible_index { spacing } else { 0.0 };
                                if current_y + cached_h < clip_rect.min.y {
                                    ui.add_space(cached_h + add_spacing);
                                    current_y += cached_h + add_spacing;
                                    continue;
                                }
                                if current_y > clip_rect.max.y + 200.0 {
                                    let (rem_h, rem_c): (_, u32) = row_heights[i..].iter().enumerate()
                                                         //.filter(|(offset, _)| ctx.search_range.contains(&(i + offset)))
                                                         .fold((0.0, 0), |(acc_h, acc_c), (_, h)| (acc_h + h, acc_c + 1));

                                    let rem_spacing = rem_c.saturating_sub(1) as f32 * spacing;
                                    ui.add_space(rem_h + rem_spacing);
                                    break;
                                }

                                let start_pos = ui.cursor().min;
                                add_array_value(value, i, ui);

                                // update cache
                                let actual_height = ui.cursor().min.y - start_pos.y;
                                if (row_heights[i] - actual_height).abs() > 0.1 {
                                    row_heights[i] = actual_height;
                                }
                                current_y += actual_height + add_spacing;
                                rendered_count += 1;
                            }
                            log::debug!("rendered {rendered_count} elements in the array");
                        });
                });
            ui.data_mut(|d| d.insert_temp(state_id, row_heights));
        });
    }
}

impl Editable for Field {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        ctx.path.push(PathNode::Field(self.hash));

        let parent_class_hash = {
            let path = &ctx.path;
            path.iter().rev().find_map(|node| {
                if let PathNode::Class(hash) = node { Some(*hash) } else { None }
            })
        };

        //parent type info
        let type_info = parent_class_hash.and_then(|h| ctx.engine_context.rsz_map.get_by_hash(h));

        // parent class
        let class_name = type_info.map(|t| t.name.clone())
            .unwrap_or_default();

        let field_info = type_info.and_then(|ti| ti.get_field_by_hash(self.hash));

        let field_name = field_info.map(|f| f.name.clone())
            .unwrap_or(format!("{:08x}", self.hash));
        // field type label
        let mut type_label = field_info.map(|f| f.original_type.clone())
            .unwrap_or(format!("{:?}", self.field_type));


        // parent class -> field -> remapped field type
        if let Some(remap) = ctx.remaps.get(&class_name)
            && let Some(remapped_type) = remap.fields.get(&field_name) {
                type_label = remapped_type.clone();
        }

        // handle custom class types here
        match type_label.as_str() {
            "app.savedata.cPhoto" => {},
            "ace.Bitset" => {}, // TODO: need a way to handle generics?
            "app.savedata.cEquipWork" => {},
            _ => {
                ui.push_id(self.hash, |ui| {
                    match &mut self.value {
                        FieldValue::Class(_) | FieldValue::Array(_) => {
                            let header_label = format!("{}: {}", field_name, type_label);
                            collapsing_header_with_buttons(ui, ctx, &mut self.value, &header_label, |ui, ctx, value| {
                                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                                    if ui.button("Copy").clicked() {
                                        *ctx.copy_buffer = CopyBuffer::FieldValue(value.clone())
                                    }
                                    if ui.button("Paste").clicked() && let Some(pasted) = ctx.copy_buffer.paste_into(value) {
                                        *value = pasted;
                                    }
                                });
                            }, |ui, ctx, value| {
                                value.ui(ui, ctx);
                            });
                        }
                        _ => {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", field_name));
                                ctx.draw_remapped_dropdown(ui, &mut self.value, &type_label, self.hash);
                                self.value.ui(ui, ctx);
                            });
                        }
                    }
                });
            }
        }
        ctx.path.pop();
    }
}

impl Editable for FieldValue {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        match self {
            FieldValue::Boolean(v) => v.ui(ui, ctx),
            FieldValue::Enum(v) => v.ui(ui, ctx),
            FieldValue::S8(v) => v.ui(ui, ctx),
            FieldValue::U8(v) => v.ui(ui, ctx),
            FieldValue::S16(v) => v.ui(ui, ctx),
            FieldValue::U16(v) => v.ui(ui, ctx),
            FieldValue::S32(v) => v.ui(ui, ctx),
            FieldValue::U32(v) => v.ui(ui, ctx),
            FieldValue::S64(v) => v.ui(ui, ctx),
            FieldValue::U64(v) => v.ui(ui, ctx),
            FieldValue::F32(v) => v.ui(ui, ctx),
            FieldValue::F64(v) => v.ui(ui, ctx),
            FieldValue::C8(v) => v.ui(ui, ctx),
            FieldValue::C16(v) => v.ui(ui, ctx),
            FieldValue::Class(c) => c.ui(ui, ctx),
            FieldValue::Array(a) => a.ui(ui, ctx),
            FieldValue::String(v) => v.ui(ui, ctx),
            FieldValue::Struct(v) => v.ui(ui, ctx),
            _ => {
                ui.label(format!("{:?}", self));
            }
        }
    }
}

pub fn collapsing_header_with_buttons<T,R>(
    ui: &mut Ui,
    ctx: &mut EditContext,
    state_data: &mut T,
    header_label: &str,
    add_header_content: impl FnOnce(&mut Ui, &mut EditContext, &mut T),
    add_body_content: impl FnOnce(&mut Ui, &mut EditContext, &mut T) -> R,
) -> Option<InnerResponse<R>> {
    let id = ui.id().with(header_label);
    let toggle_id = id.with("toggle_flag");
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);

    let should_toggle = ui.data_mut(|d| d.remove_temp::<bool>(toggle_id).unwrap_or(false));
    if should_toggle {
        state.toggle(ui);
    }

    let res = state.show_header(ui, |ui| {
        let label_response = ui.add(
            egui::Label::new(header_label)
            .selectable(false)
            .sense(egui::Sense::click())
        );

        if label_response.clicked() {
            ui.data_mut(|d| d.insert_temp(toggle_id, true));
            ui.ctx().request_repaint();
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            add_header_content(ui, ctx, state_data);
        });
    })
    .body(|ui| {
        add_body_content(ui, ctx, state_data)
    });
    res.2
}

macro_rules! derive_editable_num {
    ($( $ty:ty ),*) => {
        $(
            impl Editable for $ty {
                fn ui(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut EditContext) {
                    ui.add(
                        eframe::egui::DragValue::new(self)
                        .speed(1.0)
                        .range(<$ty>::MIN..=<$ty>::MAX)
                    );
                }
            }
        )*
    };
}

derive_editable_num!(i8, i16, i32, i64, u8, u16, u32, u64);
derive_editable_num!(f32, f64);

impl Editable for bool {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _ctx: &mut EditContext) {
        ui.checkbox(self, "");
    }
}

impl Editable for EnumValue {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        match self {
            EnumValue::E1(v) => v.ui(ui, ctx),
            EnumValue::E2(v) => v.ui(ui, ctx),
            EnumValue::E4(v) => v.ui(ui, ctx),
            EnumValue::E8(v) => v.ui(ui, ctx),
        }
    }
}

impl Editable for String {
    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &mut EditContext) {
        ui.add(TextEdit::singleline(self).clip_text(false));
    }
}

impl Editable for StringU16 {
    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &mut EditContext) {
        let mut s = String::from_utf16_lossy(&self.0);
        ui.add(TextEdit::singleline(&mut s).clip_text(false));
        let encoded: Vec<u16> = s.encode_utf16().collect();
        *self = Self(encoded);
    }
}

impl<T: Editable> Editable for Vec<T> {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            for (i, v) in self.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    v.ui(ui, ctx);
                });
            }
        });
    }
}

impl<T: Editable, const N: usize> Editable for [T; N] {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            for (i, v) in self.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    v.ui(ui, ctx);
                });
            }
        });
    }
}

macro_rules! try_edit_as {
    ($data:expr, $ui:expr, $ctx:expr, $ty:ty) => {
        if let Ok(v) = bytemuck::try_from_bytes_mut::<$ty>($data) {
            v.ui($ui, $ctx);
            return;
        }
    };
    ($name:expr, $data:expr, $ui:expr, $ctx:expr, { $( $pat:literal => $ty:ty ),* $(,)? }) => {
        match $name {
            $( $pat => try_edit_as!($data, $ui, $ctx, $ty), )*
                _ => {}
        }
    };
}

impl Editable for Struct {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        let parent_class_hash = ctx.path.iter().rev().find_map(|node| {
            if let PathNode::Class(hash) = node { Some(*hash) } else { None }
        });
        let parent_field_hash = ctx.path.iter().rev().find_map(|node| {
            if let PathNode::Field(hash) = node { Some(*hash) } else { None }
        });

        let struct_type_name: Option<String> = parent_class_hash
            .and_then(|h| ctx.engine_context.rsz_map.get_by_hash(h))
            .and_then(|type_info| {
                let field_info = type_info.get_field_by_hash(parent_field_hash?)?;
                let remap = ctx.remaps.get(&type_info.name);
                let type_label = remap
                    .and_then(|r| r.fields.get(&field_info.name))
                    .unwrap_or(&field_info.original_type);
                Some(type_label.clone())
            });


        match struct_type_name.as_deref() {
            Some(name) => {
                // this returns
                try_edit_as!(name, &mut self.data, ui, ctx, {
                    "via.Color" => Color,
                    "via.vec2" => Vec2,
                    "via.vec3" => Vec3,
                    "via.vec4" => Vec4,
                    "via.rds.Mandrake" => Mandrake,
                });
                if let Some(type_info) = ctx.engine_context.rsz_map.get_type(name) {
                    if let Err(e) = render_struct_by_schema(ui, ctx, type_info, &self.data) {
                        ui.label(format!("rsz deser error: {e}"));
                        self.data.ui(ui, ctx);
                    }
                } else {
                    ui.label(format!("unknown struct: {name}"));
                    self.data.ui(ui, ctx);
                }
            }
            None => { self.data.ui(ui, ctx); }
        }
    }
}

pub fn deserialize_struct<'a>(type_info: &'a TypeInfo, data: &[u8], rsz_map: &'a RszMap) -> Result<Vec<(&'a FieldInfo, rsz::Value)>> {
    let mut reader = Cursor::new(data);
    let mut deserializer = RszDeserializer::from_rsz_info(&mut reader, rsz_map);
    type_info.fields.iter()
        .map(|(_hash, field)| {
            let value = deserializer.deserialize_field(field, type_info)?;
            Ok((field, value))
        })
    .collect()
}

pub fn render_struct_by_schema(ui: &mut Ui, ctx: &mut EditContext, type_info: &TypeInfo, data: &[u8]) -> Result<()> {
    let mut s = deserialize_struct(type_info, data, ctx.engine_context.rsz_map)?;
    for (field, mut value) in s {
        ui.label(&field.name);
        value.ui(ui, ctx);
    }

    // TODO: serialize
    Ok(())
}

impl Editable for rsz::Value {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut EditContext) {
        use rsz::Value::*;
        match self {
            U8 (v) => v.ui(ui, ctx),
            U16(v) => v.ui(ui, ctx),
            U32(v) => v.ui(ui, ctx),
            U64(v) => v.ui(ui, ctx),
            S8 (v) => v.ui(ui, ctx),
            S16(v) => v.ui(ui, ctx),
            S32(v) => v.ui(ui, ctx),
            S64(v) => v.ui(ui, ctx),
            String(v) => v.ui(ui, ctx),
            Resource(v) => v.ui(ui, ctx),
            _ => ()
        }
    }
}

impl Editable for Mandrake {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut EditContext) {
        if let Some(mut real_val) = self.get() {
            ui.horizontal(|ui| {
                real_val.ui(ui, ctx);
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

impl Editable for Vec2 {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            ui.label("x");
            self.0.ui(ui, ctx);
            ui.label("y");
            self.1.ui(ui, ctx);
        });
    }
}

impl Editable for Vec3 {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            ui.label("x");
            self.0.ui(ui, ctx);
            ui.label("y");
            self.1.ui(ui, ctx);
            ui.label("z");
            self.2.ui(ui, ctx);
        });
    }
}

impl Editable for Vec4 {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            ui.label("x");
            self.0.ui(ui, ctx);
            ui.label("y");
            self.1.ui(ui, ctx);
            ui.label("z");
            self.2.ui(ui, ctx);
            ui.label("w");
            self.3.ui(ui, ctx);
        });
    }
}

impl Editable for Color {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut EditContext) {
        ui.horizontal(|ui| {
            ui.label("r");
            self.0.ui(ui, ctx);
            ui.label("g");
            self.1.ui(ui, ctx);
            ui.label("b");
            self.2.ui(ui, ctx);
            ui.label("a");
            self.3.ui(ui, ctx);
        });
    }
}
