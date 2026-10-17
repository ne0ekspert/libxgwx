use xgwx::*;

#[test]
fn native_tasks_decode_with_configuration_and_membership() {
    let doc = XgwxDocument::parse(include_bytes!("../fixtures/tasks/xgk-native.xgwx")).unwrap();
    assert_eq!(doc.task().list(), doc.program().tasks());
    assert_eq!(doc.task().list().len(), 5);
    let scan = doc.task().get(0).unwrap();
    assert_eq!(scan.configuration_index(), Some(0));
    assert_eq!(scan.programs().unwrap().len(), 1);
    assert!(scan.settings().is_err());
    assert_eq!(
        doc.task().get(1).unwrap().settings().unwrap().trigger,
        TaskTrigger::Periodic { interval_ms: 100 }
    );
    let word = doc.task().get(4).unwrap().settings().unwrap();
    assert_eq!(word.priority, 7);
    assert_eq!(word.task_number, 95);
    assert_eq!(
        word.trigger,
        TaskTrigger::InternalWord {
            device: "D000000".into(),
            condition: TaskWordCondition::Greater,
            value: 1234
        }
    );
    assert!(doc.task().get(usize::MAX).is_err());
    let iec = XgwxDocument::parse(include_bytes!("../fixtures/tasks/xgi-native.xgwx")).unwrap();
    assert_eq!(
        iec.task().get(2).unwrap().settings().unwrap().trigger,
        TaskTrigger::InternalBit {
            device: "%MX0".into(),
            condition: TaskBitCondition::Off
        }
    );
    assert_eq!(
        iec.task().get(3).unwrap().settings().unwrap().trigger,
        TaskTrigger::InternalWord {
            device: "%MW0".into(),
            condition: TaskWordCondition::Equal,
            value: 65535
        }
    );
}

