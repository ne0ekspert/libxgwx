use crate::*;
use std::cmp::Reverse;
use std::io::Write;
use std::ops::Range;

use flate2::Compression;
use flate2::write::GzEncoder;

const SUPPORTED_HEADER_LEN: usize = 138;
const HEADER_CHECKSUM_OFFSET: usize = 64;
const HEADER_CHECKSUM_INPUT_START: usize = 68;
const COMPRESSED_SIZE_OFFSET: usize = 134;
const MAIN_GZIP_ALIGNMENT: usize = 4;
const XG_FRAME_HEADER_LEN: usize = 8;
const XG_FRAME_TAG_LEN: usize = 8;
const XG_FRAME_FOOTER_LEN: usize = 4;
const XG_FRAME_MAGIC: &[u8; 4] = b"HEAD";
const XG_FRAME_FOOTER: &[u8; 4] = b"FOOT";
const XG_CRC64_POLYNOMIAL: u64 = 0x1b;

/// Changes to ordinary attributes on one hardware `<Module>` record.
///
/// `Base` and `Slot` identify the record and are intentionally not editable in
/// this first writer API. A `None` field leaves the corresponding attribute
/// byte-for-byte unchanged. Module `Details` must be an even-length hexadecimal
/// string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct ModulePatch {
    pub id: Option<u32>,
    pub sub_type: Option<u32>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub details: Option<String>,
}

impl XgwxDocument {
    /// Update one module selected by its unique base and slot.
    ///
    /// Only the requested XML attribute values are replaced. Element order,
    /// whitespace, unknown elements, binary payloads, and the workspace trailer
    /// remain untouched.
    pub fn update_module(
        &mut self,
        base: u32,
        slot: u32,
        patch: &ModulePatch,
    ) -> Result<(), XgwxError> {
        if let Some(details) = &patch.details {
            validate_module_details(details)?;
        }

        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let matches = document
            .descendants()
            .filter(|node| node.has_tag_name("Module"))
            .filter(|node| {
                node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .collect::<Vec<_>>();

        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => *module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };

        let mut replacements = Vec::new();
        push_u32_replacement(&mut replacements, module, base, slot, "Id", patch.id)?;
        push_u32_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "SubType",
            patch.sub_type,
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Name",
            patch.name.as_deref(),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Comment",
            patch.comment.as_deref(),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Details",
            patch.details.as_deref(),
        )?;

        replacements.sort_by_key(|replacement| Reverse(replacement.0.start));
        let mut xml = self.xml.clone();
        for (range, value) in replacements {
            xml.replace_range(range, &value);
        }

        let root = parse_xml(&xml)?;
        self.xml = xml;
        self.root = root;
        Ok(())
    }

