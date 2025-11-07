use std::{error::Error, fmt::Display, hash::Hasher, time::SystemTime};

use hex_literal::hex;
use aes::{cipher::{ KeyIvInit, StreamCipher }, Aes128};
use bytemuck::{Pod, Zeroable};
use fasthash::{city::{self}, FastHash, FastHasher};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rug::{integer::Order, Complete, Integer};

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

fn integer_to_bytes_le<const N: usize>(x: &Integer) -> [u8; N] {
    let mut buf = [0u8; N];
    let x_bytes = x.to_digits::<u8>(Order::Lsf); // Lsf = least-significant first = little-endian
    let len = x_bytes.len().min(N);
    buf[..len].copy_from_slice(&x_bytes[..len]);
    buf
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Pair([u8; 64], [u8; 64]);

// elgamal??
#[derive(Debug, Clone)]
struct AuthCtx {
    p: Integer, // prime
    q: Integer, // prime
    r: Integer, // integer
    s: Integer, // r ^ u mod q
    u: Integer, // id mod q
    e: Integer, // 0x14
}

impl AuthCtx {
    pub const P: [u8; 32] = hex!("f33b6fb972a0b72515e45c391829e182ad8a9bdc0a64d3444d79c810ab863717");
    pub const Q: [u8; 32] = hex!("f99db75c39d0db920a72ae1c8c9470c156c54d6e05b269a2a63c648855c39b0b");
    pub const R: [u8; 32] = hex!("e66f544afcce68c5ef07b9a07b277585344a1db61376e831f73b9fbd5f44f715");
    pub fn init(u: u64) -> crate::reerr::Result<Self> {
        let p = Integer::from_digits(&Self::P, Order::Lsf);
        let q = Integer::from_digits(&Self::Q, Order::Lsf);
        let r = Integer::from_digits(&Self::R, Order::Lsf);
        let u = Integer::from(u);
        let u = u.modulo(&q);
        let s = r.pow_mod_ref(&u, &p).unwrap().complete();
        let e = Integer::from(0x14u64);
        Ok(AuthCtx {
            p, q, r, s, u, e,
        })
    }

    pub fn update_u(&mut self, u: u64) {
        let u = Integer::from(u) % &self.q;
        let s = self.r.pow_mod_ref(&u, &self.p).unwrap().complete();
        self.u = u;
        self.s = s;
    }

    pub fn encrypt(&self, pt: [u8; 8]) -> Pair {
        let x0 = self.r.pow_mod_ref(&self.e, &self.p).unwrap().complete();
        let x1 = self.s.pow_mod_ref(&self.e, &self.p).unwrap().complete();
        let pt = Integer::from_digits(&pt, Order::Lsf);
        let ct = x1 * pt;
        let res0 = integer_to_bytes_le::<64>(&x0);
        let res1 = integer_to_bytes_le::<64>(&ct);
        Pair(res0, res1)
    }

    pub fn decrypt(&self, chunk: &Pair) -> [u8; 8] {
        let x0 = Integer::from_digits::<u8>(&chunk.0, Order::Lsf);
        let ct = Integer::from_digits::<u8>(&chunk.1, Order::Lsf);
        let x = x0.pow_mod_ref(&self.u, &self.p).unwrap().complete();
        let k = ct / x;
        integer_to_bytes_le::<8>(&k)
    }

    pub fn encrypt_bytes(&self, pt: [u8; 32]) -> [Pair; 4] {
        let mut chunks = [Pair([0u8; 64], [0u8; 64]); 4];
        for i in 0..4 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&pt[i*8..i*8+8]);
            chunks[i] = self.encrypt(buf);
        }
        chunks
    }

    pub fn decrypt_block(&self, block: Block) -> [u8; 32] {
        let mut res = [0u8; 32];
        for (i, chunk) in block.chunks.iter().enumerate() {
            let x = self.decrypt(chunk);
            res[i*8..i*8+8].copy_from_slice(&x);
        }
        res
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Block {
    chunks: [Pair; 4], // key/iv checker
    checksum: u64, // computed from city64 or farmhash of decrypted data
    litterally_no_idea: [u8; 8]
}

#[derive(Debug)]
pub enum MandarinError {
    InvalidChecksum{target: u64, real: u64},
    InvalidKey{target: [u8; 16], real: [u8; 16]},
    InvalidIV{target: [u8; 16], real: [u8; 16]},
    AuthFailed,
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
            Self::AuthFailed => {
                write!(f, "Auth Failed idk")
            }
        }
    }
}




#[derive(Debug)]
pub struct Mandarin {}

