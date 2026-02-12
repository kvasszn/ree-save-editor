use std::{collections::HashSet, fmt::Display, str::FromStr};

use bitfield::BitMut;
use eframe::egui::{self, CollapsingHeader, ComboBox, ScrollArea, TextEdit, Ui};

use crate::{edit::{EditContext, EditResponse, Editable, copy::CopyBuffer}, save::{remap::Remap, types::*}, sdk::{type_map::{TypeInfo, murmur3}, *}};

fn collapsing_header_with_buttons_for_field(
    ui: &mut Ui,
    id: egui::Id,
    default_open: Option<bool>,
    label: impl Into<eframe::egui::WidgetText>,
    field: &mut Field,
    ctx: &mut EditContext,
) {
    let state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(), 
        id, 
        default_open.unwrap_or(false)
    );
    let is_open = state.is_open();
    let mut label_clicked = false;
    let mut header_resp = state.show_header(ui, |ui| {
        let label_res = ui.add(egui::Button::new(label).frame(false));
        label_clicked = label_res.clicked();

        ui.add_space(10.0);

        if ui.small_button("Copy").clicked() {
            *ctx.copy_buffer = CopyBuffer::Field(field.clone())
        }

        if let CopyBuffer::Field(copied_field) = &ctx.copy_buffer {
            if field.hash == copied_field.hash && field.field_type == copied_field.field_type {
                if ui.add(egui::Button::new("Paste")).clicked() {
                    field.value = copied_field.value.clone();
                    // *ctx.copy_buffer = None; 
                }
            }
        }
    });
    if label_clicked {
        let next = !is_open;
        //ui.data_mut(|d| d.insert_temp(id, next));
        header_resp.set_open(next);
    }

    header_resp.body(|ui| field.value.edit(ui, ctx));
}


fn collapsing_header_with_buttons_for_array_member(
    ui: &mut Ui,
    id: egui::Id,
    default_open: Option<bool>,
    label: impl Into<eframe::egui::WidgetText>,
    member: &mut FieldValue,
    ctx: &mut EditContext,
) {
    let state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(), 
        id, 
        default_open.unwrap_or(false),
    );
    let is_open = state.is_open();
    let mut label_clicked = false;
    let mut header_resp = state.show_header(ui, |ui| {
        let label_res = ui.add(egui::Button::new(label).frame(false));
        label_clicked = label_res.clicked();

        ui.add_space(10.0);
        if let FieldValue::Class(member_class) = member {
            if ui.small_button("Copy").clicked() {
                *ctx.copy_buffer = CopyBuffer::Array(*member_class.clone());
            }

            if let CopyBuffer::Array(copied_class) = &ctx.copy_buffer {
                if copied_class.hash == member_class.hash {
                    if ui.add(egui::Button::new("Paste")).clicked() {
                        *member_class = copied_class.clone().into();
                        // *ctx.copy_buffer = None;
                    }
                }
            }
        }
        label_clicked
    });

    if label_clicked {
        let next = !is_open;
        header_resp.set_open(next);
    }
    header_resp.body(|ui| member.edit(ui, ctx));
}


fn edit_enum_from_field(value: &mut i32, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
    let enum_type = ctx.field_info.and_then(|field_info| ctx.type_map.enums.get(&field_info.original_type));
    let enum_str = enum_type.and_then(|e| e.get(&value.to_string()));
    let enum_text = ctx.field_info.and_then(|field_info| {
        let x = enum_str.and_then(|enum_str| ctx.type_map.get_enum_text(enum_str, &field_info.original_type, ctx.language));
        x
    });
    match enum_str {
        Some(mut enum_str) => {
            let enum_type = enum_type.unwrap();
            let mut enum_list = enum_type.iter()
                .filter_map(|x| if x.0.parse::<i32>().is_ok() {Some(x.1)} else {None})
                .collect::<Vec<_>>();
            let preview = if let Some(enum_text) = enum_text {
                format!("{enum_str}:\"{enum_text}\"")
            } else { enum_str.to_string() };
            enum_list.sort();
            ComboBox::from_id_salt(ctx.id)
                .selected_text(preview)
                .show_ui(ui, |ui|{
                    for option in enum_list {
                        let enum_text = ctx.field_info.and_then(|field_info| {
                            ctx.type_map.get_enum_text(&option, &field_info.original_type, ctx.language)
                        });

                        let preview = if let Some(enum_text) = enum_text {
                            format!("{option}:\"{enum_text}\"")
                        } else { option.to_string() };
                        ui.selectable_value(&mut enum_str, &option, preview);
                    }
                    if let Some(x) = enum_type.get(enum_str).and_then(|e| e.parse::<i32>().ok()) {
                        *value = x;
                    }
                });
        }
        None => {
            value.edit(ui, ctx);
        }
    }
    EditResponse::default()
}

fn edit_enum_from_type(value: &mut i32, enum_type_str: &str, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
    let enum_type = ctx.type_map.enums.get(enum_type_str);
    let enum_str = enum_type.and_then(|e| e.get(&value.to_string()));
    let enum_text = enum_str.and_then(|enum_str| ctx.type_map.get_enum_text(enum_str, enum_type_str, ctx.language));
    match enum_str {
        Some(mut enum_str) => {
            let enum_type = enum_type.unwrap();
            let mut enum_list = enum_type.iter()
                .filter_map(|x| if x.0.parse::<i32>().is_ok() {Some(x.1)} else {None})
                .collect::<Vec<_>>();
            enum_list.sort();

            let preview = if let Some(ref enum_text) = enum_text {
                format!("{enum_str}:\"{enum_text}\"")
            } else { enum_str.to_string() };
            ComboBox::from_id_salt(ctx.id)
                .selected_text(preview)
                .show_ui(ui, |ui|{
                    for option in enum_list {
                        let enum_text = ctx.type_map.get_enum_text(option, enum_type_str, ctx.language);
                        let preview = if let Some(ref enum_text) = enum_text {
                            format!("{option}:\"{enum_text}\"")
                        } else { option.to_string() };
                        ui.selectable_value(&mut enum_str, &option, preview);
                    }
                    if let Some(x) = enum_type.get(enum_str).and_then(|e| e.parse::<i32>().ok()) {
                        *value = x;
                    }
                });
        }
        None => {
            value.edit(ui, ctx);
        }
    }
    EditResponse::default()
}

