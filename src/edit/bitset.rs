use bitfield::BitMut;
use eframe::egui::{CollapsingHeader, ScrollArea, Ui};

use crate::{edit::{EditContext, Editable}, save::types::{Array, Class, FieldValue}};

pub fn edit_as_bitset(
    class: &mut Class,
    generic: Option<&str>,
    ui: &mut Ui,
    ctx: &mut EditContext,
) {
    //TODO: can do something like class.edit_field_and_get("_MaxElement");
    let Some(max_element) = class.get_mut::<&mut i32>("_MaxElement") else {
        return
    };
    ui.horizontal(|ui| {
        ui.label("_MaxElement");
        max_element.ui(ui, ctx);
    });
    let max_element = *max_element as usize;

    let Some(values) = class.get_mut::<&mut Array>("_Value") else {
        return
    };

    let edit_bitset = |ui: &mut Ui, a: &mut Array, generic: Option<&str>| {
        ui.horizontal(|ui| {
            let mut count = 0;

            // button to set all values to 1 or 0
            if ui.small_button("Check All").clicked() {
                for v in &mut a.values {
                    if count >= max_element {
                        break;
                    }
                    let bits = (max_element - count).min(32);
                    let mask = if bits == 32 {
                        u32::MAX
                    } else {
                        (1u32 << bits) - 1
                    };
                    if let FieldValue::U32(x) = v {
                        *x = mask;
                    }
                    count += bits;
                }
            }
            if ui.small_button("Uncheck All").clicked() {
                for v in &mut a.values {
                    if let FieldValue::U32(x) = v {
                        *x = 0;
                    }
                }
            }
        });
        ui.separator();

        let enums = generic.and_then(|g| {
            ctx.engine_context.enums.get(g.split_once("<").and_then(|x| x.0.strip_suffix(">")).unwrap_or(g))
        });
        //println!("{enums:?}");
        let mut count = 0usize;
        for v in &mut a.values {
            let Some(x) = v.as_u32_mut() else { continue };
            if count >= max_element {
                break;
            }
            for i in 0..32 {
                if count + i >= max_element {
                    break;
                }
                let mut bit_val = *x & (1 << i) != 0;
                /*if let Some(e) = enums.as_ref().and_then(|x| x.get(&(count + i).to_string())) {
                    let enum_text = generic.and_then(|generic| {
                        ctx.engine_context.get_enum_text(e, generic, ctx.engine_context.language)
                    });
                    if let Some(et) = enum_text {
                        ui.checkbox(&mut bit_val, format!("idx={}({e}:{et})", count + i));
                    } else {
                        ui.checkbox(&mut bit_val, format!("idx={}({e})", count + i));
                    }
                } else {
                    ui.checkbox(&mut bit_val, format!("{}", count + i));
                }*/
                ui.checkbox(&mut bit_val, format!("{}", count + i));
                x.set_bit(i, bit_val);
            }
            count += 32;
        }
    };

    /*let edit_outer_armor_flags = |ui: &mut Ui, a: &mut Array| {
        let (gender_enum, series_enum, part_enum) = {
            (
                ctx.engine_context.enums.get("app.CharacterDef.GENDER"),
                ctx.engine_context.enums.get("app.ArmorDef.SERIES"),
                ctx.engine_context.enums.get("app.ArmorDef.ARMOR_PARTS"),
            )
        };

        ui.horizontal(|ui| {
            let mut count = 0;
            if ui.small_button("Check All").clicked() {
                for v in &mut a.values {
                    if count >= max_element {
                        break;
                    }
                    let bits = (max_element - count).min(32);
                    let mask = if bits == 32 {
                        u32::MAX
                    } else {
                        (1u32 << bits) - 1
                    };
                    if let FieldValue::U32(x) = v {
                        *x = mask;
                    }
                    count += bits;
                }
            }
            if ui.small_button("Uncheck All").clicked() {
                for v in &mut a.values {
                    if let FieldValue::U32(x) = v {
                        *x = 0;
                    }
                }
            }
        });
        ui.separator();

        let mut count = 0usize;
        for v in &mut a.values {
            let Some(x) = v.as_u32_mut() else {
                continue;
            };
            if count >= max_element {
                break;
            }
            for i in 0..32 {
                let idx = count + i;
                if idx >= max_element {
                    break;
                }
                let mut bit_val = *x & (1 << i) != 0;
                let part = idx % 5;
                let series = (idx / 5) % (0x4b5 / 5);
                let gender = idx / 0x4b5;
                let gender = gender_enum
                    .map(|gender_enum| {
                        let enum_str = gender_enum.get(&gender.to_string());
                        match enum_str {
                            None => gender.to_string(),
                            Some(enum_str) => ctx
                                .engine_context
                                .get_enum_text(
                                    enum_str,
                                    "app.CharacterDef.GENDER",
                                    ctx.language,
                                )
                                .unwrap_or(enum_str.clone()),
                        }
                    })
                    .unwrap_or(gender.to_string());

                let mut specific = None;
                if let Some(series_enum) = series_enum {
                    if let Some(series) = series_enum.get(&series.to_string()) {
                        if let Some(part_enum) = part_enum {
                            if let Some(part) = part_enum.get(&part.to_string()) {
                                specific = ctx.engine_context.get_enum_text(
                                    &format!("{series}{part}"),
                                    "app.ArmorDef.SpecificPiece",
                                    ctx.engine_context.language,
                                );
                            }
                        }
                    }
                }
                if let Some(specific) = specific {
                    ui.checkbox(&mut bit_val, format!("idx={idx}:({gender}:\"{specific}\")"));
                    x.set_bit(i as usize, bit_val);
                } else {
                    let series = series_enum
                        .map(|series_enum| {
                            let enum_str = series_enum.get(&series.to_string());
                            match enum_str {
                                None => series.to_string(),
                                Some(enum_str) => ctx
                                    .engine_context
                                    .get_enum_text(
                                        enum_str,
                                        "app.ArmorDef.SERIES",
                                        ctx.engine_context.language,
                                    )
                                    .unwrap_or(enum_str.clone()),
                            }
                        })
                        .unwrap_or(series.to_string());

                    let part = part_enum
                        .map(|part_enum| {
                            let enum_str = part_enum.get(&part.to_string());
                            match enum_str {
                                None => part.to_string(),
                                Some(enum_str) => ctx
                                    .engine_context
                                    .get_enum_text(
                                        enum_str,
                                        "app.ArmorDef.ARMOR_PARTS",
                                        ctx.engine_context.language,
                                    )
                                    .unwrap_or(enum_str.clone()),
                            }
                        })
                        .unwrap_or(part.to_string());
                    ui.checkbox(
                        &mut bit_val,
                        format!("idx={idx}:({gender}:\"{series}\":{part})"),
                    );
                    x.set_bit(i as usize, bit_val);
                }
            }
            count += 32;
        }
    };*/

    // hopefully this scrollable area works alright?
    CollapsingHeader::new("_Value").show(ui, |ui| {
        eframe::egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_width(ui.available_width() * 0.9)
                    .min_scrolled_height(300.0)
                    .show(ui, |ui| match generic {
                        Some("Custom.OuterArmorFlags") => {
                            //edit_outer_armor_flags(ui, &mut values)
                        }
                        Some(_) | None => edit_bitset(ui, values, generic),
                    });
            });
    });
}
