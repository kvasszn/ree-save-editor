import json


mappings = {}
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

# TODO: for speciifc Armor peices, have to look at two enums to get the name of the piece
#armor_path = "natives/STM/GameDesign/Common/Equip/ArmorSeriesData.user.3.json"
#mappings.update(gen_mapping_for_excel_data(f"app.Armor.SERIES_Fixed", f"_Series", "_Name", armor_path))
print(json.dumps(mappings, indent=4))