#[cfg(feature = "write")]
mod writes {
    use super::*;
    fn native() -> XgwxDocument {
        XgwxDocument::parse(include_bytes!("../fixtures/tasks/xgk-native.xgwx")).unwrap()
    }
    fn periodic(name: &str, task_number: u32) -> TaskSettings {
        TaskSettings {
            name: name.into(),
            priority: 2,
            task_number,
            trigger: TaskTrigger::Periodic { interval_ms: 25 },
        }
    }
    fn unchanged(
        doc: &mut XgwxDocument,
        edit: impl FnOnce(&mut XgwxDocument) -> Result<(), XgwxError>,
    ) {
        let before = doc.clone();
        assert!(edit(doc).is_err());
        assert_eq!(*doc, before);
    }
    fn replace(doc: &mut XgwxDocument, from: &str, to: &str) {
        let xml = doc.container().xml().replace(from, to);
        doc.container_mut().replace_xml(xml).unwrap();
    }
    #[test]
    fn creation_updates_counts_and_all_trigger_conditions_roundtrip() {
        for model in ["XGK-CPUSN", "XGI-CPUUN"] {
            let mut doc = create_project(model, "LD").unwrap();
            let original_count = doc.project().info().wks_node_count.unwrap();
            doc.task_mut().create(0, &periodic("周期1", 31)).unwrap();
            let init = TaskSettings {
                name: "Init".into(),
                priority: 2,
                task_number: 0,
                trigger: TaskTrigger::Initialization,
            };
            doc.task_mut().create(0, &init).unwrap();
            let bits = [
                TaskBitCondition::Rising,
                TaskBitCondition::Falling,
                TaskBitCondition::Changed,
                TaskBitCondition::On,
                TaskBitCondition::Off,
            ];
            let words = [
                TaskWordCondition::Less,
                TaskWordCondition::LessOrEqual,
                TaskWordCondition::Equal,
                TaskWordCondition::GreaterOrEqual,
                TaskWordCondition::Greater,
            ];
            for (i, condition) in bits.into_iter().enumerate() {
                doc.task_mut()
                    .create(
                        0,
                        &TaskSettings {
                            name: format!("Bit{i}"),
                            priority: 7,
                            task_number: 64 + i as u32,
                            trigger: TaskTrigger::InternalBit {
                                device: if model.starts_with("XGI") {
                                    "%MX0"
                                } else {
                                    "M00000"
                                }
                                .into(),
                                condition,
                            },
                        },
                    )
                    .unwrap();
            }
            for (i, condition) in words.into_iter().enumerate() {
                doc.task_mut()
                    .create(
                        0,
                        &TaskSettings {
                            name: format!("Word{i}"),
                            priority: 3,
                            task_number: 69 + i as u32,
                            trigger: TaskTrigger::InternalWord {
                                device: if model.starts_with("XGI") {
                                    "%MW0"
                                } else {
                                    "D000000"
                                }
                                .into(),
                                condition,
                                value: 65535,
                            },
                        },
                    )
                    .unwrap();
            }
            assert_eq!(
                doc.project().info().wks_node_count,
                Some(original_count + 12)
            );
            let reparsed = XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
            for i in 1..13 {
                assert_eq!(
                    reparsed.task().get(i).unwrap().settings().unwrap(),
                    doc.task().get(i).unwrap().settings().unwrap()
                );
            }
            assert_eq!(
                reparsed.program().get(0).unwrap().summary().task,
                doc.program().get(0).unwrap().summary().task
            );
        }
    }
    #[test]
    fn assign_rename_update_delete_preserve_program_body_and_counts() {
        let mut doc = native();
        let body = doc.program().get(0).unwrap().ladder().unwrap();
        let p = doc.program().get(0).unwrap().summary();
        let id = p.object_id.unwrap();
        let scan = p.task.unwrap();
        let count = doc.project().info().wks_node_count.unwrap();
        doc.task_mut()
            .get(1)
            .unwrap()
            .assign_program(0, &id, &scan)
            .unwrap();
        assert_eq!(doc.task().get(1).unwrap().programs().unwrap()[0].index(), 0);
        unchanged(&mut doc, |d| d.task_mut().get(1)?.delete());
        doc.task_mut().get(1).unwrap().rename("FastLoop").unwrap();
        assert_eq!(
            doc.program().get(0).unwrap().summary().task.as_deref(),
            Some("FastLoop")
        );
        let mut settings = doc.task().get(1).unwrap().settings().unwrap();
        settings.name = "Faster".into();
        settings.priority = 7;
        settings.trigger = TaskTrigger::Periodic {
            interval_ms: u32::MAX,
        };
        doc.task_mut().get(1).unwrap().update(&settings).unwrap();
        assert_eq!(doc.task().get(1).unwrap().settings().unwrap(), settings);
        doc.task_mut()
            .get(0)
            .unwrap()
            .assign_program(0, &id, "Faster")
            .unwrap();
        doc.task_mut().get(1).unwrap().delete().unwrap();
        assert_eq!(doc.project().info().wks_node_count, Some(count - 1));
        assert_eq!(doc.program().get(0).unwrap().ladder().unwrap(), body);
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    }
    #[test]
    fn invalid_ranges_names_collisions_and_stale_snapshots_are_atomic() {
        let mut doc = native();
        for name in [
            "",
            "With space",
            "<bad>",
            "Periodic100",
            "periodic100",
            " A",
            "A\n",
        ] {
            unchanged(&mut doc, |d| d.task_mut().create(0, &periodic(name, 1)));
        }
        for (number, priority, interval) in [(32, 2, 1), (0, 2, 1), (1, 1, 1), (1, 8, 1), (1, 2, 0)]
        {
            let settings = TaskSettings {
                name: "Fresh".into(),
                priority,
                task_number: number,
                trigger: TaskTrigger::Periodic {
                    interval_ms: interval,
                },
            };
            unchanged(&mut doc, |d| d.task_mut().create(0, &settings));
        }
        let expected = doc.task().get(1).unwrap().snapshot().unwrap();
        doc.task_mut().get(1).unwrap().rename("Renamed").unwrap();
        unchanged(&mut doc, |d| {
            d.task_mut().get_if(1, &expected)?.rename("Stale")
        });
        unchanged(&mut doc, |d| d.task_mut().get(0)?.rename("Scan"));
        unchanged(&mut doc, |d| d.task_mut().get(0)?.delete());
        unchanged(&mut doc, |d| {
            d.task_mut().get(0)?.update(&periodic("Scan", 1))
        });
        unchanged(&mut doc, |d| d.task_mut().create(99, &periodic("Fresh", 1)));
        unchanged(&mut doc, |d| {
            d.task_mut()
                .get(1)?
                .assign_program(0, "wrong", "스캔 프로그램")
        });
        let id = doc.program().get(0).unwrap().summary().object_id.unwrap();
        unchanged(&mut doc, |d| {
            d.task_mut().get(1)?.assign_program(0, &id, "wrong")
        });
        unchanged(&mut doc, |d| {
            d.program_mut().get(0)?.update(&ProgramPatch {
                task: Some("missing".into()),
                ..Default::default()
            })
        });
    }
    #[test]
    fn failed_schedule_change_cannot_leak_changes_or_duplicate_initialization() {
        let mut doc = native();
        let mut settings = doc.task().get(1).unwrap().settings().unwrap();
        settings.name = "BitRise".into();
        settings.priority = 6;
        unchanged(&mut doc, |d| d.task_mut().get(1)?.update(&settings));
        settings.name = "OtherInit".into();
        settings.trigger = TaskTrigger::Initialization;
        unchanged(&mut doc, |d| d.task_mut().get(1)?.update(&settings));
        for (number, device) in [
            (63, "M00000"),
            (96, "M00000"),
            (64, "M00000"),
            (65, ""),
            (65, "&bad"),
            (65, "%MX0"),
        ] {
            settings.name = "Invalid".into();
            settings.task_number = number;
            settings.trigger = TaskTrigger::InternalBit {
                device: device.into(),
                condition: TaskBitCondition::On,
            };
            unchanged(&mut doc, |d| d.task_mut().create(0, &settings));
        }
    }
    #[test]
    fn unknown_attributes_survive_and_unknown_layouts_remain_readable() {
        let mut doc = native();
        replace(
            &mut doc,
            "Kind=\"2\" Priority=\"2\"",
            "Opaque=\"keep&amp;me\" Kind=\"2\" Priority=\"2\"",
        );
        let mut settings = doc.task().get(1).unwrap().settings().unwrap();
        settings.priority = 5;
        doc.task_mut().get(1).unwrap().update(&settings).unwrap();
        assert!(doc.container().xml().contains("Opaque=\"keep&amp;me\""));
        replace(&mut doc, "Kind=\"2\"", "Kind=\"99\"");
        assert!(doc.task().get(1).unwrap().settings().is_err());
        unchanged(&mut doc, |d| d.task_mut().get(1)?.update(&settings));
        replace(&mut doc, "Type=\"17\"", "Type=\"99999\"");
        assert!(!doc.task().can_create(0));
        unchanged(&mut doc, |d| d.task_mut().create(0, &periodic("Fresh", 1)));
    }
    #[test]
    fn create_program_directly_in_task_is_atomic_and_keeps_native_identity() {
        let mut doc = native();
        let patch = NewProgram {
            name: "Worker".into(),
            language: "LD".into(),
            object_id: "11111111-1111-4111-8111-111111111111".into(),
            symbol_id: "22222222-2222-4222-8222-222222222222".into(),
        };
        doc.task_mut()
            .get(1)
            .unwrap()
            .create_program(&patch)
            .unwrap();
        let p = doc.program().get(1).unwrap().summary();
        assert_eq!(p.task.as_deref(), Some("Periodic100"));
        assert_eq!(p.object_id.as_deref(), Some(patch.object_id.as_str()));
        unchanged(&mut doc, |d| d.task_mut().get(1)?.create_program(&patch));
        XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
    }
    #[test]
    fn tasks_with_same_names_in_other_plcs_are_scoped_and_cross_plc_assignment_rejects() {
        let mut doc = native();
        let xml = doc.container().xml().to_owned();
        let parsed = roxmltree::Document::parse(&xml).unwrap();
        let config = parsed
            .descendants()
            .find(|n| n.has_tag_name("Configuration"))
            .unwrap();
        let clone = xml[config.range()].replace(
            config.attribute("GUID").unwrap(),
            "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        );
        let end = config.range().end;
        let mut duplicated = xml.clone();
        duplicated.insert_str(end, &clone);
        doc.container_mut().replace_xml(duplicated).unwrap();
        assert_eq!(doc.task().get(1).unwrap().programs().unwrap().len(), 0);
        let p = doc.program().get(0).unwrap().summary();
        let id = p.object_id.unwrap();
        let scan = p.task.unwrap();
        unchanged(&mut doc, |d| {
            d.task_mut().get(6)?.assign_program(0, &id, &scan)
        });
        let first_plc = doc.task().get(1).unwrap().snapshot().unwrap();
        unchanged(&mut doc, |d| {
            d.task_mut().get_if(6, &first_plc)?.rename("WrongPLC")
        });
        doc.task_mut()
            .get(1)
            .unwrap()
            .assign_program(0, &id, &scan)
            .unwrap();
        doc.task_mut()
            .get(1)
            .unwrap()
            .rename("OnlyFirstPLC")
            .unwrap();
        assert_eq!(
            doc.program().get(0).unwrap().summary().task.as_deref(),
            Some("OnlyFirstPLC")
        );
        assert_eq!(
            doc.task().get(6).unwrap().summary().name.as_deref(),
            Some("Periodic100")
        );
        let p2 = doc.program().get(1).unwrap().summary();
        assert_eq!(p2.task.as_deref(), Some(scan.as_str()));
    }
    #[test]
    fn malformed_counts_and_missing_fields_reject_without_mutation() {
        let mut doc = native();
        replace(&mut doc, "WksNodeCount=\"18\"", "WksNodeCount=\"oops\"");
        // Also handle a fixture with a different native count without relying on a fixed value.
        let count = doc.project().info().wks_node_count.map(|n| n.to_string());
        if let Some(n) = count {
            replace(
                &mut doc,
                &format!("WksNodeCount=\"{n}\""),
                "WksNodeCount=\"oops\"",
            );
        }
        unchanged(&mut doc, |d| d.task_mut().create(0, &periodic("Fresh", 1)));
        unchanged(&mut doc, |d| d.task_mut().get(1)?.delete());
        let mut doc = native();
        let settings = doc.task().get(1).unwrap().settings().unwrap();
        replace(&mut doc, "Device=\"100\"", "");
        unchanged(&mut doc, |d| d.task_mut().get(1)?.update(&settings));
    }
}

#[test]
fn native_check_and_save_as_keep_all_task_fields_and_program_membership() {
    use std::collections::BTreeMap;
    for family in ["xgk", "xgi"] {
        let source =
            XgwxDocument::from_path(format!("fixtures/tasks/{family}-generated.xgwx")).unwrap();
        let saved =
            XgwxDocument::from_path(format!("fixtures/tasks/{family}-roundtrip.xgwx")).unwrap();
        assert_eq!(source.task().list().len(), 13);
        assert_eq!(
            source.task().list(),
            saved.task().list(),
            "{family}: every native task field"
        );
        assert_eq!(
            source.project().info().wks_node_count,
            saved.project().info().wks_node_count
        );
        let programs =
            |doc: &XgwxDocument| -> BTreeMap<String, (Option<String>, Option<String>, Vec<u8>)> {
                doc.program()
                    .list()
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        (
                            p.object_id.unwrap(),
                            (
                                p.name,
                                p.task,
                                doc.program().get(i).unwrap().ladder().unwrap().data,
                            ),
                        )
                    })
                    .collect()
            };
        assert_eq!(
            programs(&source),
            programs(&saved),
            "{family}: native identities, membership and program bodies"
        );
        for index in 0..source.task().list().len() {
            let assigned = |doc: &XgwxDocument| {
                doc.task()
                    .get(index)
                    .unwrap()
                    .programs()
                    .unwrap()
                    .iter()
                    .map(|p| p.summary().object_id.unwrap())
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                assigned(&source),
                assigned(&saved),
                "{family}: task {index} execution order"
            );
        }
    }
}
