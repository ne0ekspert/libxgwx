#![deny(missing_docs)]
//! Selected PLC tasks and their program membership.
use crate::{ProgramRef, TaskSummary, XgwxDocument, XgwxError};

#[cfg(any(feature = "ffi", feature = "wasm", test))]
mod inspection;

/// Borrowed tasks in document order, including unknown native task types.
#[derive(Debug, Clone, Copy)]
pub struct TaskView<'a> {
    doc: &'a XgwxDocument,
}
/// A task selected by document order rather than its editable name.
#[derive(Debug, Clone, Copy)]
pub struct TaskRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}
/// Captured task definition together with its owning PLC's native identity.
/// Use this snapshot for edits originating outside a live selected Rust handle.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(any(feature = "ffi", feature = "wasm"), serde(deny_unknown_fields))]
pub struct TaskSnapshot {
    /// Native GUID of the PLC configuration; must be nonempty and unique.
    pub configuration_guid: String,
    /// Complete native task definition, including unknown attributes.
    pub definition: TaskSummary,
}
impl XgwxDocument {
    /// Inspect task definitions and programs assigned to each task.
    pub fn task(&self) -> TaskView<'_> {
        TaskView { doc: self }
    }
}
impl<'a> TaskView<'a> {
    /// All task summaries, preserving unknown attributes.
    pub fn list(self) -> Vec<TaskSummary> {
        self.doc.tasks()
    }
    /// Select a task by its document-wide index.
    pub fn get(self, index: usize) -> Result<TaskRef<'a>, XgwxError> {
        self.doc
            .tasks()
            .get(index)
            .ok_or_else(|| task_error("task index not found"))?;
        Ok(TaskRef {
            doc: self.doc,
            index,
        })
    }
}
fn task_error(reason: &str) -> XgwxError {
    XgwxError::TaskEdit(reason.into())
}
impl<'a> TaskRef<'a> {
    /// Bind an index already validated by the document ownership traversal.
    pub(crate) fn from_index(doc: &'a XgwxDocument, index: usize) -> Self {
        Self { doc, index }
    }

