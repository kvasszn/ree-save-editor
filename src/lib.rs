use std::io::Result;

pub mod align;
pub mod gensdk;
pub mod reerr;
pub mod bitfield;
pub mod byte_reader;
pub mod compression;
pub mod file_ext;
pub mod msg;
pub mod rsz;
pub mod tex;
pub mod user;
pub mod pog;
pub mod font;
pub mod scn;
pub mod mesh;

pub trait Save {
    fn save(&self, path: &std::path::Path) -> Result<()>;
}

