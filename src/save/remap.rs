use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Remap {
    pub remap: HashMap<String, String>,
    pub preview: String,
    pub bitsets: HashMap<String, String>,
}

