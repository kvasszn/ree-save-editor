use aes::cipher::block_padding::NoPadding;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use elliptic_curve::zeroize::Zeroize;
use rand::random;
use sha3::Digest;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

//#[cfg(target_os = "linux")]
//use super::backend::rug::*;
//#[cfg(not(target_os = "linux"))]
use super::util::EccInteger;
use super::util::backend::num_bigint::*;
use super::util::{point_add, scalar_mult};
use hex_literal::hex;

#[derive(Debug)]
pub struct CurveParams {
    pub index: u32,
    pub p: Integer,
    pub a: Integer,
    pub b: Integer,
    pub gx: Integer,
    pub gy: Integer,
}

impl From<&CurveParamsRaw> for CurveParams {
    fn from(value: &CurveParamsRaw) -> Self {
        CurveParams {
            index: value.index,
            p: bytes_to_int(&value.p),
            a: bytes_to_int(&value.a),
            b: bytes_to_int(&value.b),
            gx: bytes_to_int(&value.gx),
            gy: bytes_to_int(&value.gy),
        }
    }
}

pub struct Citrus {
    steamid64: u64,
    d: Integer,
    param_index: Option<usize>,
}

impl Citrus {
    const KEY_IV_SIZE: usize = 0x20;
    const ENC_KEYS_SIZE: usize = 0x200;
    const ENC_DATA_SIZE: usize = 0x40000;
    const HASH_SIZE: usize = 0x20;
    const BLOCK_SIZE: usize =
        Self::KEY_IV_SIZE + Self::ENC_KEYS_SIZE + Self::ENC_DATA_SIZE + Self::HASH_SIZE;

    pub fn new(steamid64: u64, param_index: Option<usize>) -> Self {
        Self {
            steamid64,
            param_index,
            d: Integer::from(!steamid64),
        }
    }

    fn public_key(&self, params: &CurveParams) -> Option<(Integer, Integer)> {
        let g = (params.gx.clone(), params.gy.clone());
        scalar_mult(&self.d, g, &params.a, &params.p)
    }

    fn aes_encrypt(buf: &mut [u8], key: [u8; 16], iv: [u8; 16]) -> &[u8] {
        let mut patch = key;
        patch.iter_mut().zip(iv).for_each(|(a, b)| *a ^= b);

        for i in (16..buf.len()).step_by(16) {
            let block = &mut buf[i..i + 16];
            for j in 0..16 {
                block[j] ^= patch[j];
            }
        }

        Aes128CbcEnc::new(&key.into(), &iv.into())
            .encrypt_padded_mut::<NoPadding>(buf, buf.len())
            .unwrap();

        &buf[..]
    }

    fn aes_decrypt(buf: &mut [u8], key: [u8; 16], iv: [u8; 16]) -> &[u8] {
        let pt = Aes128CbcDec::new(&key.into(), &iv.into())
            .decrypt_padded_mut::<NoPadding>(buf)
            .unwrap();

        let mut patch = key;
        patch.iter_mut().zip(iv).for_each(|(a, b)| *a ^= b);

        for i in (16..pt.len()).step_by(16) {
            let block = &mut buf[i..i + 16];
            for j in 0..16 {
                block[j] ^= patch[j];
            }
        }

        &buf[..]
    }

    fn encrypt_ec_elgamal(
        k: &Integer,
        plaintext_point: (Integer, Integer),
        pub_key: (Integer, Integer), // Q = d*G
        params: &CurveParams,
    ) -> Option<((Integer, Integer), (Integer, Integer))> {
        let g = (params.gx.clone(), params.gy.clone());
        // c1 = k*G
        let c1 = scalar_mult(k, g, &params.a, &params.p)?;
        // c2 = plaintext_point + k*Q
        let kq = scalar_mult(k, pub_key, &params.a, &params.p)?;
        let c2 = point_add(Some(plaintext_point), Some(kq), &params.a, &params.p)?;
        Some((c1, c2))
    }

    fn decrypt_ec_elgamal(
        &self,
        c1: (Integer, Integer),
        c2: (Integer, Integer),
        params: &CurveParams,
    ) -> Option<(Integer, Integer)> {
        let s = scalar_mult(&self.d, c1, &params.a, &params.p)?;
        let (s_x, s_y) = s;
        let neg_s = (s_x, params.p.sub_mod(&s_y, &params.p));
        let p_m = point_add(Some(c2), Some(neg_s), &params.a, &params.p);
        p_m
    }

    fn encrypt_key_segment(
        k: &Integer,
        value: u64, // the 64-bit half of key/iv
        pub_key: (Integer, Integer),
        params: &CurveParams,
    ) -> Option<[u8; 128]> {
        // inverse of decrypt_key_segment: value = p_m.0 / 100
        // so plaintext x = value * 100
        let x = Integer::from(value) * Integer::from(100);
        // you need to recover a valid point with this x coordinate
        // y^2 = x^3 + ax + b mod p
        let x3 = x.modpow(&Integer::from(3u32), &params.p);
        let ax = (&params.a * &x) % &params.p;
        let rhs = (x3 + ax + &params.b) % &params.p;
        // compute y = rhs^((p+1)/4) mod p  (assumes p ≡ 3 mod 4)
        let exp = (&params.p + Integer::from(1u32)) / Integer::from(4u32);
        let y = rhs.modpow(&exp, &params.p);
        let plaintext_point = (x, y);

        let (c1, c2) = Self::encrypt_ec_elgamal(k, plaintext_point, pub_key, params)?;

        let mut out = [0u8; 128];
        out[0..32].copy_from_slice(&int_to_bytes_le::<32>(&c1.0));
        out[32..64].copy_from_slice(&int_to_bytes_le::<32>(&c1.1));
        out[64..96].copy_from_slice(&int_to_bytes_le::<32>(&c2.0));
        out[96..128].copy_from_slice(&int_to_bytes_le::<32>(&c2.1));
        Some(out)
    }

    fn decrypt_key_segment(&self, segment: &[u8], params: &CurveParams) -> Option<Integer> {
        let c1_x = <Integer as EccInteger>::from_bytes_le(&segment[0..32]);
        let c1_y = <Integer as EccInteger>::from_bytes_le(&segment[32..64]);
        let c2_x = <Integer as EccInteger>::from_bytes_le(&segment[64..96]);
        let c2_y = <Integer as EccInteger>::from_bytes_le(&segment[96..128]);
        let c1 = (c1_x, c1_y);
        let c2 = (c2_x, c2_y);
        let p_m = self.decrypt_ec_elgamal(c1, c2, params)?;
        let x: Integer = p_m.0 / 100;
        Some(x)
    }

    fn ecc_encrypt_keys(
        &self,
        key: [u8; 16],
        iv: [u8; 16],
        pub_key: (Integer, Integer),
        params: &CurveParams,
    ) -> Option<[u8; 512]> {
        let mut out = [0u8; 512];
        let k0 = Integer::from(rand::random::<u64>());
        let k1 = Integer::from(rand::random::<u64>());
        let k2 = Integer::from(rand::random::<u64>());
        let k3 = Integer::from(rand::random::<u64>());

        let key_lo = u64::from_le_bytes(key[0..8].try_into().unwrap());
        let key_hi = u64::from_le_bytes(key[8..16].try_into().unwrap());
        let iv_lo = u64::from_le_bytes(iv[0..8].try_into().unwrap());
        let iv_hi = u64::from_le_bytes(iv[8..16].try_into().unwrap());

        out[0..128].copy_from_slice(&Self::encrypt_key_segment(
            &k0,
            key_lo,
            pub_key.clone(),
            params,
        )?);
        out[128..256].copy_from_slice(&Self::encrypt_key_segment(
            &k1,
            key_hi,
            pub_key.clone(),
            params,
        )?);
        out[256..384].copy_from_slice(&Self::encrypt_key_segment(
            &k2,
            iv_lo,
            pub_key.clone(),
            params,
        )?);
        out[384..512].copy_from_slice(&Self::encrypt_key_segment(
            &k3,
            iv_hi,
            pub_key.clone(),
            params,
        )?);
        Some(out)
    }

    fn ecc_decrypt_keys(
        &self,
        buf: &[u8; 512],
        params: &CurveParams,
    ) -> Option<([u8; 16], [u8; 16])> {
        let mut key = [0u8; 16];
        let mut iv = [0u8; 16];
        key[0..8].copy_from_slice(
            &self
                .decrypt_key_segment(&buf[0..128], params)?
                .to_u64_le_bytes(),
        );
        key[8..16].copy_from_slice(
            &self
                .decrypt_key_segment(&buf[128..256], params)?
                .to_u64_le_bytes(),
        );
        iv[0..8].copy_from_slice(
            &self
                .decrypt_key_segment(&buf[256..384], params)?
                .to_u64_le_bytes(),
        );
        iv[8..16].copy_from_slice(
            &self
                .decrypt_key_segment(&buf[384..512], params)?
                .to_u64_le_bytes(),
        );
        Some((key, iv))
    }

    // for completeness, just do the whole thang until you find one that works
    fn brute_force_find_params(&self, buf: &[u8], decrypted_len: usize) -> Option<CurveParams> {
        let num_blocks = buf.len() / Self::BLOCK_SIZE;
        let mut key = [0u8; 16];
        let mut iv = [0u8; 16];
        let mut ecc_keys = [0u8; Self::ENC_KEYS_SIZE];
        let mut dec_buf = [0u8; Self::ENC_DATA_SIZE];
        let block = &buf[..Self::BLOCK_SIZE];

        println!(
            "decrypted_len: {decrypted_len:x}, num_blocks: {num_blocks}, buflen {:x}",
            buf.len()
        );
        println!(" {:x}", block.len());

        // Decrypt the ECC encrypted keys
        key.copy_from_slice(&block[0..16]);
        iv.copy_from_slice(&block[16..32]);
        ecc_keys.copy_from_slice(&block[32..32 + Self::ENC_KEYS_SIZE]);
        Self::aes_decrypt(&mut ecc_keys, key, iv);

        // Decrypt the main data aes keys with ECC
        // if the curve params are unset try to brute force
        for curve in &CURVES {
            let curve_params = CurveParams::from(curve);
            let (key, iv) = self.ecc_decrypt_keys(&ecc_keys, &curve_params)?;
            //let dec_buf = &mut decrypted[decrypted_bytes..decrypted_bytes + block_size];
            // when decrypting, the blocks are always padded to the 0x40000 length i think
            dec_buf.copy_from_slice(
                &block[32 + Self::ENC_KEYS_SIZE..32 + Self::ENC_KEYS_SIZE + Self::ENC_DATA_SIZE],
            );
            Self::aes_decrypt(&mut dec_buf, key, iv);
            println!("{}, {:?}", curve.index, &dec_buf[0..32]);
            /*if &dec_buf[5..8] == &[0u8; 3] {
                println!("[INFO] Found params at index {}", curve_params.index);
                return Some(curve_params)
            }*/
            // idk how much this actually works, could probably lower it to like 10%
            let num_zeros = dec_buf.iter().filter(|&&b| b == 0).count();
            if num_zeros > dec_buf.len() / 2 {
                println!("[INFO] Found params at index {}", curve_params.index);
                return Some(curve_params);
            }
        }
        None
    }

