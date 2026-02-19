pub mod edit;
pub mod gensdk;
pub mod reerr;
pub mod bitfield;
pub mod compression;
pub mod file;
pub mod save;
pub mod rsz;
pub mod sdk;
pub mod game_context;

#[cfg(feature = "scripting")]
pub mod bindings;

#[cfg(feature = "tdb")]
pub mod tdb;

#[cfg(target_arch = "wasm32")]
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;



#[cfg(target_arch = "wasm32")]
pub fn resolve_url(path: &str) -> String {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let base_uri = document.base_uri().unwrap().unwrap();
    let url = web_sys::Url::new_with_base(path, &base_uri).unwrap();
    url.href()
}

#[cfg(target_arch = "wasm32")]
async fn fetch_and_decompress(path: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    let path = resolve_url(path);
    let response = reqwest::get(&path).await?;
    let bytes = response.bytes().await?;

    if bytes.len() >= 2 && bytes[0] == 0x1F && bytes[1] == 0x8B {
        let mut gz = flate2::read::GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        gz.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    } else {
        Ok(bytes.to_vec())
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn load_from_url<T,F>(path: &str, func: F) -> Result<()>
where
    T: DeserializeOwned,
    F: FnOnce(T)
{
    let data = fetch_and_decompress(path).await?;
    let deserialized = serde_json::from_slice(&data)?;
    func(deserialized);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn with_bytes_loaded_from_url<F>(path: &str, func: F) -> Result<()>
where
    F: FnOnce(&[u8]) -> Result<()>
{
    let data = fetch_and_decompress(path).await?;
    func(&data)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn with_str_loaded_from_url<F>(path: &str, func: F) -> Result<()>
where
    F: FnOnce(&str) -> Result<()>
{
    let path = resolve_url(path);
    let response = reqwest::get(&path).await?;
    let data = response.text().await?;
    func(&data)?;
    Ok(())
}



#[cfg(test)]
mod tests {
    use std::{env, io::Cursor};

    use crate::{file::StructRW, save::{SaveFile, game::Game}};

    use super::*;

    #[test]
    fn test_save_real_decrypt() {
        dotenvy::dotenv().ok();

        // Fetch the key
        let key = env::var("STEAM_ID").map(|s| s.parse::<u64>().expect("Invalid SteamID64"))
            .expect("STEAM_ID must be set for real save tests");
        let save_path = env::var("SAVE_PATH")
            .expect("SAVE_PATH must be set for real save tests");
        let data: Vec<u8> = std::fs::read(save_path).expect("Save file not found for testing");
        let mut data = Cursor::new(data);
        let mut save_ctx = save::SaveContext {
            key: key,
            game: Game::MHWILDS,
            repair: false,
        };
        let save_file = SaveFile::read(&mut data, &mut save_ctx);
        assert!(save_file.is_ok(), "{:?}", save_file);
    }

    #[test]
    fn test_save_real_sanity() {
        dotenvy::dotenv().ok();
        // Fetch the key
        let key = env::var("STEAM_ID").map(|s| s.parse::<u64>().expect("Invalid SteamID64"))
            .expect("STEAM_ID must be set for real save tests");
        let save_path = env::var("SAVE_PATH")
            .expect("SAVE_PATH must be set for real save tests");
        let data: Vec<u8> = std::fs::read(save_path).expect("Save file not found for testing");
        let mut data = Cursor::new(data.clone());
        let mut save_ctx = save::SaveContext {
            key: key,
            game: Game::MHWILDS,
            repair: false,
        };
        let save_file = SaveFile::read(&mut data, &mut save_ctx);
        assert!(save_file.is_ok(), "{:?}", save_file);
        let save_file = save_file.expect("Error reading save file first time");
        let re_encryped = save_file.to_bytes(key);
        assert!(re_encryped.is_ok(), "{:?}", re_encryped);
        let re_encrypted = re_encryped.expect("Error encrypting");

        // might not be the same data since capcom writes over memory that's not zeroed,
        // so just check that it can read it again and that the re-read save is good
        let mut data = Cursor::new(re_encrypted);
        let save_file2 = SaveFile::read(&mut data, &mut save_ctx);
        assert!(save_file2.is_ok(), "{:?}", save_file2);
        let save_file2 = save_file2.unwrap();
        assert_eq!(save_file.to_bytes(key).expect("Could not rewrite savefile1"), save_file2.to_bytes(key).expect("Could not rewrite savefile2"));
    }
}
