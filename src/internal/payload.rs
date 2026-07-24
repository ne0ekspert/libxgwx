use super::*;

use base64::Engine;
use bzip2::read::BzDecoder;
use std::io::Read;

pub(crate) struct Base64Payload {
    pub(crate) encoded_len: usize,
    pub(crate) raw_len: usize,
    pub(crate) data: Vec<u8>,
}

pub(crate) fn decode_base64_payload(
    text: &str,
    compressed: bool,
) -> Result<Base64Payload, XgwxError> {
    let encoded = compact_ascii_bytes(text);
    if encoded.is_empty() {
        return Ok(Base64Payload {
            encoded_len: 0,
            raw_len: 0,
            data: Vec::new(),
        });
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .map_err(XgwxError::Base64)?;
    if decoded.len() > MAX_BASE64_DECODED_LEN {
        return Err(XgwxError::ResourceLimitExceeded {
            resource: "base64 decoded payload size",
            limit: MAX_BASE64_DECODED_LEN,
        });
    }

    let raw_len = decoded.len();
    let data = if compressed {
        decompress_bzip2(decoded.as_slice())
    } else {
        Ok(decoded)
    }?;

    Ok(Base64Payload {
        encoded_len: encoded.len(),
        raw_len,
        data,
    })
}

pub(crate) fn decode_hex_ascii_payload(
    text: &str,
    element: &str,
    attribute: &str,
) -> Result<Vec<u8>, XgwxError> {
    let encoded = compact_ascii_bytes(text);
    if !encoded.len().is_multiple_of(2) {
        return Err(XgwxError::InvalidHexPayload {
            element: element.to_owned(),
            attribute: attribute.to_owned(),
        });
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.chunks_exact(2) {
        let Some(high) = hex_nibble(pair[0]) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        let Some(low) = hex_nibble(pair[1]) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

pub(crate) fn collect_decoded_payloads(
    element: &XmlElement,
    path: &mut Vec<String>,
    payloads: &mut Vec<Result<DecodedPayloadSummary, XgwxError>>,
) {
    path.push(element.name.clone());

    if is_base64_payload_element(element) {
        payloads.push(decoded_payload_summary(element, path));
    }

    for child in &element.children {
        collect_decoded_payloads(child, path, payloads);
    }

    path.pop();
}

pub(crate) fn decoded_payload_summary(
    element: &XmlElement,
    path: &[String],
) -> Result<DecodedPayloadSummary, XgwxError> {
    let compressed = attr_bool(element, "Compressed").unwrap_or(false);
    let payload = decode_base64_payload(&element.text, compressed)?;

    Ok(DecodedPayloadSummary {
        path: path.join("/"),
        tag: element.name.clone(),
        compressed,
        encoded_len: payload.encoded_len,
        raw_len: payload.raw_len,
        decoded_len: payload.data.len(),
        data: payload.data,
        attributes: element.attributes.clone(),
    })
}

pub(crate) fn is_base64_payload_element(element: &XmlElement) -> bool {
    if element.attribute("Compressed").is_some() {
        return true;
    }

    if element
        .attribute("dt")
        .is_some_and(|value| value.to_ascii_lowercase().contains("base64"))
    {
        return true;
    }

    is_known_payload_tag(&element.name) && looks_like_base64_payload_text(&element.text)
}

pub(crate) fn is_known_payload_tag(name: &str) -> bool {
    matches!(
        name,
        "Symbols"
            | "HMIFlags"
            | "ProgramData"
            | "OnlineUploadData"
            | "RungTableData"
            | "TableData"
            | "ArrayData"
            | "StringTableData"
            | "PulseTableData"
            | "SafetySignature"
            | "RWItem_StateMemento"
            | "RetainValue"
            | "DataTrace"
    )
}

pub(crate) fn looks_like_base64_payload_text(text: &str) -> bool {
    let mut len = 0;
    for byte in text.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')) {
            return false;
        }
        len += 1;
    }

    len != 0 && len % 4 == 0
}

fn compact_ascii_bytes(text: &str) -> Vec<u8> {
    text.bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect()
}

fn decompress_bzip2(raw: &[u8]) -> Result<Vec<u8>, XgwxError> {
    let mut decoder = BzDecoder::new(raw);
    let mut decompressed = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = decoder.read(&mut chunk).map_err(XgwxError::Bzip2)?;
        if read == 0 {
            break;
        }
        if decompressed.len().saturating_add(read) > MAX_BZIP2_DECOMPRESSED_LEN {
            return Err(XgwxError::ResourceLimitExceeded {
                resource: "bzip2 decompressed payload size",
                limit: MAX_BZIP2_DECOMPRESSED_LEN,
            });
        }
        decompressed.extend_from_slice(&chunk[..read]);
    }
    Ok(decompressed)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
