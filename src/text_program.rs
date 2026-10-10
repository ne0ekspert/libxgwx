//! Standalone XGI ST and IEC IL use the same native UTF-16 CodeList container.
#[cfg(feature = "write")]
use crate::XgwxError;
use crate::{XgwxDocument, XmlElement};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct TextProgram {
    pub program_index: usize,
    pub object_id: String,
    pub language: String,
    pub source: Option<String>,
    pub editable: bool,
    pub reason: Option<String>,
    pub variables: Vec<crate::SfcVariable>,
    pub variables_error: Option<String>,
}

fn source(body: &XmlElement) -> Option<String> {
    let st = body.children.iter().find(|n| n.name == "ST_Program")?;
    let code = st.children.iter().find(|n| n.name == "CodeList")?;
    let count = code.attribute("CodeCount")?.parse::<usize>().ok()?;
    if count > 65536 {
        return None;
    }
    let raw = if code.text.trim().is_empty() {
        Vec::new()
    } else {
        crate::decode_base64_payload(&code.text, code.attribute("Compressed") == Some("1"))
            .ok()?
            .data
    };
    if raw.len() != count * 2 {
        return None;
    }
    String::from_utf16(
        &raw.chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .ok()
}

impl XgwxDocument {
    /// Read standalone source. Unknown layouts remain visible but cannot be written.
    pub fn text_programs(&self) -> Vec<TextProgram> {
        let configurations = self.configurations();
        let supported = configurations.len() == 1
            && configurations[0]
                .type_code
                .and_then(crate::cpu::cpu_for_type)
                .is_some_and(|_| self.program_languages().contains(&"ST"));
        self.root.descendants_named("Program").enumerate().filter_map(|(program_index, program)| {
            let language = match program.attribute("Kind") { Some("4") => "ST", Some("9") => "IL", _ => return None };
            let body = program.children.iter().find(|n| n.name == "Body");
            let source = body.and_then(source);
            let editable = supported && self.program_languages().contains(&language) && program.attribute("Encrytption").unwrap_or("").is_empty()
                && program.attribute("ObjectId").is_some_and(|id| !id.is_empty())
                && body.is_some_and(|b| b.children.len() == 1 && crate::sfc::read_st_source(b).is_some() && ["CodeList", "Bookmarks", "Breakpoints"].iter().all(|name| b.children[0].children.iter().filter(|n| n.name == *name).count() == 1));
            let (variables, variables_error) = match self.sfc_variables(program_index) { Ok(v) => (v, None), Err(e) => (Vec::new(), Some(e.to_string())) };
            Some(TextProgram { variables, variables_error, program_index, object_id: program.attribute("ObjectId").unwrap_or("").into(), language: language.into(), source, editable,
                reason: (!editable).then(|| "This source layout, encryption, or bookmark/breakpoint metadata is not supported for editing.".into()) })
        }).collect()
    }
}

#[cfg(feature = "write")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct TextProgramPatch {
    pub program_index: usize,
    pub expected_object_id: String,
    pub expected_language: String,
    pub expected_source: String,
    pub source: String,
}

#[cfg(feature = "write")]
impl XgwxDocument {
    /// Replace only the native source payload, with identity and stale-source checks.
    pub fn edit_text_program(&mut self, patch: &TextProgramPatch) -> Result<(), XgwxError> {
        use base64::Engine;
        use std::io::Write;
        let fail = |reason: &str| XgwxError::TextProgramEdit(reason.into());
        let program = self
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == patch.program_index)
            .ok_or_else(|| fail("select a standalone ST or IL program"))?;
        if !program.editable {
            return Err(fail(program.reason.as_deref().unwrap()));
        }
        if patch.expected_object_id.is_empty()
            || program.object_id != patch.expected_object_id
            || program.language != patch.expected_language
            || program.source.as_deref() != Some(&patch.expected_source)
        {
            return Err(fail(
                "the program identity, language or source changed; reload before applying",
            ));
        }
        let bytes = patch
            .source
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        if bytes.len() > 65536 * 2 || patch.source.contains('\0') {
            return Err(fail(
                "source must contain at most 65536 UTF-16 units and no NUL characters",
            ));
        }
        if patch.source == patch.expected_source {
            return Ok(());
        }
        let parsed = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let node = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(patch.program_index)
            .unwrap();
        let code = node
            .children()
            .find(|n| n.has_tag_name("Body"))
            .unwrap()
            .descendants()
            .find(|n| n.has_tag_name("CodeList"))
            .unwrap();
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        encoder.write_all(&bytes).map_err(XgwxError::Io)?;
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(encoder.finish().map_err(XgwxError::Io)?);
        let xml = format!(
            "<CodeList CodeCount=\"{}\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\" Compressed=\"1\">{encoded}</CodeList>",
            bytes.len() / 2
        );
        let mut candidate = self.clone();
        candidate.apply_xml_replacements(vec![(code.range(), xml)])?;
        let actual = candidate
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == patch.program_index)
            .unwrap();
        if actual.source.as_deref() != Some(&patch.source) {
            return Err(fail("source did not round trip"));
        }
        candidate.to_verified_bytes()?;
        *self = candidate;
        Ok(())
    }
}

#[cfg(feature = "write")]
impl XgwxDocument {
    /// Edit regular source-program declarations using the captured IEC PB50 encoder.
    pub fn edit_text_variable(
        &mut self,
        expected_object_id: &str,
        patch: &crate::SfcVariablePatch,
    ) -> Result<(), XgwxError> {
        let text = self
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == patch.program_index)
            .ok_or_else(|| {
                XgwxError::TextProgramEdit("select a standalone ST or IL program".into())
            })?;
        if !text.editable || expected_object_id.is_empty() || text.object_id != expected_object_id {
            return Err(XgwxError::TextProgramEdit(
                "program identity changed or layout is unsupported".into(),
            ));
        }
        self.edit_sfc_variable(patch)
            .map_err(|e| XgwxError::TextProgramEdit(e.to_string()))
    }
}
