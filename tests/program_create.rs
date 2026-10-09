#![cfg(feature = "write")]
use xgwx::{NewProgram, XgwxDocument};
fn request(name: &str, language: &str) -> NewProgram {
    NewProgram {
        name: name.into(),
        language: language.into(),
        object_id: "abcdef01-1234-4567-89ab-0123456789ab".into(),
        symbol_id: "abcdef02-1234-4567-89ab-0123456789ab".into(),
    }
}
#[test]
fn appends_blank_programs_and_preserves_existing_records() {
    for (file, language) in [
        ("fixtures/empty-projects/new-xgk.xgwx", "LD"),
        ("fixtures/empty-projects/new-xgi.xgwx", "LD"),
        ("fixtures/sfc/native-loop.xgwx", "SFC"),
    ] {
        let mut doc = XgwxDocument::from_path(file).unwrap();
        let parsed = roxmltree::Document::parse(&doc.xml).unwrap();
        let old: Vec<_> = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .map(|n| doc.xml[n.range()].to_owned())
            .collect();
        let count = parsed
            .root_element()
            .attribute("WksNodeCount")
            .unwrap()
            .parse::<u32>()
            .unwrap();
        doc.create_program(&request("AddedProgram", language))
            .unwrap();
        let bytes = doc.to_verified_bytes().unwrap();
        let reloaded = XgwxDocument::parse(&bytes).unwrap();
        let after = roxmltree::Document::parse(&reloaded.xml).unwrap();
        let programs: Vec<_> = after
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .collect();
        assert_eq!(programs.len(), old.len() + 1);
        assert_eq!(
            after
                .root_element()
                .attribute("WksNodeCount")
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            count + if file.contains("new-xgk") { 1 } else { 3 }
        );
        for (node, original) in programs.iter().zip(old) {
            assert_eq!(&reloaded.xml[node.range()], original);
        }
        assert_eq!(programs.last().unwrap().text(), Some("AddedProgram"));
        if language == "SFC" {
            assert_eq!(
                reloaded.sfc_programs().last().unwrap().blocks[0].editable_rows,
                Some(vec![])
            );
        } else {
            assert_eq!(
                reloaded
                    .ladder_programs()
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .data,
                vec![0; 8]
            );
        }
    }
}
#[test]
fn invalid_requests_are_atomic() {
    let mut doc = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx").unwrap();
    let before = doc.to_bytes().unwrap();
    for patch in [
        request("newprogram", "LD"),
        request("Bad name", "LD"),
        request("Added", "SFC"),
        NewProgram {
            object_id: "bad".into(),
            ..request("Added", "LD")
        },
    ] {
        assert!(doc.create_program(&patch).is_err());
        assert_eq!(doc.to_bytes().unwrap(), before);
    }
    doc.create_program(&request("Added", "LD")).unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(doc.create_program(&request("Another", "LD")).is_err());
    assert_eq!(doc.to_bytes().unwrap(), before);
}

#[test]
fn native_save_as_retains_program_identity_rows_and_declarations() {
    for stem in [
        "xgk-ld",
        "xgi-ld",
        "xgi-sfc",
        "xgk-ld-delete",
        "xgi-ld-delete",
        "xgi-sfc-delete",
        "xgi-sfc-order",
    ] {
        let a = XgwxDocument::from_path(format!("fixtures/program-create/{stem}-generated.xgwx"))
            .unwrap();
        let b = XgwxDocument::from_path(format!("fixtures/program-create/{stem}-roundtrip.xgwx"))
            .unwrap();
        let source = roxmltree::Document::parse(&a.xml).unwrap();
        let saved = roxmltree::Document::parse(&b.xml).unwrap();
        assert_eq!(
            source.root_element().attribute("WksNodeCount"),
            saved.root_element().attribute("WksNodeCount")
        );
        let originals: Vec<_> = source
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .collect();
        let retained: Vec<_> = saved
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .collect();
        assert_eq!(originals.len(), retained.len());
        for (original, retained) in originals.iter().zip(&retained) {
            assert_eq!(original.text(), retained.text());
            for attr in original.attributes() {
                assert_eq!(Some(attr.value()), retained.attribute(attr.name()));
            }
        }
        for (original, retained) in a.ladder_programs().iter().zip(b.ladder_programs()) {
            if let (Ok(original), Ok(retained)) = (original, retained) {
                assert_eq!(original.data, retained.data);
            }
        }
        for (original, retained) in a.sfc_programs().iter().zip(b.sfc_programs()) {
            assert_eq!(original.variables, retained.variables);
            for (block, saved) in original.blocks.iter().zip(retained.blocks) {
                assert_eq!(block.editable_rows, saved.editable_rows);
            }
        }
    }
}