pub fn get_enum_preview<T: Display>(v: T, enum_type_str: &str, ctx: &mut EditContext) -> Option<String> {
    let enum_str = ctx.type_map.get_enum_str(v, enum_type_str)?;
    let enum_text = ctx.type_map.get_enum_text(enum_str, enum_type_str, ctx.language);
    Some(match enum_text {
        Some(text) => format!("{enum_str}:{text}"),
        None => enum_str.to_string(),
    })
}

impl FieldValue {
    pub fn get_preview(&self, ctx: &mut EditContext) -> Option<String> {
        match self {
            FieldValue::Enum(v) => {
                ctx.field_info.and_then(|field_info| {
                    let enum_str = ctx.type_map.get_enum_str(v, &field_info.original_type)?;
                    let enum_text = ctx.type_map.get_enum_text(enum_str, &field_info.original_type, ctx.language);
                    Some(match enum_text {
                        Some(text) => format!("{enum_str}:{text}"),
                        None => enum_str.to_string(),
                    })
                })
            },
            FieldValue::S8(v) => Some(v.to_string()),
            FieldValue::U8(v) => Some(v.to_string()),
            FieldValue::S16(v) => Some(v.to_string()),
            FieldValue::U16(v) => Some(v.to_string()),
            FieldValue::S32(v) => Some(v.to_string()),
            FieldValue::U32(v) => Some(v.to_string()),
            FieldValue::S64(v) => Some(v.to_string()),
            FieldValue::U64(v) => Some(v.to_string()),
            FieldValue::F32(v) => Some(v.to_string()),
            FieldValue::F64(v) => Some(v.to_string()),
            FieldValue::C8(v) => Some(v.to_string()),
            FieldValue::C16(v) => Some(v.to_string()),
            FieldValue::Class(c) => c.get_preview(ctx),
            FieldValue::Array(a) => {
                a.values[0].get_preview(ctx).map(|s| {
                    format!("[0]={}", s)
                })
            }
            FieldValue::String(v) => Some(v.to_string()),
            FieldValue::Struct(v) => {
                match ctx.field_info.map(|s| s.name.as_str()).unwrap_or("") {
                    "via.rds.Mandrake" => {
                        if let Ok(v) = Mandrake::try_from(v.as_ref()) {
                            return Some(v.get().map(|x| x.to_string()).unwrap_or("Invalid".to_string()))
                        }
                    }
                    _ => { }
                }
                None
            },
            _ => {
                None
            }
        }
    }
}

impl Editable for FieldValue {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        match self {
            FieldValue::Enum(v) => edit_enum_from_field(v, ui, ctx),
            FieldValue::S8(v) => v.edit(ui, ctx),
            FieldValue::U8(v) => v.edit(ui, ctx),
            FieldValue::S16(v) => v.edit(ui, ctx),
            FieldValue::U16(v) => v.edit(ui, ctx),
            FieldValue::S32(v) => v.edit(ui, ctx),
            FieldValue::U32(v) => v.edit(ui, ctx),
            FieldValue::S64(v) => v.edit(ui, ctx),
            FieldValue::U64(v) => v.edit(ui, ctx),
            FieldValue::F32(v) => v.edit(ui, ctx),
            FieldValue::F64(v) => v.edit(ui, ctx),
            FieldValue::C8(v) => v.edit(ui, ctx),
            FieldValue::C16(v) => v.edit(ui, ctx),
            FieldValue::Class(c) => c.edit(ui, ctx),
            FieldValue::Array(a) => a.edit(ui, ctx),
            FieldValue::String(v) => v.edit(ui, ctx),
            FieldValue::Struct(v) => v.edit(ui, ctx),
            _ => {
                ui.label(format!("{:?}", self));
                EditResponse::default()
            }
        }
    }
}

