use std::io::{self, Seek, SeekFrom, Write};

use crate::align_up;

pub trait SeekExt: Seek {
    fn seek_assert_align_up(&mut self, target_pos: u64, align: u64) -> io::Result<u64> {
        let current_pos = self.stream_position()?;
        let expected_pos = align_up(current_pos, align);

        if expected_pos != target_pos {
            panic!(
                "This seek is expected to only align up {}. At 0x{:08X}, seeking to 0x{:08X} (Expected 0x{:08X})",
                align, current_pos, target_pos, expected_pos
            );
        }

        if current_pos != target_pos {
            self.seek(SeekFrom::Start(target_pos))?;
        }

        Ok(target_pos)
    }
    fn seek_noop(&mut self, from_start: u64) -> io::Result<u64> {
        let pos = self.stream_position()?;
        if pos != from_start {
            assert_eq!(
                pos,
                from_start,
                "This seek is expected to be no-op. At 0x{pos:08X}, seeking to 0x{from_start:08X}",
            );
        }
        Ok(pos)
    }
    fn seek_align_up(&mut self, align: u64) -> std::io::Result<u64> {
        if align == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Alignment must be greater than 0",
            ));
        }
        let pos = self.stream_position()?;
        let remainder = pos % align;
        if remainder == 0 {
            return Ok(pos)
        }
        let padding = align - remainder;
        self.seek(std::io::SeekFrom::Current(padding as i64))
    }
    fn tell(&mut self) -> io::Result<u64> {
        Ok(self.stream_position()?)
    }
    fn seek_align_up_offset(&mut self, align: u64, offset: u64) -> std::io::Result<u64> {
        let pos = self.stream_position()?;
        let r = pos % align;
        let aligned = if r <= offset {
            pos + (offset - r)
        } else {
            pos + (align - r) + offset
        };
        self.seek(SeekFrom::Start(aligned))
    }
}

impl <T: Seek + ?Sized> SeekExt for T {}

pub fn seek_align_up<S: Seek + ?Sized>(stream: &mut S, align: u64) -> std::io::Result<u64> {
    let pos = stream.stream_position()?;
    let aligned = align_up(pos, align);
    if aligned != pos {
        stream.seek(std::io::SeekFrom::Start(aligned))?;
    }
    Ok(aligned)
}

pub trait WriteAlign {
    fn write_align_up(&mut self, align: u64) -> std::io::Result<()>;
    fn write_align_up_offset(&mut self, align: u64, offset: u64) -> std::io::Result<u64>;
}

impl<W: Write + Seek> WriteAlign for W {
    fn write_align_up(&mut self, align: u64) -> std::io::Result<()> {
        if align == 0 { return Ok(()); }
        let pos = self.stream_position()?;
        let remainder = pos % align;
        if remainder != 0 {
            let pad = align - remainder;
            self.write_all(&vec![0u8; pad as usize])?; 
        }
        Ok(())
    }

    fn write_align_up_offset(&mut self, align: u64, offset: u64) -> std::io::Result<u64> {
        let pos = self.stream_position()?;
        let r = pos % align;
        let aligned = if r <= offset {
            pos + (offset - r)
        } else {
            pos + (align - r) + offset
        };
        let pad_len = aligned - pos;
        if pad_len > 0 {
            self.write_all(&vec![0u8; pad_len as usize])?;
        }
        Ok(aligned)
    }
}
