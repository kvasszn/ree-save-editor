use std::collections::HashSet;

use eframe::egui::{ComboBox, Ui};
use ree_lib::util::murmur3;
use ree_save_core::{edit::EditContext, save::types::{Class, FieldValue}};

use crate::edit::EditContextUi;

/*

armor => 0b0000_0000 | 0b0000_0001
weapon => 0b0000_1101
amulet => 0b0001_0101
kinsect => 0b0010_0101

*/

#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Gender {
    Male = 0,
    Female = 1,
}

impl Gender {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }

    pub fn ui(&mut self, ui: &mut eframe::egui::Ui) -> bool {
        let mut changed = false;
        ComboBox::from_id_salt("gender_dropdown")
            .selected_text(self.name())
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(self, Gender::Male, Gender::Male.name())
                    .changed();
                changed |= ui
                    .selectable_value(self, Gender::Female, Gender::Female.name())
                    .changed();
            });
        changed
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EquipCategory {
    Armor,
    Weapon,
    Amulet,
    Insect,
    Empty,
    Unknown(u8),
}

impl EquipCategory {
    pub fn to_raw_byte(&self) -> u8 {
        match self {
            Self::Armor => 0,
            Self::Weapon => 13,
            Self::Amulet => 21,
            Self::Insect => 37,
            Self::Empty => 141,
            Self::Unknown(id) => *id,
        }
    }

    pub fn from_raw_byte(raw: u8) -> Self {
        if raw == 141 {
            return Self::Empty;
        }

        let category_id = raw >> 3;
        match category_id {
            0 => Self::Armor,
            1 => Self::Weapon,
            2 => Self::Amulet,
            4 => Self::Insect,
            _ => Self::Unknown(category_id),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Armor => "Armor",
            Self::Weapon => "Weapon",
            Self::Amulet => "Amulet / Charm",
            Self::Insect => "Kinsect",
            Self::Empty => "Empty Slot",
            Self::Unknown(_) => "Unknown",
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Equipment Category:");
            ComboBox::from_id_salt("equip_category_dropdown")
                .selected_text(self.name())
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(self, EquipCategory::Armor, EquipCategory::Armor.name())
                        .changed();
                    changed |= ui
                        .selectable_value(self, EquipCategory::Weapon, EquipCategory::Weapon.name())
                        .changed();
                    changed |= ui
                        .selectable_value(self, EquipCategory::Amulet, EquipCategory::Amulet.name())
                        .changed();
                    changed |= ui
                        .selectable_value(self, EquipCategory::Insect, EquipCategory::Insect.name())
                        .changed();
                    changed |= ui
                        .selectable_value(self, EquipCategory::Empty, EquipCategory::Empty.name())
                        .changed();
                });
        });
        changed
    }
}

pub fn edit_armor(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext) -> Option<()> {
    let free_val0 = class.get_field_mut("FreeVal0");
    if let Some(free_val0) = free_val0 {
        ctx.draw_labeled_dropdown(ui, &mut free_val0.value, "app.ArmorDef.SERIES", "SeriesID");
    }
    let free_val1 = class.get_field_mut("FreeVal1");
    if let Some(free_val1) = free_val1 {
        ctx.draw_labeled_dropdown(
            ui,
            &mut free_val1.value,
            "app.ArmorDef.ARMOR_PARTS",
            "ArmorPart",
        );
    }
    Some(())
}

pub fn edit_weapon(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext) -> Option<()> {
    let weapon_type = class.get_field_mut("FreeVal0")?;
    ctx.draw_labeled_dropdown(
        ui,
        &mut weapon_type.value,
        "app.WeaponDef.TYPE",
        "WeaponType",
    );

    let weapon_type_specific = ctx.try_enum_str("app.WeaponDef.TYPE", &weapon_type.value)?;
    let enum_type = format!(
        "app.WeaponDef.{}Id",
        util::to_pascal_case(weapon_type_specific.as_str()).replace("_", "")
    );

    let weapon_id = class.get_field_mut("FreeVal1")?;
    ctx.draw_labeled_dropdown(ui, &mut weapon_id.value, &enum_type, "WeaponID");

    let artian_perf_type = class.get_field_mut("FreeVal2")?;
    ctx.draw_labeled_dropdown(
        ui,
        &mut artian_perf_type.value,
        "app.ArtianDef.PERFORMANCE_TYPE_Fixed",
        "PerformanceType",
    );
    let artian_perf_type = ctx.try_enum_str(
        "app.ArtianDef.PERFORMANCE_TYPE_Fixed",
        &artian_perf_type.value,
    )?;
    if "INVALID" != artian_perf_type.as_str() {
        edit_artian_weapon(class, ui, ctx);
    }
    Some(())
}