// This is the most digusting code i might've written
// actually no that's not true
// there's other stuff that's worse
// this is just a mix of me and gemini cooking stupid shit up
// but it works kinda decent so its chill
// TODO: Fix the stupid thing where subsequent scroll areas are smaller than the first
impl Editable for Array {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        ui.push_id(ctx.id, |ui| {
            let is_search_active = (!ctx.search_paths.is_empty() || !ctx.search_leaf_nodes.is_empty()) && !ctx.search_found_leaf;
            let is_path = ctx.field_info.map(|f| ctx.search_paths.contains(&(f.type_hash, f.hash))).unwrap_or(false);
            let expand = is_search_active && is_path;
            let default_open = if expand { Some(true) } else { None };

            let array_type = self.array_type;

            ui.scope(|ui| {
                let target_height = 22.0;
                ui.style_mut().spacing.item_spacing.y = 6.0;
                ui.style_mut().spacing.interact_size.y = target_height;
                let spacing = ui.style().spacing.item_spacing.y;

                let state_id = ui.make_persistent_id("row_heights");
                let mut row_heights = ui.data_mut(|d| {
                    d.get_temp::<Vec<f32>>(state_id).unwrap_or_default()
                });

                if row_heights.len() != self.values.len() {
                    row_heights.resize(self.values.len(), target_height);
                }
                let (visible_sum, visible_count): (_, u32) = row_heights.iter().enumerate()
                                                   .filter(|(i, _)| ctx.search_range.contains(i))
                                                   .fold((0.0, 0), |(acc_h, acc_c), (_, h)| (acc_h + h, acc_c + 1));

                let total_content_height = visible_sum + (visible_count.saturating_sub(1) as f32 * spacing);

                let max_height = if ctx.depth == 0 {
                    let h = ui.available_height();
                    if h.is_infinite() { 800.0 } else { h - 10.0 }
                } else {
                    400.0 
                };

                // Clamp: Be at least 40px, but no taller than max_height
                let view_height = total_content_height.clamp(40.0, max_height);

                eframe::egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .min_scrolled_height(view_height)
                            .max_width(ui.available_width() * 0.9)
                            .show(ui, |ui| {
                                let clip_rect = ui.clip_rect();
                                let mut current_y = ui.cursor().min.y;
                                // Determine the last index in our filter slice so we don't add extra spacing at the end
                                let last_visible_index = ctx.search_range.clone().last()
                                    .unwrap_or(0)
                                    .min(self.values.len().saturating_sub(1));

                                for (i, member) in self.values.iter_mut().enumerate() {
                                    if !ctx.search_range.contains(&i) { continue; }

                                    let cached_h = row_heights[i];
                                    let add_spacing = if i < last_visible_index { spacing } else { 0.0 };
                                    if current_y + cached_h < clip_rect.min.y {
                                        ui.add_space(cached_h + add_spacing);
                                        current_y += cached_h + add_spacing;
                                        continue;
                                    }

                                    if current_y > clip_rect.max.y + 200.0 {
                                        let (rem_h, rem_c): (_, u32) = row_heights[i..].iter().enumerate()
                                                             .filter(|(offset, _)| ctx.search_range.contains(&(i + offset)))
                                                             .fold((0.0, 0), |(acc_h, acc_c), (_, h)| (acc_h + h, acc_c + 1));

                                        let rem_spacing = rem_c.saturating_sub(1) as f32 * spacing;
                                        ui.add_space(rem_h + rem_spacing);
                                        break; 
                                    }


                                    let start_pos = ui.cursor().min;

                                    if default_open.is_some() || !is_search_active {
                                        let label = member.get_preview(ctx).unwrap_or("".to_string());
                                        let label = format!("{i}: {label}");

                                        let child_id = ui.make_persistent_id((i, ctx.id));
                                        let mut child_ctx = EditContext { 
                                            id: child_id.value(), 
                                            array_index: Some(i), 
                                            depth: ctx.depth + 1,
                                            copy_buffer: ctx.copy_buffer,
                                            ..*ctx 
                                        };

                                        match array_type {
                                            ArrayType::Class => {
                                                let id_salt = if expand { (i, "search_mode", ctx.id) } else { (i, "normal", ctx.id) };
                                                let header_id = ui.make_persistent_id(id_salt);
                                                collapsing_header_with_buttons_for_array_member(ui, header_id, default_open, label, member, &mut child_ctx);
                                            }
                                            ArrayType::Value => {
                                                ui.horizontal(|ui| {
                                                    ui.label(label);
                                                    member.edit(ui, &mut child_ctx);
                                                });
                                            }
                                        }
                                    }

                                    // Update Cache
                                    let actual_height = ui.cursor().min.y - start_pos.y;
                                    if (row_heights[i] - actual_height).abs() > 0.1 {
                                        row_heights[i] = actual_height;
                                    }
                                    current_y += actual_height + add_spacing;
                                }
                            });
                    });
                ui.data_mut(|d| d.insert_temp(state_id, row_heights));
            });
            EditResponse::default()
        }).inner
    }
}


impl Editable for Field {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        //let type_info = ctx.type_map.get_by_hash(self.hash);
        let field_info = ctx.parent_type.and_then(|x| x.get_by_hash(self.hash));
        let name: String = field_info.map(|x| x.name.clone())
            .unwrap_or_else(|| {
                if let Some(n) = ctx.type_map.get_hash_str(self.hash) {
                    n.clone()
                } else {
                    format!("{:08x}", self.hash)
                }
            });
        let og_type = field_info.map(|f| f.original_type.clone())
            .unwrap_or_else(|| {
                format!("{:?}", self.field_type)
            });
        let array_brackets = field_info.map(|f| f.array).unwrap_or(false);
        let array_brackets = if array_brackets {"[]"} else {""};

        let cur_type = field_info.and_then(|f| f.get_original_type(ctx.type_map));
        let name_contained = format!("{name}: {og_type}{array_brackets}");

        let is_leaf = field_info.map(|field_info| ctx.search_leaf_nodes.contains(&(field_info.type_hash, self.hash))).unwrap_or(false);
        let is_path = field_info.map(|field_info| ctx.search_paths.contains(&(field_info.type_hash, self.hash))).unwrap_or(false);
        let has_search = !ctx.search_paths.is_empty() || !ctx.search_leaf_nodes.is_empty();

        let show = !has_search || ctx.search_found_leaf || is_leaf || is_path;
        if !show {
            return EditResponse::default();
        }

        let expand = has_search && !ctx.search_found_leaf && is_path;
        let id_salt = if has_search && !ctx.search_found_leaf { (self.hash, "search_mode", ctx.id) } else { (self.hash, "normal", ctx.id) };
        let default_open = if expand { Some(true) } else { None };

        let child_id = ui.make_persistent_id((self.hash, ctx.id));
        let mut child_ctx = EditContext {
            copy_buffer: ctx.copy_buffer,
            cur_type,
            field_info,
            id: child_id.value(),
            depth: ctx.depth + 1,
            search_found_leaf: ctx.search_found_leaf || is_leaf,
            ..*ctx
        };

