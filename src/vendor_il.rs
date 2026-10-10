//! XGK IL is a textual projection of native ladder records, not IEC CodeList.
use crate::{XgwxDocument, XgwxError};
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct VendorIlProgram {
    pub program_index: usize,
    pub object_id: String,
    pub source: Option<String>,
    pub editable: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct VendorIlPatch {
    pub program_index: usize,
    pub expected_object_id: String,
    pub expected_source: String,
    pub source: String,
}
fn fail(reason: impl Into<String>) -> XgwxError {
    XgwxError::TextProgramEdit(reason.into())
}

impl XgwxDocument {
    pub fn vendor_il_programs(&self) -> Vec<VendorIlProgram> {
        let configs = self.configurations();
        if configs.len() != 1
            || !configs[0]
                .type_code
                .and_then(crate::cpu::cpu_for_type)
                .is_some_and(|c| c.family == "XGK")
        {
            return vec![];
        }
        self.ladder_programs().into_iter().enumerate().filter_map(|(index,result)| {
            let ladder=result.ok()?;
            if ladder.project_type!=Some(1) {return None;}
            let program=self.programs().into_iter().nth(index)?;
            let source=ladder.to_il().ok().map(|p|p.to_string());
            let reason=if self.xgk_auto_allocation() { Some("XGK Auto-allocation ladder IL is not yet validated for textual replacement.".into()) }
            else {source.as_deref().map_or_else(||Some("This ladder topology cannot be exported as IL.".into()),|s| compile(self,s,false).err().map(|e|e.to_string()))};
            Some(VendorIlProgram{program_index:index,object_id:program.object_id.unwrap_or_default(),editable:reason.is_none(),reason,source})
        }).collect()
    }
    pub fn edit_vendor_il(&mut self, patch: &VendorIlPatch) -> Result<(), XgwxError> {
        let before = self
            .vendor_il_programs()
            .into_iter()
            .find(|p| p.program_index == patch.program_index)
            .ok_or_else(|| fail("select an XGK ladder IL program"))?;
        if !before.editable {
            return Err(fail(before.reason.unwrap_or_default()));
        }
        if patch.expected_object_id.is_empty()
            || before.object_id != patch.expected_object_id
            || before.source.as_deref() != Some(&patch.expected_source)
        {
            return Err(fail(
                "the IL program identity or source changed; reload before applying",
            ));
        }
        if patch.source == patch.expected_source {
            return Ok(());
        }
        let payload = compile(self, &patch.source, true)?;
        let mut candidate = self.clone();
        candidate.edit_ladder_payload(patch.program_index, |_| Ok(payload))?;
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}

fn words(source: &str) -> Result<Vec<String>, XgwxError> {
    let mut result = vec![];
    let mut token = String::new();
    let mut quoted = false;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            token.push(c);
            if quoted && chars.peek() == Some(&'\'') {
                token.push(chars.next().unwrap());
            } else {
                quoted = !quoted;
            }
        } else if !quoted && (c.is_whitespace() || c == ',') {
            if !token.is_empty() {
                result.push(std::mem::take(&mut token));
            }
        } else {
            token.push(c);
        }
    }
    if quoted {
        return Err(fail("unclosed IL string literal"));
    }
    if !token.is_empty() {
        result.push(token);
    }
    Ok(result)
}
fn compile(doc: &XgwxDocument, source: &str, build: bool) -> Result<Vec<u8>, XgwxError> {
    use crate::{LadderCellEdit, LadderEditElement, LadderEditKind as K};
    if source.encode_utf16().count() > 65536 || source.contains('\0') {
        return Err(fail(
            "IL source must contain at most 65536 UTF-16 units and no NUL characters",
        ));
    }
    let mut bytes = vec![0; 8];
    let mut row = 0u32;
    let mut column = 0u8;
    let mut active = false;
    let mut completed = false;
    for (index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = words(line)?;
        let mut op = parts.remove(0).to_ascii_uppercase();
        if ["LOAD", "AND", "OUT"].contains(&op.as_str())
            && parts.first().is_some_and(|s| s.eq_ignore_ascii_case("NOT"))
        {
            parts.remove(0);
            op.push_str(" NOT");
        }
        let first = matches!(op.as_str(), "LOAD" | "LOAD NOT" | "LOADP" | "LOADN");
        if first {
            if active {
                return Err(fail(format!(
                    "line {}: previous expression has no output",
                    index + 1
                )));
            }
            if completed {
                row = row
                    .checked_add(4)
                    .ok_or_else(|| fail("too many IL networks"))?;
                if build {
                    bytes = crate::ladder_write::insert_ladder_row(&bytes, row)?;
                }
            }
            column = 0;
            active = true;
            completed = false;
        }
        let contact = match op.as_str() {
            "LOAD" | "AND" => Some(K::NormallyOpen),
            "LOAD NOT" | "AND NOT" => Some(K::NormallyClosed),
            "LOADP" | "ANDP" => Some(K::AddressedRisingPulse),
            "LOADN" | "ANDN" => Some(K::AddressedFallingPulse),
            _ => None,
        };
        let output = match op.as_str() {
            "OUT" => Some(K::Output),
            "OUT NOT" => Some(K::InverseOutput),
            "SET" => Some(K::Set),
            "RST" => Some(K::Reset),
            "OUTP" => Some(K::RisingPulseOutput),
            "OUTN" => Some(K::FallingPulseOutput),
            _ => None,
        };
        if let Some(kind) = contact.or(output) {
            if !active || parts.len() != 1 || contact.is_some() && column >= 9 {
                return Err(fail(format!(
                    "line {}: expected a LOAD expression, one operand and at most nine series contacts",
                    index + 1
                )));
            }
            let encoded = crate::ladder_write::edit_ladder_cell(
                if build { &bytes } else { &[0; 8] },
                &LadderCellEdit {
                    raw_y: if build { row } else { 0 },
                    column: if output.is_some() { 9 } else { column },
                    expected: None,
                    replacement: Some(LadderEditElement {
                        kind,
                        operand: parts[0].clone(),
                    }),
                },
            )?;
            if build {
                bytes = encoded;
            }
            if output.is_some() {
                active = false;
                completed = true;
            } else {
                column += 1;
            }
        } else {
            if matches!(
                op.as_str(),
                "OR" | "OR NOT" | "ORP" | "ORN" | "AND LOAD" | "OR LOAD"
            ) {
                return Err(fail(format!(
                    "line {}: branch IL remains read only; use the ladder editor",
                    index + 1
                )));
            }
            if !active {
                return Err(fail(format!(
                    "line {}: expected a LOAD expression before an application instruction",
                    index + 1
                )));
            }
            doc.validate_ladder_instruction_cpu(&op)?;
            let encoded = crate::ladder_write::insert_ladder_instruction(
                if build { &bytes } else { &[0; 8] },
                if build { row } else { 0 },
                &op,
                &parts,
            )?;
            if build {
                bytes = encoded;
            }
            active = false;
            completed = true;
        }
    }
    if active {
        return Err(fail("the final IL expression has no output"));
    }
    Ok(bytes)
}