    /// Set the first `Details` byte for an XGI-D24A/B digital input module.
    pub fn set_module_input_filter(
        &mut self,
        base: u32,
        slot: u32,
        filter: ModuleInputFilter,
    ) -> Result<(), XgwxError> {
        let matches = self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base) && module.slot == Some(slot))
            .collect::<Vec<_>>();
        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };
        if !module
            .name
            .as_deref()
            .is_some_and(|name| name.contains("XGI-D24A/B"))
        {
            return Err(XgwxError::InvalidModuleInputFilterTarget { base, slot });
        }

        let mut details = module
            .details
            .clone()
            .ok_or(XgwxError::MissingModuleAttribute {
                base,
                slot,
                attribute: "Details",
            })?;
        validate_module_details(&details)?;
        if details.len() < 2 {
            return Err(XgwxError::InvalidHexPayload {
                element: "Module".to_owned(),
                attribute: "Details".to_owned(),
            });
        }
        details.replace_range(..2, &format!("{:02X}", filter.raw()));
        self.update_module(
            base,
            slot,
            &ModulePatch {
                details: Some(details),
                ..ModulePatch::default()
            },
        )
    }

    /// Serialize the workspace after supported edits.
    ///
    /// An unmodified XML payload reuses the original gzip member and produces
    /// byte-identical output. Modified payloads are recompressed, padded to the
    /// four-byte boundary recorded by XG5000, and covered by the container's
    /// additive header checksum. Existing trailing Security metadata is
    /// validated and preserved byte-for-byte.
    pub fn to_bytes(&self) -> Result<Vec<u8>, XgwxError> {
        let original = parse_gzip_member(&self.main_gzip, 0)?;
        if original.data == self.xml.as_bytes() {
            return Ok(assemble_workspace(
                &self.header.raw,
                &self.main_gzip,
                &self.trailer,
            ));
        }

        let preserved_trailer = self.supported_trailer_after_main_padding()?;
        validate_preserved_security(preserved_trailer)?;

        let main_gzip = gzip_xml(self.xml.as_bytes())?;
        let aligned_len = align_up(main_gzip.len(), MAIN_GZIP_ALIGNMENT)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        let aligned_len_u32 =
            u32::try_from(aligned_len).map_err(|_| XgwxError::AuthenticatedRewriteUnsupported)?;
        let padding_len = aligned_len - main_gzip.len();

        let mut header = self.header.raw.clone();
        header[COMPRESSED_SIZE_OFFSET..SUPPORTED_HEADER_LEN]
            .copy_from_slice(&aligned_len_u32.to_le_bytes());
        let checksum = workspace_checksum(&header, aligned_len_u32, &main_gzip);
        header[HEADER_CHECKSUM_OFFSET..HEADER_CHECKSUM_INPUT_START]
            .copy_from_slice(&checksum.to_le_bytes());

        let mut bytes = Vec::with_capacity(header.len() + aligned_len + preserved_trailer.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&main_gzip);
        bytes.resize(bytes.len() + padding_len, 0);
        bytes.extend_from_slice(preserved_trailer);
        Ok(bytes)
    }

    /// Serialize and write the workspace to a file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_to(&self, path: impl AsRef<std::path::Path>) -> Result<(), XgwxError> {
        std::fs::write(path, self.to_bytes()?).map_err(XgwxError::Io)
    }

    fn supported_trailer_after_main_padding(&self) -> Result<&[u8], XgwxError> {
        if self.header.gzip_offset != SUPPORTED_HEADER_LEN
            || self.header.raw.len() != SUPPORTED_HEADER_LEN
        {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        let expected_aligned_len = align_up(self.main_gzip.len(), MAIN_GZIP_ALIGNMENT)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        if self.header.compressed_size_hint != u32::try_from(expected_aligned_len).ok() {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        let padding_len = expected_aligned_len - self.main_gzip.len();
        let padding = self
            .trailer
            .get(..padding_len)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        self.trailer
            .get(padding_len..)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)
    }
}

fn gzip_xml(xml: &[u8]) -> Result<Vec<u8>, XgwxError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml).map_err(XgwxError::Io)?;
    encoder.finish().map_err(XgwxError::Io)
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value
        .checked_add(alignment.checked_sub(1)?)
        .map(|value| value & !(alignment - 1))
}

fn workspace_checksum(header: &[u8], aligned_len: u32, main_gzip: &[u8]) -> u32 {
    let header_sum = header[HEADER_CHECKSUM_INPUT_START..COMPRESSED_SIZE_OFFSET]
        .iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(u32::from(*byte)));
    let gzip_sum = main_gzip
        .iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(u32::from(*byte)));
    header_sum.wrapping_add(aligned_len).wrapping_add(gzip_sum)
}

fn validate_preserved_security(trailer: &[u8]) -> Result<(), XgwxError> {
    let Some(offset) = find_gzip_member(trailer, 0) else {
        return Ok(());
    };
    let member = parse_gzip_member(trailer, offset)?;
    if member.data.starts_with(XG_FRAME_MAGIC) && !validate_xg_frame(&member.data) {
        return Err(XgwxError::AuthenticatedRewriteUnsupported);
    }
    Ok(())
}

