#![cfg(feature = "write")]
use xgwx::{LadderLabelEdit, XgwxDocument, create_project};

fn edit(
    doc: &mut XgwxDocument,
    row: u32,
    expected: Option<&str>,
    replacement: Option<&str>,
) -> Result<(), xgwx::XgwxError> {
    doc.program_mut().get(0)?.edit_ld_label(&LadderLabelEdit {
        raw_y: row * 4,
        expected: expected.map(str::to_owned),
        replacement: replacement.map(str::to_owned),
    })
}

#[test]
fn labels_roundtrip_in_both_families_and_reject_stale_or_duplicate_edits() {
    for cpu in ["XGK-CPUSN", "XGI-CPUE"] {
        let mut doc = create_project(cpu, "LD").unwrap();
        edit(&mut doc, 0, None, Some("LabelOne")).unwrap();
        edit(&mut doc, 3, None, Some("한글레이블")).unwrap();
        assert_eq!(
            doc.program()
                .get(0)
                .unwrap()
                .ladder()
                .unwrap()
                .labels()
                .len(),
            2
        );
        for name in [
            "",
            "123",
            "bad space",
            "abcdefghijklmnopq",
            "가나다라마바사아자",
            "_bad",
        ] {
            let before = doc.to_bytes().unwrap();
            assert!(
                edit(&mut doc, 6, None, Some(name)).is_err(),
                "{cpu}: {name}"
            );
            assert_eq!(before, doc.to_bytes().unwrap());
        }
        assert!(edit(&mut doc, 6, None, Some("LabelOne")).is_err());
        assert!(edit(&mut doc, 0, Some("stale"), Some("Renamed")).is_err());
        edit(&mut doc, 0, Some("LabelOne"), Some("Renamed")).unwrap();
        let bytes = doc.to_bytes().unwrap();
        let mut reopened = XgwxDocument::parse(&bytes).unwrap();
        assert_eq!(
            reopened
                .program()
                .get(0)
                .unwrap()
                .ladder()
                .unwrap()
                .labels()[0]
                .name,
            "Renamed"
        );
        edit(&mut reopened, 0, Some("Renamed"), None).unwrap();
        assert_eq!(
            reopened
                .program()
                .get(0)
                .unwrap()
                .ladder()
                .unwrap()
                .labels()
                .len(),
            1
        );
    }
}

#[test]
fn native_capture_label_rows_are_recognized_and_mutable() {
    let Ok(root) = std::env::var("LIBXGWX_LABEL_CAPTURE_DIR") else {
        return;
    };
    for (file, source, row) in [
        ("XGKL.XGW", "fixtures/elements.xgwx", 1),
        ("IECL.XGW", "IEC.XGW", 5),
    ] {
        let mut doc = XgwxDocument::from_path(std::path::Path::new(&root).join(file)).unwrap();
        assert_eq!(
            doc.program().get(0).unwrap().ladder().unwrap().labels()[0].name,
            "LabelOne"
        );
        let source_path = if source == "IEC.XGW" {
            std::path::Path::new(&root).join(source)
        } else {
            std::path::PathBuf::from(source)
        };
        let mut generated = XgwxDocument::from_path(source_path).unwrap();
        edit(&mut generated, row, None, Some("LabelOne")).unwrap();
        let generated_program = generated.program().get(0).unwrap().ladder().unwrap();
        let mut actual = generated_program.data.clone();
        let expected = doc.program().get(0).unwrap().ladder().unwrap().data;
        // XG5000 normalizes the retained row display-state word on Save.
        // Its offset is row prefix +17 (record start -18), not program logic.
        for offset in 0..actual.len().saturating_sub(35) {
            if actual[offset + 4..offset + 13] == [255, 67, 0, 0, 0, 0, 0, 0, 0] {
                actual[offset + 17..offset + 21]
                    .copy_from_slice(&expected[offset + 17..offset + 21]);
            }
        }
        let diffs = actual
            .iter()
            .zip(&expected)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| (i, *a, *b))
            .take(20)
            .collect::<Vec<_>>();

        assert_eq!(actual.len(), expected.len());
        assert!(diffs.is_empty(), "native payload differences {diffs:?}");
        edit(&mut doc, row, Some("LabelOne"), Some("LabelEdited")).unwrap();
        assert_eq!(
            doc.program().get(0).unwrap().ladder().unwrap().labels()[0].name,
            "LabelEdited"
        );
    }
}

#[test]
fn only_label_can_be_deleted_and_ascii_boundary_is_exact() {
    for cpu in ["XGK-CPUSN", "XGI-CPUE"] {
        let mut doc = create_project(cpu, "LD").unwrap();
        edit(&mut doc, 0, None, Some("Abcdefghijklmnop")).unwrap();
        edit(&mut doc, 0, Some("Abcdefghijklmnop"), None).unwrap();
        assert!(
            doc.program()
                .get(0)
                .unwrap()
                .ladder()
                .unwrap()
                .labels()
                .is_empty()
        );
        edit(&mut doc, 0, None, Some("가나다라마바사아")).unwrap();
        assert!(edit(&mut doc, 2, None, Some("가나다라마바사아자")).is_err());
    }
}
