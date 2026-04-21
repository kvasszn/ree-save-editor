use std::sync::atomic::AtomicUsize;

#[cfg(target_os = "linux")]
use super::util::backend::rug::*;
#[cfg(not(target_os = "linux"))]
use super::util::backend::num_bigint::*;
use aes::Aes128;
use cipher::{StreamCipher, KeyIvInit};
use hex_literal::hex;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use sha3::Digest;
use thiserror::Error;
use bytemuck::{Pod, PodCastError, Zeroable};

use crate::save::{crypto::util::{elgamal::{Elgamal, Pair}}, game::Game};

type Aes128Ofb = ofb::Ofb<Aes128>;

pub struct Lime;

// I probably could do this stuff in place, but i don't think its worht the hassle
impl Lime {
    pub fn decrypt(buf: &[u8], key: u64, decrypted_len: u64) -> Result<Vec<u8>, LimeError> {
        // 1. prng shit
        // 2. rsa init from prng
        // 3. decrypt key, iv from buf with rsa
        // 4. aes_init(key, iv)
        // 5. aes_decrypt()
        // 6. sha3
        let num_blocks = (decrypted_len >> 12) + ((decrypted_len & 0xfff) != 0) as u64;
        let mut decrypted = vec![0u8; num_blocks as usize * 0x1000];
        //let mut remaining = decrypted_len;
        let mut buf_offset = 0;
        let mut dec_offset = 0;
        let elgamal_ctx = Elgamal::init(!key)?;
        for i in 0..num_blocks {
            log::info!("Processing block {i}");
            let block_slice = &buf[buf_offset..buf_offset+0x1220];
            let block = bytemuck::try_from_bytes::<Block>(block_slice)?;
            let key_iv = elgamal_ctx.decrypt_pairs(block.enc_key);
            let key = &key_iv[0..16];
            let iv = &key_iv[16..32];
            //println!("{}", hex::encode(key_iv));
            let mut cipher = Aes128Ofb::new_from_slices(&key, &iv)?;
            let dec_buf = &mut decrypted[dec_offset..dec_offset+0x1000];
            cipher.apply_keystream_b2b(&block.data, dec_buf);

            let mut hasher = sha3::Sha3_256::new();
            hasher.update(dec_buf);
            let checksum: [u8; 32] = hasher.finalize().into();

            if checksum != block.checksum {
                log::warn!("Lime Invalid Checksum");
            }

            /*let mut dec_bytes_len = 0x1000;
            if remaining < 0x1000 {
               dec_bytes_len = remaining;
            }*/

            buf_offset += 0x1220;
            dec_offset += 0x1000;
            //remaining -= 0x1000;
        }

        // NOTE: there is a 0x80 sized thing here but i dont really care, pretty sure its just an
        // RSA mac
        decrypted.truncate(decrypted_len as usize);
        Ok(decrypted)
    }

