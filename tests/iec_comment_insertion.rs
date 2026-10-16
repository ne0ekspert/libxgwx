#![cfg(feature = "write")]
use xgwx::{XgwxDocument, create_project};

fn insert(doc: &mut XgwxDocument, row: u16, text: &str) -> Result<(), xgwx::XgwxError> {
    doc.program_mut()
        .get(0)?
        .iec_ld()?
        .insert_comment(row, text)
}

#[test]
fn first_comment_can_be_inserted_at_l25_on_a_fresh_iec_canvas() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    insert(&mut doc, 25, "L25 설명").unwrap();
    let doc = XgwxDocument::parse(&doc.to_bytes().unwrap()).unwrap();
    let program = doc.program().get(0).unwrap().ladder().unwrap();
    let rows = program.iec_row_frames().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].row_index, 25);
    assert_eq!(
        program.iec_record_frames().unwrap()[0].kind,
        xgwx::IecRecordKind::Comment
    );
    assert_eq!(comment_texts(&program), vec!["L25 설명"]);
}

#[test]
fn comment_insertion_at_l25_preserves_native_output_wire_fixture() {
    let mut doc =
        XgwxDocument::from_path("fixtures/function-output-wires/full-native.xgwx").unwrap();
    let original = doc.program().get(0).unwrap().ladder().unwrap();
    insert(&mut doc, 25, "L25 comment").unwrap();
    let changed = doc.program().get(0).unwrap().ladder().unwrap();
    assert_eq!(
        changed.iec_row_frames().unwrap().len(),
        original.iec_row_frames().unwrap().len() + 1
    );
    assert_eq!(changed.iec_geometry(), original.iec_geometry());
    assert_eq!(&changed.data[8..original.data.len()], &original.data[8..]);
    assert_eq!(comment_texts(&changed), vec!["L25 comment"]);
}

fn comment_texts(program: &xgwx::LadderProgramData) -> Vec<String> {
    program
        .iec_record_frames()
        .unwrap()
        .iter()
        .filter(|r| r.kind == xgwx::IecRecordKind::Comment)
        .map(|r| {
            let start = r.offset + 19;
            let units = usize::from(program.data[r.offset + 18]);
            let text = program.data[start..start + units * 2]
                .chunks_exact(2)
                .map(|u| u16::from_le_bytes([u[0], u[1]]))
                .collect::<Vec<_>>();
            String::from_utf16(&text).unwrap()
        })
        .collect()
}

#[test]
fn comments_reject_occupied_rows_invalid_text_and_overflow_without_mutation() {
    let mut doc =
        XgwxDocument::from_path("fixtures/function-output-wires/full-native.xgwx").unwrap();
    let original = doc.to_bytes().unwrap();
    for (row, text) in [
        (0, "occupied"),
        (1, "function body"),
        (25, ""),
        (25, "   "),
        (25, "control\ncharacter"),
        (16383, "out of range"),
    ] {
        assert!(insert(&mut doc, row, text).is_err());
        assert_eq!(doc.to_bytes().unwrap(), original);
    }
    assert!(insert(&mut doc, 25, &"x".repeat(256)).is_err());
    insert(&mut doc, 25, &"x".repeat(255)).unwrap();
    let inserted = doc.to_bytes().unwrap();
    assert!(insert(&mut doc, 25, "already occupied").is_err());
    assert_eq!(doc.to_bytes().unwrap(), inserted);
    insert(&mut doc, 30, "설명 📝").unwrap();
    let changed = doc.program().get(0).unwrap().ladder().unwrap();
    assert_eq!(
        comment_texts(&changed),
        vec!["x".repeat(255), "설명 📝".into()]
    );
}

#[test]
fn first_comment_between_function_networks_keeps_each_existing_group_payload() {
    let mut doc =
        XgwxDocument::from_path("fixtures/function-output-wires/full-native.xgwx").unwrap();
    let original = doc.program().get(0).unwrap().ladder().unwrap();
    insert(&mut doc, 4, "Between networks").unwrap();
    let changed = doc.program().get(0).unwrap().ladder().unwrap();
    let old_rows = original.iec_row_frames().unwrap();
    let new_rows = changed.iec_row_frames().unwrap();
    for group in 0..5 {
        let old = old_rows
            .iter()
            .filter(|r| r.group_index == group)
            .collect::<Vec<_>>();
        let moved_group = group + usize::from(group > 0);
        let new = new_rows
            .iter()
            .filter(|r| r.group_index == moved_group)
            .collect::<Vec<_>>();
        assert_eq!(
            &original.data[old[0].start - 6..old.last().unwrap().end],
            &changed.data[new[0].start - 6..new.last().unwrap().end]
        );
    }
    assert_eq!(
        changed.iec_function_blocks().unwrap().len(),
        original.iec_function_blocks().unwrap().len()
    );
    assert_eq!(
        changed.iec_function_references().unwrap().len(),
        original.iec_function_references().unwrap().len()
    );
    assert_eq!(
        changed.iec_function_operand_links().unwrap().len(),
        original.iec_function_operand_links().unwrap().len()
    );
}

#[test]
fn deleting_standalone_comment_leaves_existing_circuits_unchanged() {
    let mut doc =
        XgwxDocument::from_path("fixtures/function-output-wires/full-native.xgwx").unwrap();
    let original = doc.program().get(0).unwrap().ladder().unwrap();
    insert(&mut doc, 25, "Delete this comment").unwrap();
    let added = doc.program().get(0).unwrap().ladder().unwrap();
    let row = added
        .iec_row_frames()
        .unwrap()
        .into_iter()
        .find(|r| r.row_index == 25)
        .unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .delete_group(row.group_index, 25)
        .unwrap();
    let changed = doc.program().get(0).unwrap().ladder().unwrap();
    assert_eq!(comment_texts(&changed), Vec::<String>::new());
    assert_eq!(changed.iec_geometry(), original.iec_geometry());
    assert_eq!(&changed.data[6..], &original.data[6..]);
}

#[test]
fn deleting_the_only_comment_restores_empty_iec_program() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    insert(&mut doc, 25, "Only comment").unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(
        doc.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .delete_group(0, 24)
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), before);
    doc.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .delete_group(0, 25)
        .unwrap();
    assert_eq!(
        doc.program().get(0).unwrap().ladder().unwrap().data,
        vec![0; 8]
    );
    insert(&mut doc, 25, "Can insert again").unwrap();
}

#[test]
fn last_executable_network_stays_guarded() {
    let mut doc = create_project("XGI-CPUE", "LD").unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .iec_ld()
        .unwrap()
        .insert_single_element(0, 1, "contact", "NO", "%MX0")
        .unwrap();
    let before = doc.to_bytes().unwrap();
    assert!(
        doc.program_mut()
            .get(0)
            .unwrap()
            .iec_ld()
            .unwrap()
            .delete_group(0, 0)
            .is_err()
    );
    assert_eq!(doc.to_bytes().unwrap(), before);
}
