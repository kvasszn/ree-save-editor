pub mod corrupt;
pub mod crypto;
pub mod game;
pub mod types;
pub mod remap;

use std::{fs::File, io::{Cursor, Read, Seek, SeekFrom, Write}, path::Path};

use bitflags::bitflags;
use flate2::{Compression, write::{DeflateDecoder, DeflateEncoder}};
use crate::{file::{Magic, StructRW}, save::{crypto::citrus::Citrus, game::Game, types::Class}};

use util::*;

use crypto::Mandarin;

#[derive(Debug, Clone)]
pub struct SaveFile {
    pub game: Game,
    pub flags: SaveFlags,
    pub fields: Vec<(u32, Class)>
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SaveFlags: u32 {
        const BLOWFISH = 0x1;
        const PHOTO = 0x2;
        // const AUTOSTRONG = 0x16 // idk
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
            Game::MHRISE => SaveFlags::CITRUS,
            //_ => SaveFlags::empty(), 
        }
    }
}

#[derive(Debug, Clone)]
pub struct SaveContext {
    pub key: Option<u64>,
    pub game: Game,
    pub curve_index: Option<usize>
}

impl SaveFile {
    pub fn to_bytes_v2(&self, key: u64) -> crate::file::Result<Vec<u8>> {
        let mut data = Cursor::new(Vec::<u8>::new());
        let version: u32 = 2;
        let flags: u32 = self.flags.bits();
        let null: u32 = 0;

        // write some unk bytes (i forget if this is the type hash or what, might be a murmur?)
        for field in &self.fields {
            data.write(&field.0.to_le_bytes())?;
            field.1.write(&mut data)?;
        }

        // compression
        let decompressed_size: u64 = data.get_ref().len() as u64;

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(5));
        encoder.write_all(&data.get_ref())?;
        let compressed = encoder.finish()?;
        let compressed_size = compressed.len() as u64;
        let mut data = Cursor::new(Vec::<u8>::new());
        data.write(&(compressed_size + 0x10).to_le_bytes())?;
        data.write(&(1u32).to_le_bytes())?;
        data.write(&(compressed_size as u32).to_le_bytes())?;
        data.write(&decompressed_size.to_le_bytes())?;
        data.write(&compressed[..compressed_size as usize])?;

        let decrypted_size = data.get_ref().len() as u64;
        let mandarin = Mandarin::init_from_game(self.game)?;
        let key = self.game.get_key_from_steamid(key);
        let data = mandarin.encrypt(data.get_ref(), key)?;

