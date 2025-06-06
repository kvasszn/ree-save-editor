use sdk::app::{self, user_data};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, io::Result, sync::OnceLock};

#[derive(Debug, serde::Deserialize)]
struct RszJson<T> {
    offset: u64,
    r#type: String,
    rsz: T
}


impl<T> RszJson<T>
where
    T: DeserializeOwned, // allows deserializing owned data (not references)
{
    pub fn get_rsz(path: &str) -> Result<T> {
        let file_content = std::fs::read_to_string(path)?;
        let file: Vec<RszJson<T>> = serde_json::from_str(&file_content)?;
        let rsz = file.into_iter().next().unwrap().rsz;
        Ok(rsz)
    }
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
            Some(msg) => &msg.content.English.replace("\r\n", " ").replace("\"", "\"\"")//.chars().flat_map(|c| c.escape_default()).collect::<String>()
        };
        $s.push_str(&format!("\"{}\"", x));
    };
    ($s:ident, $self:ident, $field:ident, Icon) => {
        let x = &$self.$field;
        $s.push_str(&format!("{:?}", x));
    };
    ($s:ident, $self:ident, $field:ident, Color) => {
        let x = &$self.$field;
        $s.push_str(&format!("{:?}", x));
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
    content: MsgLangs
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

//define_to_csv!(sdk::app::user_data::EnemyData,[_Values]);
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

fn main() -> Result<()> {
    let enemies = RszJson::<user_data::EnemyData>::get_rsz("../../outputs/natives/STM/GameDesign/Common/Enemy/EnemyData.user.3.json")?;
    let _species = RszJson::<user_data::EnemySpeciesData>::get_rsz("../../outputs/natives/STM/GameDesign/Common/Enemy/EnemySpecies.user.3.json")?;
    println!("{}", enemies._Values.to_csv());
    /*let file = File::create("people.csv")?;
    let mut wtr = Writer::from_writer(file);

    wtr.flush()?;*/

    //println!("{:#?}", enemies._Values);
    //println!("{:#?}", species._Values[0]);
    Ok(())
}
