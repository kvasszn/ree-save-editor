use std::io::{self, Seek, SeekFrom};

use crate::align_up;

pub trait SeekExt: Seek {
    fn seek_assert_align_up(&mut self, target_pos: u64, align: u64) -> io::Result<u64> {
        let current_pos = self.stream_position()?;

        // 1. Verify the logic (Ensure we aren't jumping to a random, unaligned location)
        // We calculate where we *should* be going based on current pos
        let expected_pos = align_up(current_pos, align);

        if expected_pos != target_pos {
            // Keep your assertion logic
            panic!(
                "This seek is expected to only align up {}. At 0x{:08X}, seeking to 0x{:08X} (Expected 0x{:08X})",
                align, current_pos, target_pos, expected_pos
            );
        }

        // 2. Move the cursor (Seek instead of Read)
        // Only seek if we actually need to move
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
