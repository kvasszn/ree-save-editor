use std::sync::Arc;
use eframe::egui::{Button, Image, Ui};
use ree_save_core::{edit::{EditContext, path_to_string}, save::types::{Class, FieldValue}};

use image::{ImageFormat};

fn export_photo(bytes: &[u8], path: &std::path::Path) -> anyhow::Result<()> {
    let mut path = path.to_path_buf();

    let fmt = match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("webp") => ImageFormat::WebP,
        Some("jpg" | "jpeg") => ImageFormat::Jpeg,
        _ => { path.set_extension("png"); ImageFormat::Png }
    };

    if fmt == ImageFormat::WebP {
        std::fs::write(&path, bytes)?;
        return Ok(());
    }

    let img = image::load_from_memory_with_format(bytes, ImageFormat::WebP)?;

    match fmt {
        ImageFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let file = std::fs::File::create(&path)?;
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 95);
            enc.encode_image(&rgb)?;
        }
        _ => img.save_with_format(&path, fmt)?,
    }
    Ok(())
}

pub fn edit_as_photo(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext, data_field_name: &str) {
    let path = path_to_string(ctx.path);
    let cache_id = ui.make_persistent_id(&path);
    let uri = format!("bytes://{path}_{data_field_name}.webp");

    ui.horizontal(|ui| {
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

        let has_data = !class.get::<Vec<u8>>(data_field_name).unwrap_or_default().is_empty();

        if ui.add_enabled(has_data, Button::new("Export")).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("WebP", &["webp"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .set_file_name("photo.png")
                .save_file()
                && let Some(bytes) = class.get::<Vec<u8>>(data_field_name)
                && let Err(e) = export_photo(&bytes, &path)
        {
            log::error!("failed to export photo: {e}");
        }
    });

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
