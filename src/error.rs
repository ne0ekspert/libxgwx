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
    ModuleNotFound {
        base: u32,
        slot: u32,
    },
    AmbiguousModule {
        base: u32,
        slot: u32,
    },
    MissingModuleAttribute {
        base: u32,
        slot: u32,
        attribute: &'static str,
    },
    InvalidModuleInputFilterTarget {
        base: u32,
        slot: u32,
    },
    UnknownModuleCatalogModel {
        model: String,
    },
    UnknownModuleOption {
        model: String,
        key: String,
    },
    ModuleCatalogMismatch {
        base: u32,
        slot: u32,
    },
    ModuleOptionIndexOutOfRange {
        key: String,
        index: u32,
        count: u32,
    },
    InvalidModuleOptionValue {
        key: String,
        value: u32,
    },
    ModuleOptionDetailsTooShort {
        key: String,
    },
    ModulePlacementExceedsBase {
        base: u32,
        slot: u32,
        slot_span: u32,
        slot_count: u32,
    },
    ModulePlacementConflict {
        base: u32,
        slot: u32,
        slot_span: u32,
        conflicting_slot: u32,
    },
    ProgramNotFound {
        index: usize,
    },
    MissingProgramAttribute {
        index: usize,
        attribute: &'static str,
    },
    MissingProgramName {
        index: usize,
    },
    MissingSymbols,
    VariableNotFound {
        index: usize,
    },
    InvalidVariableRecord {
        index: usize,
    },
    VariableFieldLengthChanged {
        index: usize,
        field: &'static str,
        expected_utf16_units: usize,
        actual_utf16_units: usize,
    },
    LadderCellNotFound {
        program_index: usize,
        offset: usize,
    },
    LadderCellChanged {
        program_index: usize,
        offset: usize,
    },
    LadderCellLengthChanged {
        expected_utf16_units: usize,
        actual_utf16_units: usize,
    },
    AuthenticatedRewriteUnsupported,
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
            Self::ModuleNotFound { base, slot } => {
                write!(f, "module at base {base}, slot {slot} was not found")
            }
            Self::AmbiguousModule { base, slot } => {
                write!(f, "multiple modules match base {base}, slot {slot}")
            }
            Self::MissingModuleAttribute {
                base,
                slot,
                attribute,
            } => write!(
                f,
                "module at base {base}, slot {slot} is missing {attribute}"
            ),
            Self::InvalidModuleInputFilterTarget { base, slot } => write!(
                f,
                "module at base {base}, slot {slot} is not an XGI-D24A/B input module"
            ),
            Self::UnknownModuleCatalogModel { model } => {
                write!(f, "XGK module catalog does not contain {model}")
            }
            Self::UnknownModuleOption { model, key } => {
                write!(f, "XGK module {model} has no writable option {key}")
            }
            Self::ModuleCatalogMismatch { base, slot } => write!(
                f,
                "module at base {base}, slot {slot} does not uniquely match the XGK module catalog"
            ),
            Self::ModuleOptionIndexOutOfRange { key, index, count } => {
                write!(f, "module option {key} index {index} is outside 0..{count}")
            }
            Self::InvalidModuleOptionValue { key, value } => {
                write!(f, "module option {key} does not allow value {value}")
            }
            Self::ModuleOptionDetailsTooShort { key } => {
                write!(f, "module Details is too short for option {key}")
            }
            Self::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count,
            } => write!(
                f,
                "module at base {base}, slot {slot} occupies {slot_span} slots but the base has only {slot_count} slots"
            ),
            Self::ModulePlacementConflict {
                base,
                slot,
                slot_span,
                conflicting_slot,
            } => write!(
                f,
                "module at base {base}, slot {slot} occupies {slot_span} slots and overlaps the module at slot {conflicting_slot}"
            ),
            Self::ProgramNotFound { index } => {
                write!(f, "program at index {index} was not found")
            }
            Self::MissingProgramAttribute { index, attribute } => {
                write!(f, "program at index {index} is missing {attribute}")
            }
            Self::MissingProgramName { index } => {
                write!(f, "program at index {index} is missing its name text")
            }
            Self::MissingSymbols => write!(f, "workspace is missing its Symbols payload"),
            Self::VariableNotFound { index } => {
                write!(f, "variable at index {index} was not found")
            }
            Self::InvalidVariableRecord { index } => {
                write!(
                    f,
                    "variable at index {index} does not match the supported symbol record layout"
                )
            }
            Self::VariableFieldLengthChanged {
                index,
                field,
                expected_utf16_units,
                actual_utf16_units,
            } => write!(
                f,
                "variable {index} {field} must keep its encoded length at {expected_utf16_units} UTF-16 units (got {actual_utf16_units})"
            ),
            Self::LadderCellNotFound {
                program_index,
                offset,
            } => write!(
                f,
                "ladder cell at byte offset {offset} was not found in program {program_index}"
            ),
            Self::LadderCellChanged {
                program_index,
                offset,
            } => write!(
                f,
                "ladder cell at byte offset {offset} in program {program_index} no longer matches the expected text"
            ),
            Self::LadderCellLengthChanged {
                expected_utf16_units,
                actual_utf16_units,
            } => write!(
                f,
                "ladder cell edits must keep the encoded length at {expected_utf16_units} UTF-16 units (got {actual_utf16_units})"
            ),
            Self::AuthenticatedRewriteUnsupported => write!(
                f,
                "cannot rewrite workspace: the container does not match the validated XG5000 header, alignment, or Security layout"
            ),
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