#[test]
fn deleting_any_program_preserves_survivors_and_allows_recreation_after_last_delete() {
    for file in [
        "fixtures/program-create/xgk-ld-generated.xgwx",
        "fixtures/program-create/xgi-ld-generated.xgwx",
        "fixtures/program-create/xgi-sfc-generated.xgwx",
    ] {
        for index in [0, 1] {
            let mut doc = XgwxDocument::from_path(file).unwrap();
            let before = roxmltree::Document::parse(&doc.xml).unwrap();
            let programs: Vec<_> = before
                .descendants()
                .filter(|n| n.has_tag_name("Program"))
                .collect();
            let identity = programs[index].attribute("ObjectId").unwrap().to_owned();
            let survivor = doc.xml[programs[1 - index].range()].to_owned();
            doc.delete_program(index, &identity).unwrap();
            let bytes = doc.to_verified_bytes().unwrap();
            let mut reloaded = XgwxDocument::parse(&bytes).unwrap();
            let parsed = roxmltree::Document::parse(&reloaded.xml).unwrap();
            let remaining: Vec<_> = parsed
                .descendants()
                .filter(|n| n.has_tag_name("Program"))
                .collect();
            assert_eq!(remaining.len(), 1);
            assert_eq!(&reloaded.xml[remaining[0].range()], survivor);
            let identity = remaining[0].attribute("ObjectId").unwrap().to_owned();
            reloaded.delete_program(0, &identity).unwrap();
            assert_eq!(reloaded.programs().len(), 0);
            reloaded
                .create_program(&request("Recreated", "LD"))
                .unwrap();
            assert_eq!(reloaded.programs().len(), 1);
            reloaded.to_verified_bytes().unwrap();
        }
    }
}

#[test]
fn stale_deletion_identity_and_invalid_index_leave_the_file_unchanged() {
    let mut doc =
        XgwxDocument::from_path("fixtures/program-create/xgi-sfc-generated.xgwx").unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(
        doc.delete_program(0, "abcdef01-1234-4567-89ab-0123456789ab")
            .is_err()
    );
    assert!(
        doc.delete_program(7, "abcdef01-1234-4567-89ab-0123456789ab")
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), before);
}

#[test]
fn moving_programs_preserves_exact_records_and_outer_xml() {
    for file in ["xgk-ld", "xgi-ld", "xgi-sfc"] {
        let mut doc =
            XgwxDocument::from_path(format!("fixtures/program-create/{file}-generated.xgwx"))
                .unwrap();
        let mut patch = request("Third", if file == "xgi-sfc" { "SFC" } else { "LD" });
        patch.object_id = "abcdef03-1234-4567-89ab-0123456789ab".into();
        patch.symbol_id = "abcdef04-1234-4567-89ab-0123456789ab".into();
        doc.create_program(&patch).unwrap();
        let original = doc.xml.clone();
        let parsed = roxmltree::Document::parse(&original).unwrap();
        let nodes: Vec<_> = parsed
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .collect();
        let ids: Vec<_> = nodes
            .iter()
            .map(|n| n.attribute("ObjectId").unwrap())
            .collect();
        let records: Vec<_> = nodes.iter().map(|n| &original[n.range()]).collect();
        for (from, to) in [(0, 2), (2, 0), (0, 1), (1, 0), (1, 1)] {
            let mut moved = doc.clone();
            moved.move_program(from, to, ids[from], ids[to]).unwrap();
            let reloaded = XgwxDocument::parse(&moved.to_verified_bytes().unwrap()).unwrap();
            let parsed = roxmltree::Document::parse(&reloaded.xml).unwrap();
            let actual: Vec<_> = parsed
                .descendants()
                .filter(|n| n.has_tag_name("Program"))
                .map(|n| &reloaded.xml[n.range()])
                .collect();
            let mut expected = records.clone();
            let record = expected.remove(from);
            expected.insert(to, record);
            assert_eq!(actual, expected);
            moved
                .move_program(
                    to,
                    from,
                    ids[from],
                    parsed
                        .descendants()
                        .filter(|n| n.has_tag_name("Program"))
                        .nth(from)
                        .unwrap()
                        .attribute("ObjectId")
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(moved.xml, original);
        }
        assert!(doc.move_program(0, 1, ids[1], ids[0]).is_err());
        assert!(doc.move_program(0, 1, ids[0], "stale").is_err());
        assert!(doc.move_program(0, 7, ids[0], ids[1]).is_err());
        assert_eq!(doc.xml, original);
    }
}