    // could maybe also add something to change the buf in place
    pub fn decrypt(&self, buf: &[u8], decrypted_len: usize) -> Option<Vec<u8>> {
        let num_blocks = buf.len() / Self::BLOCK_SIZE;
        let mut decrypted = vec![0u8; decrypted_len];
        let mut offset = 0;
        let mut decrypted_bytes = 0;
        let mut key = [0u8; 16];
        let mut iv = [0u8; 16];
        let mut ecc_keys = [0u8; Self::ENC_KEYS_SIZE];
        let curve_params = self
            .param_index
            .map(|i| CurveParams::from(&CURVES[i]))
            .or_else(|| self.brute_force_find_params(buf, decrypted_len))?;

        println!(
            "decrypted_len: {decrypted_len:x}, num_blocks: {num_blocks}, buflen {:x}",
            buf.len()
        );
        for i in 0..num_blocks {
            let mut dec_buf = [0u8; Self::ENC_DATA_SIZE];
            dec_buf.zeroize();
            //println!("block: {i}");
            let block = &buf[offset..offset + Self::BLOCK_SIZE];
            //println!(" {:x}", block.len());
            // Decrypt the ECC encrypted keys
            // TODO: figure out how this shit even gets generated lmfao
            // it probably is just mersennes shitter
            // they might also be completely random!
            // hahahahahahahahahahahahahahahaahahahahhaahhahahahah
            // these are not the same on every save, so i'm hoping they are just completely random
            key.copy_from_slice(&block[0..16]);
            iv.copy_from_slice(&block[16..32]);
            println!("key={}, iv={}", hex::encode(key), hex::encode(iv));
            ecc_keys.copy_from_slice(&block[32..32 + Self::ENC_KEYS_SIZE]);
            Self::aes_decrypt(&mut ecc_keys, key, iv);

            // Decrypt the main data aes keys with ECC
            // if the curve params are unset try to brute force
            let (real_key, real_iv) = self.ecc_decrypt_keys(&ecc_keys, &curve_params)?;
            let block_size = if i == num_blocks - 1 {
                decrypted_len - decrypted_bytes
            } else {
                Self::ENC_DATA_SIZE
            };
            //let dec_buf = &mut decrypted[decrypted_bytes..decrypted_bytes + block_size];
            // when decrypting, the blocks are always padded to the 0x40000 length i think

            // this AES should decrypted to a new buffer i think instead of modifying the old one?
            dec_buf.copy_from_slice(
                &block[32 + Self::ENC_KEYS_SIZE..32 + Self::ENC_KEYS_SIZE + Self::ENC_DATA_SIZE],
            );
            Self::aes_decrypt(&mut dec_buf, real_key, real_iv);
            decrypted[decrypted_bytes..decrypted_bytes + block_size]
                .copy_from_slice(&dec_buf[..block_size]);
            dec_buf[block_size..].zeroize();

            let mut hasher = sha3::Sha3_256::new();

            // this could just be written to one big buffer for the output
            //let hash_data: Vec<u8> = [key.as_slice(), &iv, &ecc_keys, &dec_buf[..0x40000 - 0x20]].concat();
            //let _ = std::fs::write("./outputs/hash_data.bin", &hash_data);
            hasher.update(&key);
            hasher.update(&iv);
            hasher.update(&ecc_keys);
            hasher.update(&dec_buf[..0x40000 - 0x20]);
            //hasher.update(&hash_data);

            // TODO: Figure out how the last blocks checksum works, is it the whole thing, or just
            // whatever got decrytped or smoething
            let checksum: [u8; 32] = hasher.finalize().into();
            let target_checksum = &block[0x40220..0x40240];

            if checksum != target_checksum {
                eprintln!(
                    "[ERROR] Citrus Checksum not equal on block {i}, checksum={}, target={}",
                    hex::encode(checksum),
                    hex::encode(target_checksum)
                );
            } else {
                println!(
                    "[INFO] Citrus Checksum equal on block {i}, checksum={}, target={}",
                    hex::encode(checksum),
                    hex::encode(target_checksum)
                );
            }

            offset += Self::BLOCK_SIZE;
            decrypted_bytes += block_size;
        }

        // TODO: FIgure out what the fuck is after this the little 0x1008 sized shit thing
        /* Ok so, 0x8 right after the data is probably a hash of some kind, im guessing the
        * plaintext

           I THINK IVE BEEN HALLUCINATING THE 0x8 LOLOLOLOLOL thats just part of the fucking keccak hash thing shit

        * the 0x1000 block right after that can just be zeroed out, the game doesnt seem to care
        * if i had to guess, it's some signature of some key related stuff, since its so big, maybe
        * the ecc params?
        */
        // NOTE: the 0x1000 at the end is aes encrypted with something, i dont thinnk i really need it but its fine
        // TODO: for completeness, at least read the block, maybe store it in the save to copy it
        // over to a new one?
        // also check the hash
        println!("[INFO] Citrus: total decrypted_bytes: {decrypted_bytes:x}, offset: {offset:x}");
        println!("[INFO] probably at least a little decrypted");
        let _ = std::fs::write("./outputs/decrypted.bin", &decrypted);
        Some(decrypted)
    }

