//! Structural edits for the native SV5.0 global symbol table.
use crate::{VariableEditor, XgwxDocument, XgwxError, internal::decode_base64_payload};

/// A new scalar global device alias. Source references and array metadata are native defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct NewGlobalVariable {
    /// Unique identifier, without case distinctions.
    pub name: String,
    /// Native scalar type: BIT, BYTE, WORD, DWORD or LWORD.
    pub data_type: String,
    /// Direct device area: P, M, K, L or D.
    pub address_area: String,
    /// Native numeric device address (bit number for BIT, word number otherwise).
    pub address_number: u32,
    /// Description, at most 255 UTF-16 units without control characters.
    pub description: String,
}
fn invalid(message: &str) -> XgwxError {
    XgwxError::GlobalVariableEdit(message.into())
}
fn put(out: &mut Vec<u8>, text: &str) {
    out.extend([255, 254, 255, text.encode_utf16().count() as u8]);
    for unit in text.encode_utf16() {
        out.extend(unit.to_le_bytes());
    }
}
fn string(bytes: &[u8], offset: &mut usize) -> Result<String, XgwxError> {
    let header = bytes
        .get(*offset..*offset + 4)
        .ok_or_else(|| invalid("truncated global string"))?;
    if header[..3] != [255, 254, 255] {
        return Err(invalid("unsupported global string layout"));
    }
    *offset += 4;
    let end = *offset + usize::from(header[3]) * 2;
    let text = bytes
        .get(*offset..end)
        .ok_or_else(|| invalid("truncated global string"))?;
    *offset = end;
    String::from_utf16(
        &text
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .map_err(|_| invalid("invalid global UTF-16"))
}
fn skip_number(bytes: &[u8], offset: &mut usize) -> Result<(), XgwxError> {
    bytes
        .get(*offset..*offset + 4)
        .ok_or_else(|| invalid("truncated global number"))?;
    *offset += 4;
    Ok(())
}
impl VariableEditor<'_> {
    /// Insert a scalar global in name order. Returns its new native record index.
    /// Existing record bytes and all program payloads are preserved. Requires an XGK CPU
    /// and one complete SV5.0 table; unsupported layouts fail without changing the document.
    pub fn insert(&mut self, variable: &NewGlobalVariable) -> Result<usize, XgwxError> {
        self.doc.ensure_consistent_api_tree()?;
        if variable.name.is_empty()
            || variable.name == "SV5.0"
            || variable.description == "SV5.0"
            || !variable
                .name
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            || !variable
                .name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            || [&variable.name, &variable.description]
                .iter()
                .any(|s| s.encode_utf16().count() > 255 || s.chars().any(char::is_control))
        {
            return Err(invalid(
                "global name must be an identifier; name and description must fit 255 UTF-16 units without controls",
            ));
        }
        if !matches!(
            variable.data_type.as_str(),
            "BIT" | "BYTE" | "WORD" | "DWORD" | "LWORD"
        ) || !matches!(variable.address_area.as_str(), "P" | "M" | "K" | "L" | "D")
            || variable.address_area == "D" && matches!(variable.data_type.as_str(), "BIT" | "BYTE")
        {
            return Err(invalid(
                "unsupported scalar global type or direct device area",
            ));
        }
        // Keep the address inside the captured common device range, including multiword values.
        let width = match variable.data_type.as_str() {
            "DWORD" => 2,
            "LWORD" => 4,
            _ => 1,
        };
        let limit = if variable.data_type == "BIT" {
            32768 * 16
        } else {
            32768
        };
        if variable
            .address_number
            .checked_add(width)
            .is_none_or(|end| end > limit)
        {
            return Err(invalid("global address exceeds the supported device range"));
        }
        let vars = self.doc.variable().list()?;
        if vars.iter().any(|v| {
            v.name()
                .is_some_and(|n| n.to_lowercase() == variable.name.to_lowercase())
        }) {
            return Err(invalid("global variable name must be unique"));
        }
        let index = vars
            .iter()
            .position(|v| {
                v.name()
                    .is_some_and(|n| n.to_lowercase() > variable.name.to_lowercase())
            })
            .unwrap_or(vars.len());
        let mut raw = Vec::new();
        for s in ["SV5.0", &variable.name, &variable.address_area] {
            put(&mut raw, s);
        }
        raw.extend(variable.address_number.to_le_bytes());
        for s in [&variable.data_type, &variable.description, ""] {
            put(&mut raw, s);
        }
        raw.extend(0u32.to_le_bytes());
        put(&mut raw, "[0:0]");
        self.doc.splice_global(index, Some(raw))?;
        Ok(index)
    }
}
impl XgwxDocument {
    pub(crate) fn splice_global(
        &mut self,
        index: usize,
        record: Option<Vec<u8>>,
    ) -> Result<(), XgwxError> {
        self.ensure_consistent_api_tree()?;
        let before = self.variable().globals()?;
        let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let tables: Vec<_> = xml
            .descendants()
            .filter(|n| n.has_tag_name("Symbols"))
            .collect();
        let [table] = tables.as_slice() else {
            return Err(invalid("global table is absent or ambiguous"));
        };
        let configs: Vec<_> = xml
            .descendants()
            .filter(|n| n.has_tag_name("Configuration"))
            .collect();
        if configs.len() != 1
            || !crate::cpu_catalog().iter().any(|c| {
                c.family == "XGK"
                    && configs[0]
                        .attribute("Type")
                        .and_then(|t| t.parse::<u32>().ok())
                        == Some(c.type_code)
            })
        {
            return Err(invalid("global lifecycle requires an XGK CPU"));
        }
        if table.children().any(|n| !n.is_text()) {
            return Err(invalid("unsupported global table contents"));
        }
        let count = table
            .attribute_node("Count")
            .ok_or_else(|| invalid("missing global count"))?;
        if count.value().parse::<usize>().ok() != Some(before.len()) {
            return Err(invalid("global count does not match the decoded table"));
        }
        let original = table.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "True"));
        let bytes = decode_base64_payload(original, compressed)?.data;
        let mut offset = 0;
        let mut spans = Vec::new();
        for _ in 0..before.len() {
            let start = offset;
            if string(&bytes, &mut offset)? != "SV5.0" {
                return Err(invalid("unsupported global record version"));
            }
            string(&bytes, &mut offset)?;
            string(&bytes, &mut offset)?;
            skip_number(&bytes, &mut offset)?;
            for _ in 0..3 {
                string(&bytes, &mut offset)?;
            }
            skip_number(&bytes, &mut offset)?;
            string(&bytes, &mut offset)?;
            spans.push(start..offset);
        }
        if offset != bytes.len() {
            return Err(invalid("unsupported trailing global data"));
        }
        let mut updated = bytes.clone();
        let new_count;
        if let Some(record) = record {
            if index > spans.len() {
                return Err(XgwxError::VariableNotFound { index });
            }
            let at = spans.get(index).map_or(bytes.len(), |r| r.start);
            updated.splice(at..at, record);
            new_count = before.len() + 1;
        } else {
            let span = spans
                .get(index)
                .ok_or(XgwxError::VariableNotFound { index })?;
            updated.drain(span.clone());
            new_count = before.len() - 1;
        }
        let encoded = crate::writer::encode_payload_text(original, compressed, &updated)?;
        let mut edits = vec![(count.range_value(), new_count.to_string())];
        if let Some(text) = table.children().find(|n| n.is_text()) {
            edits.push((text.range(), encoded));
        } else {
            let raw = &self.xml[table.range()];
            let end = table.range().start
                + raw
                    .find('>')
                    .ok_or_else(|| invalid("invalid global table"))?;
            if raw.ends_with("/>") {
                edits.push((end - 1..end + 1, format!(">{encoded}</Symbols>")));
            } else {
                edits.push((end + 1..end + 1, encoded));
            }
        }
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(edits)?;
        let after = candidate.variable().globals()?;
        if after.len() != new_count
            || before.iter().enumerate().any(|(i, v)| {
                if new_count > before.len() {
                    after.get(if i >= index { i + 1 } else { i }) != Some(v)
                } else {
                    i != index && after.get(if i > index { i - 1 } else { i }) != Some(v)
                }
            })
        {
            return Err(XgwxError::RewriteVerificationFailed);
        }
        *self = candidate;
        Ok(())
    }
}
