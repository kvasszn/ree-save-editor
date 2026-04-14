use std::{collections::HashMap};

use regex::Regex;
use serde::{Deserialize, Deserializer, de::Error};

use crate::{sdk::Guid, save::types::{Class, FieldValue, Ref}, sdk::{self, asset::Assets, type_map::{self, ContentLanguage, TypeMap}, value::Value}};

/*#[derive(Deserialize, Debug, Clone)]
pub struct Remap {
    #[serde(default)]
    pub remap: HashMap<String, String>,
    #[serde(default)]
    pub preview: String,
    #[serde(default)]
    pub bitsets: HashMap<String, String>,
}*/

#[derive(Deserialize, Debug, Clone)]
pub struct Remap {
    #[serde(default)]
    pub fields: HashMap<String, String>,
    #[serde(default, deserialize_with = "deserialize_preview_format")]
    pub format: Format,
    #[serde(default)]
    pub data: HashMap<String, ResourceKey>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ResourceKey {
    #[serde(default, deserialize_with = "deserialize_preview_format")]
    pub key: Format,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum FormatType {
    Chain(Vec<Ref>),
    Data(String),
    Literal(String),
    VarData(String, Vec<Ref>, usize),
    Enum
}

impl FormatType {
    fn parse_data(val: &str) -> Option<Self> {
        val.split_once(':').map(|x| Self::Data(x.1.to_string()))
    }
    pub fn parse_chain(chain: &str) -> Option<Self> {
        let mut values = Vec::new();
        for s in chain.split(".") {
            if let Some((l, r)) = s.split_once("[") {
                values.push(Ref::Field(l.to_string()));
                let index = r.strip_suffix("]")?
                    .parse::<usize>().ok()?;
                values.push(Ref::Index(index));
            } else {
                values.push(Ref::Field(s.to_string()));
            }
        }
        Some(Self::Chain(values))
    }

    pub fn parse_vardata(chain: &str) -> Option<Self> {
        let mut values = Vec::new();
        let mut var_name = None;
        let mut idx = None;
        for s in chain.split(".") {
            if let Some((l, r)) = s.split_once("[") {
                values.push(Ref::Field(l.to_string()));
                let inner = r.strip_suffix("]")?;
                if let Ok(index) = inner.parse::<usize>() {
                    values.push(Ref::Index(index));
                } else if var_name == None {
                    var_name = Some(inner.to_string());
                    idx = Some(values.len());
                }
            } else {
                values.push(Ref::Field(s.to_string()));
            }
        }
        Some(Self::VarData(var_name?, values, idx?))
    }
}

#[derive(Debug)]
pub enum RszDataType<'a> {
    Index(usize),
    Value(&'a Value),
}

#[derive(Debug, Clone, Default)]
pub struct Format {
    format_str: String,
    pub format: Vec<FormatType>
}

impl Format {
    pub fn eval_msg<'a>(resource_key: &ResourceKey, field: &FieldValue, field_type: &str, language: ContentLanguage, format: &Format, type_map: &TypeMap, remaps: &HashMap<String, Remap>, assets: &Assets) -> Option<&'a crate::sdk::value::Value> {
        let f = format.format.get(0)?;
        let msg = assets.get_msg(&resource_key.path)?;
        match f {
            FormatType::VarData(name, chain, idx) => {

            }
            _ => {}
        }
        None
    }

    pub fn eval_rsz<'a>(name: &str, field: &FieldValue, type_map: &TypeMap, remap: &'a Remap, assets: &'a Assets) -> Option<RszDataType<'a>> {
        let resource_key = remap.data.get(name)?;
        let rsz = assets.get_rsz(&resource_key.path)?;
        match resource_key.key.format.get(0)? {
            FormatType::VarData(var_name, chain, idx) => {
                if var_name == name {
                    let target = Value::from(field);
                    let val = rsz.get_var_index(&chain, *idx, &target, type_map)?;
                    return Some(RszDataType::Index(val));
                } else {
                    let var_val = Self::eval_rsz(var_name, field, type_map, remap, assets)?;
                    if let RszDataType::Index(var_val) = var_val {
                        let val = rsz.get_with_var(&chain, *idx, var_val, type_map)?;
                        return Some(RszDataType::Value(val));
                    }
                }
            },
            FormatType::Chain(chain) => {
                let val = rsz.get(&chain, type_map)?;
                return Some(RszDataType::Value(val));
            }
            _ => {}
        }
        None
    }

    pub fn eval(field: &FieldValue, field_type: &str, language: ContentLanguage,format: &Format, type_map: &TypeMap, remaps: &HashMap<String, Remap>, assets: &Assets) -> Option<String> {
        let mut string = String::new();
        use std::fmt::Write;
        for format in &format.format {
            match format {
                FormatType::Chain(c) => {
                    let class = field.get::<&Class>()?;
                    let field_type = class.eval_refs_type(c, remaps, type_map)?;
                    let val = class.eval_refs(c)
                        .map(|v| v.to_string(&field_type, language, remaps, type_map, assets))?;
                    let _ = write!(&mut string, "{val}");
                },
                FormatType::Literal(l) => {
                    let _ = write!(&mut string, "{l}");
                },
                FormatType::Data(data_name) => {
                    if let Some(remap) = remaps.get(field_type) {
                        if let Some(data_key) = remap.data.get(data_name) {
                            // different cases
                            // one is trying to get data from a file based on the format
                            // the other is trying to find the value based on the value if this field
                            if let Some(msg) = &assets.get_msg(&data_key.path) {
                                let FormatType::Data(guid_data_name) = data_key.key.format.get(0)? else {return None};
                                let data_val = Self::eval_rsz(guid_data_name, field, type_map, remap, assets)?;
                                if let RszDataType::Value(value) = data_val {
                if let Value::Guid(guid) = value {
                                        let guid = Guid(guid.0);
                                        if let Some(entry) = msg.get_entry(&guid, language) {
                                            let _ = write!(&mut string, "{entry}");
                                        }
                                    }
                                }
                            } else if let Some(rsz) = assets.get_rsz(&data_key.path) {
                                //rsz.get(refs, type_map)
                            }
                        }
                    }
                },
                FormatType::Enum => {
                    if let Some(t) = type_map.get_enum_str(&field.to_string_basic(), field_type) {
                        let _ = write!(&mut string, "{}", t);
                    }
                }
                FormatType::VarData(_, _, _) => {
                    // var data is not meant to be printed
                    // TODO add something for it here later like a panic or None result
                    // for now just ignore it
                }
            }
        }
        Some(string)
    }

    pub fn eval_class(class: &Class, field_type: &str, language: ContentLanguage, format: &Format, type_map: &TypeMap, remaps: &HashMap<String, Remap>, assets: &Assets) -> Option<String> {
        let mut string = String::new();
        use std::fmt::Write;
        for format in &format.format {
            match format {
                FormatType::Chain(c) => {
                    let field_type = class.eval_refs_type(c, remaps, type_map)?;
                    let val = class.eval_refs(c)
                        .map(|v| v.to_string(&field_type, language, remaps, type_map, assets))?;
                    let _ = write!(&mut string, "{val}");
                },
                FormatType::Literal(l) => {
                    let _ = write!(&mut string, "{l}");
                },
                _ => { }
            }
        }
        Some(string)
    }

    pub fn parse(format_str: &str) -> Option<Self> {
        let mut format = Vec::new();
        let braces = Regex::new(r"\{([^:}]++)(?::([^}]*))?\}").unwrap();
        let mut last_end = 0;
        for brace in braces.captures_iter(format_str) {
            let whole_match = brace.get(0)?;
            let inner = brace.get(1)?;
            let literal = &format_str[last_end..whole_match.start()];
            if !literal.is_empty() {
                format.push(FormatType::Literal(literal.to_string()));
            }
            if let Some(suffix) = brace.get(2) {
                match inner.as_str() {
                    "var" => {
                        let var_data = FormatType::parse_vardata(suffix.as_str())?;
                        format.push(var_data);
                    }
                    "self" => {
                        format.push(FormatType::Data(suffix.as_str().to_string()));
                    }
                    "enum" => {
                        format.push(FormatType::Enum);
                    }
                    _ => {}
                }
            }
            else if let Some(chain) = FormatType::parse_chain(inner.as_str()) {
                format.push(chain);
            }
            last_end = whole_match.end();
        }
        let trailing_literal = &format_str[last_end..];
        if !trailing_literal.is_empty() {
            format.push(FormatType::Literal(trailing_literal.to_string()));
        }
        //println!("{format:?}");
        Some(Self {
            format_str: format_str.to_string(),
            format
        })
    }
}

fn deserialize_preview_format<'de, D>(deserializer: D) -> Result<Format, D::Error>
where
    D: Deserializer<'de>,
{
    let format = String::deserialize(deserializer)?;
    Format::parse(&format)
        .ok_or(Error::custom(format!("Failed to parse format string {format}")))
}
