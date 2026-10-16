//! Captured native label rows shared by XGK and IEC ladder programs.
use crate::{LadderProgramData, ladder_records::string_at};

/// A native LD jump-target label in a dedicated row.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct LadderLabel {
    /// Byte offset of the `FF 41` record in the decoded program payload.
    pub offset: usize,
    /// Native vertical coordinate (four units per row).
    pub raw_y: u32,
    /// Case-sensitive label name.
    pub name: String,
}

/// Create, rename, or remove a label. `expected` is absent for insertion;
/// `replacement` is absent for deletion. Existing names must match exactly.
#[cfg(feature = "write")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderLabelEdit {
    /// Native vertical coordinate (four units per row).
    pub raw_y: u32,
    /// Expected case-sensitive name, or `None` when inserting.
    pub expected: Option<String>,
    /// New name, or `None` when deleting.
    pub replacement: Option<String>,
}

impl LadderProgramData {
    /// Decode captured dedicated label rows in either XGK or IEC LD.
    pub fn labels(&self) -> Vec<LadderLabel> {
        if self.version.as_deref() != Some("LD VER 1.1")
            || !matches!(self.project_type, Some(1 | 2))
        {
            return Vec::new();
        }
        let mut labels = Vec::new();
        for offset in 35..self.data.len().saturating_sub(18) {
            let b = &self.data[offset..];
            if b[..6] != [255, 0x41, 0, 0, 0, 1] || b[9..15] != [0, 0, 32, 0, 0, 0] {
                continue;
            }
            let raw_y = u32::from_le_bytes([b[6], b[7], b[8], 0]);
            let row = &self.data[offset - 35..offset];
            if !raw_y.is_multiple_of(4)
                || row[..4] != (raw_y / 4).to_le_bytes()
                || row[4..17] != [255, 67, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0]
                || row[21..25] != [94, b[6], b[7], b[8]]
                || row[29..35] != [1, b[6], b[7], b[8], 1, 0]
            {
                continue;
            }
            if let Ok((name, _)) = string_at(&self.data, offset + 15) {
                labels.push(LadderLabel {
                    offset,
                    raw_y,
                    name,
                });
            }
        }
        labels
    }
}

#[cfg(feature = "write")]
pub(crate) fn validate_name(name: &str) -> Result<(), crate::XgwxError> {
    let mut chars = name.chars();
    if !chars.next().is_some_and(char::is_alphabetic)
        || !chars.all(|c| c.is_alphanumeric() || c == '_')
        || name
            .chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>()
            > 16
    {
        return Err(crate::XgwxError::InvalidLadderEdit {
            reason: "label must start with a letter and contain letters, digits or underscores (16 ASCII or 8 non-ASCII characters maximum)",
        });
    }
    Ok(())
}

#[cfg(feature = "write")]
pub(crate) fn record(raw_y: u32, name: &str) -> Vec<u8> {
    let y = raw_y.to_le_bytes();
    let mut out = vec![
        255,
        0x41,
        0,
        0,
        0,
        1,
        y[0],
        y[1],
        y[2],
        0,
        0,
        32,
        0,
        0,
        0,
        255,
        254,
        255,
        name.encode_utf16().count() as u8,
    ];
    for unit in name.encode_utf16() {
        out.extend(unit.to_le_bytes());
    }
    out
}