    pub fn encrypt(&self, buf: &[u8]) -> Option<Vec<u8>> {
        let params = self.param_index.map(|i| CurveParams::from(&CURVES[i]))?;
        let pub_key = self.public_key(&params)?;

        let num_blocks = buf.len().div_ceil(Self::ENC_DATA_SIZE);
        let total_size = num_blocks * Self::BLOCK_SIZE + 0x1000;
        let mut out = vec![0u8; total_size];

        let mut offset = 0;
        let mut pt_offset = 0;

        for i in 0..num_blocks {
            println!("[INFO] Encrypting Block {i}");
            let block = &mut out[offset..offset + Self::BLOCK_SIZE];
            let key: [u8; 16] = rand::random();
            let iv: [u8; 16] = rand::random();
            let real_key: [u8; 16] = rand::random();
            let real_iv: [u8; 16] = rand::random();
            let ecc_keys = self.ecc_encrypt_keys(real_key, real_iv, pub_key.clone(), &params)?;

            let mut ecc_keys_enc = ecc_keys.clone();
            Self::aes_encrypt(&mut ecc_keys_enc, key, iv);

            let chunk_start = pt_offset;
            let chunk_end = (pt_offset + Self::ENC_DATA_SIZE).min(buf.len());
            let chunk_len = chunk_end - chunk_start;

            let mut enc_data = [0u8; Self::ENC_DATA_SIZE];
            enc_data[..chunk_len].copy_from_slice(&buf[chunk_start..chunk_end]);
            let mut plain_data = enc_data.clone();
            plain_data[chunk_len..].fill(0);
            Self::aes_encrypt(&mut enc_data, real_key, real_iv);

            let mut hasher = sha3::Sha3_256::new();
            hasher.update(&key);
            hasher.update(&iv);
            hasher.update(&ecc_keys);
            hasher.update(&plain_data[..Self::ENC_DATA_SIZE - Self::HASH_SIZE]);
            let checksum: [u8; 32] = hasher.finalize().into();
            block[0..16].copy_from_slice(&key);
            block[16..32].copy_from_slice(&iv);
            block[32..32 + Self::ENC_KEYS_SIZE].copy_from_slice(&ecc_keys_enc);
            block[32 + Self::ENC_KEYS_SIZE..32 + Self::ENC_KEYS_SIZE + Self::ENC_DATA_SIZE]
                .copy_from_slice(&enc_data);
            block[0x40220..0x40240].copy_from_slice(&checksum);

            offset += Self::BLOCK_SIZE;
            pt_offset += chunk_len;
        }

        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_aes() {
        let key: [u8; 16] = rand::random();
        let iv: [u8; 16] = rand::random();
        let original = b"hello world 1234";
        let mut buf = original.clone();
        Citrus::aes_encrypt(&mut buf, key, iv);
        Citrus::aes_decrypt(&mut buf, key, iv);
        assert_eq!(&buf, original, "AES roundtrip failed");
    }

    #[test]
    fn test_roundtrip_ecc() {
        let steamid = 12345678u64;
        let citrus = Citrus::new(steamid, Some(0));
        let params = CurveParams::from(&CURVES[0]);
        let pub_key = citrus.public_key(&params).unwrap();

        let real_key: [u8; 16] = rand::random();
        let real_iv: [u8; 16] = rand::random();
        println!("real_key={}", hex::encode(real_key));
        println!("real_iv={}", hex::encode(real_iv));

        let ecc_keys = citrus
            .ecc_encrypt_keys(real_key, real_iv, pub_key, &params)
            .unwrap();
        let (dec_key, dec_iv) = citrus.ecc_decrypt_keys(&ecc_keys, &params).unwrap();

        println!("dec_key={}", hex::encode(dec_key));
        println!("dec_iv={}", hex::encode(dec_iv));

        assert_eq!(real_key, dec_key, "ECC key roundtrip failed");
        assert_eq!(real_iv, dec_iv, "ECC iv roundtrip failed");
    }

    #[test]
    fn test_roundtrip_full() {
        let steamid = 12345678u64;
        let citrus = Citrus::new(steamid, Some(0));
        let plaintext = vec![0x42u8; 1024];
        let encrypted = citrus.encrypt(&plaintext).unwrap();
        // pass plaintext.len() as decrypted_len
        let decrypted = citrus.decrypt(&encrypted, plaintext.len()).unwrap();
        assert_eq!(plaintext, decrypted, "Full roundtrip failed");
    }
}

pub struct CurveParamsRaw {
    pub index: u32,
    pub p: [u8; 32],
    pub a: [u8; 32],
    pub b: [u8; 32],
    pub gx: [u8; 32],
    pub gy: [u8; 32],
}

pub const CURVES: [CurveParamsRaw; 128] = [
    CurveParamsRaw {
        index: 0,
        p: hex!("a32b7275abf38ae23525a20e71da8b3bbfed37e5769c0d765d92fa0000000000"),
        a: hex!("51c0d26f74378d7014af9d013d3ef471a30952b9ea90e4a01f27e00000000000"),
        b: hex!("ba33d873aa81f32118bdf4985122ca4486b7cfde8b2c80dbe441e70000000000"),
        gx: hex!("be0e176e7f82fac0d506db34a878dc9e713dd5d3380eb8d75b9e0f0000000000"),
        gy: hex!("bcef2e72fc0b9ab45db3b72ebdf892dac13e2c9d3a7d4f3765286c0000000000"),
    },
    CurveParamsRaw {
        index: 1,
        p: hex!("4daaed424d45410039f220141b66614ad0669956cd30529c35ff2c0000000000"),
        a: hex!("490ea3b4e3d74a376dc703a1236f3d203a885d9da866570b20ab1b0000000000"),
        b: hex!("6b1af1b9f245eb421d41d0194020410f2eabc4f59dc6376bb087010000000000"),
        gx: hex!("27c23e87e7916d6480c2842ebb5bfdccdcc2d889427895b4af1e1a0000000000"),
        gy: hex!("a1a38fbdf4187a0ac2e35822a3dff19cc16ca53aa8af9ad54ab5010000000000"),
    },
    CurveParamsRaw {
        index: 2,
        p: hex!("ddf561e596e5d806bace81d46f51cb9162662dae10c62b635d638b0000000000"),
        a: hex!("b46fcf6fb4da468efd2c6b18931ce5b3d326c3e44234e35fb9a11c0000000000"),
        b: hex!("9b46d3541be46112df7b1f60344bd9412e1fd1e0f33f6f7620703f0000000000"),
        gx: hex!("2b9ab938b18e7c96c65bffc234ea780fd6acc0f485c42bef5cc5100000000000"),
        gy: hex!("06a03f467d91c7ebc926564e2c5e0c114097969dc58f41f5f65f460000000000"),
    },
    CurveParamsRaw {
        index: 3,
        p: hex!("fd69c98524c32cea516cece00c3cfaefc67e08914c4757df4cad8b0000000000"),
        a: hex!("f692588e27ed62e38e55584f746fe26b2c18ba2035d8f20733c94a0000000000"),
        b: hex!("1f2a8daf53be4de83af6f2cd556572c6e9ae71eef84ed852b2fe010000000000"),
        gx: hex!("bea1ba6dc8922d8a79ff76aa30e8b5bab33d255e5547e4c49fcc070000000000"),
        gy: hex!("8f64c6899fd54611aa37e3bf2cdf0d389b510d47e789c774eca4760000000000"),
    },
    CurveParamsRaw {
        index: 4,
        p: hex!("414f9968f8a1f98e9b5fa4c144ae8e93ef85a5fb5af863da8892270000000000"),
        a: hex!("756c10e781b7340ad4de504bba88bfe61036bf517d2c54972c970c0000000000"),
        b: hex!("1bcfbc4260f7c568947aacce5ed58c647b1632a8e7efc1a93e2a1b0000000000"),
        gx: hex!("76e46874878ec7216ba6965a39c621bc9a164f8fdd1fd38e6250050000000000"),
        gy: hex!("f62297b2936bfa0b69d5d82f4301c14afda809a330c46562b72d0c0000000000"),
    },
    CurveParamsRaw {
        index: 5,
        p: hex!("1d5ce8343985c110c1f036b0d9c9d50cccbb45e68f769b955104520000000000"),
        a: hex!("6d8173d2d51067f38d4b92f89717d524d6b4a8dff518167c8998400000000000"),
        b: hex!("bd6be100ff228e1739dcbe106a62458a1a72de06544c903369e3030000000000"),
        gx: hex!("20aeefb3ccd58b945e19b523e4c3dc4cee98c7883f21a4abdea0280000000000"),
        gy: hex!("e1caf689999500045589eafb5c13714ba4d42be81c2103029876080000000000"),
    },
    CurveParamsRaw {
        index: 6,
        p: hex!("c573233b2afce8079fd9764746f15d31c05074903ba58d0a5e8f2f0000000000"),
        a: hex!("a78da6938dce5965ed069732c666b9cd2a690abb65ce71a00782260000000000"),
        b: hex!("5e014e5a31c06c441295852cafab432578832ab42cfe173763911f0000000000"),
        gx: hex!("22d7e07fb9df211d0797c35a344915fb96319035ae51b2a75b83050000000000"),
        gy: hex!("c53c537538b38f547ec51eec7fdb44aa5db288fd5b88d763140d030000000000"),
    },
    CurveParamsRaw {
        index: 7,
        p: hex!("8542719fb111c7e921cc47da04395c7c1ff64b6c9a66fe5e6612e90000000000"),
        a: hex!("aa47d25cdf9f132f9c182d3af92dda69cf41875356336f95808c970000000000"),
        b: hex!("ae9ca57b3b2e43ee872c740ba482512cce99f48d6027174a79ad6b0000000000"),
        gx: hex!("99e7edec6c2e35b3cae574cbec3db177ef5abed5da0104f03003410000000000"),
        gy: hex!("2cc11e15942cf8823c615c2bfd523bce1f1bec5d98c9312ed534640000000000"),
    },
    CurveParamsRaw {
        index: 8,
        p: hex!("2d02c37e4161b080c5b674403a2f12e8bf6ec97a37f6a2fe8104a40000000000"),
        a: hex!("e4d9a2fe014b54c0fec1e7ed7197b6fd2c672e3ec055ff08b153380000000000"),
        b: hex!("72c060d223700fabde4f00c3edf4631e584cc55af8d5b956f015970000000000"),
        gx: hex!("99eb60857ba88491ae9638361ddaf087b2736bff6ec51dd6c0380b0000000000"),
        gy: hex!("6460b7b2efe2df02cd9fac7ca6d8d0c8f105f6852098dd591c5f2a0000000000"),
    },
    CurveParamsRaw {
        index: 9,
        p: hex!("838cba6865699f2152f8bd100a7076a3bec5d5653518800227ba430000000000"),
        a: hex!("098cecada3ef3d7613229ba1b6dd79df18e8fdf05ab5d17f2a263c0000000000"),
        b: hex!("d9aaf6be2c84ad52c4e13ae1486c376e80e2d34ec8656e0e85b71a0000000000"),
        gx: hex!("6e59b74290e2eecd9d323eae546430439f82e0eb371085fdc6f6270000000000"),
        gy: hex!("bd3cf4d8fa9ba13218f6c0eecd47ecdc5e3ad369d96d64d1d0173e0000000000"),
    },
    CurveParamsRaw {
        index: 10,
        p: hex!("4de1ef1da3ed76d28ec26540a943be9eeba8cd86d7a911641c218c0000000000"),
        a: hex!("19013860a95656a9c5ba7c5e5f00b2d5803a2a3d6bc73ee830ee870000000000"),
        b: hex!("552769d4099f65aabb957eae2ddc20766f5a37cd4b2cbbf1f7c4420000000000"),
        gx: hex!("a41ee5d7707a8ec3906fbeea9971d8e0f85bc32d05554edc89aa5f0000000000"),
        gy: hex!("8ec28cfd3946b5fb09915307d3a8f7543b6e9c01a603583fd8f5080000000000"),
    },
    CurveParamsRaw {
        index: 11,
        p: hex!("0f01376f5219b62a5e54a300845b876b56908287292258c6e4e6000000000000"),
        a: hex!("7ea34f86c7fc6c6522b1cd03f1603141307a6f2c2adcc3a8d218000000000000"),
        b: hex!("01382befb7049e92a2408addf9e968529a4927771b7b40b1cb5a000000000000"),
        gx: hex!("ce2971c4789ba3631e22e484d6733272be6ab724c1f28904e994000000000000"),
        gy: hex!("bd6b520b43cab1400baded24ca219cf49ab877c5f917f750cf7f000000000000"),
    },
    CurveParamsRaw {
        index: 12,
        p: hex!("472a996ec4b97d51a453790c89611f3dafd66695bff23dee6fffde0000000000"),
        a: hex!("9eaa6642ec4f81ffcbbafcc7c61b22f8b2efeed6d92a1bd45e634a0000000000"),
        b: hex!("24f94251e784556a778418b432b05aa9eb957f6fa94392c7a25f020000000000"),
        gx: hex!("b7e80f299caa06cc6e48687b39489fa85d09da8c9e884334711a7a0000000000"),
        gy: hex!("58b5749d732451272dfd0d2c50b3ec5224e2b0861df0e770fe7c340000000000"),
    },
    CurveParamsRaw {
        index: 13,
        p: hex!("070b7e6cbc461ca12a59fafa1bccb8937851dd07233a23952b1d3d0000000000"),
        a: hex!("74f50017e6ad17a9b18581a9b9582187b2750f1e6eb7ea9bcefc0a0000000000"),
        b: hex!("b819a8c06c7fbbbd5bf461181263e8b76a5c2ca1bcd0198185c0370000000000"),
        gx: hex!("f48f2914379a695d3d13d2e2a6d587eccf9f6a2406b0a4a7458b200000000000"),
        gy: hex!("933bb4636efb4ebe47bb8dc7cbc77248439474dbe447ec75ffeb370000000000"),
    },
    CurveParamsRaw {
        index: 14,
        p: hex!("d1b0c13d98bf4619ae7e37766425b68e6ac5bc7b18cc94646503400000000000"),
        a: hex!("75fc0bd3c5461d96043c991caf76fd110980b27ea6916fdd231a170000000000"),
        b: hex!("bf19a35a4c3f12efedb98fe7892360c4be713b70b725cae225e1270000000000"),
        gx: hex!("f7b4dc2044c66b32e7f87832054f98b615378506916a7b2c2b5b1f0000000000"),
        gy: hex!("fc0f5933b0986d69c29aa4d7144d0a1bb7163e259718c8556b650e0000000000"),
    },
    CurveParamsRaw {
        index: 15,
        p: hex!("2113675ef10f8d1bd0748638d0a5104847417a2840f92ac05efe350000000000"),
        a: hex!("4b927bbd45ff4fe9fbb3d44ef8ce4e2bfee9f9f6b37bd18858f2160000000000"),
        b: hex!("e10d1bbeaf69cacde79309e450d06cd613d3333bd354bcb51cd92d0000000000"),
        gx: hex!("8ad5eadcc11176ae0b41cdba439346c723296c50732d146516670c0000000000"),
        gy: hex!("e6684e142365155153364c020e25f2c6edfc4adad316b63bd4f21a0000000000"),
    },
    CurveParamsRaw {
        index: 16,
        p: hex!("efc37cc3944c4a2c5d0247cd6d5a4d422c6b51f0a9c4f847270bed0000000000"),
        a: hex!("98f3059e4b07737172f76907d540f6596f5dd44d542138f81a83b90000000000"),
        b: hex!("6405f436dd1fda5ff3e5c87081282e259361474160beecd45ac7460000000000"),
        gx: hex!("401122235155cc71983558d3e7698e300ee69ee3656cbb119e583a0000000000"),
        gy: hex!("a5e2983fc399d34d24845d1f946e9354fe9d257d3b0d4d0dd43b4f0000000000"),
    },
    CurveParamsRaw {
        index: 17,
        p: hex!("f19eaa97e529f58931a09642cc22f4d2c3629009a1383a32936d560000000000"),
        a: hex!("566d51823d5319010636b4fb4fa89929d757d4a06dbbd592a9391e0000000000"),
        b: hex!("86d6ad4d1cec5b877afd7ca401dcf080c9c4ed7bdb67ce895edb4e0000000000"),
        gx: hex!("ee03e0d83856c5e24ee4061db877b365094a58255110f632ffec380000000000"),
        gy: hex!("0237f04ecb93461dec6607b1265720e1fe2f7b263853d4d3986c550000000000"),
    },
    CurveParamsRaw {
        index: 18,
        p: hex!("0dece90a5eaa4c7623f089a5a283d3becfa4d523937097e28378be0000000000"),
        a: hex!("460031c55c0cae46f4fdc24462d1e183d75195cebe87341422072d0000000000"),
        b: hex!("af4cbf5d97c5647ffc956ac42f855381f96a2339675fff11c187300000000000"),
        gx: hex!("8feb0a1933f515191d203c7b769b757bd5a74fc243168e0eb8019f0000000000"),
        gy: hex!("73291298cbb934a700a87b285d1188105bde462dd9fc72dffde4000000000000"),
    },
    CurveParamsRaw {
        index: 19,
        p: hex!("ef6e33bc79e029beb2172cdb0c86751382e0a26492b06bc04023700000000000"),
        a: hex!("5c1e2c23b5efd45c270752a030cfbd8ad5fc594372924f1efbfd480000000000"),
        b: hex!("fe4c08f798f56cbe4352085e87426500f852bec928eadbda034b3d0000000000"),
        gx: hex!("2c536e93b694f242cc120d58e95bd82c96d41b8a5f71e0dfc4154e0000000000"),
        gy: hex!("3daaa3219486ecae1c0ac365b9b31107256b0b5e6afb619a7c5c6a0000000000"),
    },
    CurveParamsRaw {
        index: 20,
        p: hex!("b5fa561ff9cbd753d0c41feb0889e9ec52d3ada25ff851780f3bb50000000000"),
        a: hex!("c265a9c62ed6148a9385c417331e0c0011562d0fb6fc52ad5086a10000000000"),
        b: hex!("f01ca8443dee19a9eac451b817b3352f26fc7b94a6bdb0eb71082c0000000000"),
        gx: hex!("27bb82bd0eec21113112ec3cc0f6c111bb69c092f6d7741e5a9b980000000000"),
        gy: hex!("44de030f7ed215c25e87a95e6f1e0f4b32266938145d174f2841730000000000"),
    },
    CurveParamsRaw {
        index: 21,
        p: hex!("27b64802e5e03457704f3cf829412f343d503f35fd803c9653b5c40000000000"),
        a: hex!("072b7ccddbf3fbbc6da737582a62db16ddb75c8f98202639cc54ba0000000000"),
        b: hex!("1e32aaf3c98ae11ccbb7bad06ea39d438da2e849e926d03d249b2a0000000000"),
        gx: hex!("2d7e4c32d6b88c8927088f1a2d1251e04c23f96c59d9ead270cb390000000000"),
        gy: hex!("3b9184dc848a66a3e9f598e0da138907522e4eb7544e18921de7570000000000"),
    },
    CurveParamsRaw {
        index: 22,
        p: hex!("f153c72a99458ab86e80dcada41c52ab1e3350deb047b1cc2c8ff00000000000"),
        a: hex!("4a477a57405c600c7e1bf02dc728f8c72a50b042a3b75ce7e50b2a0000000000"),
        b: hex!("2b92ebb43d613b6d26d7b99418c3b1ff8574869ad3dacd8774f2590000000000"),
        gx: hex!("aee7cbdff4020ebd8dcaaa226178993923fbd157a639f58c36edb30000000000"),
        gy: hex!("38f07a565378dedae46a80216f9690fd08a3a3f52521900cfeca620000000000"),
    },
    CurveParamsRaw {
        index: 23,
        p: hex!("2982b6a8c9842127961ff16292c493b4448c7ea0a963c2756cbd1b0000000000"),
        a: hex!("ef54e0698a38e244852053395664f0b088b28b21930fbf5d72b40e0000000000"),
        b: hex!("d97be5d36f364f56392ee435bd17400f875b601200ae245f711a0a0000000000"),
        gx: hex!("6b62d27c56111ff132d37277c3bf6b9c49cad85b0acb246d2342070000000000"),
        gy: hex!("53099bf4e4186711e4cfaffef4939bfee24f6a8bed938282060e020000000000"),
    },
    CurveParamsRaw {
        index: 24,
        p: hex!("0b0a09be2efe11837dfd7c290784bcca9862b072b3e93de82dc6ee0000000000"),
        a: hex!("75af72bf31930accf97799a4478f5780ad0ce34a8c41979a95e0550000000000"),
        b: hex!("bd10a2ce01c8939472d8c1686a32f8119f768a69b4a85df302b5a90000000000"),
        gx: hex!("869d8ba8acabfd8e5ce0619e4182ff95ba564146c84922ff61306c0000000000"),
        gy: hex!("631b137877f8314fe14d2a4ffa268501ab743b1ffcd6d367ef77b70000000000"),
    },
    CurveParamsRaw {
        index: 25,
        p: hex!("b720aeda995824fa6432532a11ffea213569b406fac284a5e0d6800000000000"),
        a: hex!("008c41285e037f0d9748c6be87925daacd69003f4e4599db3c374c0000000000"),
        b: hex!("3aeff878c70fd9e7c4940555af2f6adb6905ee7e4fe9ef2e4f6f1a0000000000"),
        gx: hex!("89a33373a20910d21a417b7977fdf94a26b68dadfa43de723f230b0000000000"),
        gy: hex!("92b48ec874bad521065a8b0585e9d7d31fe6bb0939b9ab897b785f0000000000"),
    },
    CurveParamsRaw {
        index: 26,
        p: hex!("2d8f98269ad6bb190d056a8b13535b2b1bc1538cad12bc7bb1f2060000000000"),
        a: hex!("aebbc65d287f158af6295a03dd44e0b95bcd7766cf74696e1ce5030000000000"),
        b: hex!("6c31277768809e35690f829de6d371a76968ccb89160a521abfe000000000000"),
        gx: hex!("9b7abd66ea2cc2a46925dcb6c673876cabe577d8ca4ccb45d969040000000000"),
        gy: hex!("2ce364de8c67f7318fcaf03f262689bfab4b32dd27a46c55f62e010000000000"),
    },
    CurveParamsRaw {
        index: 27,
        p: hex!("21e80f9a35e3223e57b462b3b0c82ac7bfd4152c8c45e9456611860000000000"),
        a: hex!("c1a0a78af8c581823cbdaf9da66cd058d72747ed06df3c5f6c6b270000000000"),
        b: hex!("35c5cb694f36e57f94fe2302c9b7ad8dd425011e80a620617523760000000000"),
        gx: hex!("d96c5f37091eac869989c94968f0021a9ef07c883f55372e23732c0000000000"),
        gy: hex!("870b0659c50d80db0672e9ec001687821126a1e5c5e16d7eaca50f0000000000"),
    },
    CurveParamsRaw {
        index: 28,
        p: hex!("e120e181ced8087ad10dbd034d5e9ef014077397fc4bfce55c7df80000000000"),
        a: hex!("c4a69fd21bb3c446421397aa7ae385988b265bda53007f9c4d881b0000000000"),
        b: hex!("4832d36f05afefd4a05b07d37894fff8b89fb1b9f815ad11d7703f0000000000"),
        gx: hex!("e0bf7135d9e705f4afa350edb1e73f188ae3d6327c3af272d3ec7c0000000000"),
        gy: hex!("fadebdc5b9a4e4511ac19daaf9d7a9563ad8c3e13054331c4d2b600000000000"),
    },
    CurveParamsRaw {
        index: 29,
        p: hex!("63472d8fae3958c70f943e6a42858465907eccbf661ba8df1b52dd0000000000"),
        a: hex!("bcf5b61bab3c6a0b235d19b99b139c63e66444547b08822a9a63500000000000"),
        b: hex!("df819e65adc22a3201c20ed49da6994c9ed7c6756fcdb3f049dda50000000000"),
        gx: hex!("9210d1cdfd8cbfa0696a44406c137bd8dda3d816433270810cd67f0000000000"),
        gy: hex!("eec97fb15e65ef4bb3e6438c4bd9462dd34ecee6639f1aa996cf000000000000"),
    },
    CurveParamsRaw {
        index: 30,
        p: hex!("d9cfe643131c4d981bfe14e04aab031b854e1f99d4f3846d5948ea0000000000"),
        a: hex!("d78f89d386bc7c2f1ca17cd1d367226977d1634a94243b1ddb40030000000000"),
        b: hex!("88243549a0e48009d964ed1560f403390b131e8340bc1ae392ee360000000000"),
        gx: hex!("53dda3f0bf8defa8cfc5fc467812670122c5c98885924a107680210000000000"),
        gy: hex!("04d06279a0195d8204007e1631bbc6f823690b0654d016aee5d33f0000000000"),
    },
    CurveParamsRaw {
        index: 31,
        p: hex!("33272a561959d53899f3a555252b845787ae8a2e2df34277fb729b0000000000"),
        a: hex!("bb855525ca2f5af996455a11ddbfa8d9558c73da87b9f734aa3a840000000000"),
        b: hex!("52d31a2a74ffd0de8e1bab09354f40ee58298e0f7d9e0bafbf56880000000000"),
        gx: hex!("b8b4aef85f08638cc2a6fa496873c8da3b3d09e03d49adf0a1aa1f0000000000"),
        gy: hex!("192f05ca54f3052126aa7f606f0fc8cb521c3c6bc7a0e84b6059690000000000"),
    },
    CurveParamsRaw {
        index: 32,
        p: hex!("ab93bfb319359a16fbf7d0987cee601a30645bde00c022d39ec4d40000000000"),
        a: hex!("29687c350ae7d3f92582ca5864cf8034584755a79de36833ef66620000000000"),
        b: hex!("ad31be57d477e54f12ec12c549ea6a8b3943a4f24681d19078f0300000000000"),
        gx: hex!("d3ae64fd6b256e7ad198e4853eebac8b11c88957033db3f9ef369d0000000000"),
        gy: hex!("247098fe9dc3bfb127e2b307c9d5519356af5eefc94cda4c92ebb10000000000"),
    },
    CurveParamsRaw {
        index: 33,
        p: hex!("6f9f2c14afdb131b0785da0b902c143eec7f544c78e86d0a0a76b60000000000"),
        a: hex!("5292c39afed44286eed9962e936ac7536a1ed1ee9fa026a6851d1d0000000000"),
        b: hex!("a17de81d45f93b3fb83c119736c4a797a20dd96938a5746c988e0e0000000000"),
        gx: hex!("1b2dae74c9038a0692334fcdb9b530368766c1f385ae7f59433b500000000000"),
        gy: hex!("0be44cfff099e4f27a12468cca7e8428d9ee3cc72681ee70a36c180000000000"),
    },
    CurveParamsRaw {
        index: 34,
        p: hex!("d72ab799f91cacfbb3ba0f7e1b16841075fa7a987924381fa28a6a0000000000"),
        a: hex!("f247d1409db05109af949a9ec50c8f269588fad3bd29a9a76751250000000000"),
        b: hex!("9c0d6e292ab7f37564d559a5a3aec507b4b2b07b9ded5b24c6e33b0000000000"),
        gx: hex!("e4c5be920fb199ff21e34578b7c5d1f353587fafd2af7fa0a1a1170000000000"),
        gy: hex!("3c051ccae999f47aeeb47b1321f698ca12a6d7f789cbbec3e5414f0000000000"),
    },
    CurveParamsRaw {
        index: 35,
        p: hex!("2f7b9f410d621211a0fd3013142fe24aa5e102e1e3c5d627a057df0000000000"),
        a: hex!("cb2159983b8d698618deb195378f2d72419aeb8ed7d06ce33d0a560000000000"),
        b: hex!("bc993c75a4d0375ba549965f9f9d0bb94d86948f435e3c3cc5ccb40000000000"),
        gx: hex!("23c5bf89b4c65fc1111685ca3d5b6a73dfd5bcebd884d2c656b1070000000000"),
        gy: hex!("0907ca710ba9bd9a237db4ce3119d4595596d75963f6ebc5b9f2340000000000"),
    },
    CurveParamsRaw {
        index: 36,
        p: hex!("27bc4a80350cddb3443432f46cba2c574027fc1b00ac49b23897710000000000"),
        a: hex!("0c84654a57ccbf5f5e8f8dd948e4de640d9f50bf20d951ca83a6430000000000"),
        b: hex!("0dd3e04ccb30f2803d7cbe545db9f5ebe28f47d474c678faacdd490000000000"),
        gx: hex!("0f53a156ef560e8803b72987637f2a9aaac8f98b6241693645e8130000000000"),
        gy: hex!("c1b7176ba5717c7d49458d9a03628feaceac269e21ba8555d56a150000000000"),
    },
    CurveParamsRaw {
        index: 37,
        p: hex!("1ff7c885a7580bd0e78946ff21316e4cf97e22ad35a2acde3df1fe0000000000"),
        a: hex!("6f9cead5001fe68202fb43a439c31c1beb859119e5cca9b30025080000000000"),
        b: hex!("a46724cba35d33aea6a67a3ee5f7e240a2520f51fc26be665d83cd0000000000"),
        gx: hex!("dc29b650568a8840f18170d6f268978d7dcd2e09ff56feba55039b0000000000"),
        gy: hex!("b368ad9ef24dd83e111c28929f1e1f8c90ff330f967c530079096f0000000000"),
    },
    CurveParamsRaw {
        index: 38,
        p: hex!("c3e883188bbff932acfaa9780f03ab2726b4828b273e332ec3cb1a0000000000"),
        a: hex!("4bfc570f590c73bdebd1925bed876a7e087c23b109f933771b85140000000000"),
        b: hex!("a9cc030ba95cf411960ff2883a85df54931297d593124091e7e00e0000000000"),
        gx: hex!("9d6fa0f8bc3dc5669d752c6a755be828a04e73f04dbd021e74e1160000000000"),
        gy: hex!("d54e97b836d68e081369a25afdec4eefbc29219a80e1df56b2e1010000000000"),
    },
    CurveParamsRaw {
        index: 39,
        p: hex!("2343163539051f05d6e752efd2a991eace054b5ce658e1ef848d270000000000"),
        a: hex!("2502bbb2587c425cadcdb00fcdcd4777adcd81878a486f1fa43d0d0000000000"),
        b: hex!("fba7695369287fb0c53c777446bbdbeab1f3ae2cc42a0ef72157040000000000"),
        gx: hex!("c253bfcbfaca6f9e5885448ce04f15125e03c1210825c77278bb1e0000000000"),
        gy: hex!("0a5b1a73582f4275f0a3189eb4d286b5f5cfbc1eb3786c937f470b0000000000"),
    },
    CurveParamsRaw {
        index: 40,
        p: hex!("119b441e329071a2a68d7db6d802b3e7802664c7051657c6d54ede0000000000"),
        a: hex!("e48a82f8234c94da3a6291ee397a98445b676caca5d6ce10b6f30b0000000000"),
        b: hex!("1f0dff562e50f7bc0bc7e2efc17ec403322a2058ac14ba23be49260000000000"),
        gx: hex!("c872f7b6af5ef3c24159bae1eacbd2ae9dede508485098226dd37e0000000000"),
        gy: hex!("d37db8c026adecf28e7c59ac2a65a3e58ca310bbb35b446f2aefb80000000000"),
    },
    CurveParamsRaw {
        index: 41,
        p: hex!("076c5af5d0969b0b9accc20611d0078bbfd9df2042045fd95accec0000000000"),
        a: hex!("01889b0a02d07d8ecec19bda8ef45ac20bd3003c3bdc3fdadf2a2e0000000000"),
        b: hex!("e76e6ae9a3513d4643b5c1309a11e9e11f3d7a524290736677ee0a0000000000"),
        gx: hex!("2fe98e6701e2e5873591037661d4a519bae35c18ffbf1f2d011e7b0000000000"),
        gy: hex!("e37a86eed6e216cfc6210f5071bf4fb1e544fc2641f7e9148671d90000000000"),
    },
    CurveParamsRaw {
        index: 42,
        p: hex!("5d6512ca5f95cc1f34a24b6066ba292bfb5c78f0b4af3a60ab60cd0000000000"),
        a: hex!("96e7a2637285771f55df8beea7fc5b83ff24a563bcfbe310caba3c0000000000"),
        b: hex!("f0e93c9d189c6d38720970cd79329ed660c6474cfe858f19c785c20000000000"),
        gx: hex!("c1195669ff4262550f58678c83eed89b98decbe3be4b4c14bffa740000000000"),
        gy: hex!("7efe33418748c405f8999a9fc96e3614ae24db26a2acacf2caeb0f0000000000"),
    },
    CurveParamsRaw {
        index: 43,
        p: hex!("5b2db179cfbf5644a39ab29e81070e54d30ebf3addb5b9b56c92fe0000000000"),
        a: hex!("c1770bf61c88d498e538784afa5b183b3524c3326a0aa1d0413e1d0000000000"),
        b: hex!("885db83296f1324eb7cec6ab6a397238a4df47046e65d8511a20cd0000000000"),
        gx: hex!("3a63f925b400f61bf4a992ae7e9bc275d9db328af1b2ce32da4ee10000000000"),
        gy: hex!("ee6114808af84ff33b7deb440e213e9663429a53510a1fc1e4e9320000000000"),
    },
    CurveParamsRaw {
        index: 44,
        p: hex!("2dd515f6072a32fffee07e357fdebe4175d154147df94da51c5eae0000000000"),
        a: hex!("8042a4697e68395732d9efad60411e42ea6cf33c3000e3e25242350000000000"),
        b: hex!("ec00342a6292ae3cdf8e48aca541eef2f1ed81188698a785eaa04e0000000000"),
        gx: hex!("f9fc5b7e2225066edf67823ea08b36557d0f6c70dd18c997cfe31a0000000000"),
        gy: hex!("57fb65465821f7c65a5c831afb375d0953e012ebb37bcc40ed49750000000000"),
    },
    CurveParamsRaw {
        index: 45,
        p: hex!("b94783d57ceefd72ad643f2af387be8998f31d768d7811735edb2f0000000000"),
        a: hex!("d8e8c78cf6647484ec42588872daa8dab1a2d83779fd325cf873040000000000"),
        b: hex!("d8c0ecc6aa594e6e1aafc7f3955f0e2cea1ada7226f0868b51cc1f0000000000"),
        gx: hex!("8dbaf0be79e0197ce2999eab1c395c1416b0a97cbc08007b61d82e0000000000"),
        gy: hex!("d61c7f1be3b44055bec45e18124f409a5d89d5d78545e0be7c1e110000000000"),
    },
    CurveParamsRaw {
        index: 46,
        p: hex!("8f5efbbe3a7d9526785ab559bab7a13c12d966b76fcd8122cf6c7d0000000000"),
        a: hex!("1521e29baaeb3d357743e9ba49ee37b528f4a3d5906e1b309dfe390000000000"),
        b: hex!("7fdffd237ac270ff4370b7cae959614c2f7768d1a78b44e1d005360000000000"),
        gx: hex!("b62afd1443718a9697257e9561c16feaae6f1c88ced9d1215e2e120000000000"),
        gy: hex!("9ba89d574d4501e7e16e4bd9c723a0f28dfa6aa32e92b3acb077030000000000"),
    },
    CurveParamsRaw {
        index: 47,
        p: hex!("0d038f28b0fc594ec42750ef1573721820890f77d7faf7c459b0650000000000"),
        a: hex!("0ad7ba782ee98f80a07a931bab08c4d07a8d94fc03e5e2310d06460000000000"),
        b: hex!("6b98267135ac7bcc33cc3b6a159323ba4195e6a500c5005e22c20e0000000000"),
        gx: hex!("4bbae99b3797aecd90771d170fbafebdbf82eb071af3048200a10b0000000000"),
        gy: hex!("00a1f7b2389ec3b79a221dd0c3fd0ff19efd77093df9682957f92f0000000000"),
    },
    CurveParamsRaw {
        index: 48,
        p: hex!("4f5f247029b44041511e3cb8a909e9fb69752d32bde229284f8c710000000000"),
        a: hex!("07aadc66f1a5c393d415fd466b06718443d34d2ffeea6d9956e5680000000000"),
        b: hex!("928f69ff62d98fbfceba46f03ec93465b6804921c11574f8957f160000000000"),
        gx: hex!("aa885557c1642761c0ea5161069140b59ee9b4f4c48637b01078390000000000"),
        gy: hex!("9753055e474e9d2b832908909f611a3965d5f704093065a42f4f440000000000"),
    },
    CurveParamsRaw {
        index: 49,
        p: hex!("51ca331fd31315f857b39f863ee2782a5ded99a5035ca4ebb277230000000000"),
        a: hex!("1cef1c152da45286f7adb22ad6c09b6e8949a706298c7e226e90190000000000"),
        b: hex!("f54f1b52cbb376a9133b0630997b147c7d5c017c60b303a09cb81f0000000000"),
        gx: hex!("7be9ca69a41086442ea4ae68077c51514fc697b2c4d6aa5d4ac5220000000000"),
        gy: hex!("82da07d9f676df74d6e6a265b9278f1a5108545c935fd94af954190000000000"),
    },
    CurveParamsRaw {
        index: 50,
        p: hex!("bf8f4df10d84c7301cb06753cc45482eafc7ba9ae5b1c997f09d8e0000000000"),
        a: hex!("715bad5878da5b61b086caf90b7ea2ae70038816dbf05464718d4d0000000000"),
        b: hex!("1ee16429262285d17a7c2dbc2f1ad3bb62c2b41eb13f2d847a89110000000000"),
        gx: hex!("25bb73a289ebe5d944a30ec25968ad53d0d3aa0fe1ed45d12c20510000000000"),
        gy: hex!("b0bf2ec7137e9da46d64c335fbdb60229a87053767f1f42b9d000d0000000000"),
    },
    CurveParamsRaw {
        index: 51,
        p: hex!("892b7b2c13eb29eb54b66c96c2e21f8052e8c15333bf66f3e1bfe20000000000"),
        a: hex!("1b5b5e297bc38c6892396c5c92aafd3ceaa5b3abef23397316da980000000000"),
        b: hex!("8f51025f8e9503a95a0b94946757faf38e7fb5fe32a30e1de07a700000000000"),
        gx: hex!("c35a1bd26b1054ff601d8b5e0813473f2d1b115e505a53129a60500000000000"),
        gy: hex!("fc4a9a65ca0a0969ce3ab82b9b2dc751a882991dfd91470bc3fb260000000000"),
    },
    CurveParamsRaw {
        index: 52,
        p: hex!("4db2e5028e1e0a5ddeb45b33e29e667847af7c049f8920f6e2747f0000000000"),
        a: hex!("546a802558f334d092c1dbc190b03a5b18e25176a3af918aa8a2740000000000"),
        b: hex!("a191ecf10d13c7deb1788f238480edcd62454146b76e74224980040000000000"),
        gx: hex!("599af0fd292a37569aa80afff43422a3c6989757c460505d56a7310000000000"),
        gy: hex!("710e7ee33b6421dc4661fe057fab984cad78fb3f186b95ffaa520d0000000000"),
    },
    CurveParamsRaw {
        index: 53,
        p: hex!("8d3aae616fc7fda25419330b14ea58330fe46f0c8e63557ddd73fb0000000000"),
        a: hex!("0a49512ff67df6a9af479465176643154b3e73adfbafa579c88dd90000000000"),
        b: hex!("1ea58d87d439bd00cd3e74363c287ab9be8f59dee0ff20d2be58d90000000000"),
        gx: hex!("039c822e13bdeb356271d30b44543f8d33e2e7b4fc110463dc7a9e0000000000"),
        gy: hex!("998a06c0f19c28479701bfa7cd710b2f701fb25e528a637f05bb720000000000"),
    },
    CurveParamsRaw {
        index: 54,
        p: hex!("d5942529e330b33cd08ef36348efcafd6d1394b3fc432103eb1dfc0000000000"),
        a: hex!("c511ccc806341b7cd556130bb2fc2e21e7f97e25be7f0f95735a740000000000"),
        b: hex!("61e15f92ea45e2140196815e27e784da488f72faedf0c830e412490000000000"),
        gx: hex!("d39f2e0caefe1ef65951aabf3c8ad5b7af7ab7209384a466f4dfa20000000000"),
        gy: hex!("39cda9aca142f3f031ea6a4ae7515c7229ccc004af2d1fbe65ec670000000000"),
    },
    CurveParamsRaw {
        index: 55,
        p: hex!("ad7949c82a3b4edfc7c0ad97e7024ce581d98403b1c274d833d57c0000000000"),
        a: hex!("ceca707cb3ec2e3383852290f94a726945ff62803e8fb1b0de161a0000000000"),
        b: hex!("07171240baa6b590f81572d0613c0c1c2d2f8a35908e6b700d17320000000000"),
        gx: hex!("90fcbcdfd306930e0755733ef586a7135c656db529f976beea5a300000000000"),
        gy: hex!("038bea60d21c242b11d491839ec736d3fc116f351d24078a7e1a510000000000"),
    },
    CurveParamsRaw {
        index: 56,
        p: hex!("832b93a36c7df785ef1081388a52d76979fd6a1bed647e6e1f87990000000000"),
        a: hex!("ca524ec18eb92a050ae01afb2247969cbf1fb984cc707baf2cf0460000000000"),
        b: hex!("296e856f5b8b27e7b23b97ce204d304bafd7e520e1b0dc2e21f1480000000000"),
        gx: hex!("cd2800e32df76bed44603b05b013318ae4eb36b8f59b813f25745e0000000000"),
        gy: hex!("43de767a147895285bd99905638d97982b51eb212e350832c272260000000000"),
    },
    CurveParamsRaw {
        index: 57,
        p: hex!("e9410902cac4d429a9959cac5b474a731c56dd429d3e34ae9ba0910000000000"),
        a: hex!("94cf46f52681d3c43779f95c83c308a8ca9d9809adeb523c21c8420000000000"),
        b: hex!("cbe7e69de5b968ed3bb40375b3623e3e4317c1b536b2c5aa3b1f470000000000"),
        gx: hex!("721ad51ee6878b1a0e6cf1740b358d21f0ce1d11adcd8d71ba8e300000000000"),
        gy: hex!("68e88275382317b87a80cee96f35a2bb686e9e7f1eb65191d0c7190000000000"),
    },
    CurveParamsRaw {
        index: 58,
        p: hex!("bfa8b44403a04d5e371182b4a35d8abd79fc5e6183e8b062e6b65e0000000000"),
        a: hex!("41cb0147b6e0bd4e320061776c380d61db223e947ae68e5ab231120000000000"),
        b: hex!("443425ef939c943f4aa271ddeb8d3487c662cd70fc07e804974d0d0000000000"),
        gx: hex!("50e61922790f8a994a46ae310d7faf3355c2fffd2dbf2a2906a64f0000000000"),
        gy: hex!("080dca7c66c7ae763ac34769dca6d53b8499498f845859614f17440000000000"),
    },
    CurveParamsRaw {
        index: 59,
        p: hex!("db854bed77a65f9ca66e26286c62649186828a8fda49246fea41320000000000"),
        a: hex!("fb13d1cc2fa8c9a8dab2ee072a7b14dfe7507de042fb3db262d1020000000000"),
        b: hex!("0fc28794d05fb7c2a7e8923b101aafdee5415e7ae2bd2e21a01e0c0000000000"),
        gx: hex!("3dd9ada7e41631e4fc312f44043e955713c6079bad21006e512b040000000000"),
        gy: hex!("284e854a7e5c4f412846908955f7cff16bb25902e65a920fdd582c0000000000"),
    },
    CurveParamsRaw {
        index: 60,
        p: hex!("bd662c9ca097ae59a240eb8907d1b28166da48d0f869b0c77ca1930000000000"),
        a: hex!("a5061d2acab6eb78eb8541d47d0eed58bbc4de2db5c875fc9a211a0000000000"),
        b: hex!("2c6d783dfe8f05d28155e0476ae8d4e0c2f42d59f331c99bc0e45c0000000000"),
        gx: hex!("12923a8b1bb3c57d0148c4d93a74f482d12f6cd33fbff93ea3a47e0000000000"),
        gy: hex!("4fa6afb11856702aa7539729bc5833417f512d97b7782ac0c09d190000000000"),
    },
    CurveParamsRaw {
        index: 61,
        p: hex!("0113a7211dc4f2a10168a1749f165f5bd2cd47cc0ea2f2912476e50000000000"),
        a: hex!("410b65b28687b82a5ab589d94394e528249b9db470e4056bb0e5680000000000"),
        b: hex!("5f6848dfadad04dcb9ebff900c0f7b6aa0e5ad4e3d934ab4ffb89b0000000000"),
        gx: hex!("e9824bd0be24f2fdf308af2cb17262201abdfc054dade8c039ad0e0000000000"),
        gy: hex!("b078a0c2683daf55fc0e83dc6e26b4bbc50b60d267147b5d3420360000000000"),
    },
    CurveParamsRaw {
        index: 62,
        p: hex!("5d30513fd95521884058104b876f5e6cf4095f61a97622958210c70000000000"),
        a: hex!("9fe5422b5e5b2cc06e47fea7d0ec4600a4c05ef7dcdd14b4f980130000000000"),
        b: hex!("5c05dd301d3ab3bf499497577c87a4477c67e1b637696f58aaec440000000000"),
        gx: hex!("70c3dc21015f36c1c8da32341d10c313338057478486a94d8a62270000000000"),
        gy: hex!("6e72f8c307d2f2efdd1043f4ba0e18adbfb8058f3e6f41a27fa58c0000000000"),
    },
    CurveParamsRaw {
        index: 63,
        p: hex!("6140fac4cb3704c88573b5f0b4807555780bb2b944fee0339122960000000000"),
        a: hex!("27e88a438a720c1e4f2a57e82dd99e5fb7cf410d06f19a55b08a4b0000000000"),
        b: hex!("31ac429b708be2d92a54b0815237ad1c7e04183686a5194875ab450000000000"),
        gx: hex!("5524ab8fee894ef778cb06db7df5700ded49ee51c8970012581c5d0000000000"),
        gy: hex!("8696f91058b63cd67c3068a8ef8085568aab907d7ba770acc62f140000000000"),
    },
    CurveParamsRaw {
        index: 64,
        p: hex!("87fda9aed28c4ff04516df1de4c5956bfd585a0f87b03dd00e697b0000000000"),
        a: hex!("250a0ec0ab253034f968f899e62b7ae789b11dca31a3b8799a1b350000000000"),
        b: hex!("2a8839b7a74235ee19985354d3cadf295fb08c150935fcd29aad350000000000"),
        gx: hex!("861adb7eccb392da2b7bb1d41cd7cb17544397f9cc79fa99cbd1720000000000"),
        gy: hex!("e9e9922ba5dc8a9b645a55fa676e7e6e447a90790c09cccf84887a0000000000"),
    },
    CurveParamsRaw {
        index: 65,
        p: hex!("03d7637770b2190439b6ede9b481dfa222d905dad81edd0c53d5950000000000"),
        a: hex!("e1f05ae8ca743278550c9feeffa41e66c93aa9bd063e0e16dcdd150000000000"),
        b: hex!("f9f1a19eacf1a774cfa68a7e7de7e0522ae1df0d3dbc371817ed810000000000"),
        gx: hex!("9c30322411372732628d9f7a42e1e5b57b814ed6b53ee3489efa6b0000000000"),
        gy: hex!("8fed7820125dc5f053f31dfcc208077cb1ff16e71f0cab6350606a0000000000"),
    },
    CurveParamsRaw {
        index: 66,
        p: hex!("c1efbde5cf6ecbc10df0fe2f70f4654c5de5cc6c7332838c57c5cc0000000000"),
        a: hex!("943f36cf8549eab5295abca39e5410cb96c00c5c015ea82aeeb67e0000000000"),
        b: hex!("dad18894a8ce75360f284046feb022be6fa4df8962ec89eac385c50000000000"),
        gx: hex!("c726172135a238fe0b67d9ded811f152023e2d8724b76d139cf5040000000000"),
        gy: hex!("454fbe5a06b5f22782595379f3b89696c1b6acdf39fbc9db0946450000000000"),
    },
    CurveParamsRaw {
        index: 67,
        p: hex!("8f3a26cc6517caecc9f94607e686958de80aafc9aab5306792aad60000000000"),
        a: hex!("e52388f46e2cf986397f9866b3e3419e9c878e7a905da4ee7b11440000000000"),
        b: hex!("22575add409a275aa6e7d1780a4af65af4282c6da4d242d9eefacb0000000000"),
        gx: hex!("8750877eeb582f237eed6230a3527fc56124e2445678b884081e8f0000000000"),
        gy: hex!("20f87d9fbe6d77ca9e54d0662c5a56dcddb44f440d24c653e54d5d0000000000"),
    },
    CurveParamsRaw {
        index: 68,
        p: hex!("0de2b73c6c3b475403b0d0a8627cd5c7475563df0876d39c62646e0000000000"),
        a: hex!("75db5777f9169ed23f33f23fd92d5ea3e72347afa6ddbdeeb510470000000000"),
        b: hex!("f60611d07d1a1449bd200556637085c4d475352f33371f27de714d0000000000"),
        gx: hex!("b486970ea6e63d03467f7cae6f63ca1789ab186e23fc2aed585e590000000000"),
        gy: hex!("8aeec51f8814783d71fd7a9f8375f4327b7875f9f3ddb13e5f4d150000000000"),
    },
    CurveParamsRaw {
        index: 69,
        p: hex!("d55aec3a7c45b1963c14d223a8ed3b9fc728c4d7c1eaccb167a67c0000000000"),
        a: hex!("1b6c96e282b946a41e7779d64a6e3653c3f60732fe05ea9d2ad1680000000000"),
        b: hex!("409a9fd53e61ab9962e690c3374d9919bd874b4c83bc6c42082f580000000000"),
        gx: hex!("ca1875ae9695cd1b0189bffd20731f714a395ee5d77eed65de91070000000000"),
        gy: hex!("bc7644b4a1d02cef362bf8d81c5e14b9f8abcce7a244f558f2af370000000000"),
    },
    CurveParamsRaw {
        index: 70,
        p: hex!("c1e52194373d3ce17bc29cafb3f3b8480a0da23506780f56660a470000000000"),
        a: hex!("b878482aa9a0e3f74ce9440fb8a525e2cd27c9df72fb081d1ea6200000000000"),
        b: hex!("ed2fd1f386b6e861de022d65d686bf3af39c8dbbeeaa6026fb9c380000000000"),
        gx: hex!("2dd80cb452d684702a1c845175a8e67331c901269f72e5323a58310000000000"),
        gy: hex!("59c699e156709283e5de0f6f8c09dfd7d12da4ff3349616a820f380000000000"),
    },
    CurveParamsRaw {
        index: 71,
        p: hex!("8b4f10d745e6a9bae5c5bbdac3fc2fb2d0328ecafd9e2cfaa159fb0000000000"),
        a: hex!("c7919d4a1c4a94b76ace7671791fdb3494db967a45901c8fab07950000000000"),
        b: hex!("7a9eae6706a48d4226403b48b9592d66e6e28d0bc3bfc13503e1da0000000000"),
        gx: hex!("72c4b74d0659738e21f1398c9d4954edf8edde26f9515be6bd6adc0000000000"),
        gy: hex!("01312d31977362481be66932eeaf55a830c1104c232b1ef334f6110000000000"),
    },
    CurveParamsRaw {
        index: 72,
        p: hex!("7f78a187c4b4e76a8e7fcb6b76c96a0f9708a9d7aa107eb649e6e90000000000"),
        a: hex!("6a30ba57916b71bfff0d9b58f541167c830a71fc082d9abb0f5ad30000000000"),
        b: hex!("8dcaf15a2387aa2820dd85d883d4a2a0fa8e682632130e791a33c40000000000"),
        gx: hex!("6b776119f90f26894dd83852b596fd0dd88fc3a757628070e6fdad0000000000"),
        gy: hex!("2c524f8b6ce8254272ef60f5fbabbdcd7a19e5f8be69253b28c46b0000000000"),
    },
    CurveParamsRaw {
        index: 73,
        p: hex!("9d8567dfc0462291c0fd03794619dafb228fd42f652338e0846a240000000000"),
        a: hex!("56a91897d01e8dff6b1d0ebabed058c3a24fa823a2343f072d0d0b0000000000"),
        b: hex!("e7f574213b545b3a217e5bbe3ba0100e7fa7da87185c5c887380000000000000"),
        gx: hex!("54697b2e338949f01e6abf5d7d3f4c55b0148373e44fc6648b70000000000000"),
        gy: hex!("79823f9f5d3e984da20011447110ebcfa56772e98af7d2a87aee1f0000000000"),
    },
    CurveParamsRaw {
        index: 74,
        p: hex!("6df2af320f8d4b60f69fe60307b94613cbf1fd1f8b19a7a667b73d0000000000"),
        a: hex!("2bd2e14adfb23197c8a6af66096132a66c7da3558e7bda5d83cc120000000000"),
        b: hex!("191d6bc614cf38576ed804f5bcb16b1e234a9c1af7130345af650b0000000000"),
        gx: hex!("001f1e13c9280eb6af3e02f4a47b5de1a415e27969ad9600e31d280000000000"),
        gy: hex!("24f91c105b2bb701c03ea0a084cc9ee239bc8f5af6195ac20f0b1b0000000000"),
    },
    CurveParamsRaw {
        index: 75,
        p: hex!("bfcecfbc560a9424d99f72d08cbf32c37399a9c6c7a4956e1914b80000000000"),
        a: hex!("40cc033938c5d04e11122db77516feef252c56534c1165c06676000000000000"),
        b: hex!("3f18c83eee34027d31e9379391bbc9d2ff98fc9feb3ed175702a370000000000"),
        gx: hex!("de491d3b8c34dcc0ed66751aab8e205fd94dc6a381e3da87f3d80b0000000000"),
        gy: hex!("93665b1c24341529f2f5cbccdfe0a5913a4f74f7a348b0c0866c4c0000000000"),
    },
    CurveParamsRaw {
        index: 76,
        p: hex!("75e30c1163e17350ff4f562be9eba8ba8dfcbe4c654415ff49fee70000000000"),
        a: hex!("d3bdff1f00de3aafcd78b5df00ccd0dbcb92b26734e437d2b6351c0000000000"),
        b: hex!("c95ea276f8d627f5eabb554c62f75cfcaddb9bd39e2b5ed2a0c1d90000000000"),
        gx: hex!("2e29346d8f3f9d5139cf1641713b5eb57ea138cff261c0413e36150000000000"),
        gy: hex!("232ac52f3bed8bdabc98744b0fb169b3474f2f46fae6a3bf7c2b160000000000"),
    },
    CurveParamsRaw {
        index: 77,
        p: hex!("a7c5ac22e1e5d665f58362dcdfe9c196ebbcf2ff2c0d71491acd600000000000"),
        a: hex!("03001dc430e6fbbc58e913de415275bf4c666fa3ba31662a99744c0000000000"),
        b: hex!("0962e6e5da95149dcbff019d2f76739760b41c80c025c726c9c85d0000000000"),
        gx: hex!("9b26cab722bc55c55cf9263de207aa362668fc7db4f05e06a6f04f0000000000"),
        gy: hex!("a6222da46ba1d89cc4c3e756cbc2652469993668a9fb562588a22e0000000000"),
    },
    CurveParamsRaw {
        index: 78,
        p: hex!("79a0d6b3239798d5cf75bd481e8550e9f528eea1d4f69882eb63050000000000"),
        a: hex!("1f75014efd34be4fe9b65c0cbc548a3c6613dea3683a49959e52030000000000"),
        b: hex!("5b899e5aef17d485c0c1a3337b49e71c77ac9250ffa8fa9e9ad0020000000000"),
        gx: hex!("b57119720e12bf45a2b03d80c846e1bfcf8c33fea567511a1dee000000000000"),
        gy: hex!("84d28f09d25ef6c3b747307f8cf2e1eda4a44402f5f4252a0ae8030000000000"),
    },
    CurveParamsRaw {
        index: 79,
        p: hex!("f5c8dd3405409f115eadc3789b8c47716d378ab559f6426b2555e00000000000"),
        a: hex!("58878c9adc41a7894a78a82bae41603fd2120a680e96f57e43f9670000000000"),
        b: hex!("3a62f930ee53abd9b1af5a6174ede93071d32e4a99a1142f5ae6280000000000"),
        gx: hex!("7dd2225b36ddb5755303331d7f5ae3cb4ebd77da3f166ed2fc38850000000000"),
        gy: hex!("4aa8402553f01c4caf2c008b467245219bcee58feaa944787f43a70000000000"),
    },
    CurveParamsRaw {
        index: 80,
        p: hex!("7b851f88d8e2a96cbf879104925cacb0f44887c70aaaf934be53970000000000"),
        a: hex!("93e3c96c0d64f6efb89c4eaca0b36469c041ce6fa24d8de90d8f0c0000000000"),
        b: hex!("c5328a3a93a3c6779dd0ede954dbb9c1c3ff14a3c05dcd312f692c0000000000"),
        gx: hex!("366cd153ea925267f440ca3daad4ff6d2b5e38012c2a666d1db52d0000000000"),
        gy: hex!("fcefdd0a8bfd25ce2946cb3d4551344eb541cb272a68f5184210280000000000"),
    },
    CurveParamsRaw {
        index: 81,
        p: hex!("0f66a610fa2c7e68db22754eedd4095bf8fecd6270fd6dc5ae47b70000000000"),
        a: hex!("f20431194339d40b7d0b9b7d3bf01b8e8c11faa812ff9ff4c43aa60000000000"),
        b: hex!("7837635442be5abeb8562c9384f207b3c999f4ce9c6a3da9e23cb50000000000"),
        gx: hex!("82daa9a7a2ff88442205a6e287e1fb62318897e4ed7264e05be9270000000000"),
        gy: hex!("bb9cdbc74f1ff1ea5ee47d41af3a0d17bb5851cba42d877a2f0e040000000000"),
    },
    CurveParamsRaw {
        index: 82,
        p: hex!("bb7585ccff6de46f11b77dd7a24e637188ab6d3225b85131ac821a0000000000"),
        a: hex!("83aa7148f82a1d1caef346f140e374fc7978595010aa39574201170000000000"),
        b: hex!("2894ff0b42756174ef3679e36c9cb55c97a237478a7e0fba07c4160000000000"),
        gx: hex!("8f25a6bd542cfb861479164b45e651170f4cb5ed82755f7a7974010000000000"),
        gy: hex!("e89a2c5357d85aa5bdedd6c5e4464024ada5702f2c43399829e8040000000000"),
    },
    CurveParamsRaw {
        index: 83,
        p: hex!("49bf56c30c179dda8ae71ac99d92d5fee6c88cad3cb486833d83470000000000"),
        a: hex!("c08e4bd793369f19a59e4c8db8260f5cf08dace1ed35b33430ca1c0000000000"),
        b: hex!("bcea5bc08bdd8c420bacc861f65689d62a2c5fb7fa3e1cda169a0b0000000000"),
        gx: hex!("0bf4858a12147b20e0af66ac36be7be8aa20093b8c24c7e507172a0000000000"),
        gy: hex!("3ab65edac983a80eba22be4f5e9c6374a3f355252b619eece957250000000000"),
    },
    CurveParamsRaw {
        index: 84,
        p: hex!("776ea7e115d1b7c040ec39d84d4bd84413ca5d1cf69ae73baeaa780000000000"),
        a: hex!("fdb19bfb591c8b1779b69f785a8492bda4a6db3ea770ef65b5393b0000000000"),
        b: hex!("76255513b436736913d04e50259abe611f1cbb0836cce3f3c82e670000000000"),
        gx: hex!("048d954d1015ade6be2009febcebbbb9e5376ade05d9f4c0b9a1550000000000"),
        gy: hex!("c35813304ddd8c2c56617d1f24c7a07ac029114451a50847031e5a0000000000"),
    },
    CurveParamsRaw {
        index: 85,
        p: hex!("3956d8a1df7edbebffa35cdf3e975f247e035aa67225dee4083e340000000000"),
        a: hex!("a2a8d323cc4bf94bfc268a5afd737c95c46ffd86d56190be4834190000000000"),
        b: hex!("5e5f07cc0070f565f19bfde7228103755bda2b68d5c7091276d22a0000000000"),
        gx: hex!("e6fe7f045485752c0f5ffa8c2bb7311cbf90c4da21bd67b37e7d1c0000000000"),
        gy: hex!("4afb36ac3b60a16ff6f6cd9d311e680380133f01adee8eeeeff61c0000000000"),
    },
    CurveParamsRaw {
        index: 86,
        p: hex!("3df3d698bb908f7e2656c2bc861f7e6373f9f269b3c4f78ea277be0000000000"),
        a: hex!("639d9e4002f7feffe6166e28ad3419d3b22fbbb0dbd5c76b4d420d0000000000"),
        b: hex!("e3b4987f4ebd0c8b2bf1f31e9abecb803062d90486303e754186820000000000"),
        gx: hex!("978bbea5306f3d3534276d48b2ea2a2ac1d8ce20bced5fdac4a5930000000000"),
        gy: hex!("56d23377fa8aaadfbf31c42ed15588b9e4d24e833a4fed16ea52230000000000"),
    },
    CurveParamsRaw {
        index: 87,
        p: hex!("d94fb941d49142cf1aecba12759e403fc167aaac0f4992be7833400000000000"),
        a: hex!("bc92e3ed390e1ef5f371bdcaba724446907fc476c5f19a14d92b2e0000000000"),
        b: hex!("71330076bcaa257d39e75137a8a5f0aeb2e8d5a84c919d479e25090000000000"),
        gx: hex!("aee4b6cdc93ca0c88f1d42d529d39c007183a21d72c18e7446d92f0000000000"),
        gy: hex!("7e2f94cb40527f5982278eac890061a1edb2467c2bc6afee406e1f0000000000"),
    },
    CurveParamsRaw {
        index: 88,
        p: hex!("bf436dd882b2fe0dc2620479164f80b9dd9b7033419ea23c4d924b0000000000"),
        a: hex!("f969d4941306cea435441120bf2529d4c53bceedc185b7e075980d0000000000"),
        b: hex!("805633610ceff86c02b2ecfceb56171bc22229c1ebbbf5242b6d1a0000000000"),
        gx: hex!("cea5d101e740ffef757c22e4f24665a3ac323d3f820af1bceb56470000000000"),
        gy: hex!("e63b68b2ac9cd9d83016bd071ba5efc5a62785265b7e482005db470000000000"),
    },
    CurveParamsRaw {
        index: 89,
        p: hex!("038683af6e8af31301de90eab891f7a88a6426fc7e8899ceafd0fc0000000000"),
        a: hex!("0341d1a5c9f34513d87d08dd1d3d06cf26714dd7def0d62d59c67c0000000000"),
        b: hex!("4495d6317cf0e376db2be047e61de29426ffaae5149bf5d37b53300000000000"),
        gx: hex!("c68331bf8bdc0e6c284e667bf08127b5b89c48c4498da9a51986c90000000000"),
        gy: hex!("ef6034ab190e5727a59add791497b481945b20157092814b38087e0000000000"),
    },
    CurveParamsRaw {
        index: 90,
        p: hex!("21572a182b2d40ae5ac1f0a338bcbdd42d1ab150527861d7f2d0b30000000000"),
        a: hex!("7b0369dc27478cbf6285e3f8f81d6a1b8d0d03f136937623be93350000000000"),
        b: hex!("f102adc409909742fa97f869098507e4953a457471f8d3bb59c0030000000000"),
        gx: hex!("0bf478524dc8232458d2be5b9124d56f219faf602bae02e239a3730000000000"),
        gy: hex!("eadf84c33161dfcf233440303107c2e5751e84f91aedaa743694660000000000"),
    },
    CurveParamsRaw {
        index: 91,
        p: hex!("a5ec013bc5597326d8eb774252be904ca147c47e023b277db04c860000000000"),
        a: hex!("32b789db6f6bbaa3d59e035753e9b938c788e32537bd96de8e6f680000000000"),
        b: hex!("f628e6a4c0199af380ec8c8caa2147ab28b8c3e2fbda18ea67f8230000000000"),
        gx: hex!("96abd7ce18742b6158197ee35f9ad7dcdc66e889777edf3defad240000000000"),
        gy: hex!("d726bf6c5a7b955007f77b3b2caa1a2f0fd191b18d0512746e69500000000000"),
    },
    CurveParamsRaw {
        index: 92,
        p: hex!("31b8c34b9648654d5e7b13f38df3a7ba7e3b4ff5fe6ccdb4fb68040000000000"),
        a: hex!("209c20243ca5db1c3be431405dc1e70784293b7f46a3bc5a3b46040000000000"),
        b: hex!("ca8e5185446ff27dbe4dfadc338eb9486e23b6dcc36aa1a143f2020000000000"),
        gx: hex!("bf7f2df7bbdb31e5b252d137680d52afa9597a9b84fb53952322010000000000"),
        gy: hex!("9de64283e35d61eb68f16c4daa372e904b0e6b98bec552414258020000000000"),
    },
    CurveParamsRaw {
        index: 93,
        p: hex!("71fe5f5dc8b0742c950bec111679a67a313993bfcd9afc2ab77f220000000000"),
        a: hex!("f74108a221d3cfae9e40b3ccc41e032d3c9f0a5199912c4047260e0000000000"),
        b: hex!("3a88739cddd0b1d8765bfc5cb0ff3b11b8699e84ac05b3371b5b000000000000"),
        gx: hex!("bb5ee7112d36ff0d10120ec5466cde4e1b537b624bab9b9e51d21b0000000000"),
        gy: hex!("8b56102188b3fb296065f291873dd52eb174ea408962651e8531210000000000"),
    },
    CurveParamsRaw {
        index: 94,
        p: hex!("9548e148677b021eb1eea3526a11e0b5496bffe679552b7be09aed0000000000"),
        a: hex!("659c937b5664c3584748e48bac9ffc46c99e2d9598c91d673ed4820000000000"),
        b: hex!("74e0d059ff2506c018ad3fcc7f84800496f8707a7ef78967830bc90000000000"),
        gx: hex!("026dc4b2c01d0572ca00fdce60eb0dc96c836574de97396603a5280000000000"),
        gy: hex!("3e5774713767086e7b317b4fbf493de4ecf3715b77073aa87105990000000000"),
    },
    CurveParamsRaw {
        index: 95,
        p: hex!("1d64615201a779de658b4e08a180377e89d2a9ef1babe400f7b7c00000000000"),
        a: hex!("c8bdfc16454615e2793850134d4b408da0206257e6e7c8b8fd56030000000000"),
        b: hex!("ea4f37f1527c3a5bb1364d7821bfccac271fb47e0edcb57064171c0000000000"),
        gx: hex!("79dd704809a5ba151d4fccdfc62640485a4801206e6dd5bbfe4d080000000000"),
        gy: hex!("dc550b9244b8a438b1e84fc444e98bf0aab4eb74a535a43944b4be0000000000"),
    },
    CurveParamsRaw {
        index: 96,
        p: hex!("03adfd3206b817189f31a2bbc1a5fbd081ebeca8a41caf8d0dca5a0000000000"),
        a: hex!("d68c858edbdbbe07a43542cd740391c3672b64b25853ee331b0e400000000000"),
        b: hex!("f19d18224277aa2d449545e56f03df2eab786a87539a1acabdea220000000000"),
        gx: hex!("012b05a825cf39597fbe7ce8a5a84691700c342355f8b4812fae350000000000"),
        gy: hex!("dadfaaecaaca80208510719cc948849b8cc7b0a6c5823815c5680b0000000000"),
    },
    CurveParamsRaw {
        index: 97,
        p: hex!("7b278d4e9e17ba54c7d061e1598784749a78daf0252683e22aaff50000000000"),
        a: hex!("b4c87b490ce8479458931cef1c60b13d1496f6567b1cddd250db0b0000000000"),
        b: hex!("ff1330791cb75ab63510353fc9df4bd1707281785090321ec543370000000000"),
        gx: hex!("f37de939e81f7aea75a8f6a277d0d7f87f1f443b5d832abc88c86e0000000000"),
        gy: hex!("82bd89868d2de4dcb8f35fb5fda50ab17bceed61c3b9b156ed65ae0000000000"),
    },
    CurveParamsRaw {
        index: 98,
        p: hex!("99b64cfa52794c039cc2efab332b4582ae8a1de73cd0be757a92370000000000"),
        a: hex!("92fefea1a48bc59d28572cb54682223433045735321363699bb30f0000000000"),
        b: hex!("5b53782019803a7b758255a0f847c5e07bd70ed48de68523e3ac2d0000000000"),
        gx: hex!("f3242e80d14ce4a8aee8e9a5ed52f71c23ef68d5bdcde6b13f6a240000000000"),
        gy: hex!("6cf0fbd1f08b3bd83455b585b88e51f52601c14459c7d8ce739c230000000000"),
    },
    CurveParamsRaw {
        index: 99,
        p: hex!("f5a0693ac2056e30ad00d705ab1c3e81991cd66d716ae8f1d5726e0000000000"),
        a: hex!("660c433ea8c983240e214e3b7864114cad6d35aa88590ab242d6160000000000"),
        b: hex!("56e1639959c30b3ea3df0d253aa82eab5735f9cec04c0e21423b2c0000000000"),
        gx: hex!("4987d4973e9e5d5aaa6955bc461bc0e84cc29b96f32d84901877210000000000"),
        gy: hex!("ab5099f8b16fd2be781c1fc910a4a2388457665552ab9e697343480000000000"),
    },
    CurveParamsRaw {
        index: 100,
        p: hex!("399bf618da5c10bb5fc33bee38a5a2506c7706aa7527b40d4268380000000000"),
        a: hex!("ab2176bc995ce060fad5d64b8f48da84be74dd3905ff5eacd6541b0000000000"),
        b: hex!("b6a3b1f94f170793890dc8a380471dc0cf04eadb85cb82c5c6fd370000000000"),
        gx: hex!("95cd3ee6e197a33965a63503096a2d8f25939410937c865d1814340000000000"),
        gy: hex!("11e3f32e3e1a5a0ea1b1eb1b702943365dd14613cb87356e1d0f260000000000"),
    },
    CurveParamsRaw {
        index: 101,
        p: hex!("e5ce5ede1d160fdb00370b1ac87b1db51ec490cc678c6b16b101db0000000000"),
        a: hex!("aa1f3546821374fe2c7e1ca639dfd3db04317dd704d248f2b66fc60000000000"),
        b: hex!("b2bcacad135a7daf9f96e19f80a7d88323ed6a387cb750bd13878d0000000000"),
        gx: hex!("0d44d5a6385d8e5d7270b7880a3b536a6cc6c4e05beec3929aae9c0000000000"),
        gy: hex!("24cf0958748d918bbda48012dd0adf0590882f8dbdd5fcee659a2a0000000000"),
    },
    CurveParamsRaw {
        index: 102,
        p: hex!("13ca2c4253e70cc0f7318d2438bf2549252680f0c10a9747d3aa310000000000"),
        a: hex!("07cc4142dd00955a85ce5f6f8f1c7d8d3516aa9db72bdb0648241e0000000000"),
        b: hex!("689bda5fddcabfb560c13853e9d802400a775706fdb98d26f4722d0000000000"),
        gx: hex!("8909179b4b117878dfc7198fa0f6574d6b82fdd1b713e4aac4a61d0000000000"),
        gy: hex!("ce935d8b56cdbbb01ca87939cc622efe81c98c79f5e11e973719130000000000"),
    },
    CurveParamsRaw {
        index: 103,
        p: hex!("2380f830a15549fc4b4f2da35bec714bfb00a558bcf4a4c0c96e1d0000000000"),
        a: hex!("7e2546394c741f851dd9f7f80b792b07fd831089e5e3ff68f567070000000000"),
        b: hex!("7bb87cf6e7b1575568b1a63003320e2437b8b3b0678c6ceffc88140000000000"),
        gx: hex!("6e1501f35d15f39cfbff567d181a0fe22e96634b8b97b1efcf98070000000000"),
        gy: hex!("46f0d0c22ea5fdfdaedb5221aaa99f81c0b692fed2079bdf45a0010000000000"),
    },
    CurveParamsRaw {
        index: 104,
        p: hex!("b9c67c4de7b9293d5e86af254b1d91950ae59dbf59e4e7b0f22ed40000000000"),
        a: hex!("cc20cb3965c1e8af7c4df8326431e46a07e5c6478445a08ce8a4520000000000"),
        b: hex!("c2450b50ffc13d4fbc3463ae52f00dd302975bfca9f2be090568890000000000"),
        gx: hex!("d6691b3f41e43ae5d8c39f1bf708b8e4ca8bb8e8619aee291943590000000000"),
        gy: hex!("fe4ffc5256f664159650b12daa9c258edd50d5fcd521041d76796c0000000000"),
    },
    CurveParamsRaw {
        index: 105,
        p: hex!("ed6a505bda12769a5e2a16c5527710e228dc9987421615c18161d50000000000"),
        a: hex!("c991bdb875d6ee3d118dd94823866740e7390ae3e0bb8888c7918b0000000000"),
        b: hex!("d0975827ab99620170e0783b1f296d2ee87f165242a35bf848a8590000000000"),
        gx: hex!("18b25c42be660f6d03a068e2bb80736790f84091fe820b4f8b25190000000000"),
        gy: hex!("3f8658d1943860227b8269316f61938dec3e2b23127c14394d63c90000000000"),
    },
    CurveParamsRaw {
        index: 106,
        p: hex!("6b4359ae49f6913d3fb0874ade7088827dc723fb9b25fb5282f7050000000000"),
        a: hex!("910985aebd4edbce46ca724b35996dbe173d9500fa0cd13492c5000000000000"),
        b: hex!("d3aa899ddedb4e3fdb679bd10328b1aa1b74b51cae242dc22c69030000000000"),
        gx: hex!("c543a7d3a960b0b369fd9514357ffba9a44284f7b292a01c7a13020000000000"),
        gy: hex!("bad5d80c99a0ef20fb273767cc51493aad51b342f79ed3e148e7030000000000"),
    },
    CurveParamsRaw {
        index: 107,
        p: hex!("6d78f350cc72da81caf2689d66d04033eb127cfd498b25d59e022c0000000000"),
        a: hex!("a037f76933cf3094af427670dcde6d77096db688134d7f334f09270000000000"),
        b: hex!("58f61e2377aa7b1d735df915d31358ee6b3c2cc746c7e36b8ee8280000000000"),
        gx: hex!("bd2c089291fbbdf74473a52b65cbf58ea0ead778799805a97e62190000000000"),
        gy: hex!("f903ad5df5dc7a468635d42d06fc64cc7c421c8160bb7a77d18e0c0000000000"),
    },
    CurveParamsRaw {
        index: 108,
        p: hex!("9f56d6f9f9d81aee9c97cbeee580ccd28c67bb25cef8d850dc48de0000000000"),
        a: hex!("fdddcab351369162febf4b17f63af4f1b74061a8fb0d737b6f47070000000000"),
        b: hex!("ad92d565008d8a6e21b136a4987a90610d3ed04199d216cf2f3b090000000000"),
        gx: hex!("175442b98507b77cdc22ad792cd25344cc999e8a7cf4dc33f718680000000000"),
        gy: hex!("144b4cd7ce1483fd6db918c5daf28250e11a0dff9147a33d2529360000000000"),
    },
    CurveParamsRaw {
        index: 109,
        p: hex!("4921ecbba523092ec8d0d436af6e9dcf96e7942df7426e36cc849b0000000000"),
        a: hex!("0f34d0595d5c5541f6b6a856b1b127390a138efc3070f045bf56120000000000"),
        b: hex!("f38b20015ada182c6c1fb8202a77537be7676e806d67fc972499590000000000"),
        gx: hex!("6601b5d383c739e1c6831d4b12c5733fda53f702a47f9a351bf10b0000000000"),
        gy: hex!("de4dccabb590404faf2c7862cc3e788e825c4bb97ef825c430ae660000000000"),
    },
    CurveParamsRaw {
        index: 110,
        p: hex!("89f98b8bb785c48e992d1afd932898f174d9c4ea989b1b8edca8520000000000"),
        a: hex!("6efbb392995def1e29f72ba201bb789a8ad0ad8b569236b8670b1e0000000000"),
        b: hex!("a736bacd12aa53cc507d973befb307d2349536f1913d90e86a803b0000000000"),
        gx: hex!("9c69c2ccc1ad9d0f008f73872d403cb83062fa0a1f0e48b4de3e060000000000"),
        gy: hex!("5a0d8f1d9132cc39f1872f7ac993eb017530e10195a9936ded7b4b0000000000"),
    },
    CurveParamsRaw {
        index: 111,
        p: hex!("6b5d6b9d464562594ec42b17ae8db666046d517c7ed7a3059945100000000000"),
        a: hex!("8ff1ddfc6f313729e314fc50e8419a94b884516a3d70bdbbcab90e0000000000"),
        b: hex!("1910e1e41dc36914ab5c6f080eda0f56a778313d57d3e894ea23010000000000"),
        gx: hex!("918900a5c93f5a6ab31483108370ebf018f9c8759c4d23c7ee87010000000000"),
        gy: hex!("f6f2db267a2524c269ee79732aab03a868657822c39c4c9a3cad0b0000000000"),
    },
    CurveParamsRaw {
        index: 112,
        p: hex!("71fe5964bce5a31e788819e9e742a1bb524c5b0bf45abecff56ac90000000000"),
        a: hex!("af1703d4db22692ef485339365177db278602d25dbba9036777b8a0000000000"),
        b: hex!("d5e320f22c1993dd2c834f0ea2427cb1ecb0567653c81a8bb44c4b0000000000"),
        gx: hex!("254a5709dd50eb0da5c6270c9ef922cda7f96b0c3c719fe0f73bac0000000000"),
        gy: hex!("3e70c8c23b939445877e10200ddb4ac259cb9df84c829d2bea613d0000000000"),
    },
    CurveParamsRaw {
        index: 113,
        p: hex!("ddac135c422f5b7acb562649ac6c90ceac14ae54099be8e0a935d40000000000"),
        a: hex!("1f9da7720271885cb5141ed98ab12aa85e96c8ffe771e58cc619740000000000"),
        b: hex!("ca40da30f1628880d0f05680b239398870f466cc4a3ef07926ef070000000000"),
        gx: hex!("7c1bb3cc61ab6091e71b6b6e0cd55ee31686d5ddd2fbf0a09864440000000000"),
        gy: hex!("9ebd3b7d8036c52239e34beb96f6fecd9ca79366bd7aeb58095ab90000000000"),
    },
    CurveParamsRaw {
        index: 114,
        p: hex!("315b81dc20d0657c292d1aa8ed1a206489bc0935ef43ff3ec5073b0000000000"),
        a: hex!("08f7b891cf36dcf13ccec66d280304fe7afa945808507b04ff1e280000000000"),
        b: hex!("7db804f260ff3d5264df7fe3e536decaf7c3c8c81c3812a8c02b0c0000000000"),
        gx: hex!("eb39824cc2118612b88d9aa1356ed0fa4366d631b5dd6be808a4000000000000"),
        gy: hex!("2e56d760584261f2c4f1a8d458f48461ae1e982bc1901799d8b5260000000000"),
    },
    CurveParamsRaw {
        index: 115,
        p: hex!("d5dadda7a8df599e0897b809ca6b048ed70a7658f4ffc9c37a586b0000000000"),
        a: hex!("43c952801ad37fc260ea40bb515ce79bcbc0fb400d2818c11258030000000000"),
        b: hex!("0815bc3e222054fc1d2ecd9b108df0cad88fe6c1a07c55d9ff1e190000000000"),
        gx: hex!("3a89248eb2309f12939d0c9f9935325f4673e3265de9242d543b640000000000"),
        gy: hex!("2a17a7a6a0b6e4a3d6a2644affb7c8a9ab5ba5a69e87087dd7d1550000000000"),
    },
    CurveParamsRaw {
        index: 116,
        p: hex!("914e25107d41a5a2eb23a0235b77064138118779d86a822fab87670000000000"),
        a: hex!("efaec14030ffe0b809d717263c8a1344641451091f86123da58b340000000000"),
        b: hex!("6fdbc1702a2224cd9477a68a4762e34e39a204ab6b62e777944f1d0000000000"),
        gx: hex!("8c7aed706e9f35230a73fb681f0b008281dd92919cc9e0ab13eb2f0000000000"),
        gy: hex!("45a0dbf62983dcdcb9dab62d84210703266cf8322241790cec55520000000000"),
    },
    CurveParamsRaw {
        index: 117,
        p: hex!("650644cf259caf706be3e69042431c82e69ef0da02f3c87f2d65ce0000000000"),
        a: hex!("be54412b87a8200b43494d2b3c1fa3cec71e8bb084c67abd2fa1020000000000"),
        b: hex!("c3a64d1f059c1b4f2e04183b4a7f1357ac96ded952e9b2d5be6f470000000000"),
        gx: hex!("d1b744b96fb646cd16f5e52c71ee155720f428724186cc293067b50000000000"),
        gy: hex!("4e2d43d1d48c88ceb44879a0e152b781190148eaf206b38c75b9480000000000"),
    },
    CurveParamsRaw {
        index: 118,
        p: hex!("6b98636ce443e5621110e3e15bccb4412f8130911b2852378ae6370000000000"),
        a: hex!("8e500e359d972503dcadacb9a1305f43a02bcbc7a68cdc7e1eaa290000000000"),
        b: hex!("e0bc6939ebf5860d85ba39a294962fd85211d26409e8b365ec9f1b0000000000"),
        gx: hex!("0ca9499c47ba4ebabf6fa3162c3ea026be21155a1c6112dd96992c0000000000"),
        gy: hex!("c8e6c04f8e5fe2d2591cbe80a01b097c7a3f7350c835da4cb4ec1d0000000000"),
    },
    CurveParamsRaw {
        index: 119,
        p: hex!("fd50b3bd45d8c20dfdda5a30d546f01d7a3b676aef361e31ed803d0000000000"),
        a: hex!("495660bb7dec11292741ffcf13941312d7649f4a853578f41dbf080000000000"),
        b: hex!("03b7ae9c1262b6089c855d8bf5e7c9367aa7243857b5b5df311c3c0000000000"),
        gx: hex!("64c450a7d0b2de0597d84eaeb9bc2343f479a2b1e7fbbb1aa28f0c0000000000"),
        gy: hex!("6c302fc43219e04b9c0900a339a8950ebea802b59eb4e2f2b0d8380000000000"),
    },
    CurveParamsRaw {
        index: 120,
        p: hex!("1bf61ecdc2f0172632be75d40c482d40561ac0d00d94e9ff85219c0000000000"),
        a: hex!("73b64e5be47b52d5fc1c95af940b53e1bba8d9538c506fe12f844b0000000000"),
        b: hex!("c87e65c84ac4d6db62092c7061f8aa0e4bc30bed33717af3a8d9640000000000"),
        gx: hex!("6cd6c4cc050ed9799c275dfee3d351bd632cfef438bc9e5988ac3f0000000000"),
        gy: hex!("9b827a2ff02c78ec45353d60543b012da17dd655af96d8192b16680000000000"),
    },
    CurveParamsRaw {
        index: 121,
        p: hex!("3336aa4f54b8dafff1a4658e47fa6a277c044bdccd7379949b5c460000000000"),
        a: hex!("312698f2d87077f5902417241c50365284676f6a024dce717fc1310000000000"),
        b: hex!("d435a12ddf77571c189883ea6169c612e42fde38b83a61a519740a0000000000"),
        gx: hex!("4114cc6780b8aaba264818a299b1238f6bbb02774610e27338313d0000000000"),
        gy: hex!("d51ee40de6c77b64de18e56357ab99d0466e0d8679f701675cb60b0000000000"),
    },
    CurveParamsRaw {
        index: 122,
        p: hex!("539c08d81d13e9b9e2ba8cf28efbe5810884b161bd456c1b39dd570000000000"),
        a: hex!("2895672a15be1807c2bd11a89e479c1893f74c121d3cf79d5c00350000000000"),
        b: hex!("0b5c850f0c804ea224f5c37b2a60cb2c9823e9b3d3fd18508fb70b0000000000"),
        gx: hex!("903167c118b57027b7b7ff61ce9d9683094b7cb5049a86b9c8ba260000000000"),
        gy: hex!("6af98525876310cfa18e570429fdd217bffb178ba79affa436a0340000000000"),
    },
    CurveParamsRaw {
        index: 123,
        p: hex!("290b98b09ceb82c46865ef59bd7cea1f578d028749efb781bf6cc30000000000"),
        a: hex!("58cd8dcbff28019b7c852d68b37092f151bf55834c0c6c94144c670000000000"),
        b: hex!("e53e73d8d2e30360b4236c68a4dcd4c70fe8aee47f9d76dc8dd6b30000000000"),
        gx: hex!("daede49ed65052a0d7d66b1c4e3b8ad521f213b47f09e78cd5231f0000000000"),
        gy: hex!("6f75414a3857dd44dc5798944e3ac6e226b0af2a2d1f4949ed58c00000000000"),
    },
    CurveParamsRaw {
        index: 124,
        p: hex!("c9edcc6b01e2d8b23aff2777d1dd1d0bd6503bba204d5d1c805f670000000000"),
        a: hex!("463fc5771148e853fb6cb59df85de86c075c4a2e21ff152eb61e190000000000"),
        b: hex!("cf28ac913917f621720d79af9bd1b6d6d63bdf7fd39730fe59205d0000000000"),
        gx: hex!("20872123af8be0dce1e2a5f620dd10325e840f8ea45c92da09d0290000000000"),
        gy: hex!("59bf32fc97b47af197b251a180f39021ebbf0ba2cfc5ff160e1b120000000000"),
    },
    CurveParamsRaw {
        index: 125,
        p: hex!("df3c317200ee478bfaca609e171f57f93f8b6353b9b0331ff917f20000000000"),
        a: hex!("6f2dd330a2840a6556da920611d78f72e5858cc39303be09fa9b2a0000000000"),
        b: hex!("90d3c4478dd04d5228bef248dc70b913b7b1d25b24a6e0be13393e0000000000"),
        gx: hex!("0e0d6f116d4b0bcc58f88f89bbe6bbc38f1133f8a03e2537e7866a0000000000"),
        gy: hex!("ebdfc5b2d90f4341270749ad79319125d56dec1e4dc5aa7129fa8f0000000000"),
    },
    CurveParamsRaw {
        index: 126,
        p: hex!("4506daaee34e956a7815f735e6a112f3bd464419a7d08ed34326290000000000"),
        a: hex!("8bcf2bcfc22f8b53619454cfdc77e3bb3fdc5280ab19dceaf08c080000000000"),
        b: hex!("0140085c10ec83876803bfdcfdd06bb258386200d659d6ba6734000000000000"),
        gx: hex!("7dfed903be615df8a10cb58702ed81bb7e9a7c573479c155c930200000000000"),
        gy: hex!("9cdf0a95e2db7e5ff6ea3e9cae4b73267fff9ca48d30f9ea53991c0000000000"),
    },
    CurveParamsRaw {
        index: 127,
        p: hex!("9fe2f2d669d4e42b863a72485e1ccac59fe6fd6b67bc3367e0fc180000000000"),
        a: hex!("eaa8ca648f4f5f8dfd7986c79f6f8109dd521c6b690d0f45b0de010000000000"),
        b: hex!("2242563397831fc850f12df582d396ca4776d26f1256e8aa5cb1160000000000"),
        gx: hex!("19e7a6d01750bf46873f0d5d85e3cdd9189417231553c82020db0d0000000000"),
        gy: hex!("d9e5d622978768cd62e43cc6533ad8cef04f7a758be02a224517040000000000"),
    },
];
