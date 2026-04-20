pub mod corrupt;
pub mod crypto;
pub mod game;
pub mod remap;
pub mod types;
pub mod error;

use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    save::{crypto::{blowfish::BlowfishError, citrus::Citrus}, error::SaveError, game::Game, types::Class},
};
use bitflags::bitflags;
use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use flate2::{
    Compression,
    write::{DeflateDecoder, DeflateEncoder},
};

use crypto::Mandarin;
use util::{WriteAlign, align_up, murmur3, seek_align_up};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SaveFlags: u32 {
        const BLOWFISH = 0x1;
        const HAS_ID = 0x2;
        const CITRUS = 0x4;
        const DEFLATE = 0x8;
        const MANDARIN = 0x10;
    }
}

impl SaveFlags {
    pub fn game_default(game: Game) -> Self {
        match game {
            Game::MHWILDS => SaveFlags::DEFLATE | SaveFlags::MANDARIN,
            Game::RE9 => SaveFlags::MANDARIN,
            Game::MHST3 => SaveFlags::MANDARIN,
            Game::DD2 => SaveFlags::MANDARIN,
            Game::PRAGMATA => SaveFlags::MANDARIN,
            Game::MHRISE | Game::SF6 => SaveFlags::CITRUS,
            Game::RE2 => SaveFlags::BLOWFISH | SaveFlags::HAS_ID,
            //_ => SaveFlags::empty(),
        }
    }

    pub fn get_header_length(&self) -> usize {
        let mut res = if *self == SaveFlags::DEFLATE {
            12
        } else { 16 };
        if self.contains(SaveFlags::HAS_ID) {
            res += 8;
        }
        if self.contains(SaveFlags::BLOWFISH) {
            res += 8;
        }
        res
    }
}

#[derive(Debug, Clone)]
pub struct SaveOptions {
    pub game: Game,
    pub id: Option<u64>,
    pub curve_index: Option<usize>,
    pub brute_force: Option<(usize, usize)>
}

impl SaveOptions {
    pub const STEAM_ID_BASE: usize = 0x0110000100000000;
    pub fn new(game: Game) -> Self {
        Self {
            game,
            id: None,
            curve_index: None,
            brute_force: None
        }
    }

    pub fn id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn curve_index(mut self, index: usize) -> Self {
        self.curve_index = Some(index);
        self
    }

    pub fn brute_force(mut self, base: usize, count: usize) -> Self {
        self.brute_force = Some((base, count));
        self
    }

    pub fn brute_force_steam(mut self) -> Self {
        self.brute_force = Some((Self::STEAM_ID_BASE, u32::MAX as usize));
        self
    }

    pub fn brute_force_ps5(mut self) -> Self {
        self.brute_force = Some((0, u32::MAX as usize));
        self
    }
}

#[derive(Debug, Clone)]
pub struct SaveFile {
    pub game: Game,
    pub flags: SaveFlags,
    pub blowfish_options: u32,
    pub fields: Vec<(u32, Class)>,
}

impl SaveFile {
    pub fn save<P: AsRef<Path>>(&self, path: P, options: &SaveOptions) -> error::Result<()> {
        let bytes = self.write_save(options)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P, options: &mut SaveOptions) -> error::Result<Self> {
        let bytes = std::fs::read(path)?;
        let file = Self::read_save(bytes, options)?;
        Ok(file)
    }

