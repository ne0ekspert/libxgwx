use super::*;

use flate2::{Decompress, FlushDecompress, Status};

pub(crate) fn parse_header_label(raw: &[u8]) -> (Option<String>, Option<u32>) {
    let Some(marker_offset) = raw
        .windows(3)
        .position(|window| window == [0xff, 0xfe, 0xff])
    else {
        return (None, None);
    };

    let Some(unit_count) = raw.get(marker_offset + 3).copied().map(usize::from) else {
        return (None, None);
    };

    let start = marker_offset + 4;
    let end = start + unit_count.saturating_mul(2);
    let Some(encoded) = raw.get(start..end) else {
        return (None, None);
    };

    let label = char::decode_utf16(
        encoded
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]])),
    )
    .collect::<Result<String, _>>()
    .ok();
    let label_following_u32 = raw
        .get(end..end + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes);

    (label, label_following_u32)
}

pub(crate) fn find_gzip_member(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(GZIP_MAGIC.len())
        .position(|window| window == GZIP_MAGIC.as_slice())
        .map(|relative| start + relative)
}

pub(crate) fn find_trailer_gzip_members(bytes: &[u8], trailer_start: usize) -> Vec<GzipMember> {
    let mut members = Vec::new();
    let mut search_from = trailer_start;

    while let Some(offset) = find_gzip_member(bytes, search_from) {
        match parse_gzip_member(bytes, offset) {
            Ok(member) => {
                search_from = member.end_offset;
                members.push(member);
            }
            Err(_) => {
                search_from = offset + 1;
            }
        }
    }

    members
}

pub(crate) fn parse_gzip_member(bytes: &[u8], offset: usize) -> Result<GzipMember, XgwxError> {
    let deflate_start = parse_gzip_header(bytes, offset)?;
    let mut decompressor = Decompress::new(false);
    let mut data = Vec::new();
    let mut input = &bytes[deflate_start..];
    let mut output = [0; 8192];

    loop {
        let total_in_before = decompressor.total_in();
        let total_out_before = decompressor.total_out();
        let status = decompressor
            .decompress(input, &mut output, FlushDecompress::None)
            .map_err(XgwxError::Inflate)?;
        let consumed = usize::try_from(decompressor.total_in() - total_in_before)
            .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;
        let produced = usize::try_from(decompressor.total_out() - total_out_before)
            .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;

        data.extend_from_slice(&output[..produced]);
        input = &input[consumed..];

        if status == Status::StreamEnd {
            break;
        }

        if consumed == 0 && produced == 0 {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    let deflate_len = usize::try_from(decompressor.total_in())
        .map_err(|_| XgwxError::TruncatedGzipMember { offset })?;
    let trailer_start = deflate_start + deflate_len;
    let end_offset = trailer_start + GZIP_TRAILER_LEN;
    let trailer = bytes
        .get(trailer_start..end_offset)
        .ok_or(XgwxError::TruncatedGzipMember { offset })?;

    validate_gzip_trailer(offset, trailer, &data)?;

    Ok(GzipMember {
        offset,
        end_offset,
        data,
    })
}

pub(crate) fn parse_gzip_header(bytes: &[u8], offset: usize) -> Result<usize, XgwxError> {
    let header = bytes
        .get(offset..offset + GZIP_HEADER_LEN)
        .ok_or(XgwxError::TruncatedGzipMember { offset })?;

    if &header[0..2] != GZIP_MAGIC {
        return Err(XgwxError::InvalidGzipHeader { offset });
    }

    let method = header[2];
    if method != GZIP_METHOD_DEFLATE {
        return Err(XgwxError::UnsupportedGzipCompression { offset, method });
    }

    let flags = header[3];
    if flags & 0b1110_0000 != 0 {
        return Err(XgwxError::ReservedGzipFlags { offset, flags });
    }

    let mut cursor = offset + GZIP_HEADER_LEN;

    if flags & 0x04 != 0 {
        let xlen_bytes = bytes
            .get(cursor..cursor + 2)
            .ok_or(XgwxError::TruncatedGzipMember { offset })?;
        let xlen = u16::from_le_bytes([xlen_bytes[0], xlen_bytes[1]]) as usize;
        cursor += 2 + xlen;
        if cursor > bytes.len() {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    if flags & 0x08 != 0 {
        cursor = skip_zero_terminated(bytes, cursor, offset)?;
    }

    if flags & 0x10 != 0 {
        cursor = skip_zero_terminated(bytes, cursor, offset)?;
    }

    if flags & 0x02 != 0 {
        cursor += 2;
        if cursor > bytes.len() {
            return Err(XgwxError::TruncatedGzipMember { offset });
        }
    }

    Ok(cursor)
}

pub(crate) fn skip_zero_terminated(
    bytes: &[u8],
    cursor: usize,
    gzip_offset: usize,
) -> Result<usize, XgwxError> {
    let Some(relative_end) = bytes
        .get(cursor..)
        .and_then(|remaining| remaining.iter().position(|byte| *byte == 0))
    else {
        return Err(XgwxError::TruncatedGzipMember {
            offset: gzip_offset,
        });
    };

    Ok(cursor + relative_end + 1)
}

pub(crate) fn validate_gzip_trailer(
    offset: usize,
    trailer: &[u8],
    data: &[u8],
) -> Result<(), XgwxError> {
    let expected_crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    let actual_crc = crc32fast::hash(data);
    if expected_crc != actual_crc {
        return Err(XgwxError::GzipCrcMismatch {
            offset,
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    let expected_size = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
    let actual_size = data.len() as u32;
    if expected_size != actual_size {
        return Err(XgwxError::GzipSizeMismatch {
            offset,
            expected: expected_size,
            actual: actual_size,
        });
    }

    Ok(())
}

pub(crate) fn parse_xml(xml: &str) -> Result<XmlElement, XgwxError> {
    let document = roxmltree::Document::parse(xml).map_err(XgwxError::Xml)?;
    let root = document.root_element();
    Ok(xml_element_from_node(root))
}

pub(crate) fn xml_element_from_node(node: roxmltree::Node<'_, '_>) -> XmlElement {
    let name = node.tag_name().name().to_owned();
    let attributes = node
        .attributes()
        .map(|attribute| XmlAttribute {
            name: attribute.name().to_owned(),
            value: attribute.value().to_owned(),
        })
        .collect();
    let text = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect();
    let children = node
        .children()
        .filter(|child| child.is_element())
        .map(xml_element_from_node)
        .collect();

    XmlElement {
        name,
        attributes,
        text,
        children,
    }
}

pub(crate) struct XmlDescendantsNamed<'a> {
    name: &'a str,
    stack: Vec<&'a XmlElement>,
}

impl<'a> XmlDescendantsNamed<'a> {
    pub(crate) fn new(root: &'a XmlElement, name: &'a str) -> Self {
        Self {
            name,
            stack: vec![root],
        }
    }
}

impl<'a> Iterator for XmlDescendantsNamed<'a> {
    type Item = &'a XmlElement;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(element) = self.stack.pop() {
            self.stack.extend(element.children.iter().rev());
            if element.name == self.name {
                return Some(element);
            }
        }

        None
    }
}
