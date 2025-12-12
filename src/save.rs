pub mod crypt;
pub mod types;

use std::{fs::File, io::{Cursor, Read, Seek, SeekFrom, Write}, path::Path};

use flate2::{Compression, write::{DeflateDecoder, DeflateEncoder}};
use crate::{util::murmur3, file::{Magic, StructRW}, rsz::rszserde::{DeRszInstance, RszSerializerCtx}, save::types::Class};

use crate::file_ext::SeekExt;

use crypt::Mandarin;

#[derive(Debug)]
pub struct SaveFile {
    pub data_murmur: u32,
    pub data: Class, //Box<dyn DeRszInstance>,
    pub detail_murmur: u32,
    pub detail: Class,
}

pub struct SaveContext {
    pub key: u64,
}

impl SaveFile {
    pub fn save(&self, path: &Path, key: u64) -> crate::file::Result<()> {
        let mut data = Cursor::new(Vec::<u8>::new());
        let version: u32 = 2;
        let flags: u32 = 0x10 | 0x8; // mandarin | deflate
        let null: u32 = 0;
        let mut ctx = RszSerializerCtx {
            data: &mut data,
            base_addr: 0,
        };

        // write some unk bytes (i forget if this is the type hash or what, might be a murmur?)
        self.data_murmur.to_bytes(&mut ctx).unwrap();
        self.data.to_bytes(&mut ctx).unwrap();
        self.detail_murmur.to_bytes(&mut ctx).unwrap();
        self.detail.to_bytes(&mut ctx).unwrap();

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
        let data = Mandarin::encrypt(data.get_ref(), key)?;

        let mut fb = Cursor::new(Vec::<u8>::new());
        fb.write_all(b"DSSS")?;
        fb.write_all(&version.to_le_bytes())?;
        fb.write_all(&flags.to_le_bytes())?;
        fb.write_all(&null.to_le_bytes())?;
        fb.write_all(&data)?;
        fb.write_all(&decrypted_size.to_le_bytes())?;
        let data = &fb.into_inner();
        let file_hash = murmur3(&data, 0xffffffff);
        let mut f = File::create(path).unwrap();
        f.write_all(&data)?;
        f.write_all(&file_hash.to_le_bytes())?;
        Ok(())
    }
}

