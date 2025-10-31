use std::{io::{Read, Seek}, ops::*};

use crate::{file::Result, reerr::FileParseError};

pub fn align_up<T: Copy + Add<Output = T> + Sub<Output = T> + Rem<Output = T>>(
    value: T,
    align: T,
) -> T {
    value + (align - value % align) % align
}

pub fn seek_align_up<R: Read + Seek>(stream: &mut R, align: u64) -> Result<u64> {
    let pos = stream.stream_position()?;
    let aligned = align_up(pos, align);
    if aligned != pos {
        let mut buf = vec![0; (aligned - pos).try_into()?];
        stream.read_exact(&mut buf).map_err(|_f| Box::new(FileParseError::BadAlign(pos, align)))?;
    }
    Ok(aligned)
}
