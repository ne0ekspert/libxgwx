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
    fn selecting_a_record_does_not_enable_multi_configuration_cpu_writes() {
        let mut doc = document(
            r#"<Project><Configuration Type="17">PLC</Configuration><Configuration Type="17">PLC</Configuration></Project>"#,
        );
        let before = doc.clone();
        for index in [0, 1] {
            assert!(matches!(
                doc.configuration_mut()
                    .get(index)
                    .unwrap()
                    .select_cpu("XGK-CPUSN"),
                Err(XgwxError::AmbiguousConfiguration)
            ));
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