pub(crate) fn validate_xg_frame(frame: &[u8]) -> bool {
    let Some(payload_len_bytes) = frame.get(4..8).and_then(|bytes| bytes.try_into().ok()) else {
        return false;
    };
    if frame.get(..4) != Some(XG_FRAME_MAGIC.as_slice()) {
        return false;
    }

    let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;
    let Some(tag_start) = XG_FRAME_HEADER_LEN.checked_add(payload_len) else {
        return false;
    };
    let Some(footer_start) = tag_start.checked_add(XG_FRAME_TAG_LEN) else {
        return false;
    };
    let Some(frame_len) = footer_start.checked_add(XG_FRAME_FOOTER_LEN) else {
        return false;
    };
    if frame.len() != frame_len || frame.get(footer_start..) != Some(XG_FRAME_FOOTER.as_slice()) {
        return false;
    }

    let Some(payload) = frame.get(XG_FRAME_HEADER_LEN..tag_start) else {
        return false;
    };
    let Some(expected_tag) = frame.get(tag_start..footer_start) else {
        return false;
    };
    if xg_crc64(payload).to_le_bytes() != expected_tag {
        return false;
    }

    if payload.starts_with(XG_FRAME_MAGIC) {
        let Some(inner_payload_len_bytes) =
            payload.get(4..8).and_then(|bytes| bytes.try_into().ok())
        else {
            return false;
        };
        let inner_payload_len = u32::from_le_bytes(inner_payload_len_bytes) as usize;
        let Some(inner_len) = XG_FRAME_HEADER_LEN
            .checked_add(inner_payload_len)
            .and_then(|len| len.checked_add(XG_FRAME_TAG_LEN + XG_FRAME_FOOTER_LEN))
        else {
            return false;
        };
        let Some(inner) = payload.get(..inner_len) else {
            return false;
        };
        if !validate_xg_frame(inner) {
            return false;
        }
    }

    true
}

pub(crate) fn xg_crc64(payload: &[u8]) -> u64 {
    let padded_len = align_up(payload.len(), 8).unwrap_or(payload.len());
    let mut crc = 0u64;
    for index in 0..padded_len {
        let byte = payload.get(index).copied().unwrap_or(0);
        crc ^= u64::from(byte) << 56;
        for _ in 0..8 {
            crc = if crc & (1 << 63) != 0 {
                (crc << 1) ^ XG_CRC64_POLYNOMIAL
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn push_u32_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: Option<u32>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_replacement(
            replacements,
            module,
            base,
            slot,
            attribute,
            value.to_string(),
        )?;
    }
    Ok(())
}

fn push_string_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: Option<&str>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_replacement(
            replacements,
            module,
            base,
            slot,
            attribute,
            escape_xml_attribute(value),
        )?;
    }
    Ok(())
}

fn push_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: String,
) -> Result<(), XgwxError> {
    let range = module
        .attributes()
        .find(|item| item.name() == attribute)
        .map(|item| item.range_value())
        .ok_or(XgwxError::MissingModuleAttribute {
            base,
            slot,
            attribute,
        })?;
    replacements.push((range, value));
    Ok(())
}

fn validate_module_details(details: &str) -> Result<(), XgwxError> {
    if details.len().is_multiple_of(2) && details.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(XgwxError::InvalidHexPayload {
            element: "Module".to_owned(),
            attribute: "Details".to_owned(),
        })
    }
}

fn escape_xml_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\r' => escaped.push_str("&#xD;"),
            '\n' => escaped.push_str("&#xA;"),
            '\t' => escaped.push_str("&#x9;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn assemble_workspace(header: &[u8], main_gzip: &[u8], trailer: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(header.len() + main_gzip.len() + trailer.len());
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(main_gzip);
    bytes.extend_from_slice(trailer);
    bytes
}