        match &self.field_type {
            FieldType::Array | FieldType::Class => {
                let header_id = ui.make_persistent_id(id_salt);
                collapsing_header_with_buttons_for_field(ui, header_id, default_open, name_contained, self, &mut child_ctx);
            }
            FieldType::Struct => {
                match og_type.as_str() {
                    "via.rds.Mandrake" => {
                        let header = CollapsingHeader::new(name_contained)
                            .id_salt(id_salt);
                        let header = if let Some(state) = default_open { header.default_open(state) } else { header };
                        header.show(ui, |ui| {
                            self.value.edit(ui, &mut child_ctx);
                        });
                    },
                    _ => {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            self.value.edit(ui, &mut child_ctx);
                            ui.label(format!("{og_type}"));
                        });
                    }
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label(name);
                    self.value.edit(ui, &mut child_ctx);
                    ui.label(format!("{og_type}"));
                    //response.unite(child_resp);
                });
            }
        }
        EditResponse::default()
    }
}


impl Class {
    fn edit_as_bitset(&mut self, generic: Option<&str>, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {

        //TODO: can do something like self.edit_field_and_get("_MaxElement");
        let Some(max_element) = self.get_mut::<&mut i32>("_MaxElement") else {
            return EditResponse::default() 
        };
        ui.horizontal(|ui| {
            ui.label("_MaxElement");
            max_element.edit(ui, ctx);
        });
        let max_element = *max_element as usize;

        let Some(mut values) = self.get_mut::<&mut Array>("_Value") else {
            return EditResponse::default();
        };

        let edit_bitset = |ui: &mut Ui, a: &mut Array, generic: Option<&str>| {
            ui.horizontal(|ui| {
                let mut count = 0;
                if ui.small_button("Check All").clicked() {
                    for v in &mut a.values {
                        if count >= max_element { break; }
                        let bits = (max_element - count).min(32);
                        let mask = if bits == 32 {
                            u32::MAX
                        } else {
                            (1u32 << bits) - 1
                        };
                        if let FieldValue::U32(x) = v { *x = mask; }
                        count += bits;
                    }
                }
                if ui.small_button("Uncheck All").clicked() {
                    for v in &mut a.values {
                        if let FieldValue::U32(x) = v { *x = 0; }
                    }
                }
            });
            ui.separator();

            let enums = generic.and_then(|g| {
                if let Some((base_type, _)) = g.split_once("<") {
                    ctx.type_map.enums.get(base_type)
                } else {
                    ctx.type_map.enums.get(g)
                }
            });
            println!("{enums:?}");
            let mut count = 0usize;
            for v in &mut a.values {
                let Some(x) = v.as_u32_mut() else { continue };
                if count >= max_element { break; }
                for i in 0..32 {
                    if count + i >= max_element { break; }
                    let mut bit_val = *x & (1 << i) != 0;
                    if let Some(e) = enums.as_ref().and_then(|x| x.get(&(count + i).to_string())) {
                        let enum_text = generic.and_then(|generic| {
                            ctx.type_map.get_enum_text(e, generic, ctx.language)
                        });
                        if let Some(et) = enum_text {
                            ui.checkbox(&mut bit_val, format!("idx={}({e}:{et})", count + i));
                        } else {
                            ui.checkbox(&mut bit_val, format!("idx={}({e})", count + i));
                        }
                    } else {
                        ui.checkbox(&mut bit_val, format!("{}", count + i));
                    }
                    x.set_bit(i as usize, bit_val);
                }
                count += 32;
            }
        };


        let edit_outer_armor_flags = |ui: &mut Ui, a: &mut Array| {
            let (gender_enum, series_enum, part_enum) = {
                (
                    ctx.type_map.enums.get("app.CharacterDef.GENDER"),
                    ctx.type_map.enums.get("app.ArmorDef.SERIES"),
                    ctx.type_map.enums.get("app.ArmorDef.ARMOR_PARTS"),
                )
            };


            ui.horizontal(|ui| {
                let mut count = 0;
                if ui.small_button("Check All").clicked() {
                    for v in &mut a.values {
                        if count >= max_element { break; }
                        let bits = (max_element - count).min(32);
                        let mask = if bits == 32 {
                            u32::MAX
                        } else {
                            (1u32 << bits) - 1
                        };
                        if let FieldValue::U32(x) = v { *x = mask; }
                        count += bits;
                    }
                }
                if ui.small_button("Uncheck All").clicked() {
                    for v in &mut a.values {
                        if let FieldValue::U32(x) = v { *x = 0; }
                    }
                }
            });
            ui.separator();

            let mut count = 0usize;
            for v in &mut a.values {
                let Some(x) = v.as_u32_mut() else {
                    continue;
                };
                if count >= max_element { break; }
                for i in 0..32 {
                    let idx = count + i;
                    if idx >= max_element { break; }
                    let mut bit_val = *x & (1 << i) != 0;
                    let part = idx % 5;
                    let series = (idx/5) % (0x4b5/5);
                    let gender = idx / 0x4b5;
                    let gender = gender_enum.map(|gender_enum| {
                        let enum_str = gender_enum.get(&gender.to_string());
                        match enum_str {
                            None => gender.to_string(),
                            Some(enum_str) => ctx.type_map.get_enum_text(enum_str, "app.CharacterDef.GENDER", ctx.language)
                                .unwrap_or(enum_str.clone())
                        }
                    }).unwrap_or(gender.to_string());

                    let mut specific = None;
                    if let Some(series_enum) = series_enum {
                        if let Some(series) = series_enum.get(&series.to_string()) {
                            if let Some(part_enum) = part_enum {
                                if let Some(part) = part_enum.get(&part.to_string()) {
                                    specific = ctx.type_map.get_enum_text(&format!("{series}{part}"), "app.ArmorDef.SpecificPiece", ctx.language);
                                }
                            }
                        }
                    }
                    if let Some(specific) = specific {
                        ui.checkbox(&mut bit_val, format!("idx={idx}:({gender}:\"{specific}\")"));
                        x.set_bit(i as usize, bit_val);

                    } else {
                        let series = series_enum.map(|series_enum| {
                            let enum_str = series_enum.get(&series.to_string());
                            match enum_str {
                                None => series.to_string(),
                                Some(enum_str) => ctx.type_map.get_enum_text(enum_str, "app.ArmorDef.SERIES", ctx.language)
                                    .unwrap_or(enum_str.clone())
                            }
                        }).unwrap_or(series.to_string());

                        let part = part_enum.map(|part_enum| {
                            let enum_str = part_enum.get(&part.to_string());
                            match enum_str {
                                None => part.to_string(),
                                Some(enum_str) => ctx.type_map.get_enum_text(enum_str, "app.ArmorDef.ARMOR_PARTS", ctx.language)
                                    .unwrap_or(enum_str.clone())
                            }
                        }).unwrap_or(part.to_string());
                        ui.checkbox(&mut bit_val, format!("idx={idx}:({gender}:\"{series}\":{part})"));
                        x.set_bit(i as usize, bit_val);
                    }
                }
                count += 32;
            }
        };


        // hopefully this scrollable area works alright?
        CollapsingHeader::new("_Value")
            .show(ui, |ui| {
                eframe::egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .inner_margin(4.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .max_width(ui.available_width() * 0.9)
                            .min_scrolled_height(300.0)
                            .show(ui, |ui| {
                                match generic {
                                    Some("Custom.OuterArmorFlags") => edit_outer_armor_flags(ui, &mut values),
                                    Some(_) | None => edit_bitset(ui, &mut values, generic)
                    }
                            });
                    });
            });
        EditResponse::default()
    }


