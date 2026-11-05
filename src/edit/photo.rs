use std::sync::Arc;
use eframe::egui::{Image, Ui};

use crate::{edit::{EditContext, path_to_string}, save::types::{Class, FieldValue}};

pub fn edit_as_photo(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext, data_field_name: &str) {
    let path = path_to_string(ctx.path);
    let cache_id = ui.make_persistent_id(&path);
    let uri = format!("bytes://{path}_{data_field_name}.webp");

    if ui.button("Upload (Must be webp and possibly 1920x1080)").clicked() && let Some(path) = rfd::FileDialog::new()
        .add_filter("Images", &["webp"]).pick_file() 
    {
        if let Ok(new_bytes) = std::fs::read(&path) {
            let Some(serialize_data) = class.get_array_mut(data_field_name) else {
                ui.label(format!("Could not get image data from field {data_field_name}"));
                return;
            };

            let field_values: Vec<FieldValue> = new_bytes.iter().map(|b| FieldValue::U8(*b)).collect();
            serialize_data.values = field_values; 

            ui.data_mut(|data| {
                data.insert_temp(cache_id, Arc::<[u8]>::from(new_bytes));
            });

            ui.ctx().forget_image(&uri);
        } else {
            eprintln!("Failed to read the selected image file.");
        }
    }
    let image_bytes: Arc<[u8]> = ui.data_mut(|data| {
        let bytes: &mut Arc<[u8]> = data.get_temp_mut_or_insert_with(cache_id, || {
            class.get::<Vec<u8>>(data_field_name)
                .unwrap_or_default()
                .into()
        });
        bytes.clone()
    });

    if !image_bytes.is_empty() {
        let avail_width = ui.available_width();
        let image = Image::from_bytes(uri, image_bytes)
            .fit_to_original_size(1.0)
            .max_width(avail_width * 0.98);
        ui.add(image);
    }
}
