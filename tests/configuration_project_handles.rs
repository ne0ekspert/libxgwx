use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{XgwxDocument, XgwxError};
fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
#[test]
fn repeated_configuration_names_keep_cpu_and_bases_scoped() {
    let doc = document(
        r#"<Project>Factory<Configuration Type="17">PLC<Base Base="0"/></Configuration><Configuration Type="111">PLC<Base Base="0"/></Configuration><Options Details="foo=bar"/></Project>"#,
    );
    assert_eq!(doc.project().name(), Some("Factory"));
    assert_eq!(doc.project().info().name.as_deref(), Some("Factory"));
    assert_eq!(
        doc.project().options().unwrap().raw_details.as_deref(),
        Some("foo=bar")
    );
    let a = doc.configuration().get(0).unwrap();
    let b = doc.configuration().get(1).unwrap();
    assert_eq!(a.cpu().unwrap().model, "XGK-CPUSN");
    assert_eq!(b.cpu().unwrap().model, "XGI-CPUUN");
    assert_eq!(a.bases()[0].index(), 0);
    assert_eq!(b.bases()[0].index(), 1);
    assert!(a.hardware_profile().is_none());
    assert!(b.hardware_profile().is_none());
    assert!(matches!(
        doc.configuration().get(2),
        Err(XgwxError::ConfigurationIndexNotFound { index: 2 })
    ));
}
#[test]
fn absent_and_unknown_cpu_metadata_remains_inspectable() {
    let doc = document(r#"<Project><Configuration Type="999999"/><Configuration/></Project>"#);
    assert!(doc.configuration().get(0).unwrap().cpu().is_none());
    assert_eq!(
        doc.configuration().get(0).unwrap().summary().type_code,
        Some(999999)
    );
    assert!(doc.configuration().get(1).unwrap().cpu().is_none());
    assert!(doc.project().name().is_none());
    assert!(doc.project().options().is_none());
}
#[cfg(feature = "write")]
mod writes {
    use super::*;
    fn editable_document(xml: &str) -> XgwxDocument {
        let mut doc = xgwx::create_project("XGK-CPUH", "LD").unwrap();
        doc.container_mut().replace_xml(xml.to_owned()).unwrap();
        doc
    }
    #[test]
    fn selected_xgk_conversion_preserves_programs_networks_and_options() {
        let mut doc = xgwx::create_project("XGK-CPUH", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGL-EFMT(B)")
            .unwrap();
        let programs: Vec<_> = doc
            .program()
            .ladder_programs()
            .into_iter()
            .map(Result::unwrap)
            .collect();
        let modules = doc.modules();
        let networks = doc.network().list();
        let options = doc.project().options();
        let mut selected = doc.configuration_mut().get(0).unwrap();
        selected.select_cpu("XGK-CPUUN").unwrap();
        assert_eq!(selected.view().cpu().unwrap().model, "XGK-CPUUN");
        selected.select_cpu("XGK-CPUH").unwrap();
        assert_eq!(doc.modules(), modules);
        assert_eq!(doc.network().list(), networks);
        assert_eq!(doc.project().options(), options);
        assert_eq!(
            doc.program()
                .ladder_programs()
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            programs
        );
        assert_eq!(
            XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
                .unwrap()
                .container()
                .root(),
            doc.container().root()
        );
    }
    #[test]
    fn selected_xgi_conversion_preserves_captured_text_programs() {
        let mut doc =
            XgwxDocument::parse(include_bytes!("../fixtures/text-programs/generated.xgwx"))
                .unwrap();
        let programs = doc.program().text_programs();
        doc.configuration_mut()
            .get(0)
            .unwrap()
            .select_cpu("XGI-CPUUN")
            .unwrap();
        assert_eq!(doc.program().text_programs(), programs);
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(saved.program().text_programs(), programs);
        assert_eq!(
            saved.configuration().get(0).unwrap().cpu().unwrap().model,
            "XGI-CPUUN"
        );
    }
    #[test]
    fn cpu_limits_and_unsupported_changes_remain_atomic() {
        let mut doc = xgwx::create_project("XGK-CPUUN", "LD").unwrap();
        // This rack is legal for CPUUN and exceeds CPUSN's rack count.
        let xml = doc.container().xml().replace("Base=\"0\"", "Base=\"7\"");
        doc = document(&xml);
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(0)
                .unwrap()
                .select_cpu("XGK-CPUSN")
                .is_err()
        );
        assert_eq!(doc, before);
        for model in ["not a CPU", "XGI-CPUUN"] {
            assert!(
                doc.configuration_mut()
                    .get(0)
                    .unwrap()
                    .select_cpu(model)
                    .is_err()
            );
            assert_eq!(doc, before);
        }
    }
    #[test]
    fn selected_cpu_changes_ignore_sibling_limits_and_preserve_sibling_xml() {
        let mut doc = editable_document(
            r#"<Project WksNodeCount="7">Factory<Configuration Type="17">PLC<Base Base="0" SlotCount="8"/></Configuration><!-- keep --><Configuration Type="16">PLC<Base Base="7" SlotCount="12"/></Configuration><Options Details="keep"/></Project>"#,
        );
        let original = doc.container().xml().to_owned();
        doc.configuration_mut()
            .get(0)
            .unwrap()
            .select_cpu("XGK-CPUH")
            .unwrap();
        assert_eq!(
            doc.container().xml(),
            original.replacen("Type=\"17\"", "Type=\"0\"", 1)
        );
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(1)
                .unwrap()
                .select_cpu("XGK-CPUSN")
                .is_err()
        );
        assert_eq!(doc, before);
        // The second PLC is also independently selectable.
        doc.configuration_mut()
            .get(1)
            .unwrap()
            .select_cpu("XGK-CPUUN")
            .unwrap();
        assert_eq!(
            doc.configuration().get(0).unwrap().cpu().unwrap().model,
            "XGK-CPUH"
        );
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(saved.container().root(), doc.container().root());
    }
    #[test]
    fn selected_module_limits_do_not_include_sibling_modules() {
        let mut first = xgwx::create_project("XGK-CPUH", "LD").unwrap();
        first
            .base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGL-EFMT(B)")
            .unwrap();
        let xml = first.container().xml();
        let parsed = roxmltree::Document::parse(xml).unwrap();
        let configuration = parsed
            .descendants()
            .find(|n| n.has_tag_name("Configuration"))
            .unwrap();
        let sibling = xml[configuration.range()].replace("Base=\"0\"", "Base=\"7\"");
        let mut combined = xml.to_owned();
        combined.insert_str(configuration.range().end, &sibling);
        // Trim unused high racks only on the first PLC so CPUSN is legal there.
        let mut unused = configuration
            .descendants()
            .filter(|node| {
                node.has_tag_name("Base")
                    && node
                        .attribute("Base")
                        .and_then(|value| value.parse::<u32>().ok())
                        .is_some_and(|base| base >= 4)
            })
            .map(|node| node.range())
            .collect::<Vec<_>>();
        unused.sort_by_key(|range| std::cmp::Reverse(range.start));
        for range in unused {
            combined.replace_range(range, "");
        }
        let mut doc = editable_document(&combined);
        doc.configuration_mut()
            .get(0)
            .unwrap()
            .select_cpu("XGK-CPUSN")
            .unwrap();
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(1)
                .unwrap()
                .select_cpu("XGK-CPUSN")
                .is_err()
        );
        assert_eq!(doc, before);
        let type_range = configuration
            .attributes()
            .find(|attribute| attribute.name() == "Type")
            .unwrap()
            .range_value();
        let mut expected = combined.clone();
        expected.replace_range(type_range, "17");
        assert_eq!(doc.container().xml(), expected);
        let saved = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(saved.configuration().list().len(), 2);
        assert_eq!(saved.modules(), doc.modules());
    }
    #[test]
    fn unowned_hardware_does_not_bypass_selected_cpu_limits() {
        let mut doc = document(
            r#"<Project><Configuration Type="0">PLC</Configuration><Base Base="7" SlotCount="12"/></Project>"#,
        );
        let before = doc.clone();
        assert!(
            doc.configuration_mut()
                .get(0)
                .unwrap()
                .select_cpu("XGK-CPUSN")
                .is_err()
        );
        assert_eq!(doc, before);
    }
    #[test]
    fn selected_cpu_noops_and_unsupported_migrations_are_atomic_in_multi_plc_files() {
        let mut doc = document(
            r#"<Project><Configuration Type="17">PLC</Configuration><Configuration Type="111">PLC</Configuration></Project>"#,
        );
        let before = doc.clone();
        doc.configuration_mut()
            .get(0)
            .unwrap()
            .select_cpu("XGK-CPUSN")
            .unwrap();
        doc.configuration_mut()
            .get(1)
            .unwrap()
            .select_cpu("XGI-CPUUN")
            .unwrap();
        assert_eq!(doc, before);
        for (index, model) in [(0, "XGI-CPUUN"), (1, "XGI-CPUH"), (0, "unknown")] {
            assert!(
                doc.configuration_mut()
                    .get(index)
                    .unwrap()
                    .select_cpu(model)
                    .is_err()
            );
            assert_eq!(doc, before);
        }
    }
    #[test]
    fn rename_rebinds_and_captured_name_failures_preserve_options_and_payloads() {
        let bytes = include_bytes!("../fixtures/empty-projects/new-xgk.xgwx");
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        let before = doc.clone();
        doc.project_mut().rename("NewProject").unwrap();
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
        for (old, name) in [
            ("stale", "Factory"),
            ("NewProject", "  "),
            ("NewProject", "bad\nname"),
        ] {
            assert!(doc.project_mut().rename_if(old, name).is_err());
            assert_eq!(doc, before);
        }
        let mut project = doc.project_mut();
        project.rename("공장 & <Line 1>").unwrap();
        assert_eq!(project.view().name(), Some("공장 & <Line 1>"));
        project.rename("Factory").unwrap();
        assert_eq!(
            doc.container().xml(),
            before
                .container()
                .xml()
                .replacen("NewProject", "Factory", 1)
        );
        assert_eq!(doc.project().options(), before.project().options());
        assert_eq!(
            XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
                .unwrap()
                .project()
                .name(),
            Some("Factory")
        );
    }
    #[test]
    fn malformed_and_ambiguous_project_names_are_rejected() {
        for xml in [
            "<Project><Child/></Project>",
            "<Project>A<Child/>B</Project>",
            "<Other>Factory</Other>",
        ] {
            let mut doc = document(xml);
            let before = doc.clone();
            assert!(doc.project_mut().rename("Changed").is_err());
            assert_eq!(doc, before);
        }
    }
}

#[test]
fn programs_and_tasks_remain_scoped_with_repeated_names_and_orphans() {
    let doc = document(
        r#"<Project><Program Name="orphan"/><Task Name="orphan"/><Configuration Type="17">PLC<Programs><Program Name="Main" Task="Scan"/></Programs><Tasks><Task Name="Scan"/></Tasks></Configuration><Configuration Type="111">PLC<Programs><Program Name="Main" Task="Scan"/></Programs><Tasks><Task Name="Scan"/></Tasks></Configuration></Project>"#,
    );
    assert_eq!(doc.program().get(0).unwrap().configuration_index(), None);
    for index in 0..2 {
        let plc = doc.configuration().get(index).unwrap();
        let programs = plc.programs();
        let tasks = plc.tasks();
        assert_eq!(programs.len(), 1);
        assert_eq!(tasks.len(), 1);
        assert_eq!(programs[0].index(), index + 1);
        assert_eq!(programs[0].configuration_index(), Some(index));
        assert_eq!(tasks[0].index(), index + 1);
        assert_eq!(tasks[0].configuration_index(), Some(index));
    }
}