    fn udpate_ctx<'a>(type_info: Option<&'a TypeInfo>, id: u64, ctx: &'a mut EditContext) -> EditContext<'a> {
        let child_ctx = EditContext {
            copy_buffer: ctx.copy_buffer,
            id,
            parent_type: type_info,
            depth: ctx.depth + 1,
            ..*ctx
        };
        child_ctx
    }

    fn add_enum_edit(&mut self, type_info: Option<&TypeInfo>, field_name: &str, enum_type_str: &str, ui: &mut Ui, ctx: &mut EditContext, left: &mut HashSet<u32>) -> EditResponse {
        self.add_enum_edit_ex(type_info, field_name, field_name, enum_type_str, ui, ctx, left)
    }

    fn add_enum_edit_ex(&mut self, type_info: Option<&TypeInfo>, field_name: &str, label: &str, enum_type_str: &str, ui: &mut Ui, ctx: &mut EditContext, left: &mut HashSet<u32>) -> EditResponse {
        let hash = &murmur3(field_name, 0xffffffff);
        if let Some(field) = self.fields.get_mut(hash) {
            match field.value {
                FieldValue::S32(mut val) => {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let child_id = ui.make_persistent_id(hash).value();
                        let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                        edit_enum_from_type(&mut val, enum_type_str, ui, &mut ctx);
                        field.value = FieldValue::S32(val);
                    });
                    left.remove(hash);
                }
                FieldValue::U16(val) => {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let child_id = ui.make_persistent_id(hash).value();
                        let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                        let mut val_i32 = val as i32;
                        edit_enum_from_type(&mut val_i32, enum_type_str, ui, &mut ctx);
                        field.value = FieldValue::U16(val_i32 as u16);
                    });
                    left.remove(hash);
                }
                FieldValue::S16(val) => {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let child_id = ui.make_persistent_id(hash).value();
                        let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                        let mut val_i32 = val as i32;
                        edit_enum_from_type(&mut val_i32, enum_type_str, ui, &mut ctx);
                        field.value = FieldValue::S16(val_i32 as i16);
                    });
                    left.remove(hash);
                }
                FieldValue::U32(val) => {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let child_id = ui.make_persistent_id(hash).value();
                        let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                        let mut val_i32 = val as i32;
                        edit_enum_from_type(&mut val_i32, enum_type_str, ui, &mut ctx);
                        field.value = FieldValue::U32(val_i32 as u32);
                    });
                    left.remove(hash);
                }
                _ => { }
            }
        }
        EditResponse::default()
    }

    fn get_enum_str_from_field(&self, field_name: &str, enum_type_str: &str, ctx: &mut EditContext) -> Option<String>{
        let value: i32 = self.get(field_name)?;
        ctx.type_map.get_enum_str(value, enum_type_str).cloned()
    }

    fn get_enum_preview_from_field(&self, field_name: &str, enum_type_str: &str, ctx: &mut EditContext) -> Option<String>{
        let field_value = self.get_value(field_name)?;
        match field_value {
            FieldValue::S32(val) => {
                return ctx.type_map.get_enum_str(val, enum_type_str)
                    .map(|s| {
                        match get_enum_preview(val, enum_type_str, ctx) {
                            Some(text) => text,
                            None => s.clone(),
                        }
                    });
            }
            FieldValue::U16(val) => {
                return ctx.type_map.get_enum_str(val, enum_type_str)
                    .map(|s| {
                        match get_enum_preview(val, enum_type_str, ctx) {
                            Some(text) => text,
                            None => s.clone(),
                        }
                    });
            }
            _ => {

            }
        }
        None
    }
    // Category_Gender is a bitset of app.EquipDef.CATEGORY and some kinda gender thing i think, as
    // well as an empty flag in bit 8
    // What the fuck is this function holy shit it works but wtf is this commplete garbage, and
    // whhy does crapcom use a mix of fixed and not fixed and wtf is up with that stupid decimal
    // encoding, do they not know what numbers are?
    fn edit_as_equip_work(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        let mut left = HashSet::new();
        for i in self.fields.keys() {
            left.insert(*i);
        }

        let type_info = ctx.type_map.get_by_hash(self.hash);
        let mut category = 141;
        if let Some(cat_gen_val) = self.get_field_mut("Category_Gender") {
            ui.horizontal(|ui| {
                ui.label("Category_Gender: ");
                cat_gen_val.value.edit(ui, ctx);
                category = cat_gen_val.value.as_u8().unwrap_or(141);
            });
            left.remove(&cat_gen_val.hash);
        };

        // No _Fixed here?
        match category {
            // armor ?, lsb is gender
            0b0000_0000 | 0b0000_0001 => {
                self.add_enum_edit_ex(type_info, "FreeVal0", "FreeVal0(SeriesID)", "app.ArmorDef.SERIES", ui, ctx, &mut left);
                self.add_enum_edit_ex(type_info, "FreeVal1", "FreeVal1(ArmorPart)", "app.ArmorDef.ARMOR_PARTS", ui, ctx, &mut left);
            },
            // Weapon 13
            0b0000_1101 => {
                self.add_enum_edit_ex(type_info, "FreeVal0", "FreeVal0(WeaponType)", "app.WeaponDef.TYPE", ui, ctx, &mut left);
                let val = self.get_enum_str_from_field("FreeVal0", "app.WeaponDef.TYPE", ctx);
                if let Some(val) = val {
                    let enum_type = format!("app.WeaponDef.{}Id", util::to_pascal_case(val.as_str()).replace("_", ""));
                    self.add_enum_edit_ex(type_info, "FreeVal1", "FreeVal1(WeaponID)", &enum_type, ui, ctx, &mut left);
                    let artian_perf_type = self.get_enum_str_from_field("FreeVal2", "app.ArtianDef.PERFORMANCE_TYPE_Fixed", ctx);
                    self.add_enum_edit_ex(type_info, "FreeVal2", "FreeVal2(PerformanceType)", "app.ArtianDef.PERFORMANCE_TYPE_Fixed", ui, ctx, &mut left);
                    if Some("INVALID".to_string()) != artian_perf_type {
                        let hash = &murmur3("BonusByCreating", 0xffffffff);
                        if let Some(bonus_by_creating_mut) = self.get_mut::<&mut u32>("BonusByCreating") {
                            let bonus_by_creating = *bonus_by_creating_mut;
                            let skill_digit_1 = (bonus_by_creating / 10_000_000) % 10;
                            let skill_digit_2 = (bonus_by_creating / 10_000) % 10;
                            let skill_digit_3 = (bonus_by_creating / 10) % 10;
                            let mut artian_skill_fixed = ((skill_digit_1 * 100) + (skill_digit_2 * 10) + skill_digit_3) as i32;
                            let mut bonus_type_1 = (bonus_by_creating / 1_000_000) % 10;
                            let mut bonus_type_2 = (bonus_by_creating / 1_000) % 10;
                            let mut bonus_type_3 = bonus_by_creating % 10;
                            ui.horizontal(|ui| {
                                ui.label("Skills");
                                let child_id = ui.make_persistent_id((ctx.id, "Skills", hash)).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut artian_skill_fixed, "app.ArtianDef.ArtianSkillType_Fixed", ui, &mut child_ctx);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Bonuses(Atk=1/Aff=2/Elem=3(ithink))");
                                bonus_type_1.edit(ui, ctx);
                                bonus_type_2.edit(ui, ctx);
                                bonus_type_3.edit(ui, ctx);
                            });
                            let bonus_by_creating = (artian_skill_fixed / 100)  as u32* 10_000_000
                                + bonus_type_1 * 1_000_000
                                + ((artian_skill_fixed / 10) % 10) as u32 * 10_000
                                + bonus_type_2 * 1_000
                                + (artian_skill_fixed % 10) as u32 * 10
                                + bonus_type_3;
                            *bonus_by_creating_mut = bonus_by_creating;
                            left.remove(hash);
                        }
                        if let Some(bonus_by_grinding_mut) = self.get_mut::<&mut u64>("BonusByGrinding") {
                            let bonus_by_grinding = *bonus_by_grinding_mut;
                            let mut bonus_1 = ((bonus_by_grinding / 1_000_000_000_000)  % 1000) as i32;
                            let mut bonus_2 = ((bonus_by_grinding / 1_000_000_000)  % 1000) as i32;
                            let mut bonus_3 = ((bonus_by_grinding / 1_000_000)  % 1000) as i32;
                            let mut bonus_4 = ((bonus_by_grinding / 1_000)  % 1000) as i32;
                            let mut bonus_5 = (bonus_by_grinding  % 1000) as i32;
                            ui.label("BonusesByGrinding");
                            // SO GROSS BUT IT WORKS HAHA HA
                            ui.indent((hash, ctx.id), |ui| {
                                let child_id = ui.make_persistent_id((hash, ctx.id, "Bonus1")).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut bonus_1, "app.ArtianDef.BONUS_ID_Fixed", ui, &mut child_ctx);
                                let child_id = ui.make_persistent_id((hash, ctx.id, "Bonus2")).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut bonus_2, "app.ArtianDef.BONUS_ID_Fixed", ui, &mut child_ctx);
                                let child_id = ui.make_persistent_id((hash, ctx.id, "Bonus3")).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut bonus_3, "app.ArtianDef.BONUS_ID_Fixed", ui, &mut child_ctx);
                                let child_id = ui.make_persistent_id((hash, ctx.id, "Bonus4")).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut bonus_4, "app.ArtianDef.BONUS_ID_Fixed", ui, &mut child_ctx);
                                let child_id = ui.make_persistent_id((hash, ctx.id, "Bonus5")).value();
                                let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                                edit_enum_from_type(&mut bonus_5, "app.ArtianDef.BONUS_ID_Fixed", ui, &mut child_ctx);
                            });
                            let bonus_by_grind: u64 = bonus_1 as u64 * 1_000_000_000_000
                                + bonus_2 as u64 * 1_000_000_000
                                + bonus_3 as u64 * 1_000_000
                                + bonus_4 as u64 * 1_000
                                + bonus_5 as u64;
                            *bonus_by_grinding_mut = bonus_by_grind; 
                            left.remove(hash);
                        }
                    }
                }
            }
            // idfk 21
            0b0001_0101 => {
                self.add_enum_edit_ex(type_info, "FreeVal0", "FreeVal0(AmuletType)", "app.ArmorDef.AmuletType", ui, ctx, &mut left);
            }
            // idfk again 37
            0b0010_0101 => {

            }
            _ => {

            }
        };

        for (h, field) in self.fields.iter_mut() {
            if left.contains(h) {
                let child_id = ui.make_persistent_id(h).value();
                let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                field.edit(ui, &mut ctx);
            }
        }

        EditResponse::default()
    }

    // this could potentially return the name of the type for arrays
    pub fn get_preview_from_field(&self, field_name: &str, ctx: &mut EditContext) -> Option<String> {
        self.fields.get(&murmur3(field_name, 0xffffffff)).and_then(|x| {
            x.value.get_preview(ctx)
        })
    }

    pub fn get_preview_from_remap(&self, remap: &Remap, ctx: &mut EditContext) -> Option<String> {
        if let Some(field) = self.get_field(&remap.preview) {
            return match remap.remap.get(&remap.preview) {
                Some(enum_type) => self.get_enum_preview_from_field(&remap.preview, enum_type, ctx),
                None => field.value.get_preview(ctx)
            }
        }
        None
    }

    fn edit_as_remapped(&mut self, remap: &Remap, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        let mut left = HashSet::new();
        for i in self.fields.keys() {
            left.insert(*i);
        }

        let type_info = ctx.type_map.get_by_hash(self.hash);
        for (k, v) in &remap.remap {
            self.add_enum_edit(type_info, k, v, ui, ctx, &mut left);
        }

        for (h, field) in self.fields.iter_mut() {
            if left.contains(h) {
                let child_id = ui.make_persistent_id(h).value();
                let mut ctx = Class::udpate_ctx(type_info, child_id, ctx);
                field.edit(ui, &mut ctx);
            }
        }

        EditResponse::default()
    }

    pub fn get_preview(&self, ctx: &mut EditContext) -> Option<String> {
        let type_info = ctx.type_map.get_by_hash(self.hash);
        type_info.and_then(|type_info| {
            if let Some(v) = ctx.remaps.get(&type_info.name) {
                let r = self.get_preview_from_remap(v, ctx);
                return r;
            }
            match type_info.name.as_str() {
                "app.savedata.cEquipWork" => {
                    if let Some(cat_gen_val) = self.get::<u8>("Category_Gender") {
                        let is_empty = cat_gen_val & 0x80 != 0;
                        if is_empty {
                            return Some("Empty".to_string())
                        }
                        let gender = cat_gen_val & 0b1;
                        match cat_gen_val {
                            // armor ?, lsb is gender
                            0b0000_0000 | 0b0000_0001 => {
                                let gender = if gender == 1 {"female"} else {"male"};
                                let preview_series = self.get_enum_str_from_field("FreeVal0", "app.ArmorDef.SERIES", ctx);
                                let preview_type = self.get_enum_str_from_field("FreeVal1", "app.ArmorDef.ARMOR_PARTS", ctx);
                                let preview = preview_series.and_then(|ser| {
                                    if let Some(ty) = preview_type {
                                        Some(ctx.type_map.get_enum_text(format!("{ser}{ty}").as_str(), "app.ArmorDef.SpecificPiece", ctx.language)
                                            .unwrap_or(format!("{ser}:{ty}")))
                                    } else {
                                        Some(ser)
                                    }
                                });
                                if let Some(preview) = preview {
                                    return Some(format!("Armor({gender}): {preview}"))
                                } else {
                                    return Some(format!("Armor({gender})"))
                                }
                            },
                            // Weapon 13
                            0b0000_1101 => {
                                let preview_type = self.get_enum_str_from_field("FreeVal0", "app.WeaponDef.TYPE", ctx);
                                let preview = preview_type.and_then(|ty| {
                                    let enum_type = format!("app.WeaponDef.{}Id", util::to_pascal_case(ty.as_str()).replace("_", ""));
                                    let preview_id = self.get_enum_str_from_field("FreeVal1", &enum_type, ctx);
                                    if let Some(id) = preview_id {
                                        Some(ctx.type_map.get_enum_text(&id, &enum_type, ctx.language).map(|x| format!("{id}:{x}"))
                                            .unwrap_or(format!("{id}:{ty}")))
                                    } else {
                                        Some(ty)
                                    }
                                });
                                if let Some(preview) = preview {
                                    return Some(format!("Weapon: {preview}"))
                                } else {
                                    return Some(format!("Weapon"))
                                }
                            },
                            // idfk 21
                            0b0001_0101 => {
                                let preview_id = self.get_enum_str_from_field("FreeVal0", "app.ArmorDef.AmuletType", ctx);
                                let preview = preview_id.and_then(|preview_id| {
                                    Some(ctx.type_map.get_enum_text(&preview_id, "app.ArmorDef.AmuletType", ctx.language).map(|x| format!("{preview_id}:{x}"))
                                        .unwrap_or(format!("{preview_id}")))
                                });
                                if let Some(preview) = preview {
                                    return Some(format!("Amulet: {preview}"))
                                } else {
                                    return Some(format!("Amulet"))
                                }
                            },
                            // idfk again 37
                            0b0010_0101 => return Some("Bug?".to_string()),
                            _ => return Some(cat_gen_val.to_string())
                        };
                    }
                    None
                }
                _ => {
                    return None
                }
            }

        })
    }
}

