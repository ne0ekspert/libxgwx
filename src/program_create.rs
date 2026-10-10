//! New scan programs built from captured native blank program records.
use crate::{XgwxDocument, XgwxError};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct NewProgram {
    pub name: String,
    pub language: String,
    pub object_id: String,
    pub symbol_id: String,
}

fn valid_guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

impl XgwxDocument {
    /// Move a program to a final list index without changing its task or contents.
    pub fn move_program(
        &mut self,
        from: usize,
        to: usize,
        expected_object_id: &str,
        expected_target_id: &str,
    ) -> Result<(), XgwxError> {
        let fail = |reason: &str| XgwxError::ProgramReorder(reason.into());
        let parsed = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let programs: Vec<_> = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .collect();
        let source = *programs
            .get(from)
            .ok_or(XgwxError::ProgramNotFound { index: from })?;
        let target = *programs
            .get(to)
            .ok_or(XgwxError::ProgramNotFound { index: to })?;
        if expected_object_id.is_empty()
            || expected_target_id.is_empty()
            || source.attribute("ObjectId") != Some(expected_object_id)
            || target.attribute("ObjectId") != Some(expected_target_id)
        {
            return Err(fail("the program list changed; drag the program again"));
        }
        let parent = source
            .parent()
            .filter(|n| n.has_tag_name("Programs"))
            .ok_or_else(|| fail("only top-level programs can be reordered"))?;
        if target.parent() != Some(parent)
            || programs[from.min(to)..=from.max(to)]
                .iter()
                .any(|n| n.parent() != Some(parent))
        {
            return Err(fail("programs must belong to the same Programs container"));
        }
        if from == to {
            return Ok(());
        }
        let mut reordered = programs.clone();
        let moved = reordered.remove(from);
        reordered.insert(to, moved);
        let replacements = (from.min(to)..=from.max(to))
            .map(|index| {
                (
                    programs[index].range(),
                    self.xml[reordered[index].range()].to_owned(),
                )
            })
            .collect();
        self.apply_xml_replacements(replacements)
    }