impl Mandarin {
    // this is not very feasible unless i optimize the fuck out of it somehow
    pub fn brute_force(encrypted: &[u8], decrypted_len: u64) -> u64 {
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize];
        let mut state_p: u64 = 0x7A36955255266CED;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        let initial_state_p = state_p;
        //let block_size = block_sizes[0] as usize * 0x4000;
        let start = 0x10000000u64;
        let end = 0xffffffffu64;
        let count = end - start;
        println!("trying {} keys", count);
        let s = SystemTime::now();
        let good_key = (start..end).into_par_iter().find_first(|i| {
            let mut buf = [0u8; 0x210];
            let auth = AuthCtx::init(!i).unwrap();
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

            let auth_block = bytemuck::from_bytes::<Block>(&buf[..0x210]);
            let key_iv2 = auth.decrypt_block(*auth_block);
            key_iv2[0..16] == key && key_iv2[16..32] == iv
        });

        let taken = s.elapsed().unwrap().as_secs_f64();
        println!("time taken for {} keys {}, {}k/s", count, taken, count as f64 / taken);
        if let Some(good_key) = good_key {
            println!("[Key/IV check] passed with key={:#x}", 0x0110000100000000 + good_key);
            return 0x0110000100000000 + good_key
        }
        0x0
    }

    pub fn decrypt(encrypted: &[u8], decrypted_len: u64, key: u64) -> Result<Vec<u8>, MandarinError>{
        // generate 32 bytes of randomness, first 8 bytes or the key inverted
        // used for encrypting the key with RSA and appending it to the encrypted stuff
        // doesn't seem to get used in decryption, maybe it does after calling decrypt from
        // mandarin
        // although n is a constant so it might just be for capcom save recovery (if they know p q)
        let mut state_a: u64 = 0xBFACF76C3F96;
        let mut rands: [u8; 0x20] = [0u8; 0x20];
        for i in 0..32 {
            state_a = SplitMix64::next_int(&mut state_a); // this doesnt even actually do anything???
            rands[i] = state_a as u8;
        }
        rands[0..8].copy_from_slice(&(!key).to_le_bytes());


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
        let mut encryption_len_leftover = decrypted_len;
        let mut num_real_blocks = 0;
        for i in 0..num_potential_blocks {
            let b = block_sizes[i as usize] as u64;
            num_real_blocks += 1;
            if encryption_len_leftover <= b * 0x4000 {
                break
            }
            encryption_len_leftover -= b * 0x4000;
        }

        //state_p = state_p.wrapping_add(unsafe{*(rands.as_ptr() as *const u64)});
        state_p = state_p.wrapping_add(!key);

        // loop through each real block that fits in the decrypted length
        let mut encrypted_start = 0;
        let mut decrypted_start = 0;
        let mut buf = vec![0u8; 0x20210];
        let mut decrypted = vec![0u8; decrypted_len as usize];
        let mut remaining_bytes = decrypted_len as usize;
        let auth = AuthCtx::init(!key).unwrap();

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

            // key check
            let auth_block = bytemuck::from_bytes::<Block>(&buf[..0x210]);
            let key_iv2 = auth.decrypt_block(*auth_block);
            println!("mul={:?}", auth_block.chunks[0].0);
            println!("ct={:?}", auth_block.chunks[0].0);

            if key_iv2[0..16] != key {
                println!("[Key/IV check] block {i}: key mismatch, skipping check");
                //return Err(MandarinError::InvalidKey { target: key2, real: key})
            }
            if key_iv2[16..32] != iv {
                println!("[Key/IV check] block {i}: IV mismatch, skipping check");
                //return Err(MandarinError::InvalidIV { target: iv2, real: iv})
            } 
            if key_iv2[0..16] == key && key_iv2[16..32] == iv {
                println!("[Key/IV check] block {i}: passed");
            }

            let target_checksum = auth_block.checksum;
            type Aes128Ofb = ofb::Ofb<Aes128>;
            let mut cipher = Aes128Ofb::new(&key.into(), &iv.into());
            let mut data = &mut buf[0x210..0x210+block_size];
            cipher.apply_keystream(&mut data);

            let bytes_to_copy = block_size.min(remaining_bytes);
            //checksum
            let checksum = city::Hash64::hash(&data[..bytes_to_copy]);
            if checksum != target_checksum {
                println!("[Checksum] block {i}: failed");
                return Err(MandarinError::InvalidChecksum { target: target_checksum, real: checksum })
            } else {
                println!("[Checksum] block {i}: passed")
            }
            println!("{key:?}, {iv:?}, {checksum}");
            decrypted[decrypted_start..decrypted_start + bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
            remaining_bytes = remaining_bytes.wrapping_sub(block_size);
            decrypted_start += bytes_to_copy;
            encrypted_start += encrypted_read_size;
            println!("remaining_bytes={remaining_bytes:#x}");
        }

        Ok(decrypted)
    }

    pub fn encrypt(data: &[u8], key: u64) -> Result<Vec<u8>, MandarinError>{
        let mut state_a: u64 = 0xBFACF76C3F96;
        let mut rands: [u8; 0x20] = [0u8; 0x20];
        for i in 0..32 {
            SplitMix64::next_int(&mut state_a); // this doesnt even actually do anything???
            rands[i] = state_a as u8;
        }
        rands[0..8].copy_from_slice(&(!key).to_le_bytes());

        let rands_int = Integer::from_digits(&rands[0..32], Order::Lsf);
        let n = hex!("4fa448364f5b3507e945075cc21994bdedef96962c74d53159d50a5c62ed50864885ddfe79705dfad0b638220ca2299fccae152164590cc89d33698452a8f6416107a6952f126bb21ee3e332d2285db728c09bfa8cbd4c3b13b358b9838dea7cf39dc12e37066a09cf7809a0d0ea06c3bbaa14776400f403f863ed83b5bdd3c2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let n = Integer::from_digits(&n, Order::Lsf);
        let e = Integer::from(65537);
        let encrypted_key = rands_int.pow_mod_ref(&e, &n).unwrap().complete();

        // calculate the block sizes >> 0xe for each "potential block"
        let data_len = data.len() as u64;
        let num_potential_blocks = ((data_len & 0x3fff != 0) as u64) + (data_len >> 0xe);
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
        let mut len_leftover = data_len;
        let mut num_real_blocks = 0;
        for i in 0..num_potential_blocks {
            let b = block_sizes[i as usize] as u64;
            num_real_blocks += 1;
            if len_leftover <= b * 0x4000 {
                break
            }
            len_leftover -= b * 0x4000;
        }

        //state_p = state_p.wrapping_add(unsafe{*(rands.as_ptr() as *const u64)});
        state_p = state_p.wrapping_add(!key);

        // loop through each real block that fits in the decrypted length
        let mut encrypted_start = 0;
        let mut decrypted_start = 0;
        let mut buf = vec![0u8; 0x20210];
        let mut encrypted = vec![0u8; 0x200000];
        let mut remaining_bytes = data_len as usize;

        let auth = AuthCtx::init(!key).unwrap();

        for i in 0..num_real_blocks as usize {
            // generate key and iv
            let mut key = [0u8; 16];
            let mut iv = [0u8; 16];
            for j in 0..16 {
                state_p = SplitMix64::next_int(&mut state_p);
                key[j] = state_p as u8;
                iv[j] = (state_p >> 8) as u8;
            }

            //Copy the data to the work buffer, offset by 0x210 for the metadata, might be clearer
            //to separate the two
            let block_size = block_sizes[i] as usize * 0x4000;
            let bytes_to_copy = block_size.min(remaining_bytes);
            println!("to_copy={bytes_to_copy}, block_size={block_size}, remaining_bytes={remaining_bytes}");
            buf[0x210..bytes_to_copy + 0x210].copy_from_slice(&data[decrypted_start..decrypted_start+bytes_to_copy]);

            let mut key_iv = [0u8; 32];
            key_iv[0..16].copy_from_slice(&key);
            key_iv[16..32].copy_from_slice(&iv);
            let chunks = auth.encrypt_bytes(key_iv);
            let checksum = city::Hash64::hash(&buf[0x210..0x210+bytes_to_copy]);
            println!("{key:?}, {iv:?}, {checksum}");

            type Aes128Ofb = ofb::Ofb<Aes128>;
            let mut cipher = Aes128Ofb::new(&key.into(), &iv.into());
            cipher.apply_keystream(&mut buf[0x210..0x210+block_size]);

            //  i have no idea what the last 0x8 bytes are in this block thing
            let block = Block {
                chunks,
                checksum,
                litterally_no_idea: [0u8; 8]
            };

            println!("mul={:?}", block.chunks[0].0);
            println!("ct={:?}", block.chunks[0].1);

            let auth_block = bytemuck::bytes_of::<Block>(&block);
            buf[0..0x210].copy_from_slice(auth_block);
            for j in 0..0x210 {
                state_p = SplitMix64::next_int(&mut state_p);
                buf[j] = buf[j] ^ state_p as u8;
            }

            encrypted[encrypted_start..encrypted_start + block_size + 0x210].copy_from_slice(&buf[..block_size + 0x210]);
            remaining_bytes = remaining_bytes.wrapping_sub(bytes_to_copy);
            decrypted_start += bytes_to_copy;
            encrypted_start += block_size + 0x210;
            println!("remaining_bytes={remaining_bytes:#x}");
        }
        let integer = integer_to_bytes_le::<0x80>(&encrypted_key);
        encrypted[encrypted_start..encrypted_start+0x80].copy_from_slice(&integer);
        Ok(encrypted[..encrypted_start+0x80].to_vec())
    }
}
