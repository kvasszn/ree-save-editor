use std::{error::Error, fmt::Display, time::SystemTime, hash::Hasher};

use hex_literal::hex;
use aes::{cipher::{ KeyIvInit, StreamCipher }, Aes128};
use bytemuck::{Pod, Zeroable};
use fasthash::{city::Hasher64, FastHasher};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

pub struct SplitMix64;

impl SplitMix64 {
    pub const ADD: u64 = 0x9e3779b97f4a7c15;
    pub const MUL1: u64 = 0xbf58476d1ce4e5b9;
    pub const MUL2: u64 = 0x94d049bb133111eb;
    pub fn next_int(state: &mut u64) -> u64 {
        *state = state.wrapping_add(Self::ADD);
        let mut z: u64 = *state;
        z = (z ^ (z >> 30)).wrapping_mul(Self::MUL1);
        z = (z ^ (z >> 27)).wrapping_mul(Self::MUL2);
        return z ^ (z >> 31);
    }
}

// Metadata key and IV bigint math
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct EncodedKeyChunk {
    b: [u8; 64],
    a: [u8; 64],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct BlockMetaDataRaw {
    chunks: [EncodedKeyChunk; 4], // key/iv checker
    checksum: u64, // computed from city64 or farmhash of decrypted data
    litterally_no_idea: [u8; 8]
}

impl BlockMetaDataRaw {
    pub const _M: [u8; 32] = hex!("f33b6fb972a0b72515e45c391829e182ad8a9bdc0a64d3444d79c810ab863717");
    pub const _P2: [u8; 32] = hex!("f99db75c39d0db920a72ae1c8c9470c156c54d6e05b269a2a63c648855c39b0b");
    pub const _A: [u8; 32] = hex!("a1002346d9d854e5c18e4ce4eb641bed7d282226c7268648c1909d59abfa7215");
    pub const _B: [u8; 32] = hex!("e66f544afcce68c5ef07b9a07b277585344a1db61376e831f73b9fbd5f44f715");
    pub const _E: u64 = 0x14;

    pub const POW_B_E_MOD_M: [u8; 32] = hex!("bb39c81702d5495b671844d2f5ee2b77b811eb6e076b236e41ee7ce342f51612");
    pub const MUL: [u8; 32] = hex!("73365201b3ee153739a4d55137cdd5d5783d1bc8b4c005141ef689ceb2bf9901");

    pub fn get_key(&self) -> [u8; 16] {
        let mut key = [0u8; 16];
        for i in 0..2 {
            let chunk = self.chunks[i];
            let a = num_bigint::BigUint::from_bytes_le(&chunk.a);
            let _b = num_bigint::BigUint::from_bytes_le(&chunk.b);
            let mul = num_bigint::BigUint::from_bytes_le(&Self::MUL);
            let k = a / mul;
            key[i*8..i*8+8].copy_from_slice(&k.to_bytes_le()[0..8]);
        }
        key
    }
    pub fn get_iv(&self) -> [u8; 16] {
        let mut iv = [0u8; 16];
        for i in 0..2 {
            let chunk = self.chunks[i + 2];
            let a = num_bigint::BigUint::from_bytes_le(&chunk.a);
            let _b = num_bigint::BigUint::from_bytes_le(&chunk.b);
            let mul = num_bigint::BigUint::from_bytes_le(&Self::MUL);
            let k = a / mul;
            iv[i*8..i*8+8].copy_from_slice(&k.to_bytes_le()[0..8]);
        }
        iv
    }
}

#[derive(Debug)]
pub enum MandarinError {
    InvalidChecksum{target: u64, real: u64},
    InvalidKey{target: [u8; 16], real: [u8; 16]},
    InvalidIV{target: [u8; 16], real: [u8; 16]},
}

impl Error for MandarinError {}

impl Display for MandarinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::InvalidChecksum { target, real } => {
                write!(f, "Invalid Checksum target={target:#018x}, real={real:#018x}")
            }
            Self::InvalidKey { target, real } => {
                write!(f, "Key Mismatch target={:#018x}, real={:#018x}", u128::from_be_bytes(*target), u128::from_be_bytes(*real))
            }
            Self::InvalidIV { target, real } => {
                write!(f, "IV Mismatch target={:#018x}, real={:#018x}", u128::from_be_bytes(*target), u128::from_be_bytes(*real))
            }
        }
    }
}




#[derive(Debug)]
pub struct Mandarin {}

impl Mandarin {
    pub fn brute_force(encrypted: &[u8], decrypted_len: u64) -> u64 {

        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize]; // honestly no idea when this is
                                                                        // allocated, its on the stack
                                                                        // but like variable size
        let mut state_p: u64 = 0x7A36955255266CED;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        // determine how many blocks we actually have, have to do this in order like this for the
        // prng state to be correct
        /*let mut decryption_len_leftover = decrypted_len;
        let mut num_real_blocks = 0;
        for i in 0..num_potential_blocks {
            let b = block_sizes[i as usize] as u64;
            num_real_blocks += 1;
            if decryption_len_leftover <= b * 0x4000 {
                break
            }
            decryption_len_leftover -= b * 0x4000;
        }*/

        // loop through each real block that fits in the decrypted length

