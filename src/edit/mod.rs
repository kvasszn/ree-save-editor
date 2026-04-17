pub mod copy;
pub mod types;

use std::{collections::HashMap, fmt::Debug};

use std::collections::HashSet;
use eframe::egui::{CollapsingHeader,  Response, Ui};
use crate::edit::copy::CopyBuffer;
use crate::sdk::asset::Assets;
use crate::sdk::type_map::{ContentLanguage, FieldInfo, TypeInfo, TypeMap};

use crate::save::remap::Remap;
use crate::save::{SaveFile, SaveFlags};

pub trait Editable {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse;
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
    pub assets: &'a Assets
}

impl<'a> EditContext<'a> {
    pub fn new(type_map: &'a TypeMap, search_paths: &'a HashSet<(u32, u32)>, search_leaf_nodes: &'a HashSet<(u32, u32)>, search_range: &'a core::ops::Range<usize>, copy_buffer: &'a mut CopyBuffer, language: ContentLanguage, remaps: &'a HashMap<String, Remap>, assets: &'a Assets) -> Self {
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
            assets
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


impl Editable for SaveFlags {
    fn edit(&mut self, ui: &mut Ui, _ctx: &mut EditContext) -> EditResponse {
        ui.vertical(|ui| {
            let flag_checkbox = |ui: &mut Ui, flags: &mut SaveFlags, flag: SaveFlags, label: &str| {
                let mut is_on = flags.contains(flag);
                if ui.checkbox(&mut is_on, label).changed() {
                    flags.set(flag, is_on);
                }
            };

            flag_checkbox(ui, self, SaveFlags::BLOWFISH, "Blowfish");
            flag_checkbox(ui, self, SaveFlags::RAW, "Raw");
            flag_checkbox(ui, self, SaveFlags::CITRUS, "Citrus");
            flag_checkbox(ui, self, SaveFlags::DEFLATE, "Deflate");
            flag_checkbox(ui, self, SaveFlags::MANDARIN, "Mandarin");
        });
        EditResponse::default()
    }
}

impl Editable for SaveFile {
    fn edit(&mut self, ui: &mut Ui, ctx: &mut EditContext) -> EditResponse {
        CollapsingHeader::new("Save Flags")
            .default_open(false)
            .show(ui, |ui| {
                self.flags.edit(ui, ctx);
            });
        for field in &mut self.fields {
            let child_id = ui.make_persistent_id(field.0);
            let mut child_ctx = EditContext {
                copy_buffer: ctx.copy_buffer,
                id: child_id.value(),
                ..*ctx
            };
            let type_name = ctx.type_map.get_by_hash(field.1.hash)
                .map(|t| t.name.clone())
                .unwrap_or(format!("{:08x}", field.1.hash));
            let field_name = ctx.type_map.get_hash_str(field.0)
                .cloned()
                .unwrap_or(format!("{:08x}", field.0));
            let header = format!("{field_name}: {type_name}");
            /*let label = match (type_info, field_name) {
              (None, Some(field_name)) => format!("{}: {}", field.0, type_info.name),
              (Some(type_info), Some(field_name)) => format!("{}: {}", field_name, type_info.name),
              _ => format!("{}: {:08x}", field.0, field.1.hash)
              };*/
            /*let header = if let Some(type_info) = type_info {
              format!("{}: {},{:08x}", field_name, type_info.name, field.1.hash)
              } else {
              if let Some(type_name) = ctx.type_map.get_hash_str(field.1.hash) {
              format!("{}: {},{:08x}", field_name, type_name, field.1.hash)
              } else {
              format!("{}: {:08x}", field_name, field.1.hash)
              }
              };*/
            CollapsingHeader::new(header)
                .id_salt(child_id)
                .default_open(true)
                .show(ui, |ui| {
                    field.1.edit(ui, &mut child_ctx);
                });
        }
        EditResponse::default()
    }
}