impl StructRW<SaveContext> for SaveFile {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut SaveContext) -> crate::file::Result<Self>
    where
        Self: Sized {
            let magic = Magic::<4>::read(reader, &mut ())?;
            if &magic != b"DSSS" {
                return Err(format!("Magic {}, != DSSS", String::from_utf8(magic.0.to_vec())?).into())
            }
            let version = u32::read(reader, &mut ())?;
            if version != 2 {
                return Err(format!("Save file version incorrect I hope: {version}").into())
            }
            let flags = u32::read(reader, &mut ())?;
            println!("Version={version}, Save Flags: {:034b}", flags); // theres flags for encryption type, compression,
                                                                       // etc
            let _save_or_user_i_think = u32::read(reader, &mut ())?;
            //println!("{_save_or_user_i_think}");
            let mandarin = flags & 0x10 != 0;
            let blowfish = flags & 0x1 != 0;
            let _citrus = flags & 0x4 != 0;
            let deflate = flags & 0x8 != 0;
            // 0x4 is something related to the usage of mandarin and deflate i think
            println!("deflate={deflate}, mandarin={mandarin}, blowfish={blowfish}");

            let data_start = reader.tell()?;
            reader.seek(std::io::SeekFrom::End(-12))?;
            let decrypted_len = u64::read(reader, &mut ())?;
            println!("decrypted_len={decrypted_len:x}");
            let end_hash = u32::read(reader, &mut ())?;
            println!("end_hash={end_hash:x}");
            let len = reader.stream_position()?;
            reader.seek(SeekFrom::Start(0))?;
            let mut file_bytes: Vec<u8> = vec![];
            reader.read_to_end(&mut file_bytes)?;
            let file_hash = murmur3(&file_bytes[..(len as usize - 4)], 0xffffffff);
            if end_hash != file_hash {
                println!("[File Hash Check] Invalid File Hashes: target={:x}, calculated={:x}", end_hash, file_hash);
            } else {
                println!("[File Hash Check] File Hashes equal: target={:x}, calculated={:x}", end_hash, file_hash);
            }

            // Decryption
            reader.seek(SeekFrom::Start(data_start))?;
            let mut encrypted = vec![];
            reader.read_to_end(&mut encrypted)?;
            let data = if mandarin && deflate || true {
                let key = if ctx.key == 0 {
                    Mandarin::brute_force(&encrypted, decrypted_len as u64)
                } else {ctx.key};
                let decrypted_buf = Mandarin::decrypt(&encrypted, decrypted_len as u64, key)?;
                println!("[Decrypted]");
                /*let encrypted_buf = Mandarin::encrypt(&decrypted_buf, key)?;
                  println!("[Re-Encrypted]");
                  let mut data_cursor = Cursor::new(Vec::new());
                  data_cursor.write_all(b"DSSS")?;
                  data_cursor.write_all(&version.to_le_bytes())?;
                  data_cursor.write_all(&flags.to_le_bytes())?;
                  data_cursor.write_all(&_save_or_user_i_think.to_le_bytes())?;
                  data_cursor.write_all(&encrypted_buf)?;
                  data_cursor.write_all(&decrypted_len.to_le_bytes())?;
                  let data = &data_cursor.into_inner();
                  let file_hash = murmur3::Hash32::hash_with_seed(&data, 0xffffffff);
                  let mut f = File::create("./outputs/saves/sanity/recrypt.bin").unwrap();
                  f.write_all(&data)?;
                  f.write_all(&file_hash.to_le_bytes())?;
                  let mut f = File::create("./outputs/tests/decrypted.bin").unwrap();
                  f.write_all(&decrypted_buf)?;*/

                // Decompression
                let mut decrypted_buf = Cursor::new(&decrypted_buf);
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
                    println!("[Decompression] expected size not equal to buffer size: continuing...");
                }
                /*let mut f = File::create("./outputs/tests/stream.bin").unwrap();
                  f.write_all(b"aaaaaaaaaaaaaaaa")?;
                  f.write_all(&decompressed)?;*/
                /*{
                //let mut compressor = libdeflater::Compressor::new(CompressionLvl::new(4).unwrap());
                //let mut compressed = vec![0u8; 0x100000];
                //let compressed_size = compressor.deflate_compress(&decompressed, &mut compressed).unwrap() as u64;
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(5));
                encoder.write_all(&decompressed)?;
                let compressed_data = encoder.finish()?;
                let compressed_size = compressed_data.len() as u64;
                let mut data = Cursor::new(Vec::<u8>::new());
                data.write(&(compressed_size + 0x10).to_le_bytes())?;
                data.write(&(1u32).to_le_bytes())?;
                data.write(&(compressed_size as u32).to_le_bytes())?;
                data.write(&decompressed_size.to_le_bytes())?;
                data.write(&compressed_data[..compressed_size as usize])?;
                let mut f = File::create("./outputs/saves/sanity/recompressed.bin").unwrap();
                f.write_all(&data.get_ref())?;
                let decrypted_len: u64 = data.get_ref().len() as u64;
                println!("recompressed_buf_len={decrypted_len:x}");

                let encrypted_buf = Mandarin::encrypt(&data.get_ref(), key)?;
                println!("[Re-Encrypted]");
                let mut data_cursor = Cursor::new(Vec::new());
                data_cursor.write_all(b"DSSS")?;
                data_cursor.write_all(&version.to_le_bytes())?;
                data_cursor.write_all(&flags.to_le_bytes())?;
                data_cursor.write_all(&_save_or_user_i_think.to_le_bytes())?;
                data_cursor.write_all(&encrypted_buf)?;
                data_cursor.write_all(&decrypted_len.to_le_bytes())?;
                let data = &data_cursor.into_inner();
                let file_hash = murmur3::Hash32::hash_with_seed(&data, 0xffffffff);
                let mut f = File::create("./outputs/saves/sanity/recrypt_recompress.bin").unwrap();
                f.write_all(&data)?;
                f.write_all(&file_hash.to_le_bytes())?;
                }*/

                println!("[Decompressed]");
                decompressed
            } else {
                encrypted
            };
            let data = &mut Cursor::new(&data);
            let unk = u32::read(data, &mut ())?;
            let savedata = types::Class::read(data, &mut ())?;
            let unk2 = u32::read(data, &mut ())?;
            let detail = types::Class::read(data, &mut ())?;

            Ok(SaveFile {
                data_murmur: unk,
                data: savedata,
                detail_murmur: unk2,
                detail
            })
        }
}