        let mut fb = Cursor::new(Vec::<u8>::new());
        fb.write_all(b"DSSS")?; // 0
        fb.write_all(&version.to_le_bytes())?; // 4
        fb.write_all(&flags.to_le_bytes())?; //  8
        fb.write_all(&null.to_le_bytes())?; // C
        fb.write_all(&data)?; // 0x10
        fb.write_all(&decrypted_size.to_le_bytes())?;
        let data = &fb.into_inner();
        let file_hash = murmur3(&data, 0xffffffff);
        let mut f = Cursor::new(Vec::new());
        f.write_all(&data)?;
        f.write_all(&file_hash.to_le_bytes())?;
        Ok(f.into_inner())
    }

    pub fn to_bytes(&self, key: u64) -> crate::file::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut wrapper = Cursor::new(&mut buf);

        // this is so dumb but whatever
        let virtual_offset = if self.flags == SaveFlags::DEFLATE { 12 } else { 0 };
        if virtual_offset > 0 {
            wrapper.write_all(&vec![0u8; virtual_offset])?;
        }

        for (hash, class) in &self.fields {
            wrapper.write_all(&hash.to_le_bytes())?;
            class.write(&mut wrapper)?; 
        }

        let mut current_data = buf[virtual_offset..].to_vec();

        if self.flags.contains(SaveFlags::DEFLATE) {
            let decompressed_size = current_data.len() as u64;

            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(5));
            encoder.write_all(&current_data)?;
            let compressed = encoder.finish()?;
            let compressed_size = compressed.len() as u64;

            let mut packed = Vec::with_capacity(compressed.len() + 24);
            packed.extend_from_slice(&(compressed_size + 0x10).to_le_bytes());
            packed.extend_from_slice(&1u32.to_le_bytes());
            // this is actually so fucking stupid when i think about it, above its a u64, here its a
            // u32, same number but 0x10 diff
            packed.extend_from_slice(&(compressed_size as u32).to_le_bytes());
            packed.extend_from_slice(&decompressed_size.to_le_bytes());
            packed.extend_from_slice(&compressed[..compressed_size as usize]);

            current_data = packed;
        }

        if self.flags.contains(SaveFlags::MANDARIN) {
            let decrypted_size = current_data.len() as u64;
            let mandarin = Mandarin::init_from_game(self.game)?;
            let steam_key = self.game.get_key_from_steamid(key);
            current_data = mandarin.encrypt(&current_data, steam_key)?;
            current_data.extend_from_slice(&decrypted_size.to_le_bytes());
        }

        let mut final_out = Vec::with_capacity(current_data.len() + 20);
        final_out.extend_from_slice(b"DSSS");
        final_out.extend_from_slice(&2u32.to_le_bytes());
        final_out.extend_from_slice(&self.flags.bits().to_le_bytes());
        final_out.extend_from_slice(&0u32.to_le_bytes()); 
        final_out.extend_from_slice(&current_data);

        let file_hash = murmur3(&final_out, 0xffffffff);
        final_out.extend_from_slice(&file_hash.to_le_bytes());

        Ok(final_out)
    }

    pub fn save(&self, path: &Path, key: u64) -> crate::file::Result<()> {
        let bytes = self.to_bytes(key)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn read_data<R: Read + Seek>(reader: &mut R, ctx: &mut SaveContext) -> crate::file::Result<Vec<u8>> {
        let magic = Magic::<4>::read(reader, &mut ())?;
        if &magic != b"DSSS" {
            return Err(format!("Magic {:#04X?}, != DSSS", &magic.0).into())
        }
        let version = u32::read(reader, &mut ())?;
        if version != 2 {
            return Err(format!("Save file version incorrect I think: {version}").into())
        }
        let flags = u32::read(reader, &mut ())?;
        println!("Version={version}, Save Flags: {:034b}", flags); // theres flags for encryption type, compression,
        let _save_or_user_i_think = u32::read(reader, &mut ())?;
        let flags = SaveFlags::from_bits_truncate(flags);
        println!("Flags: {:?}", flags);

        if flags.contains(SaveFlags::PHOTO) {
            let _steamid32 = reader.read_u32()?;
            let _null = reader.read_u32()?;
        }

        let data_start = reader.tell()?;
        reader.seek(std::io::SeekFrom::End(-12))?;
        let real_data_len = reader.stream_position()? - data_start;
        let decrypted_len = u64::read(reader, &mut ())?;
        log::info!("decrypted_len={decrypted_len:x}");
        let end_hash = u32::read(reader, &mut ())?;
        log::info!("end_hash={end_hash:x}");
        let len = reader.stream_position()?;
        reader.seek(SeekFrom::Start(0))?;
        let mut file_bytes: Vec<u8> = vec![];
        reader.read_to_end(&mut file_bytes)?;
        let file_hash = murmur3(&file_bytes[..(len as usize - 4)], 0xffffffff);
        if end_hash != file_hash {
            log::info!("[File Hash Check] Invalid File Hashes: target={:x}, calculated={:x}", end_hash, file_hash);
        } else {
            log::info!("[File Hash Check] File Hashes equal: target={:x}, calculated={:x}", end_hash, file_hash);
        }

        // Decryption
        reader.seek(SeekFrom::Start(data_start))?;
        // TODO: figure out the magical + 8
        let mut encrypted = vec![0; real_data_len as usize + 8];
        reader.read_exact(&mut encrypted)?;
        //reader.read_to_end(&mut encrypted)?;
        let data = if flags.contains(SaveFlags::MANDARIN) {
            let mandarin = Mandarin::init_from_game(ctx.game)?;
            let key = ctx.key
                .map(|steamid| ctx.game.get_key_from_steamid(steamid))
                .unwrap_or_else(|| {mandarin.brute_force(&encrypted, decrypted_len)});
            ctx.key = Some(key);
            let decrypted_buf = mandarin.decrypt(&encrypted, decrypted_len as u64, key)?;
            log::info!("[Decrypted]");
            decrypted_buf
        } else if flags.contains(SaveFlags::CITRUS) {
            let key = ctx.key.unwrap_or(0);
            let citrus = Citrus::new(key, ctx.curve_index);
            let decrypted = citrus.decrypt(&encrypted, decrypted_len as usize).unwrap();
            decrypted 
        } else {
            encrypted
        };
        let data = if flags.contains(SaveFlags::DEFLATE) {
            // Decompression
            let mut decrypted_buf = Cursor::new(&data);
            // this is so fucking stupid
            let _compressed_size_from_afterthis = u64::read(&mut decrypted_buf, &mut ())?;
            let _unk = u32::read(&mut decrypted_buf, &mut ())?; // this is just 1?
            let compressed_buffer_size = u32::read(&mut decrypted_buf, &mut ())?;
            let decompressed_size = u64::read(&mut decrypted_buf, &mut ())?;
            let pos = decrypted_buf.position() as usize;
            let compressed = &decrypted_buf.get_ref()[pos..pos+compressed_buffer_size as usize];
            let mut decoder = DeflateDecoder::new(Vec::new());
            decoder.write_all(&compressed)?;
            let decompressed = decoder.finish()?;
            if decompressed_size != decompressed.len() as u64 {
                log::info!("[Decompression] expected size not equal to buffer size: continuing...");
            }
            log::info!("[Decompressed]");
            decompressed
        } else {data};


        //let good_header = [0x99, 0xF1, 0xE3, 0xDB, 0x03, 0x00, 0x00, 0x00, 0xDC, 0xCC, 0x7F, 0x82, 0x27, 0x36, 0x5A, 0x69];
        //data[0..16].copy_from_slice(&good_header);
        Ok(data)
    }
}

