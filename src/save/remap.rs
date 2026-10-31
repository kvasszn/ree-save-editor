use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Deserializer};
use ree_lib::data::{DataSource, deserialize_data_sources};

#[derive(Deserialize, Debug, Clone)]
pub struct Remap {
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_format_string")]
    pub format: Vec<FormatNode>,
    // remaps a field from one type to another
    #[serde(default)]
    pub fields: HashMap<String, String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_data_sources")]
    pub queries: HashMap<String, DataSource>, 
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_data_sources")]
    pub data: HashMap<String, DataSource>, 
}

// TODO: add a raw, and a pipe as a backup?
#[derive(Debug, Clone)]
pub enum FormatNode {
    Literal(String),        // "Enemy:", just  literal
    Data(String),           // "{data:name}" enters the data entry at "name"
    Convert(String),        // "{convert:app.EnemyDef.ID_Fixed}" i.e "aap.EnemyDef.ID" -> "app.EnemyDef.ID_Fixed"
    Enum,                   // "{enum:}", just puts in the enum value
    Field(String),          // "{_BasicData}", value of a field, only valid on classes
}

use regex::Regex;

pub fn parse_format_string(format_str: &str) -> Vec<FormatNode> {
    let mut nodes = Vec::new();

    let re = Regex::new(r"\{([^}]+)\}").unwrap();
    let mut last_end = 0;

    for cap in re.captures_iter(format_str) {
        let whole_match = cap.get(0).unwrap();
        let inner_text = cap.get(1).unwrap().as_str();

        if whole_match.start() > last_end {
            let literal = &format_str[last_end..whole_match.start()];
            nodes.push(FormatNode::Literal(literal.to_string()));
        }

        let parts: Vec<&str> = inner_text.splitn(2, ':').collect();
        let command = parts[0].trim();
        let arg = if parts.len() > 1 { parts[1].trim() } else { "" };

        match command {
            "data" => nodes.push(FormatNode::Data(arg.to_string())),
            "convert" => nodes.push(FormatNode::Convert(arg.to_string())),
            "enum" => nodes.push(FormatNode::Enum),
            _ if arg.is_empty() => nodes.push(FormatNode::Field(command.to_string())),
            _ => nodes.push(FormatNode::Literal(whole_match.as_str().to_string())),
        }

        last_end = whole_match.end();
    }

    if last_end < format_str.len() {
        nodes.push(FormatNode::Literal(format_str[last_end..].to_string()));
    }

    nodes
}

fn deserialize_format_string<'de, D>(deserializer: D) -> Result<Vec<FormatNode>, D::Error>
where
    D: Deserializer<'de>,
{
    let format_str = String::deserialize(deserializer)?;
    Ok(parse_format_string(&format_str))
}

pub fn get_asset_paths(remaps: &HashMap<String, Remap>) -> HashSet<String> {
    let mut res = HashSet::new();
    for remap in remaps.values() {
        for query in remap.queries.values() {
            match query {
                DataSource::MsgLookup { msg_file: file, .. }
                | DataSource::RszQuery { rsz_file: file, .. } => {
                    res.insert(file.to_string());
                },
            }
        }
        for query in remap.data.values() {
            match query {
                DataSource::MsgLookup { msg_file: file, .. }
                | DataSource::RszQuery { rsz_file: file, .. } => {
                    res.insert(file.to_string());
                },
            }
        }
    }
    res
}
