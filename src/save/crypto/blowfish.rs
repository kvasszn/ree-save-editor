pub const KEY: &'static [u8] = b"K<>$cl%isqA|~nV4W5~3z_Q)j]5DHdB9sb{cI9Hn&Gqc-zO8O6zf";

use blowfish::{BlowfishLE};
use cbc::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use cipher::block_padding::NoPadding;

type EncryptBlowfishCBC = cbc::Encryptor<BlowfishLE>;
type DecryptBlowfishCBC = cbc::Decryptor<BlowfishLE>;

pub fn decrypt_in_place<'a>(data: &'a mut [u8]) -> Result<&'a [u8], Box<dyn std::error::Error>> {
    let cipher = DecryptBlowfishCBC::new_from_slices(KEY, &[0u8; 8])?;
    let aligned_len = data.len() - (data.len() % 8);
    let decrypted = cipher.decrypt_padded::<NoPadding>(&mut data[..aligned_len])?;
    Ok(decrypted)
}

pub fn encrypt_in_place<'a>(data: &'a mut [u8]) -> Result<&'a [u8], Box<dyn std::error::Error>> {
    let cipher = EncryptBlowfishCBC::new_from_slices(KEY, &[0u8; 8])?;
    let aligned_len = data.len() - (data.len() % 8);
    let decrypted = cipher.encrypt_padded::<NoPadding>(&mut data[..aligned_len], aligned_len)?;
    Ok(decrypted)
}
