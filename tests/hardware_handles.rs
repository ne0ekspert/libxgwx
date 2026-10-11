use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{Module, XgwxDocument, XgwxError};
fn document(xml: &str) -> XgwxDocument {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    XgwxDocument::parse(&bytes).unwrap()
}
#[test]
fn read_handles_keep_repeated_coordinates_scoped_and_indices_installed() {
    let doc = document(
        r#"<Project><Configuration>A<Base Base="0"><Module Base="0" Slot="0" Id="42242" SubType="0"/></Base></Configuration><Configuration>B<Base Base="0"><Module Base="0" Slot="0" Id="99999" SubType="0"/></Base></Configuration></Project>"#,
    );
    assert_eq!(doc.module().list().len(), 2);
    let first = doc.base().get(0).unwrap().module(0).unwrap();
    let second = doc.base().get(1).unwrap().module(0).unwrap();
    assert_eq!((first.index(), first.configuration_index()), (0, Some(0)));
    assert_eq!((second.index(), second.configuration_index()), (1, Some(1)));
    assert!(matches!(first.typed(), Module::Io(_)));
    assert!(matches!(second.typed(), Module::Other(_)));
    assert_eq!(first.base().unwrap().index(), 0);
    assert_eq!(second.base().unwrap().index(), 1);
    assert!(matches!(
        doc.module().at(0, 0),
        Err(XgwxError::AmbiguousModule { .. })
    ));
    assert!(matches!(
        doc.module().get(2),
        Err(XgwxError::ModuleIndexNotFound { index: 2 })
    ));
}
#[test]
fn explicit_base_ownership_and_unplaced_hardware_remain_visible() {
    let doc = document(
        r#"<Project><Base Base="1"><Module Slot="0" Id="1"/></Base><Module Base="99" Slot="2" Id="2"/></Project>"#,
    );
    assert_eq!(doc.base().get(0).unwrap().installed_modules().len(), 1);
    assert_eq!(
        doc.module().get(0).unwrap().base().unwrap().number(),
        Some(1)
    );
    assert!(doc.module().get(1).unwrap().base().is_none());
}
#[test]
fn read_selection_does_not_require_native_base_coordinates() {
    let doc = document(r#"<Project><Base><Module Slot="0" Id="1"/></Base></Project>"#);
    let base = doc.base().get(0).unwrap();
    assert_eq!(base.number(), None);
    assert_eq!(base.module(0).unwrap().index(), 0);
}
#[cfg(feature = "write")]
mod writes {
    use super::*;
    use xgwx::{ModuleFieldEdit, ModulePatch, create_project};
    #[test]
    fn bound_base_and_module_edits_preserve_ladder_and_rebind_after_model_change() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        let programs = doc.program().ladder_programs();
        {
            let mut base = doc.base_mut().by_number(0).unwrap();
            base.set_slot_count(4).unwrap();
            base.insert_module(0, "XGI-D24A/B").unwrap();
            base.insert_module(1, "XGL-EFMT(B)").unwrap();
            let mut module = base.module(0).unwrap();
            module
                .update(&ModulePatch {
                    comment: Some("Updated".into()),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(module.summary().comment.as_deref(), Some("Updated"));
            module.select_model("XGL-EFMT(B)").unwrap();
            assert_eq!(module.view().configuration_index(), Some(0));
            assert!(matches!(module.view().typed(), Module::Network(_)));
            module
                .update(&ModulePatch {
                    comment: Some("After replacement".into()),
                    ..Default::default()
                })
                .unwrap();
            let before = module.summary();
            assert!(module.select_model("unknown model").is_err());
            assert_eq!(module.summary(), before);
        }
        doc.module_mut().at(0, 0).unwrap().delete().unwrap();
        assert_eq!(doc.module().list().len(), 1);
        assert_eq!(doc.module().get(0).unwrap().summary().slot, Some(1));
        assert_eq!(doc.program().ladder_programs().len(), programs.len());
        assert_eq!(
            doc.program().ladder_program(0).unwrap().unwrap().data,
            programs[0].as_ref().unwrap().data
        );
        let reopened = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
        assert_eq!(reopened.module().summaries(), doc.module().summaries());
    }
    #[test]
    fn failed_placement_and_slot_shrink_leave_document_unchanged() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(5, "XGL-EFMT(B)")
            .unwrap();
        let before = doc.clone();
        assert!(
            doc.base_mut()
                .by_number(0)
                .unwrap()
                .set_slot_count(4)
                .is_err()
        );
        assert_eq!(doc, before);
        assert!(
            doc.base_mut()
                .by_number(0)
                .unwrap()
                .insert_module(5, "XGL-EFMT(B)")
                .is_err()
        );
        assert_eq!(doc, before);
    }
    #[test]
    fn module_field_guard_rejects_stale_values_and_base_operations() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGL-EFMT(B)")
            .unwrap();
        let mut module = doc.module_mut().at(0, 0).unwrap();
        let edit = ModuleFieldEdit {
            operation: "comment".into(),
            expected_value: module.summary().comment.unwrap_or_default(),
            replacement: "Changed".into(),
            key: None,
            index: None,
        };
        module.edit_field(&edit).unwrap();
        let before = module.summary();
        assert!(module.edit_field(&edit).is_err());
        assert!(
            module
                .edit_field(&ModuleFieldEdit {
                    operation: "slotCount".into(),
                    ..edit
                })
                .is_err()
        );
        assert_eq!(module.summary(), before);
    }
    #[test]
    fn duplicate_coordinates_and_conflicting_ownership_never_mutate() {
        for xml in [
            r#"<Project><Configuration>A<Base Base="0"><Module Base="0" Slot="0" Comment="A"/></Base></Configuration><Configuration>B<Base Base="0"><Module Base="0" Slot="0" Comment="B"/></Base></Configuration></Project>"#,
            r#"<Project><Base Base="1"><Module Base="0" Slot="0" Comment="A"/></Base></Project>"#,
        ] {
            let mut doc = document(xml);
            let before = doc.clone();
            assert!(
                doc.module_mut()
                    .get(0)
                    .unwrap()
                    .update(&ModulePatch {
                        comment: Some("Changed".into()),
                        ..Default::default()
                    })
                    .is_err()
            );
            assert_eq!(doc, before);
        }
    }
    #[test]
    fn unsupported_base_layout_is_rejected() {
        let mut doc = document(
            r#"<Project><Base Base="0" SlotCount="4"><Module Base="0" Slot="0" Comment="A"/></Base></Project>"#,
        );
        let before = doc.clone();
        assert!(doc.base_mut().get(0).is_err());
        assert_eq!(doc, before);
    }
    #[test]
    fn base_field_edits_keep_stale_guards_and_rebind_after_success() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        let mut base = doc.base_mut().by_number(0).unwrap();
        let expected = base
            .summary()
            .attributes
            .iter()
            .find(|a| a.name == "SlotCount")
            .unwrap()
            .value
            .clone();
        base.edit_slot_count(&expected, "4").unwrap();
        assert!(base.edit_slot_count(&expected, "4").is_err());
        base.set_slot_count(6).unwrap();
        assert_eq!(base.summary().slot_count, Some(6));
    }
    #[test]
    fn selected_base_does_not_enable_ambiguous_multi_configuration_writes() {
        let mut doc = document(
            r#"<Project><Configuration>A<Parameter Type="IO PARAMETER"><BaseInfo><Base Base="0" SlotCount="4"/></BaseInfo></Parameter></Configuration><Configuration>B<Parameter Type="IO PARAMETER"><BaseInfo><Base Base="0" SlotCount="4"/></BaseInfo></Parameter></Configuration></Project>"#,
        );
        let before = doc.clone();
        assert!(matches!(
            doc.base_mut().by_number(0),
            Err(XgwxError::AmbiguousBase { base: 0 })
        ));
        assert!(
            doc.base_mut()
                .get(1)
                .unwrap()
                .insert_module(0, "XGL-EFMT(B)")
                .is_err()
        );
        assert_eq!(doc, before);
    }
    #[test]
    fn option_and_filter_edits_retain_existing_catalog_validation() {
        let mut doc = create_project("XGK-CPUSN", "LD").unwrap();
        doc.base_mut()
            .by_number(0)
            .unwrap()
            .insert_module(0, "XGI-D24A/B")
            .unwrap();
        let mut module = doc.module_mut().at(0, 0).unwrap();
        module
            .set_input_filter(xgwx::ModuleInputFilter::from_raw(5))
            .unwrap();
        assert_eq!(module.summary().input_filter_raw, Some(5));
        let before = module.summary();
        assert!(module.set_option("unknown-option", 0, 0).is_err());
        assert_eq!(module.summary(), before);
        assert_eq!(
            module.option_values().unwrap(),
            module.view().option_values().unwrap()
        );
    }
}
