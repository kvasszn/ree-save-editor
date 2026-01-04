pub mod copy;
pub mod types;

use std::{collections::HashMap, fmt::Debug};

use std::collections::HashSet;
use eframe::egui::{CollapsingHeader,  Response, Ui};
use crate::edit::copy::CopyBuffer;
use crate::sdk::type_map::{ContentLanguage, FieldInfo, TypeInfo, TypeMap};

use crate::save::remap::Remap;
use crate::save::SaveFile;

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
