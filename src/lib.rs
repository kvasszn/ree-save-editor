use std::io::Result;

pub mod edit;
pub mod align;
pub mod gensdk;
pub mod reerr;
pub mod bitfield;
pub mod compression;
pub mod file_ext;
pub mod file;
pub mod save;
pub mod msg;
pub mod rsz;
pub mod tdb;
pub mod tex;
pub mod user;
pub mod pog;
pub mod font;
pub mod scn;
pub mod mesh;

pub trait Save {
    fn save(&self, path: &std::path::Path) -> Result<()>;
}

