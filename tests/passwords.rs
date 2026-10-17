use xgwx::{PasswordError, PasswordScope, XgwxDocument, XgwxError};
#[cfg(feature = "write")]
const PLAIN: &[u8] = include_bytes!("../fixtures/passwords/xgk-plain.xgwx");
const PROJECT: &[u8] = include_bytes!("../fixtures/passwords/xgk-project-ascii.xgwx");
const EIGHT: &[u8] = include_bytes!("../fixtures/passwords/xgk-project-eight.xgwx");
const PROGRAM: &[u8] = include_bytes!("../fixtures/passwords/xgk-program-ascii.xgwx");
#[test]
fn project_password_requires_exact_case_and_preserves_bytes() {
    assert!(matches!(
        XgwxDocument::parse(PROJECT),
        Err(XgwxError::Password(PasswordError::Required(
            PasswordScope::Project
        )))
    ));
    assert!(matches!(
        XgwxDocument::parse_with_password(PROJECT, "ab1! x"),
        Err(XgwxError::Password(PasswordError::Incorrect(
            PasswordScope::Project
        )))
    ));
    let doc = XgwxDocument::parse_with_password(PROJECT, "Ab1! x").unwrap();
    assert!(doc.project().password_protected().unwrap());
    assert_eq!(doc.project().name(), Some("PROJPASS"));
    XgwxDocument::parse_with_password(EIGHT, "12345678").unwrap();
    #[cfg(feature = "write")]
    assert_eq!(doc.to_verified_bytes().unwrap(), PROJECT);
}
#[test]
fn program_open_reads_real_body_and_retains_protection() {
    let mut doc = XgwxDocument::parse(PROGRAM).unwrap();
    let program = doc.program().get(0).unwrap();
    assert!(program.is_locked());
    assert!(program.password_protected());
    assert!(matches!(
        program.ladder(),
        Err(XgwxError::Password(PasswordError::Required(
            PasswordScope::Program { index: 0 }
        )))
    ));
    let before = doc.clone();
    assert!(doc.unlock_program(0, "wrong").is_err());
    assert_eq!(doc, before);
    doc.unlock_program(0, "Ab1! x").unwrap();
    assert!(!doc.program().get(0).unwrap().is_locked());
    assert_eq!(
        doc.program().get(0).unwrap().ladder().unwrap().data,
        vec![0; 8]
    );
    doc.unlock_program(0, "Ab1! x").unwrap();
    #[cfg(feature = "write")]
    assert_eq!(doc.to_verified_bytes().unwrap(), PROGRAM);
}
#[cfg(feature = "write")]
#[test]
fn project_set_change_remove_preserves_source_and_trailer() {
    let mut doc = XgwxDocument::parse(PLAIN).unwrap();
    let xml = doc.container().xml().to_owned();
    let trailer = doc.container().trailer().to_vec();
    doc.project_mut()
        .set_password(None, Some("Ab1! x"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    assert_eq!(&bytes[68..134], &PROJECT[68..134]);
    let mut doc = XgwxDocument::parse_with_password(&bytes, "Ab1! x").unwrap();
    assert_eq!(doc.container().xml(), xml);
    assert_eq!(doc.container().trailer(), trailer);
    let before = doc.clone();
    assert!(doc.project_mut().set_password(Some("bad"), None).is_err());
    assert!(
        doc.project_mut()
            .set_password(Some("Ab1! x"), Some("123456789"))
            .is_err()
    );
    assert_eq!(doc, before);
    doc.project_mut()
        .set_password(Some("Ab1! x"), Some("12345678"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    assert_eq!(&bytes[68..134], &EIGHT[68..134]);
    let mut doc = XgwxDocument::parse_with_password(&bytes, "12345678").unwrap();
    doc.project_mut()
        .set_password(Some("12345678"), None)
        .unwrap();
    assert_eq!(doc.to_verified_bytes().unwrap(), PLAIN);
}
#[cfg(feature = "write")]
#[test]
fn program_set_change_remove_preserves_actual_body() {
    let mut doc = XgwxDocument::parse(PLAIN).unwrap();
    let before = doc.program().get(0).unwrap().ladder().unwrap().data;
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(None, Some("Ab1! x"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut reopened = XgwxDocument::parse(&bytes).unwrap();
    assert!(reopened.program().get(0).unwrap().is_locked());
    reopened.unlock_program(0, "Ab1! x").unwrap();
    assert_eq!(
        reopened.program().get(0).unwrap().ladder().unwrap().data,
        before
    );
    let unchanged = reopened.clone();
    assert!(
        reopened
            .program_mut()
            .get(0)
            .unwrap()
            .set_password(Some("wrong"), None)
            .is_err()
    );
    assert_eq!(reopened, unchanged);
    reopened
        .program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("Ab1! x"), Some("12345678"))
        .unwrap();
    let bytes = reopened.to_verified_bytes().unwrap();
    let mut reopened = XgwxDocument::parse(&bytes).unwrap();
    reopened.unlock_program(0, "12345678").unwrap();
    reopened
        .program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("12345678"), None)
        .unwrap();
    let plain = XgwxDocument::parse(&reopened.to_verified_bytes().unwrap()).unwrap();
    assert!(!plain.program().get(0).unwrap().password_protected());
    assert_eq!(
        plain.program().get(0).unwrap().ladder().unwrap().data,
        before
    );
    assert!(!plain.container().xml().contains("<Body2"));
}

#[test]
fn native_korean_program_password_uses_cp949_without_truncation() {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/passwords/xgk-program-korean.xgwx"
    ))
    .unwrap();
    assert!(doc.unlock_program(0, "가나다라마바사").is_err());
    doc.unlock_program(0, "가나다라마바사아").unwrap();
    assert_eq!(
        doc.program().get(0).unwrap().ladder().unwrap().data,
        vec![0; 8]
    );
}

#[cfg(feature = "write")]
#[test]
fn korean_eight_characters_and_invalid_passwords_are_transactional() {
    let mut doc = XgwxDocument::parse(PLAIN).unwrap();
    doc.project_mut()
        .set_password(None, Some("가나다라마바사아"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    assert_eq!(
        &bytes[68..84],
        &hex_bytes("F57C0F74772614EE0DC34F627B2AC44E")
    );
    let mut reopened = XgwxDocument::parse_with_password(&bytes, "가나다라마바사아").unwrap();
    for password in ["", "가나다라마바사아자", "🙂", "abc\0def", "E"] {
        let before = reopened.clone();
        assert!(
            reopened
                .project_mut()
                .set_password(Some("가나다라마바사아"), Some(password))
                .is_err()
        );
        assert_eq!(reopened, before);
    }
    reopened
        .project_mut()
        .set_password(Some("가나다라마바사아"), None)
        .unwrap();
    assert_eq!(reopened.to_verified_bytes().unwrap(), PLAIN);
}
#[cfg(feature = "write")]
fn hex_bytes(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect()
}

#[cfg(feature = "write")]
#[test]
fn editing_an_unlocked_program_updates_body2_and_keeps_both_passwords() {
    use xgwx::{LadderCellEdit, LadderEditElement, LadderEditKind};
    let mut doc = XgwxDocument::parse(PLAIN).unwrap();
    doc.project_mut()
        .set_password(None, Some("Ab1! x"))
        .unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(None, Some("12345678"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse_with_password(&bytes, "Ab1! x").unwrap();
    assert!(doc.program_mut().get(0).unwrap().ladder().is_err());
    doc.unlock_program(0, "12345678").unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .ladder()
        .unwrap()
        .edit_cell(&LadderCellEdit {
            raw_y: 0,
            column: 0,
            expected: None,
            replacement: Some(LadderEditElement {
                kind: LadderEditKind::NormallyOpen,
                operand: "M00002".into(),
            }),
        })
        .unwrap();
    let expected = doc.program().get(0).unwrap().ladder().unwrap().data;
    let bytes = doc.to_verified_bytes().unwrap();
    let mut reopened = XgwxDocument::parse_with_password(&bytes, "Ab1! x").unwrap();
    assert!(reopened.program().get(0).unwrap().is_locked());
    reopened.unlock_program(0, "12345678").unwrap();
    assert_eq!(
        reopened.program().get(0).unwrap().ladder().unwrap().data,
        expected
    );
    assert_eq!(reopened.to_verified_bytes().unwrap(), bytes);
}

#[cfg(feature = "write")]
#[test]
fn malformed_or_ambiguous_program_envelopes_do_not_unlock_or_mutate() {
    for xml in [
        "<Project><Program ObjectId='same' Encrytption='04BF8DFFE387'><Body/><Body2/></Program></Project>",
        "<Project><Program ObjectId='same' Encrytption='00'><Body></Body><Body2></Body2></Program></Project>",
        "<Project><Program ObjectId='same' Encrytption='ZZ'><Body></Body><Body2></Body2></Program></Project>",
        "<Project><Program ObjectId='same' Encrytption='04BF8DFFE387'><Body></Body></Program></Project>",
        "<Project><Program ObjectId='same' Encrytption='04BF8DFFE387'><Body></Body><Body2></Body2></Program><Program ObjectId='same'/></Project>",
    ] {
        let mut doc = XgwxDocument::parse(PLAIN).unwrap();
        doc.container_mut().replace_xml(xml).unwrap();
        let before = doc.clone();
        assert!(doc.unlock_program(0, "Ab1! x").is_err());
        assert_eq!(doc, before);
    }
}

#[cfg(feature = "write")]
#[test]
fn setting_an_unchanged_native_password_keeps_native_checksum_byte_exact() {
    for (bytes, password) in [
        (PROJECT, "Ab1! x"),
        (EIGHT, "12345678"),
        (
            include_bytes!("../fixtures/passwords/generated-both-resaved.xgwx").as_slice(),
            "Ab1! x",
        ),
    ] {
        let mut doc = XgwxDocument::parse_with_password(bytes, password).unwrap();
        doc.project_mut()
            .set_password(Some(password), Some(password))
            .unwrap();
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
}

#[test]
fn native_save_as_retains_generated_passwords_and_actual_ladder() {
    for (bytes, operand) in [
        (
            include_bytes!("../fixtures/passwords/generated-both-resaved.xgwx").as_slice(),
            "M00000",
        ),
        (
            include_bytes!("../fixtures/passwords/generated-edit-resaved.xgwx").as_slice(),
            "M00002",
        ),
    ] {
        assert!(matches!(
            XgwxDocument::parse(bytes),
            Err(XgwxError::Password(PasswordError::Required(
                PasswordScope::Project
            )))
        ));
        let mut doc = XgwxDocument::parse_with_password(bytes, "Ab1! x").unwrap();
        assert!(doc.program().get(0).unwrap().is_locked());
        doc.unlock_program(0, "12345678").unwrap();
        let body = doc.program().get(0).unwrap().ladder().unwrap();
        let operands: Vec<_> = body.elements.iter().map(|e| e.value.as_str()).collect();
        assert_eq!(operands, vec![operand, "M00001"]);
        #[cfg(feature = "write")]
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
}

#[test]
fn native_save_as_after_password_removal_has_no_protection_or_backup_body() {
    let bytes = include_bytes!("../fixtures/passwords/generated-removed-resaved.xgwx");
    let doc = XgwxDocument::parse(bytes).unwrap();
    assert!(!doc.project().password_protected().unwrap());
    assert!(!doc.program().get(0).unwrap().password_protected());
    assert!(!doc.container().xml().contains("<Body2"));
    let body = doc.program().get(0).unwrap().ladder().unwrap();
    assert_eq!(
        body.elements
            .iter()
            .map(|e| e.value.as_str())
            .collect::<Vec<_>>(),
        vec!["M00002", "M00001"]
    );
    #[cfg(feature = "write")]
    assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
}

#[cfg(feature = "write")]
#[test]
fn gzip_magic_in_password_bytes_is_not_mistaken_for_the_xml_payload() {
    let mut doc = XgwxDocument::parse(PLAIN).unwrap();
    doc.project_mut().set_password(None, Some("ZV")).unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    assert_eq!(&bytes[68..70], b"\x1f\x8b");
    assert!(matches!(
        XgwxDocument::parse(&bytes),
        Err(XgwxError::Password(PasswordError::Required(
            PasswordScope::Project
        )))
    ));
    assert_eq!(
        XgwxDocument::parse_with_password(&bytes, "ZV")
            .unwrap()
            .project()
            .name(),
        Some("PWBASE")
    );
}

#[cfg(feature = "write")]
#[test]
fn native_program_password_updates_preserve_extended_header_and_restore_plain_header() {
    let mut doc = XgwxDocument::parse(PROGRAM).unwrap();
    assert_eq!(doc.container().header().gzip_offset, 142);
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("Ab1! x"), Some("Ab1! x"))
        .unwrap();
    assert_eq!(doc.to_verified_bytes().unwrap(), PROGRAM);
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("Ab1! x"), None)
        .unwrap();
    assert_eq!(doc.container().header().gzip_offset, 138);
    XgwxDocument::parse(&doc.to_verified_bytes().unwrap()).unwrap();
}

fn iec_native_samples() -> [&'static [u8]; 4] {
    [
        include_bytes!("../fixtures/passwords/xgi-ld-native.xgwx"),
        include_bytes!("../fixtures/passwords/xgi-st-native.xgwx"),
        include_bytes!("../fixtures/passwords/xgi-il-native.xgwx"),
        include_bytes!("../fixtures/passwords/xgi-sfc-native.xgwx"),
    ]
}

#[test]
fn native_iec_single_body_programs_authenticate_without_swapping_source() {
    for bytes in iec_native_samples() {
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        let original_xml = doc.container().xml().to_owned();
        let before = doc.clone();
        assert!(doc.program().get(0).unwrap().declarations().is_err());
        assert!(doc.unlock_program(0, "wrong").is_err());
        assert_eq!(doc, before);
        doc.unlock_program(0, "Ab1! x").unwrap();
        assert!(!doc.program().get(0).unwrap().is_locked());
        assert_eq!(doc.container().xml(), original_xml);
        let program = doc.program().get(0).unwrap();
        if program.summary().kind == Some(3) {
            program.sfc().unwrap();
        } else if program.summary().kind == Some(0) {
            assert_eq!(program.ladder().unwrap().project_type, Some(2));
            program.variables().unwrap();
        } else {
            program.declarations().unwrap();
            assert!(program.text().unwrap().source.unwrap().contains("%MX0"));
        }
        #[cfg(feature = "write")]
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
}

#[cfg(feature = "write")]
#[test]
fn iec_program_password_set_change_remove_preserves_source_and_local_variables() {
    for language in ["LD", "ST", "IL", "SFC"] {
        let mut doc = xgwx::create_project("XGI-CPUE", language).unwrap();
        let original_xml = doc.container().xml().to_owned();
        doc.program_mut()
            .get(0)
            .unwrap()
            .set_password(None, Some("Ab1! x"))
            .unwrap();
        assert!(!doc.container().xml().contains("<Body2"));
        let bytes = doc.to_verified_bytes().unwrap();
        let mut doc = XgwxDocument::parse(&bytes).unwrap();
        let before = doc.clone();
        for (current, replacement) in [
            (None, Some("12345678")),
            (Some("wrong"), None),
            (Some("Ab1! x"), Some("")),
        ] {
            assert!(
                doc.program_mut()
                    .get(0)
                    .unwrap()
                    .set_password(current, replacement)
                    .is_err()
            );
            assert_eq!(doc, before);
        }
        // Changing a password on a locked program authenticates atomically.
        doc.program_mut()
            .get(0)
            .unwrap()
            .set_password(Some("Ab1! x"), Some("12345678"))
            .unwrap();
        let bytes = doc.to_verified_bytes().unwrap();
        let mut doc = XgwxDocument::parse(&bytes).unwrap();
        assert!(doc.unlock_program(0, "Ab1! x").is_err());
        doc.program_mut()
            .get(0)
            .unwrap()
            .set_password(Some("12345678"), None)
            .unwrap();
        assert_eq!(doc.container().header().gzip_offset, 138);
        assert_eq!(doc.container().xml(), original_xml);
        assert!(
            !XgwxDocument::parse(&doc.to_verified_bytes().unwrap())
                .unwrap()
                .program()
                .get(0)
                .unwrap()
                .is_locked()
        );
    }
}

#[cfg(feature = "write")]
#[test]
fn multiple_program_passwords_follow_identity_and_keep_header_until_last_removal() {
    let mut doc = xgwx::create_project("XGI-CPUE", "ST").unwrap();
    doc.program_mut()
        .create(&xgwx::NewProgram {
            name: "Second".into(),
            language: "IL".into(),
            object_id: "aaaa1111-2222-4333-8444-555555555555".into(),
            symbol_id: "bbbb1111-2222-4333-8444-555555555555".into(),
        })
        .unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(None, Some("first"))
        .unwrap();
    doc.program_mut()
        .get(1)
        .unwrap()
        .set_password(None, Some("second"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    doc.unlock_program(1, "second").unwrap();
    doc.program_mut().get(1).unwrap().move_to(0).unwrap();
    assert!(!doc.program().get(0).unwrap().is_locked());
    assert!(doc.program().get(1).unwrap().is_locked());
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("second"), None)
        .unwrap();
    assert_eq!(doc.container().header().gzip_offset, 142);
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    assert!(!doc.program().get(0).unwrap().password_protected());
    assert!(doc.program().get(1).unwrap().is_locked());
    doc.program_mut()
        .get(1)
        .unwrap()
        .set_password(Some("first"), None)
        .unwrap();
    assert_eq!(doc.container().header().gzip_offset, 138);
}

#[cfg(feature = "write")]
#[test]
fn protected_iec_text_edits_preserve_password_and_hide_locked_source_and_declarations() {
    let mut doc = xgwx::create_project("XGI-CPUE", "ST").unwrap();
    doc.program_mut()
        .get(0)
        .unwrap()
        .set_password(None, Some("first"))
        .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    let snapshot = &doc.program().text_programs()[0];
    assert!(snapshot.source.is_none());
    assert!(snapshot.variables.is_empty());
    assert!(!snapshot.editable);
    assert!(snapshot.variables_error.is_some());
    assert!(doc.program_mut().get(0).unwrap().text().is_err());
    doc.unlock_program(0, "first").unwrap();
    let mut program = doc.program_mut().get(0).unwrap();
    let mut text = program.text().unwrap();
    let expected_source = text.source().unwrap().unwrap();
    let source = "%MX1 := NOT %MX0;\r\n";
    text.update(&xgwx::TextSourcePatch {
        expected_source,
        source: source.into(),
    })
    .unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let mut doc = XgwxDocument::parse(&bytes).unwrap();
    assert!(doc.program().get(0).unwrap().is_locked());
    doc.unlock_program(0, "first").unwrap();
    assert_eq!(
        doc.program()
            .get(0)
            .unwrap()
            .text()
            .unwrap()
            .source
            .as_deref(),
        Some(source)
    );
}

#[cfg(feature = "write")]
#[test]
fn unknown_single_body_layouts_are_rejected_atomically() {
    let plain = xgwx::create_project("XGI-CPUE", "ST").unwrap();
    for xml in [
        plain.container().xml().replace("Kind=\"4\"", "Kind=\"99\""),
        plain
            .container()
            .xml()
            .replace("ST_Program", "Unknown_Program"),
    ] {
        let mut doc = plain.clone();
        doc.container_mut().replace_xml(xml).unwrap();
        let before = doc.clone();
        assert!(matches!(
            doc.program_mut()
                .get(0)
                .unwrap()
                .set_password(None, Some("first")),
            Err(XgwxError::Password(PasswordError::UnsupportedLayout))
        ));
        assert_eq!(doc, before);
    }
}

#[test]
fn native_iec_save_as_preserves_set_change_and_remove_for_all_languages() {
    let cases: [(&[u8], Option<&str>); 3] = [
        (
            include_bytes!("../fixtures/passwords/xgi-set-resaved.xgwx"),
            Some("Ab1! x"),
        ),
        (
            include_bytes!("../fixtures/passwords/xgi-change-resaved.xgwx"),
            Some("12345678"),
        ),
        (
            include_bytes!("../fixtures/passwords/xgi-remove-resaved.xgwx"),
            None,
        ),
    ];
    let mut reference_ladder = None;
    let mut reference_sfc = None;
    for (bytes, password) in cases {
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        assert_eq!(doc.program().programs().len(), 4);
        assert_eq!(
            doc.container().header().gzip_offset,
            if password.is_some() { 142 } else { 138 }
        );
        for i in 0..4 {
            assert_eq!(
                doc.program().get(i).unwrap().is_locked(),
                password.is_some()
            );
            if let Some(password) = password {
                assert!(doc.unlock_program(i, "wrong").is_err());
                doc.unlock_program(i, password).unwrap();
            }
        }
        let ladder = doc.program().get(0).unwrap().ladder().unwrap().data;
        assert_eq!(
            &ladder,
            reference_ladder.get_or_insert_with(|| ladder.clone())
        );
        assert_eq!(
            doc.program()
                .get(1)
                .unwrap()
                .text()
                .unwrap()
                .source
                .as_deref(),
            Some("%MX1 := %MX0;\r\n")
        );
        assert_eq!(
            doc.program()
                .get(2)
                .unwrap()
                .text()
                .unwrap()
                .source
                .as_deref(),
            Some("LD %MX0\r\nST %MX1\r\n")
        );
        let sfc = doc.program().get(3).unwrap().sfc().unwrap();
        assert_eq!(&sfc, reference_sfc.get_or_insert_with(|| sfc.clone()));
        #[cfg(feature = "write")]
        assert_eq!(doc.to_verified_bytes().unwrap(), bytes);
    }
}

#[cfg(feature = "write")]
#[test]
fn session_reparse_inherits_only_unchanged_credentials_and_unique_program_identities() {
    let mut doc = XgwxDocument::parse(PROGRAM).unwrap();
    doc.unlock_program(0, "Ab1! x").unwrap();
    let bytes = doc.to_verified_bytes().unwrap();
    let inherited = XgwxDocument::parse_in_session(&bytes, &doc).unwrap();
    assert!(!inherited.program().get(0).unwrap().is_locked());
    let mut changed = doc.clone();
    changed
        .program_mut()
        .get(0)
        .unwrap()
        .set_password(Some("Ab1! x"), Some("12345678"))
        .unwrap();
    let bytes = changed.to_verified_bytes().unwrap();
    assert!(
        XgwxDocument::parse_in_session(&bytes, &doc)
            .unwrap()
            .program()
            .get(0)
            .unwrap()
            .is_locked()
    );
    doc.project_mut().set_password(None, Some("first")).unwrap();
    let mut changed = doc.clone();
    changed
        .project_mut()
        .set_password(Some("first"), Some("second"))
        .unwrap();
    assert!(XgwxDocument::parse_in_session(&changed.to_verified_bytes().unwrap(), &doc).is_err());
    assert!(XgwxDocument::parse_in_session(&doc.to_verified_bytes().unwrap(), &doc).is_ok());
}

#[cfg(feature = "write")]
#[test]
fn native_iec_version_three_headers_support_password_edits_and_gzip_like_credentials() {
    for bytes in iec_native_samples() {
        assert_eq!(bytes[4], 3);
        let mut doc = XgwxDocument::parse(bytes).unwrap();
        doc.program_mut()
            .get(0)
            .unwrap()
            .set_password(Some("Ab1! x"), Some("nextpass"))
            .unwrap();
        doc.project_mut().set_password(None, Some("ZV")).unwrap();
        let bytes = doc.to_verified_bytes().unwrap();
        assert_eq!(&bytes[68..70], b"\x1f\x8b");
        assert!(matches!(
            XgwxDocument::parse(&bytes),
            Err(XgwxError::Password(PasswordError::Required(
                PasswordScope::Project
            )))
        ));
        let mut doc = XgwxDocument::parse_with_password(&bytes, "ZV").unwrap();
        doc.program_mut()
            .get(0)
            .unwrap()
            .set_password(Some("nextpass"), None)
            .unwrap();
        doc.project_mut().set_password(Some("ZV"), None).unwrap();
        let bytes = doc.to_verified_bytes().unwrap();
        assert_eq!(bytes[4], 3);
        assert_eq!(doc.container().header().gzip_offset, 138);
        assert!(
            !XgwxDocument::parse(&bytes)
                .unwrap()
                .program()
                .get(0)
                .unwrap()
                .is_locked()
        );
    }
}
