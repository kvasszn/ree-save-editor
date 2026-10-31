use ree_lib::rsz::Value;
use ree_lib::types::Guid;

use crate::{edit::EditContext, save::{remap::Remap, types::FieldValue}}; 
use ree_lib::data::DataSource;

impl<'a> EditContext<'a> {
    pub fn try_remap(&mut self, remap_key: &str, data_name: &str, current_val: &Value) -> Option<String> {
        let remap = self.remaps.get(remap_key)?;
        self.eval_data(data_name, remap, current_val)
    }

    pub fn eval_data(&mut self, data_name: &str, remap: &Remap, current_val: &Value) -> Option<String> {
        let source = remap.data.get(data_name)?;
        match source {
            DataSource::MsgLookup { msg_file, target_query, target_field } => {
                let query_def = remap.queries.get(target_query)?;
                let source_rsz_file = match query_def {
                    DataSource::RszQuery { rsz_file, .. } => rsz_file,
                    _ => return None,
                };

                let row_object = self.resolve_query(target_query, remap, current_val)?;
                let guid_val = self.extract_field(source_rsz_file, &row_object, target_field)?;

                let guid = match guid_val {
                    Value::Guid(g) => Guid(g.0),
                    _ => return None,
                };

                self.engine_context.get_msg_entry(msg_file, &guid)
            }

            DataSource::RszQuery { rsz_file, array_path, match_field } => {
                let result_obj = self.engine_context.query_rsz_array(rsz_file, array_path, match_field, current_val)?;
                Some(format!("{:?}", result_obj)) 
            }
        }
    }

    // TODO: results from each of these lookups should get cached
    // this should handle remapping itself i think
    pub fn remap_format(&mut self, remap_key: &str, val: &FieldValue) -> Option<String> {
        let remap = self.remaps.get(remap_key)?;
        let mut res = String::new();
        let rsz_val: Value = Value::from(val);
        let type_info = self.engine_context.rsz_map.get_type(remap_key);
        for node in &remap.format {

            use super::remap::FormatNode::*;
            match node {
                Literal(l) => res.push_str(l),
                Data(d) => {
                    let data = self.try_remap(remap_key, d, &rsz_val)?;
                    res.push_str(&data);
                },
                // TODO: enum could probably actually be formatted like {enum:app.ItemDef.ID_Fixed}
                // that way i can freely do alot of shit, i can probably even have an {enum_val:...}
                Enum => {
                    let data = self.try_enum_str(remap_key, val)?;
                    res.push_str(&data);
                },
                Field(f) => {
                    if let FieldValue::Class(class) = val {
                        if let Some(field) = class.get_field(f) && let Some(field_info) = type_info.and_then(|t| t.get_field(f)) {
                            let type_label = remap.fields.get(f).unwrap_or(&field_info.original_type);
                            if let FieldValue::Class(_) = field.value {
                                let val = self.remap_format(type_label, &field.value)?;
                                res.push_str(&val);
                            } else if self.remaps.contains_key(type_label) {
                                let val = self.remap_format(type_label, &field.value)?;
                                res.push_str(&val);
                            } else {
                                res.push_str(&field.value.to_string())
                            }
                        }
                    } else {
                        log::info!("Field({f}) cannot be used on non-classes");
                    }
                },
                Convert(c) => {
                    let enum_str = self.try_enum_str(remap_key, val)?;
                    let converted_val = self.try_enum_field_val(c, &enum_str)?;
                    let converted_str = self.remap_format(c, &converted_val)?;
                    res.push_str(&converted_str);
                },
            }
        }
        Some(res)
    }

    fn resolve_query(&mut self, query_name: &str, remap: &Remap, current_val: &Value) -> Option<Value> {
        let cache_key = (query_name.to_string(), format!("{:?}", current_val));
        if let Some(cached_val) = self.query_cache.get(&cache_key) {
            return Some(cached_val.clone());
        }

        let query_source = remap.queries.get(query_name)?;

        let result = match query_source {
            DataSource::RszQuery { rsz_file, array_path, match_field } => {
                self.engine_context.query_rsz_array(rsz_file, array_path, match_field, current_val).cloned()
            }
            _ => None,
        }?;

        self.query_cache.insert(cache_key, result.clone());

        Some(result)
    }

    fn extract_field(&mut self, rsz_file: &str, object: &Value, field_name: &str) -> Option<Value> {
        let obj_id = match object {
            Value::Object(id) => *id as usize,
            _ => return None,
        };

        let rsz = self.engine_context.assets.get_rsz(rsz_file).ok()?; 

        let instance = rsz.instances.get(obj_id)?;
        let type_info = self.engine_context.rsz_map.get_by_hash(instance.hash)?;
        let field_idx = type_info.get_field_idx(field_name)?;

        instance.fields.get(field_idx).cloned()
    }

    pub fn try_enum_str(&self, enum_type_name: &str, value: &FieldValue) -> Option<String> {
        let enum_def = self.engine_context.enums.get(enum_type_name)?;
        match value {
            FieldValue::Enum(e) => {
                enum_def.get_name_i64(e.as_i64())
                    .or_else(|| enum_def.get_name_u64(e.as_u64()))
                    .cloned()
            }
            FieldValue::S8(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::S16(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::S32(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::S64(v) => enum_def.get_name_i64(*v).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),

            FieldValue::U8(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::U16(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::U32(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v as u64)).cloned(),
            FieldValue::U64(v) => enum_def.get_name_i64(*v as i64).or_else(|| enum_def.get_name_u64(*v)).cloned(),
            _ => None,
        }
    }


    pub fn try_enum_field_val(&self, enum_type_name: &str, enum_str: &str) -> Option<FieldValue> {
        let enum_def = self.engine_context.enums.get(enum_type_name)?;
        Some(FieldValue::U64(enum_def.get_value_u64(enum_str)?))
    }
}

