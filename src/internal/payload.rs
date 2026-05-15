use super::*;

use base64::Engine;
use bzip2::read::BzDecoder;
use std::io::Read;

pub(crate) fn decode_base64_payload(text: &str, compressed: bool) -> Result<Vec<u8>, XgwxError> {
    let encoded = compact_ascii_bytes(text);
    if encoded.is_empty() {
        return Ok(Vec::new());
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(XgwxError::Base64)?;

    if compressed {
        decompress_bzip2(decoded.as_slice())
    } else {
        Ok(decoded)
    }
}

pub(crate) fn decode_base64_payload_with_raw(
    text: &str,
    compressed: bool,
) -> Result<(Vec<u8>, Vec<u8>), XgwxError> {
    let encoded = compact_ascii_bytes(text);
    if encoded.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(XgwxError::Base64)?;

    if !compressed {
        return Ok((decoded.clone(), decoded));
    }

    let raw = decoded;
    let decompressed = decompress_bzip2(raw.as_slice())?;
    Ok((raw, decompressed))
}

pub(crate) fn decode_hex_ascii_payload(
    text: &str,
    element: &str,
    attribute: &str,
) -> Result<Vec<u8>, XgwxError> {
    let encoded = compact_ascii_bytes(text);
    if encoded.len() % 2 != 0 {
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
    let encoded_len = compact_ascii_len(&element.text);
    let (raw, data) = decode_base64_payload_with_raw(&element.text, compressed)?;

    Ok(DecodedPayloadSummary {
        path: path.join("/"),
        tag: element.name.clone(),
        compressed,
        encoded_len,
        raw_len: raw.len(),
        decoded_len: data.len(),
        data,
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

fn compact_ascii_len(text: &str) -> usize {
    text.bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .count()
}

fn decompress_bzip2(raw: &[u8]) -> Result<Vec<u8>, XgwxError> {
    let mut decoder = BzDecoder::new(raw);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(XgwxError::Bzip2)?;
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
