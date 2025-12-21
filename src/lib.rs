pub mod edit;
pub mod gensdk;
pub mod reerr;
pub mod bitfield;
pub mod compression;
pub mod file;
pub mod save;
pub mod rsz;
#[cfg(feature = "tdb")]
pub mod tdb;

#[cfg(test)]
mod tests {
    use std::{env, fs::File, io::Cursor};

    use sdk::type_map::TypeMap;

    use crate::{file::StructRW, save::SaveFile};

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
            key: key
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
            key: key
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
