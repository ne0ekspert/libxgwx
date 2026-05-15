use std::fmt;
use std::io;

/// Errors returned while parsing `.xgwx` files.
#[derive(Debug)]
pub enum XgwxError {
    Io(io::Error),
    InvalidMagic,
    MissingMainPayload,
    InvalidGzipHeader {
        offset: usize,
    },
    UnsupportedGzipCompression {
        offset: usize,
        method: u8,
    },
    ReservedGzipFlags {
        offset: usize,
        flags: u8,
    },
    TruncatedGzipMember {
        offset: usize,
    },
    Inflate(flate2::DecompressError),
    GzipCrcMismatch {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    GzipSizeMismatch {
        offset: usize,
        expected: u32,
        actual: u32,
    },
    MissingProgramData,
    Base64(base64::DecodeError),
    InvalidHexPayload {
        element: String,
        attribute: String,
    },
    Bzip2(io::Error),
    Utf8(std::string::FromUtf8Error),
    Xml(roxmltree::Error),
    ResourceLimitExceeded {
        resource: &'static str,
        limit: usize,
    },
}

impl fmt::Display for XgwxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read xgwx file: {error}"),
            Self::InvalidMagic => write!(f, "file does not start with XG magic"),
            Self::MissingMainPayload => write!(f, "missing gzip-compressed XML payload"),
            Self::InvalidGzipHeader { offset } => write!(f, "invalid gzip header at byte {offset}"),
            Self::UnsupportedGzipCompression { offset, method } => {
                write!(
                    f,
                    "unsupported gzip compression method {method} at byte {offset}"
                )
            }
            Self::ReservedGzipFlags { offset, flags } => {
                write!(f, "reserved gzip flags 0x{flags:02x} at byte {offset}")
            }
            Self::TruncatedGzipMember { offset } => {
                write!(f, "truncated gzip member at byte {offset}")
            }
            Self::Inflate(error) => write!(f, "failed to inflate gzip payload: {error}"),
            Self::GzipCrcMismatch {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "gzip CRC mismatch at byte {offset}: expected 0x{expected:08x}, got 0x{actual:08x}"
            ),
            Self::GzipSizeMismatch {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "gzip size mismatch at byte {offset}: expected {expected}, got {actual}"
            ),
            Self::MissingProgramData => write!(f, "program is missing a ProgramData element"),
            Self::Base64(error) => write!(f, "failed to base64-decode ProgramData: {error}"),
            Self::InvalidHexPayload { element, attribute } => {
                write!(f, "{element} {attribute} is not valid hex ASCII payload")
            }
            Self::Bzip2(error) => write!(f, "failed to bzip2-decompress ProgramData: {error}"),
            Self::Utf8(error) => write!(f, "XML payload is not valid UTF-8: {error}"),
            Self::Xml(error) => write!(f, "XML payload is not well-formed: {error}"),
            Self::ResourceLimitExceeded { resource, limit } => {
                write!(f, "{resource} exceeds parser limit ({limit} bytes/items)")
            }
        }
    }
}

impl std::error::Error for XgwxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Inflate(error) => Some(error),
            Self::Base64(error) => Some(error),
            Self::Bzip2(error) => Some(error),
            Self::Utf8(error) => Some(error),
            Self::Xml(error) => Some(error),
            _ => None,
        }
    }
}
