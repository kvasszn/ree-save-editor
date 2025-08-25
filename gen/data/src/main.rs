use askama::Template;
use image::{DynamicImage, GenericImageView, RgbaImage};
use sdk::app::{self, hunterdef::{self, Skill_Fixed}, user_data::{self, itemdata, skillcommondata, skilldata, ItemData, SkillData}};
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::{HashMap, HashSet}, fs::File, io::{Result, Write}, sync::OnceLock};
use mhtame::user::User;


pub fn get_user_rsz<T: DeserializeOwned>(path: &str) -> Result<T> {
    let path = BASE.to_string() + path + ".3.json";
    let file_content = std::fs::read_to_string(path)?;
    let file: serde_json::Value = serde_json::from_str(&file_content)?;
    let rsz = file.get("rsz").unwrap();
    let rsz = rsz.get("rsz").unwrap().get(0).unwrap();
    let rsz: T = serde_json::from_value(rsz.clone())?;
    Ok(rsz)
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct MsgLangs {
    Japanese: String,
    English: String,
    French: String,
    Italian: String,
    German: String,
    Spanish: String,
    Russian: String,
    Polish: String,
    Dutch: String,
    Portuguese: String,
    PortugueseBr: String,
    Korean: String,
    TransitionalChinese: String,
    SimplelifiedChinese: String,
    Finnish: String,
    Swedish: String,
    Danish: String,
    Norwegian: String,
    Czech: String,
    Hungarian: String,
    Slovak: String,
    Arabic: String,
    Turkish: String,
    Bulgarian: String,
    Greek: String,
    Romanian: String,
    Thai: String,
    Ukrainian: String,
    Vietnamese: String,
    Indonesian: String,
    Fiction: String,
    Hindi: String,
    LatinAmericanSpanish: String,
}




pub trait ToCsv {
    fn csv_header() -> &'static str;
    fn to_csv(&self) -> String;
}

macro_rules! csvd {
    ($s:ident, $self:ident, $field:ident, Guid) => {
        let x = match msg_map().msgs.get(&$self.$field) {
            None => "",
            Some(msg) => &msg.content.get("English").unwrap().replace("\r\n", " ").replace("\"", "\"\"")//.chars().flat_map(|c| c.escape_default()).collect::<String>()
        };
        $s.push_str(&format!("\"{}\"", x));
    };
    ($s:ident, $self:ident, $field:ident, Icon) => {
        let x = &$self.$field;
        $s.push_str(&format!("{:?}", x));
    };
    ($s:ident, $self:ident, $field:ident, Enum) => {
        let x = &$self.$field;
        $s.push_str(&format!("{:?}", x));
    };
    ($s:ident, $self:ident, $field:ident, EnumSer) => {
        let x = &$self.$field._Value;
        $s.push_str(&format!("{:?}", x));
    };
    ($s:ident, $self:ident, $field:ident, Color) => {
        let x = &$self.$field;
        $s.push_str(&format!("{:?}", x));
    };
    ($s:ident, $self:ident, $field:ident, ToCsv) => {
        let x = &$self.$field;
        $s.push_str(&format!("{}", x.to_csv()));
    };
    ($s:ident, $self:ident, $field:ident) => {
        $s.push_str(&format!("{:?}", &$self.$field));
    };
}

macro_rules! define_to_csv {
    (
        $struct_ty:ty,
        [ $( $field:ident $(=> $handler:ident)?),* ]
    ) => {
        impl ToCsv for $struct_ty {
            fn csv_header() -> &'static str {
                concat!( $( stringify!($field), "," ),* ).trim_end_matches(',')
            }
            fn to_csv(&self) -> String {
                let mut s = String::new();
                $(
                    csvd!(s, self, $field $(, $handler)?);
                    s.push(',');
                )*
                s.pop();
                s
            }
        }
    };
}

impl<T: ToCsv> ToCsv for Vec<T> {
    fn csv_header() -> &'static str {
        T::csv_header()
    }
    fn to_csv(&self) -> String {
        let mut res = String::new();
        res.push_str(Self::csv_header());
        res.push('\n');
        for e in self {
            res.push_str(&e.to_csv());
            res.push('\n')
        }
        res
    }
}

#[derive(serde::Deserialize)]
pub struct MsgEntry {
    name: String,
    hash: u32,
    content: HashMap<String, String>
}

pub type NameMap = HashMap<String, String>;
pub type MsgMap = HashMap<String, MsgEntry>;

