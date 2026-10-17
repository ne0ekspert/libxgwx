use flate2::{Compression, write::GzEncoder};
use std::{
    io::Write,
    time::{Duration, Instant},
};
use xgwx::{PasswordError, PasswordScope, XgwxDocument, XgwxError};

fn bytes(xml: &str) -> Vec<u8> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes
}

#[test]
fn bulk_program_inspection_does_not_restart_indexed_lock_checks() {
    for count in [10_000, 20_000] {
        let doc = XgwxDocument::parse(&bytes(&format!(
            "<Project>{}</Project>",
            "<Program/>".repeat(count)
        )))
        .unwrap();
        let start = Instant::now();
        assert!(doc.program().sfc_programs().is_empty());
        let elapsed = start.elapsed();
        eprintln!("{count} program SFC enumeration: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(5),
            "enumeration took {elapsed:?}"
        );
    }
}

#[test]
fn large_empty_and_text_batches_keep_all_program_slots() {
    for entry in [
        "<Program/>",
        "<Program Kind=\"4\"/>",
        "<Program Kind=\"9\"/>",
    ] {
        let count = 100_000;
        let input = bytes(&format!("<Project>{}</Project>", entry.repeat(count)));
        let doc = XgwxDocument::parse(&input).unwrap();
        let start = Instant::now();
        assert!(doc.program().sfc_programs().is_empty());
        let locals = doc.variable().iec_local_symbols();
        assert_eq!(locals.len(), count);
        assert!(locals.iter().all(|v| v.as_ref().is_ok_and(Vec::is_empty)));
        assert_eq!(doc.program().ladder_programs().len(), count);
        assert_eq!(
            doc.program().text_programs().len(),
            if entry == "<Program/>" { 0 } else { count }
        );
        let restored = XgwxDocument::parse_in_session(&input, &doc).unwrap();
        assert_eq!(restored.program().programs().len(), count);
        let elapsed = start.elapsed();
        eprintln!("100000 {entry} bulk inspection: {elapsed:?}");
        assert!(
            elapsed < Duration::from_secs(10),
            "inspection took {elapsed:?}"
        );
    }
}

#[test]
fn locked_bulk_slots_keep_original_indices_and_hide_sources() {
    let doc = XgwxDocument::parse(&bytes("<Project><Program/><Group><Program Kind='4' Encrytption='ZZ'/><Program Kind='9' ObjectId='locked' Encrytption='00'/></Group><Program/></Project>")).unwrap();
    let locals = doc.variable().iec_local_symbols();
    let ladder = doc.program().ladder_programs();
    assert_eq!(locals.len(), 4);
    for index in [1, 2] {
        for result in [
            locals[index].as_ref().map(|_| ()),
            ladder[index].as_ref().map(|_| ()),
        ] {
            assert!(
                matches!(result, Err(XgwxError::Password(PasswordError::Required(PasswordScope::Program {index:i}))) if *i == index)
            );
        }
    }
    assert!(locals[0].as_ref().unwrap().is_empty());
    assert!(locals[3].as_ref().unwrap().is_empty());
    for text in doc.program().text_programs() {
        assert!([1, 2].contains(&text.program_index));
        assert!(text.source.is_none() && !text.editable && text.variables.is_empty());
        assert_eq!(
            text.variables_error,
            Some(
                XgwxError::Password(PasswordError::Required(PasswordScope::Program {
                    index: text.program_index
                }))
                .to_string()
            )
        );
    }
}

#[test]
fn authenticated_native_bulk_readers_match_selected_programs_and_session_reparse() {
    for bytes in [
        include_bytes!("../fixtures/passwords/xgi-ld-native.xgwx").as_slice(),
        include_bytes!("../fixtures/passwords/xgi-st-native.xgwx").as_slice(),
        include_bytes!("../fixtures/passwords/xgi-il-native.xgwx").as_slice(),
        include_bytes!("../fixtures/passwords/xgi-sfc-native.xgwx").as_slice(),
    ] {
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        assert!(doc.variable().iec_local_symbols()[0].is_err());
        assert!(doc.program().ladder_programs()[0].is_err());
        assert!(doc.program().sfc_programs().is_empty());
        assert!(
            doc.program()
                .text_programs()
                .iter()
                .all(|text| text.source.is_none())
        );
        doc.unlock_program(0, "Ab1! x").unwrap();
        let selected = doc.program().get(0).unwrap();
        match selected.summary().kind {
            Some(3) => assert_eq!(doc.program().sfc_programs()[0], selected.sfc().unwrap()),
            Some(4 | 9) => assert_eq!(doc.program().text_programs()[0], selected.text().unwrap()),
            _ => assert_eq!(
                doc.program().ladder_programs()[0].as_ref().unwrap().data,
                selected.ladder().unwrap().data
            ),
        }
        let restored = XgwxDocument::parse_in_session(bytes, &doc).unwrap();
        assert!(!restored.program().get(0).unwrap().is_locked());
        assert_eq!(
            restored.program().text_programs(),
            doc.program().text_programs()
        );
        assert_eq!(
            restored.program().sfc_programs(),
            doc.program().sfc_programs()
        );
    }
}