impl Editable for Class {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        let type_info = ctx.type_map.get_by_hash(self.hash);
        ui.push_id(ctx.id, |ui| {
            if let Some(ti) = type_info {
                if let Some(remap) = ctx.remaps.get(&ti.name) {
                    if !remap.remap.is_empty() {
                        return self.edit_as_remapped(remap, ui, ctx);
                    }
                }
                match ti.name.as_str() {
                    "ace.Bitset" => {
                        println!("{type_info:?}");
                        let bitset = ctx.field_info.and_then(|f| {
                            let x = ctx.parent_type.and_then(|t| {
                                ctx.remaps.get(&t.name).and_then(|remap| {
                                    remap.bitsets.get(&f.name)
                                })
                            });
                            x
                        });

                        /*let generics: Vec<&str> = if let Some(start) = remapped.find('<') {
                          if let Some(end) = remapped.rfind('>') {
                          let content = &remapped[start + 1..end];
                          content.split(',').map(|s| s.trim()).collect::<Vec<&str>>()
                          } else {
                          Vec::new()
                          }
                          } else {Vec::new()};*/

                        self.edit_as_bitset(bitset.map(|x| x.as_str()), ui, ctx);
                    }
                    "app.savedata.cEquipWork" => {
                        self.edit_as_equip_work(ui, ctx);
                    },
                    _ => {
                        for (field_hash, field) in &mut self.fields {
                            let child_id = ui.make_persistent_id(field_hash);
                            let mut child_ctx = EditContext {
                                copy_buffer: ctx.copy_buffer,
                                id: child_id.value(),
                                parent_type: type_info,
                                depth: ctx.depth + 1,
                                ..*ctx
                            };
                            // need to make a get field name helper for TypeInfo
                            field.edit(ui, &mut child_ctx);
                        }

                    }
                }
            } else {
                for (field_hash, field) in &mut self.fields {
                    let child_id = ui.make_persistent_id(field_hash);
                    let mut child_ctx = EditContext {
                        copy_buffer: ctx.copy_buffer,
                        id: child_id.value(),
                        parent_type: type_info,
                        depth: ctx.depth + 1,
                        ..*ctx
                    };
                    // need to make a get field name helper for TypeInfo
                    field.edit(ui, &mut child_ctx);
                }

            }
            EditResponse::default()
        }).inner
    }
}