    pub fn encrypt(buf: &[u8], key: u64) -> Result<Vec<u8>, LimeError> {
        let len = buf.len();
        let num_blocks = (len >> 12) + ((len & 0xfff) != 0) as usize;
        let mut encrypted = vec![0u8; num_blocks as usize * 0x1220 + 0x80];
        let mut buf_offset = 0;
        let mut enc_offset = 0;
        let mut remaining = len;
        let elgamal_ctx = Elgamal::init(!key)?;
        for i in 0..num_blocks {
            log::info!("Processing block {i}");
            let block_slice = &mut encrypted[enc_offset..enc_offset+0x1220];
            let block = bytemuck::try_from_bytes_mut::<Block>(block_slice)?;

            let key_iv: [u8; 32] = rand::random();
            let key = &key_iv[0..16];
            let iv = &key_iv[16..32];
            block.enc_key = elgamal_ctx.encrypt_bytes(key_iv);

            let mut dec_bytes_len = 0x1000;
            if remaining < 0x1000 {
               dec_bytes_len = remaining;
            }

            block.data[..dec_bytes_len]
                .copy_from_slice(&buf[buf_offset..buf_offset+dec_bytes_len]);

            let mut hasher = sha3::Sha3_256::new();
            hasher.update(block.data);
            block.checksum = hasher.finalize().into();

            let mut cipher = Aes128Ofb::new_from_slices(&key, &iv)?;
            cipher.apply_keystream(&mut block.data);

            enc_offset += 0x1220;
            buf_offset += 0x1000;
            remaining -= 0x1000;
        }

        // this might not be right, but it doesnt matter since it doesnt even get read (i just took
        // the one from wilds/mandarin)
        let n = hex!(
            "4fa448364f5b3507e945075cc21994bdedef96962c74d53159d50a5c62ed50864885ddfe79705dfad0b638220ca2299fccae152164590cc89d33698452a8f6416107a6952f126bb21ee3e332d2285db728c09bfa8cbd4c3b13b358b9838dea7cf39dc12e37066a09cf7809a0d0ea06c3bbaa14776400f403f863ed83b5bdd3c2"
        );
        let rands_int = Integer::from(key);
        let n = bytes_to_int(&n);
        let e = Integer::from(65537);
        let encrypted_key = mod_exp(&rands_int, &e, &n);
        let integer = int_to_bytes_le::<0x80>(&encrypted_key);
        encrypted[enc_offset..enc_offset+ 0x80].copy_from_slice(&integer);
        Ok(encrypted)
    }



    /*pub fn optimized_lime_brute_force(
        block: &Block, 
        p: &Integer, 
        known_magic_bytes: &[u8], 
        steamid_list: &[u64]
    ) -> Option<u64> {

        // C1 from the ElGamal ciphertext
        let c1 = bytes_to_int(&block.enc_key[0].0); 

        let found = steamid_list.into_par_iter().find_any(|&steamid| {
            // 1. Init private key
            let inv_key = !steamid;
            let private_key = Integer::from(inv_key);

            // 2. Heavy Math: GMP-accelerated ModPow
            // shared_secret = C1^private_key mod P
            let shared_secret = c1.clone().secure_pow_mod(&private_key, p);

            // 3. Extract AES Key/IV (Requires BigInt division/multiplicative inverse)
            let aes_material = decrypt_elgamal_payload(&block.enc_key, &shared_secret, p);
            let key = &aes_material[0..16];
            let iv = &aes_material[16..32];

            // 4. EARLY EXIT: Only decrypt the first 16 bytes (1 AES Block)
            let mut cipher = Aes128Ofb::new_from_slices(key, iv).unwrap();
            let mut first_block = block.data[0..16].to_vec();
            cipher.apply_keystream(&mut first_block);

            // 5. Check Magic Bytes before wasting time on SHA3
            if !first_block.starts_with(known_magic_bytes) {
                return false;
            }

            // 6. Full Verification (Only runs if magic bytes match)
            let mut full_data = block.data.clone();
            let mut full_cipher = Aes128Ofb::new_from_slices(key, iv).unwrap();
            full_cipher.apply_keystream(&mut full_data);

            let checksum: [u8; 32] = sha3::Sha3_256::digest(&full_data).into();
            checksum == block.checksum
        });

        found.copied()
    }*/
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Block {
    pub enc_key: [Pair; 4],
    pub data: [u8; 0x1000],
    pub checksum: [u8; 0x20]
}

#[derive(Error, Debug)]
pub enum LimeError {
    #[error("block error {0}")]
    BlockError(PodCastError),
    #[error("cipher error {0}")]
    CipherError(#[from] cipher::InvalidLength),
    #[error("error {0}")]
    Misc(#[from] Box<dyn std::error::Error>)
}

impl From<PodCastError> for LimeError {
    fn from(value: PodCastError) -> Self {
        Self::BlockError(value)
    }
}