    // returns the decrypted/decompressed save data and the data offset where reading would starts
    pub fn process_bytes_to_stream(data: Vec<u8>, options: &mut SaveOptions) -> error::Result<(Vec<u8>, u64, u32, SaveFlags)> {
        let len = data.len();

        let mut end_hash = [0u8; 4];
        end_hash.copy_from_slice(&data[len-4..len]);
        let end_hash = u32::from_le_bytes(end_hash);
        let file_hash = murmur3(&data[..(len - 4)], 0xffffffff);
        if end_hash != file_hash {
            log::warn!(
                "[File Hash Check] Invalid File Hashes: target={:x}, calculated={:x}",
                end_hash,
                file_hash
            );
        } else {
            log::info!(
                "[File Hash Check] File Hashes equal: target={:x}, calculated={:x}",
                end_hash,
                file_hash
            );
        }

        let mut cursor = Cursor::new(data);
        let magic = cursor.read_u32::<LE>()?.to_le_bytes();
        if &magic != b"DSSS" {
            return Err(SaveError::InvalidMagic)
        }
        let version = cursor.read_u32::<LE>()?;
        if version != 2 {
            return Err(SaveError::UnsupportedVersion(version))
        }

        let flags = cursor.read_u32::<LE>()?;
        let mut flags = SaveFlags::from_bits_truncate(flags);
        log::info!("Flags: {flags:?}");
        let mut blowfish_option = 0;

        // Parse the header according to the flags and version
        if flags.contains(SaveFlags::BLOWFISH) {
            blowfish_option = cursor.read_u32::<LE>()?;
            log::info!("blowfish_option={blowfish_option}");
            if blowfish_option > 0 {
                let mut dsss_dsss = [0u8; 8];
                cursor.read_exact(&mut dsss_dsss)?;

                if &dsss_dsss == b"DSSSDSSS" {
                    blowfish_option &= 0xfffffffc; // this forces the lower two bits to 0, meaning
                                                   // the other id isnt encrypted
                    log::warn!("DSSSDSSS not encrypted while blowfish flag is set");
                    flags.remove(SaveFlags::BLOWFISH); // already decrypted
                }
                // i think there is some other logic if its not 3,
                // but it usually should be three
                else if blowfish_option == 3 { 
                    crypto::blowfish::decrypt_in_place(&mut dsss_dsss)?;
                    if &dsss_dsss != b"DSSSDSSS" {
                        log::error!("expected DSSSDSSS in header");
                        return Err(BlowfishError::HeaderError.into());
                    }
                }
            }
        }

        if flags.contains(SaveFlags::HAS_ID) {
            seek_align_up(&mut cursor, 8)?;
            let mut id = cursor.read_u64::<LE>()?.to_le_bytes();
            if blowfish_option > 0 && flags.contains(SaveFlags::BLOWFISH) {
                crypto::blowfish::decrypt_in_place(&mut id)?;
            }
            let id = u64::from_le_bytes(id);
            // the game does check that the steam id is good here, but we can kinda just soft check
            // since we might not care, I could maybe add a strict flag
            if let Some(option_id) = options.id.as_mut() {
                if *option_id != id && *option_id & 0xffffffff != id {
                    log::warn!("Invalid id: {:x} != {:x}", *option_id, id);
                    *option_id = id;
                }
            } else {
                log::info!("Found id {id:016x}={id}");
                options.id = Some(id);
            }
        }

        if flags.intersects(SaveFlags::MANDARIN | SaveFlags::CITRUS) {
            seek_align_up(&mut cursor, 16)?;
        }

        // Read the actual data in the file according to flags
        // I think games accept versions from some minimum up to 2, (0, 1, 2 maybe?), but I
        // already check for that above, strictly for 2
        // it also actually does strictly check for v2 i think, ive never seen lower

        // the game also checks some passed in options for what flags are accepted, and only
        // decrypts if the files flags match that, this is irrelevant here

        // first check blowfish
        if flags.contains(SaveFlags::BLOWFISH) {
            let offset = cursor.position() as usize;
            let data = cursor.get_mut().as_mut_slice();
            let enc_len = len - offset - 4; // minus the murmur3 hash size
            crypto::blowfish::decrypt_in_place(&mut data[offset..offset+enc_len])?;
            log::info!("Decrypted Blowfish");
        }

        // After blowfish, citrus and mandarin are checked
        // I haven't found a game where both occur at the same time so I'll just assume that citrus
        // happens first
        if flags.intersects(SaveFlags::CITRUS | SaveFlags::MANDARIN) {
            let offset = cursor.position() as usize;
            cursor.seek(SeekFrom::End(-12))?;
            let decrypted_len = cursor.read_u64::<LE>()?;
            cursor.seek(SeekFrom::Start(offset as u64))?;
            let enc_len = len - offset - 4; // minus the murmur3 hash size
            let data = &mut cursor.get_mut().as_mut_slice()[offset..offset+enc_len];
            if flags.contains(SaveFlags::MANDARIN) {
                let mandarin = Mandarin::init_from_game(options.game)?;
                let id = if let Some((base, count)) = options.brute_force {
                    let id = mandarin.brute_force(&data, decrypted_len, options.game, base, count);
                    if id == 0 {
                        log::warn!("likely could not brute force the id");
                    } else {
                        log::info!("brute forced id={id}");
                    }
                    options.id = Some(id);
                    id
                } else {
                    options.id
                        .ok_or(SaveError::RequiresID(SaveFlags::MANDARIN))?
                };
                let key = options.game.get_key_from_steamid(id);
                let decrypted = mandarin.decrypt(&data, decrypted_len as u64, key)?;
                data[..decrypted.len()].copy_from_slice(&decrypted);
                cursor.get_mut().truncate(offset + decrypted.len());
                log::info!("Decrypted Mandarin");
            }
            else if flags.contains(SaveFlags::CITRUS) {
                let key = options.id
                        .ok_or(SaveError::RequiresID(SaveFlags::CITRUS))?;
                let citrus = Citrus::new(key, options.curve_index);
                let mut curve_index = options.curve_index;
                if let Some(curve) = citrus.brute_force_find_params(&data, decrypted_len as usize) {
                    curve_index = Some(curve.index as usize);
                }
                let citrus = Citrus::new(key, curve_index);
                let decrypted = citrus.decrypt(&data, decrypted_len as usize).unwrap();
                options.curve_index = curve_index;
                data[..decrypted.len()].copy_from_slice(&decrypted);
                cursor.get_mut().truncate(offset + decrypted.len());
                log::info!("Decrypted Citrus");
            }

            if decrypted_len > 0 {
                cursor.get_mut().truncate(offset + decrypted_len as usize);
            }
        }
        //let _ = std::fs::write("./outputs/decrypted.bin", cursor.get_ref().as_slice());

        // the game parses the data based on the offset position before decompression
        let data_offset = cursor.position();

        let data = if flags.contains(SaveFlags::DEFLATE) {
            seek_align_up(&mut cursor, 8)?;
            let _compressed_size_plus_0x10 = cursor.read_u64::<LE>()?;
            let _unk_one = cursor.read_u32::<LE>()?;
            let compressed_size = cursor.read_u32::<LE>()? as usize;
            let decompressed_size = cursor.read_u64::<LE>()? as usize;
            //println!("{:x}, {:x}, {:x}, {:x}, {:x}", data_offset, _compressed_size_plus_0x10, _unk_one, compressed_size, decompressed_size);
            let offset = cursor.position() as usize;
            let data = &mut cursor.get_mut().as_mut_slice();
            let compressed = &mut data[offset..offset + compressed_size];
            let mut decoder = DeflateDecoder::new(Vec::new());
            decoder.write_all(&compressed)
                .map_err(SaveError::CompressionError)?;
            let decompressed = decoder.finish()
                .map_err(SaveError::CompressionError)?;
            if decompressed_size != decompressed.len() {
                log::warn!("[Decompression] expected size not equal to buffer size: continuing...");
            }
            let mut new_buffer = cursor.get_ref()[..data_offset as usize].to_vec();
            new_buffer.extend_from_slice(&decompressed);
            log::info!("Deflated");
            new_buffer
        } else {
            cursor.into_inner()
        };

        Ok((data, data_offset, blowfish_option, flags))
    }

