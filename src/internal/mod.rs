use crate::*;

pub(crate) const XG_MAGIC: &[u8; 2] = b"XG";
pub(crate) const GZIP_MAGIC: &[u8; 2] = b"\x1f\x8b";
pub(crate) const GZIP_HEADER_LEN: usize = 10;
pub(crate) const GZIP_TRAILER_LEN: usize = 8;
pub(crate) const GZIP_METHOD_DEFLATE: u8 = 8;
pub(crate) const UTF16_MARKER: &[u8; 3] = b"\xff\xfe\xff";
pub(crate) const MAX_GZIP_MEMBER_DECOMPRESSED_LEN: usize = 16 * 1024 * 1024;
pub(crate) const MAX_TRAILER_GZIP_MEMBERS: usize = 16;
pub(crate) const MAX_BASE64_DECODED_LEN: usize = 16 * 1024 * 1024;
pub(crate) const MAX_BZIP2_DECOMPRESSED_LEN: usize = 16 * 1024 * 1024;

pub(crate) fn text_value(element: &XmlElement) -> Option<String> {
    let text = element.text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

pub(crate) fn attr_string(element: &XmlElement, name: &str) -> Option<String> {
    element.attribute(name).map(str::to_owned)
}

pub(crate) fn attr_u32(element: &XmlElement, name: &str) -> Option<u32> {
    element.attribute(name)?.trim().parse().ok()
}

pub(crate) fn attr_u8(element: &XmlElement, name: &str) -> Option<u8> {
    element.attribute(name)?.trim().parse().ok()
}

pub(crate) fn attr_i32(element: &XmlElement, name: &str) -> Option<i32> {
    element.attribute(name)?.trim().parse().ok()
}

pub(crate) fn attr_bool(element: &XmlElement, name: &str) -> Option<bool> {
    match element.attribute(name)?.trim() {
        "1" | "true" | "TRUE" | "True" => Some(true),
        "0" | "false" | "FALSE" | "False" => Some(false),
        _ => None,
    }
}

mod container;
mod ladder;
mod parameters;
mod payload;

pub(crate) use container::*;
pub(crate) use ladder::*;
pub(crate) use parameters::*;
pub(crate) use payload::*;