#[derive(serde::Deserialize)]
pub struct CombinedMsgMap {
    msgs: MsgMap,
    name_to_uuid: NameMap,
}

pub fn msg_map() -> &'static CombinedMsgMap {
    static HASHMAP: OnceLock<CombinedMsgMap> = OnceLock::new();
    HASHMAP.get_or_init(|| {
        let file = std::fs::read_to_string("../../outputs/combined_msgs.json").unwrap();
        let m: CombinedMsgMap = serde_json::from_str(&file).unwrap();
        m
    })
}

pub enum Handler {
    Guid,
    Icon,//(IconType),
    Color
}

pub enum IconType {
    Enemy,
    Zako,
    Item,
    Map,
    Animal,
}

define_to_csv!(
    sdk::app::user_data::EnemyData,
    [_Values => ToCsv]
);
define_to_csv!(
    sdk::app::user_data::enemydata::cData,
    [   
        _Index,
        _enemyId,
        _Species,
        _EnemyName => Guid,
        _EnemyExp => Guid,
        _EnemyExtraName => Guid,
        _EnemyBossExp => Guid,
        _EnemyFrenzyName => Guid,
        _EnemyLegendaryName => Guid,
        _EnemyLegendaryKingName => Guid,
        _EnemyFeatures => Guid,
        _EnemyTips => Guid,
        _FirstCapture => Guid,
        _Memo => Guid,
        _Grammar => Guid,
        _BossIconType => Icon,
        _ZakoIconType => Icon,
        _ItemIconType => Icon,
        _MapIconType => Icon,
        _AnimalIconType => Icon,
        _IconColor => Color
    ]
);

define_to_csv!(
    sdk::app::user_data::EnemySpeciesData,
    [_Values => ToCsv]
);

define_to_csv!(
    sdk::app::user_data::enemyspeciesdata::cData,
    [   
        _Index,
        _EmSpecies,
        _EmSpeciesName => Guid
    ]
);

macro_rules! save_to_csv {
    (
        $data_path:expr,
        $file_save_path:expr,
        $struct_ty:ty
    ) => {
        let rsz = get_user_rsz::<$struct_ty>($data_path)?;
        let data = rsz.to_csv();
        let mut file = File::create($file_save_path).unwrap();
        file.write_all(data.as_bytes()).unwrap();

    }
}


define_to_csv!(
    sdk::app::user_data::ArmorData,
    [ _Values => ToCsv ]
);

define_to_csv!(
    sdk::app::armordef::ARMOR_PARTS_Serializable,
    [ _Value => Enum ]
);

define_to_csv!(
    sdk::app::armordef::SERIES_Serializable,
    [ _Value => Enum ]
);

define_to_csv!(
    sdk::app::user_data::armordata::cData,
    [   
        _Index,
        _DataValue,
        _Series => EnumSer,
        _PartsType => EnumSer,
        _Name => Guid,
        _Explain => Guid
        /*_Defense,
        _Resistance,
        _SlotLevel,
        _Skill,
        _SkillLevel*/
    ]
);
pub const BASE: &str = "../../outputs/natives/STM/";

fn split_image(img: &DynamicImage, rows: u32, cols: u32, pad_x: u32, pad_y: u32) -> Vec<RgbaImage> {
    let (width, height) = img.dimensions();
    let tile_width = (width - pad_x) / cols;
    let tile_height = (height - pad_y) / rows;
    let mut tiles = Vec::new();

    for row in 0..rows {
        for col in 0..cols {
            let x = col * tile_width;
            let y = row * tile_height;
            let tile = img.crop_imm(x, y, tile_width, tile_height);
            tiles.push(tile.to_rgba8());
        }
    }

    tiles
}


#[derive(Template)]
#[template(path="skills.html", escape = "none")]
struct SkillsTemplate<'a> {
    skills: Vec<SkillTemplate<'a> >
}

#[derive(Template)]
#[template(path="skill.html")]
struct SkillTemplate<'a> {
    common: &'a skillcommondata::cData,
    levels: &'a Vec<(sdk::S32, &'a skilldata::cData)>,
}

impl<'a> SkillTemplate<'a> {
    pub fn get_icon_path(&self) -> String {
        let icon_name = format!("{:?}", self.common._SkillIconType).to_lowercase().replace("_", "-");
        let icon_path = format!("/resources/skills/{}.png", icon_name);
        icon_path
    }

    pub fn get_icon_idx(&self) -> usize {
        let icon_idx = &(self.common._SkillIconType as usize - 1);
        *icon_idx
    }
}

