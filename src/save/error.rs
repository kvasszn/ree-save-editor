use thiserror::Error;

use crate::save::{SaveFlags, crypto::{LimeError, MandarinError, blowfish}};

pub type Result<T> = std::result::Result<T, SaveError>;
#[derive(Error, Debug)]
pub enum SaveError {
    #[error("invalid magic")]
    InvalidMagic,
    #[error("unsupported version {0}")]
    UnsupportedVersion(u32),
    #[error("unsupported flag {0:08x}")]
    UnsupportedFlag(u32),
    #[error("{0:?} save requires ID")]
    RequiresID(SaveFlags),
    #[error("blowfish error: {0}")]
    BlowfishError(#[from] blowfish::BlowfishError),
    #[error("compression error: {0}")]
    CompressionError(std::io::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("mandarin error: {0}")]
    MandarinError(#[from] MandarinError),
    #[error("lime error: {0}")]
    LimeError(#[from] LimeError),
    #[error("serialize error: {0}")]
    SerializationError(#[from] Box<dyn std::error::Error>),
}

