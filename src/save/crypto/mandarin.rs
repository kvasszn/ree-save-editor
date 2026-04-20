use crate::save::game::Game;

//#[cfg(target_os = "linux")]
//use super::backend::rug::*;
//#[cfg(not(target_os = "linux"))]
use super::util::SplitMix64;
use super::util::backend::num_bigint::*;

use std::{
    error::Error,
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

use aes::{
    Aes128,
    cipher::{KeyIvInit, StreamCipher},
};
use bytemuck::{Pod, Zeroable};
use hex_literal::hex;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use web_time::Instant;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Pair([u8; 64], [u8; 64]);

// elgamal??
#[derive(Debug, Clone)]
struct AuthCtx {
    p: Integer, // prime
    #[allow(unused)]
    q: Integer, // prime
    r: Integer, // integer
    s: Integer, // r ^ u mod q
    u: Integer, // id mod q
    e: Integer, // 0x14
}

type Aes128Ofb = ofb::Ofb<Aes128>;

impl AuthCtx {
    pub const P: [u8; 32] =
        hex!("f33b6fb972a0b72515e45c391829e182ad8a9bdc0a64d3444d79c810ab863717");
    pub const Q: [u8; 32] =
        hex!("f99db75c39d0db920a72ae1c8c9470c156c54d6e05b269a2a63c648855c39b0b");
    pub const R: [u8; 32] =
        hex!("e66f544afcce68c5ef07b9a07b277585344a1db61376e831f73b9fbd5f44f715");

    pub fn init(u: u64) -> crate::reerr::Result<Self> {
        let p = bytes_to_int(&Self::P);
        let q = bytes_to_int(&Self::Q);
        let r = bytes_to_int(&Self::R);
        let u = Integer::from(u);
        let u = u % &q;
        let s = mod_exp(&r, &u, &p);
        let e = Integer::from(0x14u64);
        Ok(AuthCtx { p, q, r, s, u, e })
    }

    /*pub fn update_u(&mut self, u: u64) {
    let u = Integer::from(u) % &self.q;
    let s = self.r.pow_mod_ref(&u, &self.p).unwrap().complete();
    self.u = u;
    self.s = s;
    }*/

    pub fn encrypt(&self, pt: [u8; 8]) -> Pair {
        let x0 = mod_exp(&self.r, &self.e, &self.p);
        let x1 = mod_exp(&self.s, &self.e, &self.p);
        let pt = bytes_to_int(&pt);
        let ct = x1 * pt;
        let res0 = int_to_bytes_le::<64>(&x0);
        let res1 = int_to_bytes_le::<64>(&ct);
        Pair(res0, res1)
    }

    pub fn decrypt(&self, chunk: &Pair) -> [u8; 8] {
        let x0 = bytes_to_int(&chunk.0);
        let ct = bytes_to_int(&chunk.1);
        let x = mod_exp(&x0, &self.u, &self.p);
        let k = ct / x;
        int_to_bytes_le::<8>(&k)
    }

    pub fn encrypt_bytes(&self, pt: [u8; 32]) -> [Pair; 4] {
        let mut chunks = [Pair([0u8; 64], [0u8; 64]); 4];
        for i in 0..4 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&pt[i * 8..i * 8 + 8]);
            chunks[i] = self.encrypt(buf);
        }
        chunks
    }

    pub fn decrypt_block(&self, block: Block) -> [u8; 32] {
        let mut res = [0u8; 32];
        for (i, chunk) in block.chunks.iter().enumerate() {
            let x = self.decrypt(chunk);
            res[i * 8..i * 8 + 8].copy_from_slice(&x);
        }
        res
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Block {
    chunks: [Pair; 4], // key/iv checker
    checksum: u64,     // computed from city64 or farmhash of decrypted data
    litterally_no_idea: [u8; 8],
}

#[derive(Debug)]
pub enum MandarinError {
    InvalidChecksum { target: u64, real: u64 },
    InvalidKey { target: [u8; 16], real: [u8; 16] },
    InvalidIV { target: [u8; 16], real: [u8; 16] },
    GameNotSupported { game: Game },
    AuthFailed,
    // (index, len)
    OutOfBounds(usize, usize),
}

impl Error for MandarinError {}

impl Display for MandarinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::InvalidChecksum { target, real } => {
                write!(
                    f,
                    "Invalid Checksum target={target:#018x}, real={real:#018x}"
                )
            }
            Self::InvalidKey { target, real } => {
                write!(
                    f,
                    "Key Mismatch target={:#018x}, real={:#018x}",
                    u128::from_be_bytes(*target),
                    u128::from_be_bytes(*real)
                )
            }
            Self::InvalidIV { target, real } => {
                write!(
                    f,
                    "IV Mismatch target={:#018x}, real={:#018x}",
                    u128::from_be_bytes(*target),
                    u128::from_be_bytes(*real)
                )
            }
            Self::GameNotSupported { game } => {
                write!(f, "Game {game:?} does not support Mandarin Encryption")
            }
            Self::AuthFailed => {
                write!(f, "Auth Failed idk")
            }
            Self::OutOfBounds(index, len) => {
                write!(f, "{index} out of bounds for buffer of len {len}")
            }
        }
    }
}