impl Editable for StringU16 {
    fn edit(&mut self, ui: &mut Ui, _: &mut EditContext) -> EditResponse {
        let mut s = String::from_utf16_lossy(&self.0);
        let response = ui.add(TextEdit::singleline(&mut s).clip_text(false));
        let encoded: Vec<u16> = s.encode_utf16().collect();
        *self = Self(encoded);
        EditResponse::from(response)
    }
}

impl Editable for String {
    fn edit(&mut self, ui: &mut Ui, _: &mut EditContext) -> EditResponse {
        let response = ui.add(TextEdit::singleline(self).clip_text(false));
        EditResponse::from(response)
    }
}

impl<T: Editable> Editable for Vec<T> {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        for e in self {
            e.edit(ui, ctx);
        }
        EditResponse::default()
    }
}


pub trait TryEdit<T>: Editable {
    fn try_edit(value: &mut T, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse;
}

impl TryEdit<Struct> for Mandrake {
    fn try_edit(value: &mut Struct, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        let s = Mandrake::try_from(&*value);
        if let Ok(mut s) = s {
            let r = s.edit(ui, ctx);
            value.data = s.to_buf().to_vec();
            r
        } else {
            value.edit(ui, ctx)
        }
    }
}

impl Editable for Guid {
    fn edit(&mut self, ui: &mut Ui, _: &mut EditContext) -> EditResponse {
        let mut disp = uuid::Uuid::from_bytes_le(self.0).to_string();
        let resp = ui.add(TextEdit::singleline(&mut disp).clip_text(false));
        if let Ok(edited) = uuid::Uuid::from_str(&disp) {
            self.0 = edited.to_bytes_le()
        } else {
            println!("Invalid Value for Guid");
        }
        let mut r = EditResponse::default();
        r.changed = resp.changed();
        r
    }
}

impl TryEdit<Struct> for Guid {
    fn try_edit(value: &mut Struct, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        if value.data.len() > 16 {
            return value.edit(ui, ctx)
        }
        let mut buf = [0u8; 16];
        buf[0..].copy_from_slice(&value.data);
        let mut guid = Guid(buf);
        let resp = guid.edit(ui, ctx);
        if resp.changed {
            value.data = guid.0.to_vec();
        }
        resp
    }
}

impl Editable for Struct {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        match ctx.cur_type {
            Some(t) => {
                let r = match t.name.as_str() {
                    "via.rds.Mandrake" => { 
                        Mandrake::try_edit(self, ui, ctx)
                    },
                    "System.Guid" => { 
                        Guid::try_edit(self, ui, ctx)
                    },
                    _ => {
                        self.data.edit(ui, ctx)
                    }
                };
                r
            }
            None => {
                self.data.edit(ui, ctx)
            }
        }
    }
}


impl Editable for Mandrake {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        let mut resp = EditResponse::default();
        if let Some(mut real_val) = self.get() {
            ui.horizontal(|ui| {
                ui.label("  real_value");
                let r = real_val.edit(ui, ctx);
                resp.changed = r.changed;
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
        resp
    }
}