    /// Capture the complete definition and unique PLC identity for stale-edit checks.
    pub fn snapshot(self) -> Result<TaskSnapshot, XgwxError> {
        let configs = self.doc.configurations();
        let owner = self
            .configuration_index()
            .and_then(|i| configs.get(i))
            .ok_or_else(|| task_error("task has no owning PLC"))?;
        let guid = owner
            .guid
            .as_ref()
            .filter(|g| !g.is_empty())
            .ok_or_else(|| task_error("task's PLC has no native GUID"))?;
        if configs
            .iter()
            .filter(|c| {
                c.guid
                    .as_ref()
                    .is_some_and(|g| g.eq_ignore_ascii_case(guid))
            })
            .count()
            != 1
        {
            return Err(task_error("task's PLC GUID is ambiguous"));
        }
        Ok(TaskSnapshot {
            configuration_guid: guid.clone(),
            definition: self.summary(),
        })
    }
    /// Document-wide task index, distinct from native `TaskIndex`.
    pub fn index(self) -> usize {
        self.index
    }
    /// Native metadata including all raw scheduling fields.
    pub fn summary(self) -> TaskSummary {
        self.doc.tasks()[self.index].clone()
    }
    /// Owning PLC configuration, if the task is inside a configuration.
    pub fn configuration_index(self) -> Option<usize> {
        self.doc
            .configuration_record_owners("Task")
            .get(self.index)
            .copied()
            .flatten()
    }
    /// Programs attached to this task, in their execution/list order.
    /// Ambiguous duplicate task names are rejected instead of conflating membership.
    pub fn programs(self) -> Result<Vec<ProgramRef<'a>>, XgwxError> {
        let xml = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
        let node = selected_task(&xml, self.index)?;
        let owner = task_owner(node)?;
        let name = task_name(node)?;
        unique_task(owner, name)?;
        xml.descendants()
            .filter(|n| n.has_tag_name("Program"))
            .enumerate()
            .filter(|(_, p)| {
                p.ancestors().find(|n| n.has_tag_name("Configuration")) == Some(owner)
                    && p.attribute("Task") == Some(name)
            })
            .map(|(i, _)| self.doc.program().get(i))
            .collect()
    }
}
fn selected_task<'a, 'i>(
    xml: &'a roxmltree::Document<'i>,
    index: usize,
) -> Result<roxmltree::Node<'a, 'i>, XgwxError> {
    xml.descendants()
        .filter(|n| n.has_tag_name("Task"))
        .nth(index)
        .ok_or_else(|| task_error("task index not found"))
}
fn task_owner<'a, 'i>(task: roxmltree::Node<'a, 'i>) -> Result<roxmltree::Node<'a, 'i>, XgwxError> {
    task.parent()
        .filter(|n| n.has_tag_name("Tasks"))
        .and_then(|n| n.parent())
        .filter(|n| n.has_tag_name("Configuration"))
        .ok_or_else(|| task_error("task must belong directly to a PLC Tasks container"))
}
fn task_name<'a, 'i>(task: roxmltree::Node<'a, 'i>) -> Result<&'a str, XgwxError> {
    task.text()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| task_error("missing task name"))
}
fn unique_task<'a, 'i>(
    owner: roxmltree::Node<'a, 'i>,
    name: &str,
) -> Result<roxmltree::Node<'a, 'i>, XgwxError> {
    let mut found = owner
        .children()
        .filter(|n| n.has_tag_name("Tasks"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("Task") && n.text() == Some(name));
    let task = found
        .next()
        .ok_or_else(|| task_error("target task does not exist in this PLC"))?;
    if found.next().is_some() {
        return Err(task_error("task name is ambiguous in this PLC"));
    }
    Ok(task)
}

#[cfg(feature = "write")]
mod editors {
    use super::*;
    /// Exclusive collection of task definitions.
    #[derive(Debug)]
    pub struct TaskCollectionEditor<'a> {
        pub(super) doc: &'a mut XgwxDocument,
    }
    /// Selected task with a captured definition used to reject stale edits.
    #[derive(Debug)]
    pub struct Task<'a> {
        pub(super) doc: &'a mut XgwxDocument,
        pub(super) index: usize,
        pub(super) expected: TaskSnapshot,
    }
    impl XgwxDocument {
        /// Exclusively borrow PLC task definitions for guarded edits.
        pub fn task_mut(&mut self) -> TaskCollectionEditor<'_> {
            TaskCollectionEditor { doc: self }
        }
    }
    impl<'a> TaskCollectionEditor<'a> {
        /// Append a native user task to a supported PLC and maintain workspace counts.
        pub fn create(
            &mut self,
            configuration_index: usize,
            settings: &TaskSettings,
        ) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            super::write::create(self.doc, configuration_index, settings)
        }
        /// Select the current definition for editing.
        pub fn get(self, index: usize) -> Result<Task<'a>, XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            let expected = self.doc.task().get(index)?.snapshot()?;
            Ok(Task {
                doc: self.doc,
                index,
                expected,
            })
        }
        /// Select only when the complete previously inspected definition still matches.
        pub fn get_if(self, index: usize, expected: &TaskSnapshot) -> Result<Task<'a>, XgwxError> {
            let selected = self.get(index)?;
            if &selected.expected != expected {
                return Err(task_error("task definition changed; inspect it again"));
            }
            Ok(selected)
        }
    }
    impl Task<'_> {
        /// Create an empty program directly in this task using the native
        /// language templates. Unsupported multi-PLC creation leaves the document unchanged.
        pub fn create_program(&mut self, program: &crate::NewProgram) -> Result<(), XgwxError> {
            self.ensure_current()?;
            let mut candidate = self.doc.clone();
            let index = candidate.program().list().len();
            candidate.program_mut().create(program)?;
            let summary = candidate.program().get(index)?.summary();
            let scan = summary
                .task
                .as_deref()
                .ok_or_else(|| task_error("new program has no scan assignment"))?;
            super::write::assign(&mut candidate, self.index, index, &program.object_id, scan)?;
            *self.doc = candidate;
            Ok(())
        }
        /// Replace supported scheduling fields and atomically rename program references.
        /// Unknown native attributes and program bodies are preserved.
        pub fn update(&mut self, settings: &TaskSettings) -> Result<(), XgwxError> {
            self.ensure_current()?;
            super::write::update(self.doc, self.index, settings)?;
            self.expected = self.view().snapshot()?;
            Ok(())
        }
        /// Inspect the selected task and its program membership.
        pub fn view(&self) -> TaskRef<'_> {
            TaskRef {
                doc: self.doc,
                index: self.index,
            }
        }
        /// Rename this task and all attached program references atomically.
        /// The built-in scan task cannot be renamed.
        pub fn rename(&mut self, name: &str) -> Result<(), XgwxError> {
            self.ensure_current()?;
            super::write::rename(self.doc, self.index, name)?;
            self.expected = self.view().snapshot()?;
            Ok(())
        }
        /// Delete an empty user task. The built-in scan task is protected.
        /// Reassign or delete its programs explicitly before deleting a task.
        pub fn delete(self) -> Result<(), XgwxError> {
            self.ensure_current()?;
            super::write::delete(self.doc, self.index)
        }
        /// Assign an identified program from the same PLC to this task.
        /// Both its native ObjectId and previous task name must still match.
        pub fn assign_program(
            &mut self,
            program_index: usize,
            expected_object_id: &str,
            expected_task: &str,
        ) -> Result<(), XgwxError> {
            self.ensure_current()?;
            super::write::assign(
                self.doc,
                self.index,
                program_index,
                expected_object_id,
                expected_task,
            )
        }
        fn ensure_current(&self) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            if self.doc.task().get(self.index)?.snapshot()? != self.expected {
                return Err(task_error("task definition changed; inspect it again"));
            }
            Ok(())
        }
    }
}
#[cfg(feature = "write")]
pub use editors::{Task, TaskCollectionEditor};

