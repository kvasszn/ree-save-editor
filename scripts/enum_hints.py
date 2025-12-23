import enum
import json


mappings = {}
msgs = json.load(open("./assets/combined_msgs.json"))["msgs"]
enums = json.load(open("./assets/enumsmhwilds.json"))

def gen_mapping_for_excel_data(enum_type:str, enum_field:str, name_field: str, file_name: str):
    mappings = {enum_type: {}}
    items = json.load(open(f"./outputs/wilds_files/{file_name}"))["rsz"]["rsz"][0]
    for value in items["_Values"]:
        enum = str(value[enum_field])
        name = value[name_field]
        mappings[enum_type][enum] = name
    return mappings

def gen_mapping_for_excel_data_v2(enum_type:str, enum_field:str, name_field: str, file_name: str):
    mappings = {enum_type: {}}
    items = json.load(open(f"./outputs/wilds_files/{file_name}"))["rsz"]["rsz"][0]
    for value in items["_Values"]:
        enum = value[enum_field]["_Value"]
        name = value[name_field]
        mappings[enum_type][enum] = name
    return mappings


def gen_mapping_for_excel_data_custom(enum_type:str, enum_field:str, enum_field2:str, name_field: str, file_name: str):
    mappings = {enum_type: {}}
    items = json.load(open(f"./outputs/wilds_files/{file_name}"))["rsz"]["rsz"][0]
    for value in items["_Values"]:
        enum1 = value[enum_field]["_Value"]
        enum2 = value[enum_field2]["_Value"]
        name = value[name_field]
        mappings[enum_type][enum1 + enum2] = name
    return mappings


item_path = "natives/STM/GameDesign/Common/Item/itemData.user.3.json"
mappings.update(gen_mapping_for_excel_data("app.ItemDef.ID_Fixed", "_ItemId", "_RawName", item_path))
mappings.update(gen_mapping_for_excel_data("app.ItemDef.ID", "_ItemId", "_RawName", item_path))

weapons = [
        "LongSword",
        "ShortSword",
        "TwinSword",
        "Tachi",
        "Hammer",
        "Whistle",
        "Lance",
        "GunLance",
        "SlashAxe",
        "ChargeAxe",
        "Rod",
        "Bow",
        "HeavyBowgun",
        "LightBowgun"
        ]
for weapon in weapons:
    weapon_path = f"natives/STM/GameDesign/Common/Weapon/{weapon}.user.3.json"
    mappings.update(gen_mapping_for_excel_data(f"app.WeaponDef.{weapon}Id_Fixed", f"_{weapon}", "_Name", weapon_path))

for weapon in weapons:
    weapon_path = f"natives/STM/GameDesign/Common/Weapon/{weapon}.user.3.json"
    mappings.update(gen_mapping_for_excel_data(f"app.WeaponDef.{weapon}Id", f"_{weapon}", "_Name", weapon_path))

armor_path = "natives/STM/GameDesign/Common/Equip/ArmorSeriesData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArmorDef.SERIES_Fixed", f"_Series", "_Name", armor_path))
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArmorDef.SERIES", f"_Series", "_Name", armor_path))

armor_path = "natives/STM/GameDesign/Common/Equip/ArmorData.user.3.json"
mappings.update(gen_mapping_for_excel_data_custom("app.ArmorDef.SpecificPiece", "_Series", "_PartsType", "_Name", armor_path))

wp_series_path = "natives/STM/GameDesign/Common/Equip/WeaponSeriesData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2(f"app.WeaponDef.SERIES_Fixed", f"_Series", "_Name", wp_series_path))
mappings.update(gen_mapping_for_excel_data_v2(f"app.WeaponDef.SERIES", f"_Series", "_Name", wp_series_path))


artian_perf = "natives/STM/GameDesign/Facility/ArtianPerformanceData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArtianDef.PERFORMANCE_TYPE_Fixed", f"_PerformanceType", "_Name", artian_perf))
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArtianDef.PERFORMANCE_TYPE", f"_PerformanceType", "_Name", artian_perf))

amulet_path = "natives/STM/GameDesign/Common/Equip/AmuletData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArtianDef.AmuletType_Fixed", f"_AmuletType", "_Name", amulet_path))
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArmorDef.AmuletType", f"_AmuletType", "_Name", amulet_path))

skill_path = "natives/STM/GameDesign/Common/Equip/SkillCommonData.user.3.json"
mappings.update(gen_mapping_for_excel_data(f"app.HunterDef.Skill", f"_skillId", "_skillName", skill_path))
mappings.update(gen_mapping_for_excel_data(f"app.HunterDef.Skill_Fixed", f"_skillId", "_skillName", skill_path))


artian_path = "natives/STM/GameDesign/Facility/ArtianBonusData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArtianDef.BONUS_ID_Fixed", f"_BonusId", "_Name", artian_path))
mappings.update(gen_mapping_for_excel_data_v2(f"app.ArtianDef.BONUS_ID", f"_BonusId", "_Name", artian_path))

def gen_mapping_for_artian_skills(enum_type:str, file_name: str):
    global mappings
    res_mappings = {enum_type: {}}
    items = json.load(open(f"./outputs/wilds_files/{file_name}"))["rsz"]["rsz"][0]
    for value in items["_Values"]:
        art_enum = value["_ArtianSkillType"]
        skill1_enum = value["_GroupSkillId"]
        skill2_enum= value["_SeriesSkillId"]
        guid1 = mappings["app.HunterDef.Skill"][skill1_enum]
        guid2 = mappings["app.HunterDef.Skill"][skill2_enum]
        #s1 = msgs[guid1]["content"][1]
        #s2 = msgs[guid2]["content"][1]
        res_mappings[enum_type][art_enum] = f"{guid1},{guid2}"
    return res_mappings

artian_skill_path = "natives/STM/GameDesign/Common/Equip/ArtianSkillGroupData.user.3.json"
mappings.update(gen_mapping_for_artian_skills("app.ArtianDef.ArtianSkillType_Fixed", artian_skill_path))
mappings.update(gen_mapping_for_artian_skills("app.ArtianDef.ArtianSkillType", artian_skill_path))

artian_parts = "natives/STM/GameDesign/Facility/ArtianPartsData.user.3.json"
mappings.update(gen_mapping_for_excel_data_v2("app.ArtianDef.PARTS_TYPE_Fixed", "_PartsType", "_Name", artian_parts))
mappings.update(gen_mapping_for_excel_data_v2("app.ArtianDef.PARTS_TYPE", "_PartsType", "_Name", artian_parts))
print(json.dumps(mappings, indent=4))
