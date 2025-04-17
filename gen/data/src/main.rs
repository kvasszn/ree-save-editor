use sdk::app::enemydef;
use sdk::app::user_data::enemydata;
use sdk::app::user_data;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io::Result;

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

fn main() -> Result<()> {
    let enemies = RszJson::<user_data::EnemyData>::get_rsz("../../outputs/natives/STM/GameDesign/Common/Enemy/EnemyData.user.3.json")?;
    let species = RszJson::<user_data::EnemySpeciesData>::get_rsz("../../outputs/natives/STM/GameDesign/Common/Enemy/EnemySpecies.user.3.json")?;

    println!("{:#?}", enemies._Values);
    println!("{:#?}", species._Values[0]);
    Ok(())
}