pub fn get_msg<'a>(guid: &str) -> Option<&'a str> {
    msg_map().msgs.get(guid).map(|x| x.content["English"].as_str())
}

pub fn enum_to_html_str(enum_str: &str) -> String {
    enum_str.to_lowercase().replace(" ", "-")
}


#[derive(Template)]
#[template(path="item.html")]
struct ItemTemplate<'a> {
    item: &'a itemdata::cData,
}

impl<'a> ItemTemplate<'a> {
    pub fn get_icon_path(&self) -> String {
        let icon_name = format!("{:?}", self.item._IconType).to_lowercase().replace("_", "-");
        let icon_path = format!("/resources/item/{}.png", icon_name);
        icon_path
    }

    pub fn get_icon_idx(&self) -> usize {
        let icon_idx = &(self.item._IconType as usize - 1 + 40);
        *icon_idx
    }
}


#[derive(Template)]
#[template(path="items.html", escape="none")]
struct ItemsTemplate<'a> {
    items: Vec<ItemTemplate<'a>>,
}


fn main() -> anyhow::Result<()> {
    /*save_to_csv!(
        "GameDesign/Common/Enemy/EnemyData.user",
        "enemies.csv",
        sdk::app::user_data::EnemyData
    );*/
    let img = image::open(BASE.to_owned() + "GUI/ui_texture/tex000000/tex000201_1_IMLM4.tex.241106027.png")?;
    let gen_icons = split_image(&img, 20, 20, 48, 48);
    let mut genered: HashSet<u32> = HashSet::new();
    std::fs::create_dir_all("dist/resources/skills")?;
    save_to_csv!(
        "GameDesign/Common/Equip/ArmorData.user",
        "armor.csv",
        sdk::app::user_data::ArmorData
    );
    let skill_data = get_user_rsz::<app::user_data::SkillData>("GameDesign/Common/Equip/SkillData.user")?;
    let skill_common_data = get_user_rsz::<app::user_data::SkillCommonData>("GameDesign/Common/Equip/SkillCommonData.user")?;
    let mut skill_levels: HashMap<&Skill_Fixed, Vec<(sdk::S32, &skilldata::cData)>> = HashMap::new();
    for skill in skill_data._Values.iter() {
        if let Some(x) = skill_levels.get_mut(&skill._skillId) {
            x.push((skill._SkillLv, skill));
            x.sort_by(|a, b| a.0.cmp(&b.0));
        } else {
            let mut spec_skill_lvls = Vec::new();
            spec_skill_lvls.push((skill._SkillLv, skill));
            skill_levels.insert(&skill._skillId, spec_skill_lvls);
        }
    }
    let skills = skill_common_data._Values.iter().filter_map(|s| {
        if skill_levels.get(&s._skillId).is_none() || s._skillId == hunterdef::Skill_Fixed::NONE {
            return None;
        }
        let icon_name = format!("{:?}", s._SkillIconType).to_lowercase().replace("_", "-");
        let icon_path = format!("/resources/skills/{}.png", icon_name);
        let icon_idx = &(s._SkillIconType as u32 - 1);
        if !genered.contains(icon_idx) {
            let _res = gen_icons[*icon_idx as usize].save("dist/".to_string() + &icon_path);
            genered.insert(*icon_idx);
        }

        let temp = SkillTemplate { common: &s, levels: &skill_levels[&s._skillId]};
        Some(temp)
    }).collect::<Vec<SkillTemplate>>();

    let skills = SkillsTemplate{skills: skills};
    std::fs::create_dir_all("dist/skills/")?;
    let mut f = std::fs::File::create("dist/skills/index.html")?;
    f.write_all(skills.render()?.as_bytes())?;


    let item_data = get_user_rsz::<app::user_data::ItemData>("GameDesign/Common/Item/itemData.user")?;
    let items = ItemsTemplate{items: item_data._Values.iter().map(|x| ItemTemplate {item: x}).collect()};
    std::fs::create_dir_all("dist/items/")?;
    let mut f = std::fs::File::create("dist/items/index.html")?;
    f.write_all(items.render()?.as_bytes())?;

    /*save_to_csv!(
      "../../outputs/natives/STM/GameDesign/Common/Enemy/EnemySpecies.user.3.json",
      "species.csv",
      sdk::app::user_data::EnemySpeciesData
      );
      save_to_csv!(
      "../../outputs/natives/STM/GameDesign/Common/Equip/ArmorData.user.3.json",
      "armor.csv",
      sdk::app::user_data::ArmorData
      );*/
    Ok(())
}
