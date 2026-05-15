use super::*;

use base64::Engine;
use bzip2::read::BzDecoder;
use std::io::Read;

pub(crate) fn decode_base64_payload(text: &str, compressed: bool) -> Result<Vec<u8>, XgwxError> {
    let (_, decoded) = decode_base64_payload_with_raw(text, compressed)?;
    Ok(decoded)
}

pub(crate) fn decode_base64_payload_with_raw(
    text: &str,
    compressed: bool,
) -> Result<(Vec<u8>, Vec<u8>), XgwxError> {
    let encoded = text.split_whitespace().collect::<String>();
    if encoded.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(XgwxError::Base64)?;

    if !compressed {
        return Ok((decoded.clone(), decoded));
    }

    let raw = decoded;
    let mut decoder = BzDecoder::new(raw.as_slice());
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(XgwxError::Bzip2)?;
    Ok((raw, decompressed))
}

pub(crate) fn decode_hex_ascii_payload(
    text: &str,
    element: &str,
    attribute: &str,
) -> Result<Vec<u8>, XgwxError> {
    let encoded = text.split_whitespace().collect::<String>();
    if encoded.len() % 2 != 0 {
        return Err(XgwxError::InvalidHexPayload {
            element: element.to_owned(),
            attribute: attribute.to_owned(),
        });
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    let mut chars = encoded.chars();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let Some(high) = high.to_digit(16) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        let Some(low) = low.to_digit(16) else {
            return Err(XgwxError::InvalidHexPayload {
                element: element.to_owned(),
                attribute: attribute.to_owned(),
            });
        };
        bytes.push(((high << 4) | low) as u8);
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
    let encoded = element.text.split_whitespace().collect::<String>();
    let (raw, data) = decode_base64_payload_with_raw(&element.text, compressed)?;

    Ok(DecodedPayloadSummary {
        path: path.join("/"),
        tag: element.name.clone(),
        compressed,
        encoded_len: encoded.len(),
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
    let encoded = text.split_whitespace().collect::<String>();
    !encoded.is_empty()
        && encoded.len() % 4 == 0
        && encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}
