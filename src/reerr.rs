use std::error::Error;
use std::fmt;
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum FileParseError {
    MagicError { real_magic: String, read_magic: String },
    TexReadError { source: String },
    GDeflateError { source: String },
    Unsupported { source: String },
    InvalidBool(u8),
    BadAlign(u64, u64),
    InvalidRszTypeHash(u32),
}

impl Error for FileParseError {}

impl fmt::Display for FileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MagicError { real_magic, read_magic } => write!(f, "File magic does not match. Should Be: {}, Is => {}", real_magic, read_magic),
            Self::TexReadError { source } => write!(f, "{}", source),
            Self::GDeflateError { source } => write!(f, "{}", source),
            Self::Unsupported { source } => write!(f, "{}", source),
            Self::InvalidBool(v) => write!(f, "Invalid value {} for bool", *v),
            Self::BadAlign(pos, align) => write!(f, "Non-zero padding with pos:{:08X}, align:{:08X}", *pos, *align),
            Self::InvalidRszTypeHash(v) => write!(f, "Invalid type hash {:08X} not found in rsz", *v),
        }
    }
}

#[derive(Debug)]
pub enum RszError<'a> {
    UnsetDeserializer(&'a str),
    UnsetSerializer(&'a str),
    InvalidRszTypeHash(u32),
    InvalidRszObjectIndex(u32, u32),
    MissingFieldDescription(String),
}
impl<'a> Error for RszError<'a> {}

impl<'a> fmt::Display for RszError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRszTypeHash(v) => write!(f, "Invalid type hash {:08X} not found in rsz", *v),
            Self::UnsetDeserializer(v) => write!(f, "Deserializer not set for {}", *v,),
            Self::UnsetSerializer(v) => write!(f, "Deserializer not set for {}", *v),
            Self::InvalidRszObjectIndex(i, h) => write!(f, "Invalid Object Index {} for hash {}", *i, *h),
            Self::MissingFieldDescription(source) => write!(f, "Missing field description when reading data at {}", source),
        }
    }
}

