//! Request-local task inspection; no per-task document queries or XML reparsing.
use super::*;
use std::collections::BTreeMap;

pub(crate) struct TaskInspection {
    pub(crate) index: usize,
    pub(crate) definition: TaskSummary,
    pub(crate) configuration_index: Option<usize>,
    pub(crate) expected: Option<TaskSnapshot>,
    pub(crate) settings: Option<TaskSettings>,
    pub(crate) program_indices: Result<Vec<usize>, String>,
}

impl TaskInspection {
    fn new(
        index: usize,
        definition: TaskSummary,
        configuration_index: Option<usize>,
        guid: Option<&str>,
        program_indices: Result<Vec<usize>, String>,
    ) -> Self {
        Self {
            index,
            settings: settings_from_summary(&definition).ok(),
            expected: guid.map(|guid| TaskSnapshot {
                configuration_guid: guid.to_owned(),
                definition: definition.clone(),
            }),
            definition,
            configuration_index,
            program_indices,
        }
    }
}

impl TaskView<'_> {
    #[cfg(any(feature = "wasm", test))]
    pub(crate) fn inspections(self) -> Vec<TaskInspection> {
        self.inspection_page(0, usize::MAX).1
    }

    pub(crate) fn inspection_page(
        self,
        offset: usize,
        limit: usize,
    ) -> (usize, Vec<TaskInspection>) {
        let definitions = self.list();
        let total = definitions.len();
        if offset >= total || limit == 0 {
            return (total, Vec::new());
        }
        let configurations = self.doc.configurations();
        let mut guid_counts = BTreeMap::<String, usize>::new();
        for configuration in &configurations {
            if let Some(guid) = &configuration.guid {
                *guid_counts.entry(guid.to_ascii_lowercase()).or_default() += 1;
            }
        }
        let guids = configurations
            .iter()
            .map(|configuration| {
                configuration.guid.as_deref().filter(|guid| {
                    !guid.is_empty() && guid_counts.get(&guid.to_ascii_lowercase()) == Some(&1)
                })
            })
            .collect::<Vec<_>>();
        let xml = match roxmltree::Document::parse(&self.doc.xml) {
            Ok(xml) => xml,
            Err(error) => {
                let error = XgwxError::Xml(error).to_string();
                return (
                    total,
                    definitions
                        .into_iter()
                        .enumerate()
                        .skip(offset)
                        .take(limit)
                        .map(|(index, definition)| {
                            TaskInspection::new(index, definition, None, None, Err(error.clone()))
                        })
                        .collect(),
                );
            }
        };
        let configuration_indices = xml
            .descendants()
            .filter(|node| node.has_tag_name("Configuration"))
            .enumerate()
            .map(|(index, node)| (node.id().get(), index))
            .collect::<BTreeMap<_, _>>();
        let tasks = xml
            .descendants()
            .filter(|node| node.has_tag_name("Task"))
            .collect::<Vec<_>>();
        // Membership uses the first raw XML text node, whereas TaskSummary uses
        // concatenated, trimmed direct text. Keep those identities separate.
        let mut name_counts = BTreeMap::<(u32, &str), usize>::new();
        for &task in &tasks {
            if let Ok(owner) = task_owner(task)
                && let Some(name) = task.text()
            {
                *name_counts.entry((owner.id().get(), name)).or_default() += 1;
            }
        }
        let mut programs = BTreeMap::<(u32, &str), Vec<usize>>::new();
        for (index, program) in xml
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .enumerate()
        {
            if let Some(owner) = program
                .ancestors()
                .find(|node| node.has_tag_name("Configuration"))
                && let Some(name) = program.attribute("Task")
            {
                programs
                    .entry((owner.id().get(), name))
                    .or_default()
                    .push(index);
            }
        }
        let selected = definitions
            .into_iter()
            .enumerate()
            .skip(offset)
            .take(limit)
            .map(|(index, definition)| {
                let task = tasks.get(index).copied();
                let configuration_index = task
                    .and_then(|task| {
                        task.ancestors()
                            .find(|node| node.has_tag_name("Configuration"))
                    })
                    .and_then(|owner| configuration_indices.get(&owner.id().get()).copied());
                let guid =
                    configuration_index.and_then(|index| guids.get(index).copied().flatten());
                let members = (|| {
                    let task = task.ok_or_else(|| task_error("task index not found"))?;
                    let owner = task_owner(task)?;
                    let name = task_name(task)?;
                    let key = (owner.id().get(), name);
                    match name_counts.get(&key).copied().unwrap_or(0) {
                        0 => Err(task_error("target task does not exist in this PLC")),
                        1 => Ok(programs.get(&key).cloned().unwrap_or_default()),
                        _ => Err(task_error("task name is ambiguous in this PLC")),
                    }
                })()
                .map_err(|error| error.to_string());
                TaskInspection::new(index, definition, configuration_index, guid, members)
            })
            .collect();
        (total, selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn document(xml: &str) -> XgwxDocument {
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(xml.as_bytes()).unwrap();
        let mut bytes = b"XG".to_vec();
        bytes.extend(gzip.finish().unwrap());
        XgwxDocument::parse(&bytes).unwrap()
    }

    fn matches_selected_handles(doc: &XgwxDocument) {
        let inspected = doc.task().inspections();
        assert_eq!(inspected.len(), doc.task().list().len());
        for (index, inspection) in inspected.iter().enumerate() {
            assert_eq!(inspection.index, index);
            let selected = doc.task().get(index).unwrap();
            assert_eq!(inspection.definition, selected.summary());
            assert_eq!(
                inspection.configuration_index,
                selected.configuration_index()
            );
            assert_eq!(inspection.expected, selected.snapshot().ok());
            assert_eq!(inspection.settings, selected.settings().ok());
            assert_eq!(
                inspection.program_indices,
                selected
                    .programs()
                    .map(|programs| programs
                        .into_iter()
                        .map(|program| program.index())
                        .collect())
                    .map_err(|error| error.to_string()),
                "task {index}"
            );
        }
    }

    #[test]
    fn task_inspection_matches_native_membership_settings_and_snapshots() {
        for bytes in [
            include_bytes!("../../fixtures/tasks/xgk-native.xgwx").as_slice(),
            include_bytes!("../../fixtures/tasks/xgi-native.xgwx").as_slice(),
        ] {
            matches_selected_handles(&XgwxDocument::parse(bytes).unwrap());
        }
    }

    #[test]
    fn task_inspection_keeps_raw_names_ownership_guid_ambiguity_and_error_precedence() {
        for xml in [
            "<Project/>",
            "<Project><Task>orphan</Task><Program Task='A'/><Configuration GUID='outer'><Tasks><Task>A</Task><Task> A </Task><Task>a</Task><Task>A<Part/>B</Task><Task><Part/>A</Task><Task>  </Task><Task/></Tasks><Program Task='A'/><Program Task=' A '/><Program Task='a'/><Program Task='AB'/><Configuration GUID='inner'><Tasks><Task>A</Task></Tasks><Program Task='A'/></Configuration><Group><Task>A</Task><Program Task='A'/></Group></Configuration></Project>",
            "<Project><Configuration GUID='Aa'><Tasks><Task>A</Task></Tasks><Configuration GUID='aA'><Tasks><Task>A</Task></Tasks><Program Task='A'/></Configuration></Configuration><Configuration GUID=' '><Tasks><Task>B</Task></Tasks></Configuration><Configuration><Tasks><Task>C</Task></Tasks></Configuration><Configuration GUID=''><Tasks><Task>D</Task></Tasks></Configuration></Project>",
            "<Project><Configuration GUID='unique'><Tasks><Task>A</Task><Task>A</Task><Task>B</Task></Tasks><Tasks><Task>B</Task></Tasks><Program Task='A'/><Program Task='B'/></Configuration></Project>",
            "<Configuration GUID='root'><Tasks><Task Kind='2' Device='25' Priority='999' TaskIndex='500' Version='unknown'>A<Part/>B</Task><Task>AB</Task><Task Kind='3' DeviceType='1' WordCondition='99'>Unknown</Task></Tasks><Program Task='A'/><Program Task='AB'/></Configuration>",
            "<Project xmlns:t='urn:tasks'><t:Configuration GUID='namespaced'><t:Tasks><t:Task>A</t:Task></t:Tasks><t:Program Task='A'/></t:Configuration></Project>",
        ] {
            matches_selected_handles(&document(xml));
        }
    }

    #[test]
    fn task_inspection_pages_keep_global_indices_without_enriching_other_records() {
        let doc = document(
            "<Project><Configuration GUID='native'><Tasks><Task>A</Task><Task>B</Task><Task>C</Task></Tasks><Program Task='B'/></Configuration></Project>",
        );
        for (offset, limit, indices) in [
            (0, 1, vec![0]),
            (1, 1, vec![1]),
            (2, 1000, vec![2]),
            (3, 1, vec![]),
            (usize::MAX, 1, vec![]),
        ] {
            let (total, selected) = doc.task().inspection_page(offset, limit);
            assert_eq!(total, 3);
            assert_eq!(
                selected.iter().map(|row| row.index).collect::<Vec<_>>(),
                indices
            );
            for row in selected {
                assert_eq!(row.definition, doc.task().get(row.index).unwrap().summary());
                assert_eq!(
                    row.program_indices.unwrap(),
                    if row.index == 1 { vec![0] } else { vec![] }
                );
            }
        }
    }
}