pub fn edit_artian_weapon(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext) -> Option<()> {
    let bonus_by_creating_mut = class.get_mut::<&mut u32>("BonusByCreating")?;
    let bonus_by_creating = *bonus_by_creating_mut;
    let skill_digit_1 = (bonus_by_creating / 10_000_000) % 10;
    let skill_digit_2 = (bonus_by_creating / 10_000) % 10;
    let skill_digit_3 = (bonus_by_creating / 10) % 10;
    let mut artian_skill_fixed =
        FieldValue::S32(((skill_digit_1 * 100) + (skill_digit_2 * 10) + skill_digit_3) as i32);
    let mut bonus_type_1 = (bonus_by_creating / 1_000_000) % 10;
    let mut bonus_type_2 = (bonus_by_creating / 1_000) % 10;
    let mut bonus_type_3 = bonus_by_creating % 10;
    ctx.draw_labeled_dropdown(
        ui,
        &mut artian_skill_fixed,
        "app.ArtianDef.ArtianSkillType_Fixed",
        "Skills",
    );
    Some(())
}

pub fn edit_as_equip_work(class: &mut Class, ui: &mut Ui, ctx: &mut EditContext) {
    let type_info = ctx.engine_context.rsz_map.get_by_hash(class.hash);

    let Some(category_gender) = class.get_mut::<&mut u8>("Category_Gender") else {
        return;
    };

    let gender_bit = *category_gender & 1; // male = 0, female = 1
    let mut gender = if gender_bit == 1 {
        Gender::Female
    } else {
        Gender::Male
    };
    let mut category = EquipCategory::from_raw_byte(*category_gender);

    if category.ui(ui) {
        *category_gender = category.to_raw_byte() | gender as u8;
    }

    match category {
        EquipCategory::Armor => {
            if gender.ui(ui) {
                *category_gender = category.to_raw_byte() | gender as u8;
            }
            let _ = edit_armor(class, ui, ctx);
        }
        EquipCategory::Weapon => {
            let _ = edit_weapon(class, ui, ctx);
        }
        EquipCategory::Amulet => {}
        EquipCategory::Insect => {}
        _ => {}
    }

    let free_val0 = class.get_field_mut("FreeVal0");
    let free_val1 = class.get_field_mut("FreeVal1");
    let free_val2 = class.get_field_mut("FreeVal2");
    // No _Fixed here?
    /* match category {
        // armor ?, lsb is gender
        0b0000_0000 | 0b0000_0001 => edit_as_equip_work(class, ui, ctx),
        // Weapon 13
        0b0000_1101 => {
        if let Some(free_val0) = free_val0 {
        ui.label("WeaponType");
        ctx.draw_labeled_dropdown(ui, &mut free_val0.value, "app.WeaponDef.TYPE", "WeaponType");
        }
        let val = free_val0.and_then(|v| ctx.try_enum_str("app.WeaponDef.TYPE", &v.value));
        if let Some(val) = val {
        let enum_type = format!(
        "app.WeaponDef.{}Id",
        util::to_pascal_case(val.as_str()).replace("_", "")
        );

        if let Some(free_val1) = free_val1 {
        ui.label("WeaponID");
        ctx.draw_labeled_dropdown(ui, &mut free_val1.value, &enum_type, "WeaponID");
        }
        let artian_perf_type = class.get_enum_str_from_field(
        "FreeVal2",
        "app.ArtianDef.PERFORMANCE_TYPE_Fixed",
        ctx,
        );

        if let Some(free_val2) = free_val2 {
        ui.label("PerformanceType");
        ctx.draw_labeled_dropdown(ui, &mut free_val2.value, "app.ArtianDef.PERFORMANCE_TYPE_Fixed", "PerformanceType");
        }

        if Some("INVALID".to_string()) != artian_perf_type {
        let hash = &murmur3("BonusByCreating");
        if let Some(bonus_by_creating_mut) =
        class.get_mut::<&mut u32>("BonusByCreating")
        {
        let bonus_by_creating = *bonus_by_creating_mut;
        let skill_digit_1 = (bonus_by_creating / 10_000_000) % 10;
        let skill_digit_2 = (bonus_by_creating / 10_000) % 10;
        let skill_digit_3 = (bonus_by_creating / 10) % 10;
        let mut artian_skill_fixed =
        ((skill_digit_1 * 100) + (skill_digit_2 * 10) + skill_digit_3)
        as i32;
        let mut bonus_type_1 = (bonus_by_creating / 1_000_000) % 10;
        let mut bonus_type_2 = (bonus_by_creating / 1_000) % 10;
        let mut bonus_type_3 = bonus_by_creating % 10;
        ui.horizontal(|ui| {
        ui.label("Skills");
        let child_id =
        ui.make_persistent_id((ctx.id, "Skills", hash)).value();
        let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
        edit_i32_enum_from_type(
        &mut artian_skill_fixed,
        "app.ArtianDef.ArtianSkillType_Fixed",
        ui,
        &mut child_ctx,
        );
        });
        ui.horizontal(|ui| {
        ui.label("Bonuses(Atk=1/Aff=2/Elem=3(ithink))");
        bonus_type_1.ui(ui, ctx);
        bonus_type_2.ui(ui, ctx);
        bonus_type_3.ui(ui, ctx);
        });
        let bonus_by_creating = (artian_skill_fixed / 100) as u32 * 10_000_000
        + bonus_type_1 * 1_000_000
        + ((artian_skill_fixed / 10) % 10) as u32 * 10_000
        + bonus_type_2 * 1_000
        + (artian_skill_fixed % 10) as u32 * 10
        + bonus_type_3;
         *bonus_by_creating_mut = bonus_by_creating;
        left.remove(hash);
    }
    if let Some(bonus_by_grinding_mut) =
    class.get_mut::<&mut u64>("BonusByGrinding")
    {
        let bonus_by_grinding = *bonus_by_grinding_mut;
        let mut bonus_1 =
            ((bonus_by_grinding / 1_000_000_000_000) % 1000) as i32;
        let mut bonus_2 = ((bonus_by_grinding / 1_000_000_000) % 1000) as i32;
        let mut bonus_3 = ((bonus_by_grinding / 1_000_000) % 1000) as i32;
        let mut bonus_4 = ((bonus_by_grinding / 1_000) % 1000) as i32;
        let mut bonus_5 = (bonus_by_grinding % 1000) as i32;
        ui.label("BonusesByGrinding");
        // SO GROSS BUT IT WORKS HAHA HA
        ui.indent((hash, ctx.id), |ui| {
            let child_id =
                ui.make_persistent_id((hash, ctx.id, "Bonus1")).value();
            let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
            edit_i32_enum_from_type(
                &mut bonus_1,
                "app.ArtianDef.BONUS_ID_Fixed",
                ui,
                &mut child_ctx,
            );
            let child_id =
                ui.make_persistent_id((hash, ctx.id, "Bonus2")).value();
            let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
            edit_i32_enum_from_type(
                &mut bonus_2,
                "app.ArtianDef.BONUS_ID_Fixed",
                ui,
                &mut child_ctx,
            );
            let child_id =
                ui.make_persistent_id((hash, ctx.id, "Bonus3")).value();
            let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
            edit_i32_enum_from_type(
                &mut bonus_3,
                "app.ArtianDef.BONUS_ID_Fixed",
                ui,
                &mut child_ctx,
            );
            let child_id =
                ui.make_persistent_id((hash, ctx.id, "Bonus4")).value();
            let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
            edit_i32_enum_from_type(
                &mut bonus_4,
                "app.ArtianDef.BONUS_ID_Fixed",
                ui,
                &mut child_ctx,
            );
            let child_id =
                ui.make_persistent_id((hash, ctx.id, "Bonus5")).value();
            let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
            edit_i32_enum_from_type(
                &mut bonus_5,
                "app.ArtianDef.BONUS_ID_Fixed",
                ui,
                &mut child_ctx,
            );
        });
        let bonus_by_grind: u64 = bonus_1 as u64 * 1_000_000_000_000
            + bonus_2 as u64 * 1_000_000_000
            + bonus_3 as u64 * 1_000_000
            + bonus_4 as u64 * 1_000
            + bonus_5 as u64;
        *bonus_by_grinding_mut = bonus_by_grind;
        left.remove(hash);
    }
    }
    }
    }
    // idfk 21
    0b0001_0101 => {
        if let Some(free_val0) = free_val0 {
            ui.label("AmuletType");
            ctx.draw_labeled_dropdown(ui, &mut free_val0.value, "app.ArmorDef.AmuletType", "AmuletType");
        }
        //let amulet_type = class.get::<i32>("FreeVal0").unwrap_or(0);
        let free_flag_1 = class.get_mut::<&mut i32>("FreeVal1");
        if let Some(free_flag_1) = free_flag_1 {
            ui.label("This should equal 1 for Unknown/Historical/Secret/Golden Age Charms, otherwise 2 for most other charms");
            ui.horizontal(|ui| {
                ui.label("FreeVal1");
                free_flag_1.ui(ui, ctx);
                //left.remove(&murmur3("FreeVal1", 0xffffffff));
            });
            if *free_flag_1 == 1 {
                let skills = class.get_array_mut("BowgunCustomizeId");
                let mut edit_skill = |ui: &mut Ui, idx: usize, skill: &mut i16| {
                    let mut level = *skill / 1_000i16;
                    let mut id = (*skill % 1_000i16) as i32;
                    ui.horizontal(|ui| {
                        ui.label(format!("Skill #{idx}: "));
                        let child_id =
                            ui.make_persistent_id(("AmuletEdit", ctx.id, idx)).value();
                        let mut child_ctx = Class::udpate_ctx(type_info, child_id, ctx);
                        edit_i32_enum_from_type(
                            &mut id,
                            "app.HunterDef.Skill",
                            ui,
                            &mut child_ctx,
                        );
                        ui.label("Level: ");
                        level.ui(ui, &mut child_ctx);
                    });
                    *skill = level * 1_000 + id as i16;
                };
                if let Some(skills) = skills {
                    let skill_1 = skills.get_value_mut(0).and_then(|x| x.as_i16_mut());
                    if let Some(skill) = skill_1 {
                        edit_skill(ui, 1, skill);
                    }
                    let skill_2 = skills.get_value_mut(1).and_then(|x| x.as_i16_mut());
                    if let Some(skill) = skill_2 {
                        edit_skill(ui, 2, skill);
                    }
                    let skill_3 = skills.get_value_mut(2).and_then(|x| x.as_i16_mut());
                    if let Some(skill) = skill_3 {
                        edit_skill(ui, 3, skill);
                    }
                    let slots = skills.get_value_mut(3).and_then(|x| x.as_i16_mut());
                    if let Some(slots) = slots {
                        let mut slot_1 = *slots / 1_000;
                        let mut slot_2 = (*slots / 100) % 10;
                        let mut slot_3 = (*slots / 10) % 10;
                        let mut slot_4 = *slots % 10;
                        ui.label("Decoration Slots (W/A lvl, A lvl, A lvl)");
                        ui.horizontal(|ui| {
                            ComboBox::new((ctx.id, "AmuletSlots"), "")
                                .selected_text(if slot_4 == 1 { "Weapon" } else { "Armor" })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut slot_4, 1, "Weapon");
                                    ui.selectable_value(&mut slot_4, 0, "Armor");
                                });
                            slot_1.ui(ui, ctx);
                            ui.label("Armor");
                            slot_2.ui(ui, ctx);
                            ui.label("Armor");
                            slot_3.ui(ui, ctx);
                        });
                        *slots = slot_1 * 1000 + slot_2 * 100 + slot_3 * 10 + slot_4;
                    }
                }
            }
        }
    }
    // idfk again 37
    0b0010_0101 => {}
    _ => {}
    };*/
}