#[cfg(feature = "write")]
mod write {
    use super::*;
    use std::ops::Range;
    pub(super) fn escaped(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
    fn editable(task: roxmltree::Node<'_, '_>) -> Result<(), XgwxError> {
        if task.attribute("Kind") == Some("0") {
            return Err(task_error(
                "the built-in scan task cannot be changed or deleted",
            ));
        }
        if task.attribute("Version") != Some("257")
            || task.attribute("Type") != Some("0")
            || task.attribute("Attribute") != Some("2")
            || !matches!(task.attribute("Kind"), Some("1" | "2" | "3"))
            || task.children().any(|n| n.is_element())
        {
            return Err(task_error("unsupported native task layout"));
        }
        Ok(())
    }
    fn validate_settings(
        owner: roxmltree::Node<'_, '_>,
        selected: Option<roxmltree::Node<'_, '_>>,
        settings: &TaskSettings,
    ) -> Result<Vec<(&'static str, String)>, XgwxError> {
        valid_name(&settings.name)?;
        distinct_name(owner, selected, &settings.name)?;
        if !(2..=7).contains(&settings.priority) {
            return Err(task_error("user task priority must be 2 through 7"));
        }
        let mut kind = 1;
        let mut device = String::new();
        let mut device_type = 0;
        let mut word_value = 0;
        let mut word_condition = 0;
        let mut bit_condition = 0;
        match &settings.trigger {
            TaskTrigger::Initialization => {
                if settings.task_number > 95 {
                    return Err(task_error(
                        "initialization task's preserved number must be 0 through 95",
                    ));
                }
                if owner
                    .children()
                    .filter(|n| n.has_tag_name("Tasks"))
                    .flat_map(|n| n.children())
                    .any(|n| {
                        n.has_tag_name("Task")
                            && Some(n) != selected
                            && n.attribute("Kind") == Some("1")
                    })
                {
                    return Err(task_error("this PLC already has an initialization task"));
                }
            }
            TaskTrigger::Periodic { interval_ms } => {
                kind = 2;
                if *interval_ms == 0 {
                    return Err(task_error("periodic interval must be at least 1 ms"));
                }
                if settings.task_number > 31 {
                    return Err(task_error("periodic task number must be 0 through 31"));
                }
                device = interval_ms.to_string();
            }
            TaskTrigger::InternalBit {
                device: d,
                condition,
            } => {
                kind = 3;
                device = d.clone();
                bit_condition = *condition as u32;
            }
            TaskTrigger::InternalWord {
                device: d,
                condition,
                value,
            } => {
                kind = 3;
                device = d.clone();
                device_type = 1;
                word_condition = *condition as u32;
                word_value = *value as u32;
            }
        }
        if kind == 3 {
            if !(64..=95).contains(&settings.task_number) {
                return Err(task_error(
                    "internal-device task number must be 64 through 95",
                ));
            }
            validate_device(&device, device_type == 0)?;
            let iec = owner
                .attribute("Type")
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|n| n >= 100);
            if iec != device.starts_with('%') {
                return Err(task_error(
                    "use canonical %IX/%QX/%MX or %IW/%QW/%MW addresses for XGI tasks, and native device addresses for XGK tasks",
                ));
            }
        }
        if kind != 1
            && owner
                .children()
                .filter(|n| n.has_tag_name("Tasks"))
                .flat_map(|n| n.children())
                .any(|n| {
                    n.has_tag_name("Task")
                        && Some(n) != selected
                        && matches!(n.attribute("Kind"), Some("2" | "3"))
                        && n.attribute("TaskIndex").and_then(|v| v.parse::<u32>().ok())
                            == Some(settings.task_number)
                })
        {
            return Err(task_error("native task number already exists in this PLC"));
        }
        Ok(vec![
            ("Kind", kind.to_string()),
            ("Priority", settings.priority.to_string()),
            ("TaskIndex", settings.task_number.to_string()),
            ("Device", device),
            ("DeviceType", device_type.to_string()),
            ("WordValue", word_value.to_string()),
            ("WordCondition", word_condition.to_string()),
            ("BitCondition", bit_condition.to_string()),
        ])
    }
    fn validate_device(device: &str, bit: bool) -> Result<(), XgwxError> {
        // Task records use physical device expressions, not arbitrary source code.
        // CPU memory bounds and physical interrupt availability require native Check Program.
        let direct = if let Some(rest) = device.strip_prefix('%') {
            let bytes = rest.as_bytes();
            bytes.len() > 2
                && matches!(bytes[0], b'I' | b'Q' | b'M')
                && bytes[1] == if bit { b'X' } else { b'W' }
                && rest[2..].parse::<u32>().is_ok()
        } else if bit {
            crate::ladder_write::valid_bit_device_address(device)
        } else {
            let split = device
                .find(|c: char| c.is_ascii_digit())
                .unwrap_or(device.len());
            matches!(
                &device[..split],
                "P" | "M" | "K" | "F" | "L" | "T" | "C" | "D" | "R" | "ZR" | "U" | "N" | "Z"
            ) && device[split..].parse::<u32>().is_ok()
        };
        if !direct {
            return Err(task_error(
                "use a physical device address matching the bit/word trigger type",
            ));
        }
        Ok(())
    }
    fn supported(doc: &XgwxDocument, owner: roxmltree::Node<'_, '_>) -> Result<(), XgwxError> {
        let code = owner.attribute("Type").and_then(|v| v.parse::<u32>().ok());
        if !doc
            .configurations()
            .iter()
            .enumerate()
            .any(|(i, c)| c.type_code == code && doc.task().can_create(i))
        {
            return Err(task_error(
                "task scheduling writes are not captured for this CPU",
            ));
        }
        Ok(())
    }
    pub(super) fn create(
        doc: &mut XgwxDocument,
        configuration_index: usize,
        settings: &TaskSettings,
    ) -> Result<(), XgwxError> {
        let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
        let owner = xml
            .descendants()
            .filter(|n| n.has_tag_name("Configuration"))
            .nth(configuration_index)
            .ok_or_else(|| task_error("configuration index not found"))?;
        supported(doc, owner)?;
        let mut containers = owner.children().filter(|n| n.has_tag_name("Tasks"));
        let tasks = containers
            .next()
            .ok_or_else(|| task_error("missing Tasks container"))?;
        if containers.next().is_some()
            || tasks
                .children()
                .filter(|n| n.has_tag_name("Task") && n.attribute("Kind") == Some("0"))
                .count()
                != 1
        {
            return Err(task_error(
                "PLC must have exactly one Tasks container and scan task",
            ));
        }
        let fields = validate_settings(owner, None, settings)?;
        let mut record = String::from("<Task Version=\"257\" Type=\"0\" Attribute=\"2\"");
        for (name, value) in fields {
            record.push_str(&format!(" {name}=\"{}\"", escaped(&value)));
        }
        record.push_str(&format!(">{}</Task>", escaped(&settings.name)));
        let end = tasks.range().end - "</Tasks>".len();
        if &doc.xml[end..tasks.range().end] != "</Tasks>" {
            return Err(task_error("unsupported Tasks closing tag"));
        }
        let count = node_count(&xml, 1)?;
        doc.apply_xml_replacements(vec![(end..end, record), count])
    }
    pub(super) fn update(
        doc: &mut XgwxDocument,
        index: usize,
        settings: &TaskSettings,
    ) -> Result<(), XgwxError> {
        let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
        let task = selected_task(&xml, index)?;
        editable(task)?;
        let owner = task_owner(task)?;
        supported(doc, owner)?;
        let fields = validate_settings(owner, Some(task), settings)?;
        let mut replacements = Vec::new();
        for (name, value) in fields {
            let a = task
                .attributes()
                .find(|a| a.name() == name)
                .ok_or_else(|| task_error("missing native scheduling attribute"))?;
            replacements.push((a.range_value(), escaped(&value)));
        }
        // Apply both phases to a candidate so a rejected rename cannot leak schedule edits.
        let mut candidate = doc.clone();
        candidate.apply_xml_replacements(replacements)?;
        rename(&mut candidate, index, &settings.name)?;
        *doc = candidate;
        Ok(())
    }
    pub(super) fn valid_name(name: &str) -> Result<(), XgwxError> {
        if name.is_empty()
            || name.trim() != name
            || name.encode_utf16().count() > 32
            || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(task_error(
                "task names must contain 1 to 32 letters, digits or underscores",
            ));
        }
        Ok(())
    }
    fn distinct_name(
        owner: roxmltree::Node<'_, '_>,
        selected: Option<roxmltree::Node<'_, '_>>,
        name: &str,
    ) -> Result<(), XgwxError> {
        if owner
            .children()
            .filter(|n| n.has_tag_name("Tasks"))
            .flat_map(|n| n.children())
            .any(|n| {
                n.has_tag_name("Task")
                    && Some(n) != selected
                    && n.text()
                        .is_some_and(|s| s.to_lowercase() == name.to_lowercase())
            })
        {
            return Err(task_error("task name already exists in this PLC"));
        }
        Ok(())
    }
    pub(super) fn rename(
        doc: &mut XgwxDocument,
        index: usize,
        name: &str,
    ) -> Result<(), XgwxError> {
        valid_name(name)?;
        let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
        let task = selected_task(&xml, index)?;
        editable(task)?;
        let owner = task_owner(task)?;
        let old = task_name(task)?;
        unique_task(owner, old)?;
        distinct_name(owner, Some(task), name)?;
        if old == name {
            return Ok(());
        }
        let text = task
            .children()
            .find(|n| n.is_text())
            .ok_or_else(|| task_error("missing task name text"))?;
        let mut edits = vec![(text.range(), escaped(name))];
        for program in owner.descendants().filter(|n| {
            n.has_tag_name("Program")
                && n.attribute("Task") == Some(old)
                && n.ancestors().find(|n| n.has_tag_name("Configuration")) == Some(owner)
        }) {
            let attr = program.attributes().find(|a| a.name() == "Task").unwrap();
            edits.push((attr.range_value(), escaped(name)));
        }
        doc.apply_xml_replacements(edits)
    }
    fn node_count(
        xml: &roxmltree::Document<'_>,
        delta: i32,
    ) -> Result<(Range<usize>, String), XgwxError> {
        let attr = xml
            .root_element()
            .attributes()
            .find(|a| a.name() == "WksNodeCount")
            .ok_or_else(|| task_error("missing workspace node count"))?;
        let count = attr
            .value()
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_add_signed(delta))
            .ok_or_else(|| task_error("invalid workspace node count"))?;
        Ok((attr.range_value(), count.to_string()))
    }
    pub(super) fn delete(doc: &mut XgwxDocument, index: usize) -> Result<(), XgwxError> {
        if !doc.task().get(index)?.programs()?.is_empty() {
            return Err(task_error(
                "task still owns programs; reassign or delete them first",
            ));
        }
        let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
        let task = selected_task(&xml, index)?;
        editable(task)?;
        task_owner(task)?;
        let count = node_count(&xml, -1)?;
        doc.apply_xml_replacements(vec![(task.range(), String::new()), count])
    }
    pub(super) fn assign(
        doc: &mut XgwxDocument,
        task_index: usize,
        program_index: usize,
        expected_id: &str,
        expected_task: &str,
    ) -> Result<(), XgwxError> {
        let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
        let task = selected_task(&xml, task_index)?;
        let owner = task_owner(task)?;
        let name = task_name(task)?;
        unique_task(owner, name)?;
        let program = xml
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;
        if expected_id.is_empty()
            || program.attribute("ObjectId") != Some(expected_id)
            || program.attribute("Task") != Some(expected_task)
        {
            return Err(task_error(
                "program identity or task assignment changed; inspect it again",
            ));
        }
        if program
            .ancestors()
            .find(|n| n.has_tag_name("Configuration"))
            != Some(owner)
            || !program.parent().is_some_and(|n| n.has_tag_name("Programs"))
        {
            return Err(task_error("program and task must belong to the same PLC"));
        }
        let attr = program.attributes().find(|a| a.name() == "Task").unwrap();
        doc.apply_xml_replacements(vec![(attr.range_value(), escaped(name))])
    }
    impl XgwxDocument {
        pub(crate) fn validate_program_task(
            &self,
            program_index: usize,
            name: &str,
        ) -> Result<(), XgwxError> {
            let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
            let program = xml
                .descendants()
                .filter(|n| n.has_tag_name("Program"))
                .nth(program_index)
                .ok_or(XgwxError::ProgramNotFound {
                    index: program_index,
                })?;
            let owner = program
                .ancestors()
                .find(|n| n.has_tag_name("Configuration"))
                .ok_or_else(|| task_error("program has no owning PLC"))?;
            unique_task(owner, name)?;
            Ok(())
        }
    }
}