    /// Remove the selected program, checking its identity before changing XML.
    pub fn delete_program(
        &mut self,
        program_index: usize,
        expected_object_id: &str,
    ) -> Result<(), XgwxError> {
        let fail = |reason: &str| XgwxError::ProgramDeletion(reason.into());
        let parsed = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;
        if expected_object_id.is_empty()
            || program.attribute("ObjectId") != Some(expected_object_id)
        {
            return Err(fail("the selected program changed; select it again"));
        }
        if !program.parent().is_some_and(|n| n.has_tag_name("Programs")) {
            return Err(fail("only top-level programs can be deleted"));
        }
        let project = parsed.root_element();
        let attribute = project
            .attributes()
            .find(|a| a.name() == "WksNodeCount")
            .ok_or_else(|| fail("missing workspace node count"))?;
        let nodes = if program.children().any(|n| n.has_tag_name("LocalVar")) {
            3
        } else {
            1
        };
        let count = attribute
            .value()
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_sub(nodes))
            .ok_or_else(|| fail("invalid workspace node count"))?;
        self.apply_xml_replacements(vec![
            (program.range(), String::new()),
            (attribute.range_value(), count.to_string()),
        ])
    }

    /// Append an empty scan program without copying existing application code.
    pub fn create_program(&mut self, patch: &NewProgram) -> Result<(), XgwxError> {
        let fail = |reason: &str| XgwxError::ProgramCreation(reason.into());
        let name = &patch.name;
        if name.is_empty()
            || !name
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || i > 0 && c.is_ascii_digit())
        {
            return Err(fail(
                "use a program name beginning with a letter or underscore, followed by letters, digits or underscores",
            ));
        }
        if !valid_guid(&patch.object_id)
            || !valid_guid(&patch.symbol_id)
            || patch.object_id.eq_ignore_ascii_case(&patch.symbol_id)
        {
            return Err(fail(
                "new program and symbol identities must be distinct GUIDs",
            ));
        }
        let parsed = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let configs: Vec<_> = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Configuration"))
            .collect();
        if configs.len() != 1 {
            return Err(fail("program creation requires one PLC configuration"));
        }
        let config = configs[0];
        let cpu = config
            .attribute("Type")
            .and_then(|t| t.parse().ok())
            .and_then(crate::cpu::cpu_for_type)
            .ok_or_else(|| fail("unknown CPU type"))?;
        if !self.program_languages().contains(&patch.language.as_str()) {
            return Err(fail("this program language is unvalidated for the selected CPU or project mode"));
        }
        let template = match (cpu.family, patch.language.as_str()) {
            ("XGK", "LD") if self.xgk_auto_allocation() => include_str!("program_templates/xgk-auto-ld.xml"),
            ("XGK", "LD" | "IL") => include_str!("program_templates/xgk-ld.xml"),
            ("XGI" | "XGK" | "XGB" | "XGR", "ST") => include_str!("program_templates/xgi-st.xml"),
            ("XGI" | "XGB" | "XGR", "IL") => include_str!("program_templates/xgi-il.xml"),
            ("XGI", "LD") => include_str!("program_templates/xgi-ld.xml"),
            ("XGI", "SFC") if [100, 102, 104, 106, 107, 111].contains(&cpu.type_code) => {
                include_str!("program_templates/xgi-sfc.xml")
            }
            _ => {
                return Err(fail(
                    "this program language is unsupported for the selected CPU",
                ));
            }
        };
        let lists: Vec<_> = config
            .descendants()
            .filter(|n| n.has_tag_name("Programs"))
            .collect();
        if lists.len() != 1 {
            return Err(fail("missing or ambiguous Programs container"));
        }
        let list = lists[0];
        if list
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .any(|p| p.text().unwrap_or("").trim().eq_ignore_ascii_case(name))
        {
            return Err(fail("a program with this name already exists"));
        }
        for attribute in parsed.descendants().flat_map(|n| n.attributes()) {
            let value = attribute.value().replace('_', "-");
            if value.eq_ignore_ascii_case(&patch.object_id)
                || value.eq_ignore_ascii_case(&patch.symbol_id)
            {
                return Err(fail(
                    "new program identities already exist in the workspace",
                ));
            }
        }
        let task = config
            .descendants()
            .find(|n| n.has_tag_name("Task") && n.attribute("Type") == Some("0"))
            .and_then(|n| n.text())
            .ok_or_else(|| fail("missing scan task"))?;
        let native = roxmltree::Document::parse(template).map_err(XgwxError::Xml)?;
        let root = native.root_element();
        let object = root.attribute("ObjectId").unwrap();
        let symbol = root
            .descendants()
            .find(|n| n.has_tag_name("LocalVar"))
            .and_then(|n| n.attribute("SymDocGUID"));
        let mut program = template
            .replace(object, &patch.object_id)
            .replace("NewProgram", name)
            .replace(
                "스캔 프로그램",
                &task
                    .replace('&', "&amp;")
                    .replace('\"', "&quot;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            );
        if let Some(symbol) = symbol {
            program = program.replace(symbol, &patch.symbol_id.replace('-', "_"));
        }
        let project = parsed.root_element();
        let count_attribute = project
            .attributes()
            .find(|a| a.name() == "WksNodeCount")
            .ok_or_else(|| fail("missing workspace node count"))?;
        let count = count_attribute
            .value()
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_add(if cpu.family == "XGK" { 1 } else { 3 }))
            .ok_or_else(|| fail("invalid workspace node count"))?;
        let end = list.range().end;
        let closing = self.xml[list.range()]
            .rfind("</Programs>")
            .ok_or_else(|| fail("unsupported empty Programs encoding"))?
            + list.range().start;
        debug_assert!(closing < end);
        self.apply_xml_replacements(vec![
            (closing..closing, program),
            (count_attribute.range_value(), count.to_string()),
        ])
    }
}
