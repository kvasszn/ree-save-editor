pub mod crypt;
pub mod types;

use std::{fs::File, io::{Cursor, Read, Seek, SeekFrom, Write}};

use fasthash::murmur3;
use crate::{file::{Magic, StructRW}, save::types::Class};
use fasthash::FastHash;

use crate::file_ext::SeekExt;

use crypt::Mandarin;

#[derive(Debug)]
pub struct SaveFile {
    pub data: Class, //Box<dyn DeRszInstance>,
    pub detail: Class,
}

pub struct SaveContext {
    pub key: u64,
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
        println!("Save Flags: {:034b}", flags); // theres flags for encryption type, compression,
                                                // etc
        let _save_or_user_i_think = u32::read(reader, &mut ())?;
        let mandarin = flags & 0x10 != 0;
        let blowfish = flags & 0x1 != 0;
        let deflate = flags & 0x8 != 0;
        // 0x4 is something related to the usage of mandarin and deflate i think
        println!("deflate={deflate}, mandarin={mandarin}, blowfish={blowfish}");

        let data_start = reader.tell()?;
        reader.seek(std::io::SeekFrom::End(-12))?;
        let decrypted_len = u64::read(reader, &mut ())?;
        println!("{decrypted_len:x}");
        let end_hash = u32::read(reader, &mut ())?;
        println!("{end_hash:x}");
        let len = reader.stream_position()?;
        reader.seek(SeekFrom::Start(0))?;
        let mut file_bytes: Vec<u8> = vec![];
        reader.read_to_end(&mut file_bytes)?;
        let file_hash = murmur3::Hash32::hash_with_seed(&file_bytes[..(len as usize - 4)], 0xffffffff);
        if end_hash != file_hash {
            println!("[File Hash Check] Invalid File Hashes: target={:x}, calculated={:x}", end_hash, file_hash);
        } else {
            println!("[File Hash Check] File Hashes equal: target={:x}, calculated={:x}", end_hash, file_hash);
        }

        // Decryption
        reader.seek(SeekFrom::Start(data_start))?;
        let mut encrypted = vec![];
        reader.read_to_end(&mut encrypted)?;
        let data = if mandarin && deflate {
            let key = if ctx.key == 0 {
                Mandarin::brute_force(&encrypted, decrypted_len as u64)
            } else {ctx.key};
            //println!("Found key: {:#x}", key);
            println!("{decrypted_len}");
            let decrypted_buf = Mandarin::decrypt(&encrypted, decrypted_len as u64, key)?;
            println!("[Decrypted]");
            /*let encrypted_buf = Mandarin::encrypt(&decrypted_buf, key)?;
            println!("[Re-Encrypted]");
            let decrypted_buf = Mandarin::decrypt(&encrypted_buf, decrypted_buf.len() as u64, key)?;
            println!("[Re-Decrypted]");
            let mut data_cursor = Cursor::new(Vec::new());
            data_cursor.write_all(b"DSSS")?;
            data_cursor.write_all(&version.to_le_bytes())?;
            data_cursor.write_all(&flags.to_le_bytes())?;
            data_cursor.write_all(&_save_or_user_i_think.to_le_bytes())?;
            data_cursor.write_all(&encrypted_buf)?;
            data_cursor.write_all(&decrypted_len.to_le_bytes())?;
            let data = &data_cursor.into_inner();
            let file_hash = murmur3::Hash32::hash_with_seed(&data, 0xffffffff);
            let mut f = File::create("./outputs/misc/saves/data001Slot.bin").unwrap();
            f.write_all(&data)?;
            f.write_all(&file_hash.to_le_bytes())?;*/

            // Decompression
            let mut decrypted_buf = Cursor::new(&decrypted_buf);
            let _compressed_size = u64::read(&mut decrypted_buf, &mut ())?;
            let _unk = u32::read(&mut decrypted_buf, &mut ())?;
            // this might just be an offset for smoehting
            let _comrpressed_size_sub0x10 = u32::read(&mut decrypted_buf, &mut ())?;
            let decompressed_size = u64::read(&mut decrypted_buf, &mut ())?;
            //println!("{:#018x}, {:010x}, {:010x}", compressed_size, unk, comrpressed_size_sub0x10);
            let pos = decrypted_buf.position() as usize;
            let compressed = &decrypted_buf.get_ref()[pos..];
            let mut decompressor = libdeflater::Decompressor::new();
            let mut decompressed = vec![0u8; decompressed_size as usize];
            decompressor.deflate_decompress(&compressed, &mut decompressed)?;
            println!("[Decompressed]");
            decompressed
        } else {
            encrypted
        };
        let data = &mut Cursor::new(&data);
        let unk = u32::read(data, &mut ())?;
        let savedata = types::Class::read(data, &mut ())?;
        let unk2 = u32::read(data, &mut ())?;
        //let detail = read_value(data, FieldType::Class, None)?;
        let detail = types::Class::read(data, &mut ())?;
        println!("{unk:#x}, {unk2:#x}");

        // Reading
        Ok(SaveFile {
            data: savedata,
            detail
        })
    }
}