/// Native bit-device trigger condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    serde(rename_all = "snake_case")
)]
pub enum TaskBitCondition {
    /// False to true.
    Rising,
    /// True to false.
    Falling,
    /// Either edge.
    Changed,
    /// Device is true.
    On,
    /// Device is false.
    Off,
}
/// Native unsigned word-device comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    serde(rename_all = "snake_case")
)]
pub enum TaskWordCondition {
    /// Device is less than the comparison value.
    Less,
    /// Device is less than or equal to the comparison value.
    LessOrEqual,
    /// Device equals the comparison value.
    Equal,
    /// Device is greater than or equal to the comparison value.
    GreaterOrEqual,
    /// Device is greater than the comparison value.
    Greater,
}
/// Execution condition of a user task. The built-in scan task is not user-created.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum TaskTrigger {
    /// Execute during STOP-to-RUN initialization.
    Initialization,
    /// Execute periodically. Zero milliseconds is rejected.
    Periodic {
        /// Period in milliseconds, 1 through u32::MAX.
        interval_ms: u32,
    },
    /// Execute on a native bit-device condition.
    InternalBit {
        /// Native device expression, using the selected CPU's addressing syntax.
        device: String,
        /// Edge or level condition.
        condition: TaskBitCondition,
    },
    /// Execute on a native unsigned word-device comparison.
    InternalWord {
        /// Native device expression, using the selected CPU's addressing syntax.
        device: String,
        /// Comparison operator.
        condition: TaskWordCondition,
        /// Unsigned 16-bit comparison constant.
        value: u16,
    },
}
/// Complete user-task settings used for creation or replacement.
/// Scheduling is independent of task list order and program list order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "ffi", feature = "wasm"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(any(feature = "ffi", feature = "wasm"), serde(deny_unknown_fields))]
pub struct TaskSettings {
    /// Unique task name within the owning PLC.
    pub name: String,
    /// Priority, 2 (highest user priority) through 7.
    pub priority: u32,
    /// Native task number: 0..31 periodic, 64..95 internal device.
    /// Ignored by native initialization tasks; preserved on update.
    pub task_number: u32,
    /// Condition under which this task runs.
    pub trigger: TaskTrigger,
}