    // just pass in the data as mutable so that we can do decryption in place, no copies, no reference
    pub fn read_save(data: Vec<u8>, options: &mut SaveOptions) -> error::Result<Self> {
        let (data, data_offset, blowfish_options, flags): (Vec<u8>, u64, u32, SaveFlags) = Self::process_bytes_to_stream(data, options)?;
        let mut cursor = Cursor::new(data.as_slice());
        cursor.set_position(data_offset);

        let end = data.len() as u64 - 7;
        let classes = Self::parse_classes(&mut cursor, end)?;

        Ok(SaveFile {
            fields: classes,
            flags,
            game: options.game,
            blowfish_options
        })
    }

    pub fn write_save(&self, options: &SaveOptions) -> error::Result<Vec<u8>> {
        let file_buf = Vec::new();
        let mut cursor = Cursor::new(file_buf);
        cursor.write_all(b"DSSS")?;
        cursor.write_u32::<LE>(2)?;
        cursor.write_u32::<LE>(self.flags.bits())?;

        let blowfish_opt = self.blowfish_options; 

        if self.flags.contains(SaveFlags::BLOWFISH) {
            cursor.write_u32::<LE>(blowfish_opt)?;
            if blowfish_opt > 0 {
                let mut dsss = *b"DSSSDSSS";
                if blowfish_opt == 3 {
                    crypto::blowfish::encrypt_in_place(&mut dsss)?;
                }
                cursor.write_all(&dsss)?;
            }
        }

        if self.flags.contains(SaveFlags::HAS_ID) {
            cursor.write_align_up(8)?;
            // i think this is supposed to be steamid32, so just use that in case with a mask
            let id = options.id.ok_or(SaveError::RequiresID(SaveFlags::HAS_ID))? & 0xffffffff;
            let mut id_bytes = id.to_le_bytes();
            if blowfish_opt > 0 && self.flags.contains(SaveFlags::BLOWFISH) {
                crypto::blowfish::encrypt_in_place(&mut id_bytes)?;
            }
            cursor.write_all(&id_bytes)?;
        }

        if self.flags.intersects(SaveFlags::MANDARIN | SaveFlags::CITRUS) {
            cursor.write_align_up(16)?;
        }

        let data_offset = cursor.position() as usize;
        self.write_classes(&mut cursor)?;
        let mut file_buf = cursor.into_inner();
        let raw_classes = file_buf.split_off(data_offset);

        let mut payload = if self.flags.contains(SaveFlags::DEFLATE) {
            let decompressed_size = raw_classes.len() as u64;

            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(5));
            encoder.write_all(&raw_classes).map_err(SaveError::CompressionError)?;
            let compressed = encoder.finish().map_err(SaveError::CompressionError)?;
            let compressed_size = compressed.len() as u64;

            let mut packed = Vec::with_capacity(compressed.len() + 24);

            let pad_len = (8 - (data_offset % 8)) % 8;
            packed.extend_from_slice(&vec![0u8; pad_len]); // this should usually just be 4 if no
                                                           // mandarin, otherwise 0
            packed.extend_from_slice(&(compressed_size + 0x10u64).to_le_bytes());
            packed.extend_from_slice(&1u32.to_le_bytes());
            packed.extend_from_slice(&(compressed_size as u32).to_le_bytes()); 
            packed.extend_from_slice(&decompressed_size.to_le_bytes());
            packed.extend_from_slice(&compressed);
            packed
        } else {
            raw_classes
        };

