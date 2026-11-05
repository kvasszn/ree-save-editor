use std::collections::HashMap;

use ree_lib::{context::EngineContext, rsz::Value};
use serde::{Deserialize, Serialize};

use crate::{copy::CopyBuffer, save::{remap::Remap, types::{Array, Class}}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathNode {
    Class(u32),
    Field(u32),
    Index(usize),
}

pub fn path_to_string(path: &Vec<PathNode>) -> String {
    let mut res = String::new();
    for node in path {
        match node {
            PathNode::Class(v) | PathNode::Field(v) => res.push_str(&v.to_string()),
            PathNode::Index(v) => res.push_str(&v.to_string()),
        }
    }
    res
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