impl StructRW<SaveContext> for SaveFile {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut SaveContext) -> crate::file::Result<Self>
    where
        Self: Sized {
            let magic = Magic::<4>::read(reader, &mut ())?;
            if &magic != b"DSSS" {
                return Err(format!("Magic {:#04X?}, != DSSS", &magic.0).into())
            }
            let version = u32::read(reader, &mut ())?;
            if version != 2 {
                return Err(format!("Save file version incorrect I think: {version}").into())
            }
            let flags = u32::read(reader, &mut ())?;
            println!("Version={version}, Save Flags: {:034b}", flags); // theres flags for encryption type, compression,
            let _save_or_user_i_think = u32::read(reader, &mut ())?;
            let flags = SaveFlags::from_bits_truncate(flags);
            println!("Flags: {:?}", flags);

            if flags.contains(SaveFlags::PHOTO) {
                let _steamid32 = reader.read_u32()?;
                let _null = reader.read_u32()?;
            }

            let data_start = reader.tell()?;
            reader.seek(std::io::SeekFrom::End(-12))?;
            let real_data_len = reader.stream_position()? - data_start;
            let decrypted_len = u64::read(reader, &mut ())?;
            log::info!("decrypted_len={decrypted_len:x}");
            let end_hash = u32::read(reader, &mut ())?;
            log::info!("end_hash={end_hash:x}");
            let len = reader.stream_position()?;
            reader.seek(SeekFrom::Start(0))?;
            let mut file_bytes: Vec<u8> = vec![];
            reader.read_to_end(&mut file_bytes)?;
            let file_hash = murmur3(&file_bytes[..(len as usize - 4)], 0xffffffff);
            if end_hash != file_hash {
                log::info!("[File Hash Check] Invalid File Hashes: target={:x}, calculated={:x}", end_hash, file_hash);
            } else {
                log::info!("[File Hash Check] File Hashes equal: target={:x}, calculated={:x}", end_hash, file_hash);
            }

            // Decryption
            reader.seek(SeekFrom::Start(data_start))?;
            // TODO: figure out the magical + 8
            let mut encrypted = vec![0; real_data_len as usize + 8];
            reader.read_exact(&mut encrypted)?;
            //reader.read_to_end(&mut encrypted)?;
            let data = if flags.contains(SaveFlags::MANDARIN) {
                let mandarin = Mandarin::init_from_game(ctx.game)?;
                let key = ctx.key
                    .map(|steamid| ctx.game.get_key_from_steamid(steamid))
                    .unwrap_or_else(|| {mandarin.brute_force(&encrypted, decrypted_len)});
                ctx.key = Some(key);
                let decrypted_buf = mandarin.decrypt(&encrypted, decrypted_len as u64, key)?;
                log::info!("[Decrypted]");
                decrypted_buf
            } else if flags.contains(SaveFlags::CITRUS) {
                let key = ctx.key.unwrap_or(0);
                let citrus = Citrus::new(key, ctx.curve_index);
                let decrypted = citrus.decrypt(&encrypted, decrypted_len as usize).unwrap();
                decrypted 
            } else {
                encrypted
            };
            let data = if flags.contains(SaveFlags::DEFLATE) {
                // Decompression
                let mut decrypted_buf = Cursor::new(&data);
                // this is so fucking stupid
                let _compressed_size_from_afterthis = u64::read(&mut decrypted_buf, &mut ())?;
                let _unk = u32::read(&mut decrypted_buf, &mut ())?; // this is just 1?
                let compressed_buffer_size = u32::read(&mut decrypted_buf, &mut ())?;
                let decompressed_size = u64::read(&mut decrypted_buf, &mut ())?;
                let pos = decrypted_buf.position() as usize;
                println!("{}, {}, {}", pos, compressed_buffer_size, decrypted_buf.get_ref().len());
                let compressed = &decrypted_buf.get_ref()[pos..pos+compressed_buffer_size as usize];
                let mut decoder = DeflateDecoder::new(Vec::new());
                decoder.write_all(&compressed)?;
                let decompressed = decoder.finish()?;
                if decompressed_size != decompressed.len() as u64 {
                    log::info!("[Decompression] expected size not equal to buffer size: continuing...");
                }
                log::info!("[Decompressed]");
                decompressed
            } else {data};

            //let good_header = [0x99, 0xF1, 0xE3, 0xDB, 0x03, 0x00, 0x00, 0x00, 0xDC, 0xCC, 0x7F, 0x82, 0x27, 0x36, 0x5A, 0x69];
            //data[0..16].copy_from_slice(&good_header);
            //let _ = std::fs::write("./outputs/raw_save.bin", &data);
            // for some reason ps5 saves with just deflate are offset by 4 (might be all deflate
            // things, but idk)

            let buf = if flags == SaveFlags::DEFLATE {
                let mut padded = vec![0u8; 12];
                padded.extend(&data);
                padded
            } else {
                data
            };

            let mut data = Cursor::new(buf);

            if flags == SaveFlags::DEFLATE {
                data.set_position(12);
            }
            let mut fields = Vec::new();
            while let Ok(h) = u32::read(&mut data, &mut ()) {
                match types::Class::read(&mut data) {
                    Ok(field_value) => fields.push((h, field_value)),
                    Err(e) => {
                        println!("[ERROR] Error reading class native_field_hash={h:010x}: {e}");
                    }
                }
            }

            Ok(SaveFile {
                fields,
                flags,
                game: ctx.game
            })
        }
}