impl TaskRef<'_> {
    /// Decode supported user-task scheduling fields. Scan and unknown layouts
    /// remain inspectable through [`Self::summary`].
    pub fn settings(self) -> Result<TaskSettings, XgwxError> {
        settings_from_summary(&self.summary())
    }
}
fn settings_from_summary(s: &TaskSummary) -> Result<TaskSettings, XgwxError> {
    let missing = || task_error("missing or unsupported task scheduling fields");
    let device = || s.device.clone().ok_or_else(missing);
    let bit = [
        TaskBitCondition::Rising,
        TaskBitCondition::Falling,
        TaskBitCondition::Changed,
        TaskBitCondition::On,
        TaskBitCondition::Off,
    ];
    let word = [
        TaskWordCondition::Less,
        TaskWordCondition::LessOrEqual,
        TaskWordCondition::Equal,
        TaskWordCondition::GreaterOrEqual,
        TaskWordCondition::Greater,
    ];
    let trigger = match (s.kind, s.device_type) {
        (Some(1), _) => TaskTrigger::Initialization,
        (Some(2), _) => TaskTrigger::Periodic {
            interval_ms: device()?.parse().map_err(|_| missing())?,
        },
        (Some(3), Some(0)) => TaskTrigger::InternalBit {
            device: device()?,
            condition: *bit
                .get(s.bit_condition.ok_or_else(missing)? as usize)
                .ok_or_else(missing)?,
        },
        (Some(3), Some(1)) => TaskTrigger::InternalWord {
            device: device()?,
            condition: *word
                .get(s.word_condition.ok_or_else(missing)? as usize)
                .ok_or_else(missing)?,
            value: s
                .word_value
                .and_then(|n| n.try_into().ok())
                .ok_or_else(missing)?,
        },
        _ => return Err(missing()),
    };
    Ok(TaskSettings {
        name: s.name.clone().ok_or_else(missing)?,
        priority: s.priority.ok_or_else(missing)?,
        task_number: s.task_index.ok_or_else(missing)?,
        trigger,
    })
}
impl TaskView<'_> {
    /// Whether this PLC uses the captured standard XGK/XGI user-task layout.
    /// Compact, redundant and unknown CPUs remain read-only for scheduling writes.
    pub fn can_create(self, configuration_index: usize) -> bool {
        self.doc
            .configurations()
            .get(configuration_index)
            .is_some_and(|c| task_creation_supported(c.type_code))
    }
}

pub(crate) fn task_creation_supported(type_code: Option<u32>) -> bool {
    matches!(
        type_code,
        Some(0 | 1 | 3 | 4 | 5 | 14 | 16 | 17 | 100 | 102 | 104 | 106 | 107 | 111)
    )
}