// Note: keys are different for pragmata, seems to not be steamid, or they just don't care because
// its the demo, or a single player game, its constant
#[derive(Debug)]
pub struct Mandarin {
    seed_for_rsa_rand: u64,
    seed_for_enc_rand: u64,
}

impl Mandarin {
    pub fn init_from_game(game: Game) -> Result<Self, MandarinError> {
        let seeds = game.get_mandarin_seeds();
        match seeds {
            None => Err(MandarinError::GameNotSupported { game }),
            Some(x) => Ok(Self {
                seed_for_rsa_rand: x.0,
                seed_for_enc_rand: x.1,
            }),
        }
    }

    /*pub fn recover_state_from_mask(mask: [u8; 8]) -> u64 {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);

        // Create our unknown 64-bit starting state
        let mut state = BV::new_const(&ctx, "state", 64);
        let start_state = state.clone();

        let add = BV::from_u64(&ctx, SplitMix64::ADD, 64);
        let mul1 = BV::from_u64(&ctx, SplitMix64::MUL1, 64);
        let mul2 = BV::from_u64(&ctx, SplitMix64::MUL2, 64);

        for &expected_byte in &mask {
            // state = state.wrapping_add(ADD);
            state = state.bvadd(&add);
            let mut z = state.clone();

            // z = (z ^ (z >> 30)).wrapping_mul(MUL1);
            let z_sh30 = z.bvlshr(&BV::from_u64(&ctx, 30, 64));
            z = z.bvxor(&z_sh30).bvmul(&mul1);

            // z = (z ^ (z >> 27)).wrapping_mul(MUL2);
            let z_sh27 = z.bvlshr(&BV::from_u64(&ctx, 27, 64));
            z = z.bvxor(&z_sh27).bvmul(&mul2);

            // state = z ^ (z >> 31);
            let z_sh31 = z.bvlshr(&BV::from_u64(&ctx, 31, 64));
            state = z.bvxor(&z_sh31);

            // mask[i] == state as u8; (extract the lowest 8 bits)
            let extracted_byte = state.extract(7, 0);
            let target = BV::from_u64(&ctx, expected_byte as u64, 8);
            solver.assert(&extracted_byte._eq(&target));
        }

        // Crunch the equations
        assert_eq!(solver.check(), z3::SatResult::Sat, "Z3 failed to find a valid PRNG state");

        let model = solver.get_model().unwrap();
        let resolved = model.eval(&start_state, true).unwrap();

        resolved.as_u64().unwrap()
    }*/

    /*pub fn find_seed(&self, encrypted: &[u8], key: u64, decrypted_len: u64) -> u64 {
        let p = bytes_to_int(&AuthCtx::P);
        let r = bytes_to_int(&AuthCtx::R);
        let e = Integer::from(0x14u64);

        let expected_x0 = mod_exp(&r, &e, &p);
        let expected_x0_bytes = int_to_bytes_le::<64>(&expected_x0);

        let mut target_prng_mask = [0u8; 8];
        for i in 0..8 {
            target_prng_mask[i] = encrypted[i] ^ expected_x0_bytes[i];
        }
        println!("{target_prng_mask:?}");

        // 1. Solve for the true 64-bit state using our leaked 8 bytes
        println!("[+] Running Z3 solver to reconstruct 64-bit state from mask...");
        let mut state = 0;//Self::recover_state_from_mask(target_prng_mask);

        // 2. Undo the 16 key/iv iterations (MUST subtract ADD)
        for _ in 0..16 {
            state = SplitMix64::unmix(state).wrapping_sub(SplitMix64::ADD);
        }

        // 3. Undo the inverted key injection
        state = state.wrapping_sub(!key);

        // 4. Undo the block sizes loop
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        for _ in 0..num_potential_blocks {
            state = SplitMix64::unmix(state).wrapping_sub(SplitMix64::ADD);
        }

        println!("[+] Successfully recovered seed_for_enc_rand: {:#018x} ({})", state, state);

        state
    }*/