        let decrypted_size = payload.len() as u64;

        if self.flags.contains(SaveFlags::MANDARIN) {
            let id = options.id.ok_or(SaveError::RequiresID(SaveFlags::MANDARIN))?;
            let mandarin = Mandarin::init_from_game(options.game)?;
            let steam_key = options.game.get_key_from_steamid(id);
            payload = mandarin.encrypt(&payload, steam_key)?;
        } else if self.flags.contains(SaveFlags::CITRUS) {
            let id = options.id.ok_or(SaveError::RequiresID(SaveFlags::CITRUS))?;
            let citrus = Citrus::new(id, options.curve_index);
            payload = citrus.encrypt(&payload).expect("Failed to encrypt with citrus");
        }

        if self.flags.intersects(SaveFlags::MANDARIN | SaveFlags::CITRUS) {
            payload.extend_from_slice(&decrypted_size.to_le_bytes());
        }

        if self.flags.contains(SaveFlags::BLOWFISH) {
            crypto::blowfish::encrypt_in_place(&mut payload)?;
        }

        file_buf.extend_from_slice(&payload);

        let aligned_len = align_up(file_buf.len(), 4);
        file_buf.resize(aligned_len, 0);

        let file_hash = murmur3(&file_buf, 0xffffffff);
        file_buf.extend_from_slice(&file_hash.to_le_bytes());

        Ok(file_buf)
    }

    pub fn parse_classes(data: &mut Cursor<&[u8]>, end: u64) -> Result<Vec<(u32, Class)>, SaveError> {
        let mut fields = Vec::new();
        while let Ok(h) = data.read_u32::<LE>() {
            match types::Class::read(data) {
                Ok(field_value) => fields.push((h, field_value)),
                Err(e) => {
                    log::error!("error reading class native_field_hash={h:010x}: {e}");
                }
            }
            if data.position() >= end {
                break
            }
        }
        Ok(fields)
    }

    pub fn write_classes<W: Write + Seek>(&self, writer: &mut W) -> Result<(), SaveError> {
        for (hash, class) in &self.fields {
            writer.write_all(&hash.to_le_bytes())?;
            class.write(writer)?;
        }
        Ok(())
    }
}