        let initial_state_p = state_p;
        //let block_size = block_sizes[0] as usize * 0x4000;
        let c = 0xffffffffu64;
        println!("trying {} keys", c);
        let s = SystemTime::now();
        let good_key = (0..0xffffffff).into_par_iter().find_first(|i| {
            let mut buf = [0u8; 0x210];
            let key = 0x0110000100000000 + i;
            let mut state_p = initial_state_p.wrapping_add(!key);
            buf[0..0x210].copy_from_slice(&encrypted[0..0x210]);

            // generate key and iv
            let mut key = [0u8; 16];
            let mut iv = [0u8; 16];
            for j in 0..16 {
                state_p = SplitMix64::next_int(&mut state_p);
                key[j] = state_p as u8;
                iv[j] = (state_p >> 8) as u8;
            }

            // this contains meta data for the block
            // 4 pairs of big ints and a 16 byte checksum
            for j in 0..0x210 {
                state_p = SplitMix64::next_int(&mut state_p);
                buf[j] = buf[j] ^ state_p as u8;
            }

            let metadata_raw = bytemuck::from_bytes::<BlockMetaDataRaw>(&buf[..0x210]);
            let key2 = metadata_raw.get_key();
            let iv2 = metadata_raw.get_iv();

            key2 == key && iv2 == iv
        });

        let taken = s.elapsed().unwrap().as_secs_f64();
        println!("time taken for {} keys {}, {}k/s", c, taken, c as f64 / taken);
        if let Some(good_key) = good_key {
            println!("[Key/IV check] passed with key={:#x}", 0x0110000100000000 + good_key);
            return 0x0110000100000000 + good_key
        }
        0x0
    }

    pub fn decrypt(encrypted: &[u8], decrypted_len: u64, key: u64) -> Result<Vec<u8>, MandarinError>{
        // generate 32 bytes of randomness, first 8 bytes or the key inverted
        // This doesn't even do anything, idk why its in the binary, only the bottom bytes of rands are
        // used, which end up just being the inverse of the key
        /*let mut state_a: u64 = 0xBFACF76C3F96;
          let mut rands: [u8; 0x20] = [0u8; 0x20];
          for i in 0..32 {
          state_a = SplitMix64::next_int(&mut state_a); // this doesnt even actually do anything???
          rands[i] = state_a as u8;
          }*/

        // copy key inverse to first u64 of rands
        //rands[0..8].copy_from_slice(&(!key).to_le_bytes());

        // calculate the block sizes >> 0xe for each "potential block"
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize]; // honestly no idea when this is
                                                                        // allocated, its on the stack
                                                                        // but like variable size
        let mut state_p: u64 = 0x7A36955255266CED;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        // determine how many blocks we actually have, have to do this in order like this for the
        // prng state to be correct
        let mut decryption_len_leftover = decrypted_len;
        let mut num_real_blocks = 0;
        for i in 0..num_potential_blocks {
            let b = block_sizes[i as usize] as u64;
            num_real_blocks += 1;
            if decryption_len_leftover <= b * 0x4000 {
                break
            }
            decryption_len_leftover -= b * 0x4000;
        }

        //state_p = state_p.wrapping_add(unsafe{*(rands.as_ptr() as *const u64)});
        state_p = state_p.wrapping_add(!key);

        // loop through each real block that fits in the decrypted length
        let mut encrypted_start = 0;
        let mut decrypted_start = 0;
        let mut buf = vec![0u8; 0x20210];
        let mut decrypted = vec![0u8; decrypted_len as usize];
        let mut remaining_bytes = decrypted_len as usize;

        for i in 0..num_real_blocks as usize {
            //println!("BLOCK {} out of {num_real_blocks}", i + 1);
            let block_size = block_sizes[i] as usize * 0x4000;
            let encrypted_read_size = block_size + 0x210;
            buf[0..encrypted_read_size].copy_from_slice(&encrypted[encrypted_start..encrypted_start+encrypted_read_size]);

            // generate key and iv
            let mut key = [0u8; 16];
            let mut iv = [0u8; 16];
            for j in 0..16 {
                state_p = SplitMix64::next_int(&mut state_p);
                key[j] = state_p as u8;
                iv[j] = (state_p >> 8) as u8;
            }

            // this contains meta data for the block
            // 4 pairs of big ints and a 16 byte checksum
            for j in 0..0x210 {
                state_p = SplitMix64::next_int(&mut state_p);
                buf[j] = buf[j] ^ state_p as u8;
            }

            /*for b in &buf[0x200..0x210] {
              print!("{b:02x}");
            }
            println!();*/

            // key check
            let metadata_raw = bytemuck::from_bytes::<BlockMetaDataRaw>(&buf[..0x210]);
            let key2 = metadata_raw.get_key();
            let iv2 = metadata_raw.get_iv();

            if key2 != key {
                println!("[Key/IV check] key mismatch, skipping check");
                //return Err(MandarinError::InvalidKey { target: key2, real: key})
            }
            if iv2 != iv {
                println!("[Key/IV check] IV mismatch, skipping check");
                //return Err(MandarinError::InvalidIV { target: iv2, real: iv})
            } 
            if key2 == key && iv2 == iv {
                println!("[Key/IV check] passed");
            }

            let target_checksum = metadata_raw.checksum;
            type Aes128Ofb = ofb::Ofb<Aes128>;
            let mut cipher = Aes128Ofb::new(&key.into(), &iv.into());
            let mut data = &mut buf[0x210..0x210+block_size];
            cipher.apply_keystream(&mut data);


            let bytes_to_copy = block_size.min(remaining_bytes);
            //checksum
            let mut h = Hasher64::new();
            h.write(&data[..bytes_to_copy]);
            let checksum = h.finish();
            if checksum != target_checksum {
                println!("[Checksum] failed");
                return Err(MandarinError::InvalidChecksum { target: target_checksum, real: checksum })
            } else {
                println!("[Checksum] passed")
            }
            decrypted[decrypted_start..decrypted_start + bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
            remaining_bytes = remaining_bytes.wrapping_sub(block_size);
            decrypted_start += bytes_to_copy;
            encrypted_start += encrypted_read_size;
            //println!("remaining_bytes={remaining_bytes:#x}");
        }

        Ok(decrypted)
    }
}