    #[cfg(target_arch = "wasm32")]
    pub fn brute_force(&self, encrypted: &[u8], decrypted_len: u64, game: Game, base: u64, count: u64) -> u64 {
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize];
        let mut state_p: u64 = self.seed_for_enc_rand;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        let p = bytes_to_int(&AuthCtx::P);
        let r = bytes_to_int(&AuthCtx::R);
        let e = Integer::from(0x14u64);

        let expected_x0 = mod_exp(&r, &e, &p);
        let expected_x0_bytes = int_to_bytes_le::<64>(&expected_x0);

        let mut target_prng_mask = [0u8; 8];
        for i in 0..8 {
            target_prng_mask[i] = encrypted[i] ^ expected_x0_bytes[i];
        }

        let initial_state_p = state_p;
        let s = Instant::now();
        log::info!("[BRUTE FORCE] Starting brute force for {} keys", count);
        let good_key = (0..count as usize)
            .into_par_iter()
            .find_map_any(|steamid| {
                let mut mask = [0u8; 8];
                let steamid = steamid as u64 + base;
                let steamid_game = game.get_key_from_steamid(steamid as u64);
                let inv_key = !(steamid_game as u64);
                let mut state_p = initial_state_p.wrapping_add(inv_key);
                for _ in 0..16 {
                    state_p = SplitMix64::next_int(&mut state_p);
                }

                for i in 0..8 {
                    state_p = SplitMix64::next_int(&mut state_p);
                    mask[i] = state_p as u8;
                }
                if mask == target_prng_mask[0..8] {
                    Some(steamid)
                } else {
                    None
                }
            });

        let taken = s.elapsed().as_secs_f64();
        log::info!("time taken: {taken:.2}s");
        if let Some(good_key) = good_key {
            println!("[Key/IV check] passed with key={}", good_key);
            return good_key;
        }
        0x0
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn brute_force(&self, encrypted: &[u8], decrypted_len: u64, game: Game, base: u64, count: u64) -> u64 {
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize];
        let mut state_p: u64 = self.seed_for_enc_rand;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        let p = bytes_to_int(&AuthCtx::P);
        let r = bytes_to_int(&AuthCtx::R);
        let e = Integer::from(0x14u64);

        let expected_x0 = mod_exp(&r, &e, &p);
        let expected_x0_bytes = int_to_bytes_le::<64>(&expected_x0);

        let mut target_prng_mask = [0u8; 8];
        for i in 0..8 {
            target_prng_mask[i] = encrypted[i] ^ expected_x0_bytes[i];
        }

        let progress = AtomicUsize::new(0);
        let initial_state_p = state_p;
        let s = Instant::now();
        let chunk_size = 1_000_000;
        println!("[BRUTE FORCE] Starting brute force for {} keys", count);
        let good_key = (0..count as usize)
            .into_par_iter()
            .chunks(chunk_size)
            .find_map_any(|steamids| {
                let mut found_key = None;
                let mut checked = chunk_size;
                let mut mask = [0u8; 8];
                for (i, steamid) in steamids.iter().enumerate() {
                    let steamid = *steamid as u64 + base;
                    let steamid_game = game.get_key_from_steamid(steamid);
                    let inv_key = !(steamid_game as u64);
                    let mut state_p = initial_state_p.wrapping_add(inv_key);
                    for _ in 0..16 {
                        state_p = SplitMix64::next_int(&mut state_p);
                    }

                    for i in 0..8 {
                        state_p = SplitMix64::next_int(&mut state_p);
                        mask[i] = state_p as u8;
                    }
                    if mask == target_prng_mask[0..8] {
                        found_key = Some(steamid);
                        checked = i;
                        break;
                    }
                }
                let completed = progress.fetch_add(checked, Ordering::Relaxed);
                print!("\rChecked {} / {} keys", completed, count);
                use std::io::Write;
                let _ = std::io::stdout().flush();
                found_key
            });
        println!("");

        let taken = s.elapsed().as_secs_f64();
        let completed = progress.load(Ordering::Relaxed);
        println!(
            "time taken for {completed} keys: {taken:.2}s @ {} keys/s",
            completed as f64 / taken
        );
        if let Some(good_key) = good_key {
            println!("[Key/IV check] passed with key={}", good_key);
            return good_key;
        }
        0x0
    }

    pub fn decrypt(
        &self,
        encrypted: &[u8],
        decrypted_len: u64,
        key: u64,
    ) -> Result<Vec<u8>, MandarinError> {
        // This is different for dif games, i think it actually just gets generated somehow before
        // the module gets launched, but whatever values you find work for anyone
        let mut state_a: u64 = self.seed_for_rsa_rand;
        let mut rands: [u8; 0x20] = [0u8; 0x20];
        for i in 0..32 {
            state_a = SplitMix64::next_int(&mut state_a); // this doesnt even actually do anything???
            rands[i] = state_a as u8;
        }
        rands[0..8].copy_from_slice(&(!key).to_le_bytes());

        let len = encrypted.len();
        let encrypted_key = &encrypted[len - 0x80 - 12..len - 12];
        println!("[DECRYPT] RSA Integer {encrypted_key:?}");

        // calculate the block sizes >> 0xe for each "potential block"
        let num_potential_blocks = ((decrypted_len & 0x3fff != 0) as u64) + (decrypted_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize]; // honestly no idea when this is
        // allocated, its on the stack
        // but like variable size

        let mut state_p: u64 = self.seed_for_enc_rand;
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
                break;
            }
            encryption_len_leftover -= b * 0x4000;
        }

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
            let encrypted_end = encrypted_start + encrypted_read_size;
            if encrypted_end > encrypted.len() {
                return Err(MandarinError::OutOfBounds(encrypted_end, encrypted.len()));
            }
            if encrypted_read_size > buf.len() {
                return Err(MandarinError::OutOfBounds(encrypted_read_size, buf.len()));
            }
            buf[0..encrypted_read_size].copy_from_slice(&encrypted[encrypted_start..encrypted_end]);

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
            //println!("mul={:?}", auth_block.chunks[0].0);
            //println!("ct={:?}", auth_block.chunks[0].1);

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
            let mut cipher = Aes128Ofb::new(&key.into(), &iv.into());
            let mut data = &mut buf[0x210..0x210 + block_size];
            cipher.apply_keystream(&mut data);

            let bytes_to_copy = block_size.min(remaining_bytes);
            //checksum
            let checksum = cityhasher::hash::<u64>(&data[..bytes_to_copy]);
            if checksum != target_checksum {
                log::error!("[Checksum] block {i}: failed");
                return Err(MandarinError::InvalidChecksum {
                    target: target_checksum,
                    real: checksum,
                });
            } else {
                println!("[Checksum] block {i}: passed")
            }
            //println!("{key:?}, {iv:?}, {checksum:x}");
            decrypted[decrypted_start..decrypted_start + bytes_to_copy]
                .copy_from_slice(&data[..bytes_to_copy]);
            remaining_bytes = remaining_bytes.wrapping_sub(block_size);
            decrypted_start += bytes_to_copy;
            encrypted_start += encrypted_read_size;
            //println!("remaining_bytes={remaining_bytes:#x}");
        }

        //let integer_buf = &encrypted[encrypted_start..encrypted_start+0x80];
        //let integer = bytes_to_int(&integer_buf);
        //println!("RSA Integer={integer:?}");
        //println!("encrypted_key={encrypted_key:?}, {integer:?}");
        Ok(decrypted)
    }

    pub fn encrypt(&self, data: &[u8], key: u64) -> Result<Vec<u8>, MandarinError> {
        //let mut state_a: u64 = 0xBFACF76C3F96;
        //let n = hex!("c2d3bdb583ed63f803f400647714aabbc306ead0a00978cf096a06372ec19df37cea8d83b958b3133b4cbd8cfa9bc028b75d28d232e3e31eb26b122f95a6076141f6a8528469339dc80c59642115aecc9f29a20c2238b6d0fa5d7079fedd85488650ed625c0ad55931d5742c9696efedbd9419c25c0745e907355b4f3648a44f");
        let mut state_a: u64 = self.seed_for_rsa_rand;
        let mut rands: [u8; 0x20] = [0u8; 0x20];
        for i in 0..32 {
            SplitMix64::next_int(&mut state_a); // this doesnt even actually do anything???
            rands[i] = state_a as u8;
        }
        rands[0..8].copy_from_slice(&(!key).to_le_bytes());

        let n = hex!(
            "4fa448364f5b3507e945075cc21994bdedef96962c74d53159d50a5c62ed50864885ddfe79705dfad0b638220ca2299fccae152164590cc89d33698452a8f6416107a6952f126bb21ee3e332d2285db728c09bfa8cbd4c3b13b358b9838dea7cf39dc12e37066a09cf7809a0d0ea06c3bbaa14776400f403f863ed83b5bdd3c2"
        );
        //let rands_int = Integer::from_digits(&rands[0..32], Order::Lsf);
        let rands_int = bytes_to_int(&rands[0..32]);
        let n = bytes_to_int(&n);
        let e = Integer::from(65537);
        let encrypted_key = mod_exp(&rands_int, &e, &n);

        //let rands_int = bytes_to_int(&rands[0..32]);
        //println!("{n:?}");
        //let n = Integer::from_digits(&n, Order::Lsf);

        // calculate the block sizes >> 0xe for each "potential block"
        let data_len = data.len() as u64;
        let num_potential_blocks = ((data_len & 0x3fff != 0) as u64) + (data_len >> 0xe);
        let mut block_sizes = vec![0u8; num_potential_blocks as usize]; // honestly no idea when this is
        // allocated, its on the stack
        // but like variable size
        let mut state_p: u64 = self.seed_for_enc_rand;
        for i in 0..num_potential_blocks as usize {
            block_sizes[i] = (state_p & 7) as u8 + 1;
            state_p = SplitMix64::next_int(&mut state_p);
        }

        // determine how many blocks we actually have, have to do this in order like this for the
        // prng state to be correct
        let mut len_leftover = data_len;
        let mut num_real_blocks = 0;
        let mut total_encrypted_len = 0x80;
        for i in 0..num_potential_blocks {
            let b = block_sizes[i as usize] as u64;
            let block_size = b * 0x4000;
            num_real_blocks += 1;
            total_encrypted_len += block_size + 0x210;
            if len_leftover <= block_size {
                break;
            }
            len_leftover -= block_size;
        }

        state_p = state_p.wrapping_add(!key);

        // loop through each real block that fits in the decrypted length
        let mut encrypted_start = 0;
        let mut decrypted_start = 0;
        let mut buf = vec![0u8; 0x20210];
        let mut encrypted = vec![0u8; total_encrypted_len as usize];
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
            //println!("to_copy={bytes_to_copy}, block_size={block_size}, remaining_bytes={remaining_bytes}");
            buf[0x210..bytes_to_copy + 0x210]
                .copy_from_slice(&data[decrypted_start..decrypted_start + bytes_to_copy]);

            let mut key_iv = [0u8; 32];
            key_iv[0..16].copy_from_slice(&key);
            key_iv[16..32].copy_from_slice(&iv);
            let chunks = auth.encrypt_bytes(key_iv);
            let checksum = cityhasher::hash::<u64>(&buf[0x210..0x210 + bytes_to_copy]);
            //println!("{key:?}, {iv:?}, {checksum}");

            type Aes128Ofb = ofb::Ofb<Aes128>;
            let mut cipher = Aes128Ofb::new(&key.into(), &iv.into());
            cipher.apply_keystream(&mut buf[0x210..0x210 + block_size]);

            //  i have no idea what the last 0x8 bytes are in this block thing
            let block = Block {
                chunks,
                checksum,
                litterally_no_idea: [0u8; 8],
            };

            //println!("mul={:?}", block.chunks[0].0);
            //println!("ct={:?}", block.chunks[0].1);

            let auth_block = bytemuck::bytes_of::<Block>(&block);
            buf[0..0x210].copy_from_slice(auth_block);
            for j in 0..0x210 {
                state_p = SplitMix64::next_int(&mut state_p);
                buf[j] = buf[j] ^ state_p as u8;
            }

            encrypted[encrypted_start..encrypted_start + block_size + 0x210]
                .copy_from_slice(&buf[..block_size + 0x210]);
            remaining_bytes = remaining_bytes.wrapping_sub(bytes_to_copy);
            decrypted_start += bytes_to_copy;
            encrypted_start += block_size + 0x210;
            //println!("remaining_bytes={remaining_bytes:#x}");
        }
        let integer = int_to_bytes_le::<0x80>(&encrypted_key);
        println!("[ENCRYPTION] RSA Integer={encrypted_key:?}");
        //println!("encrypted_key={encrypted_key:?}, {integer:?}");
        encrypted[encrypted_start..encrypted_start + 0x80].copy_from_slice(&integer);
        Ok(encrypted[..encrypted_start + 0x80].to_vec())
    }
}
