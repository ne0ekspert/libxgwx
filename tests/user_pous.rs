#![cfg(feature = "write")]
use base64::Engine;
use std::io::Read;
use xgwx::*;
fn decode_base64_payload(s: &str, compressed: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let b = base64::engine::general_purpose::STANDARD
        .decode(s.chars().filter(|c| !c.is_whitespace()).collect::<String>())?;
    if !compressed {
        return Ok(b);
    }
    let mut out = Vec::new();
    bzip2::read::BzDecoder::new(b.as_slice()).read_to_end(&mut out)?;
    Ok(out)
}
fn doc() -> XgwxDocument {
    create_project("XGI-CPUE", "ST").unwrap()
}
fn native() -> XgwxDocument {
    let mut d = doc();
    let xml = d.container().xml().replace(
        "<UserFunctions></UserFunctions>",
        include_str!("../fixtures/user-pous/native-definitions.xml"),
    );
    d.container_mut().replace_xml(xml).unwrap();
    d
}
fn var(name: &str, scope: UserPouScope, ty: &str) -> UserPouVariable {
    UserPouVariable {
        name: name.into(),
        scope,
        data_type: ty.into(),
        description: String::new(),
    }
}
fn request(kind: UserPouKind) -> NewUserPou {
    NewUserPou {
        name: if kind == UserPouKind::Function {
            "Scale"
        } else {
            "Accumulator"
        }
        .into(),
        kind,
        object_id: "f0000000-0000-4000-8000-000000000001".into(),
        symbol_id: "f0000000-0000-4000-8000-000000000002".into(),
        return_type: (kind == UserPouKind::Function).then(|| "BOOL".into()),
        variables: vec![
            var("InputValue", UserPouScope::Input, "DINT"),
            var("OutputValue", UserPouScope::Output, "DINT"),
        ],
        source: if kind == UserPouKind::Function {
            "OutputValue := InputValue;\r\nScale := TRUE;"
        } else {
            "OutputValue := InputValue;"
        }
        .into(),
        comment: "A < B & C".into(),
    }
}
#[test]
fn reads_native_uf50_and_ub50() {
    let d = native();
    let p = d.user_pou().list();
    assert_eq!(p.len(), 2);
    assert!(p.iter().all(|p| p.editable));
    assert_eq!(p[0].name, "NativeFunction");
    assert_eq!(
        p[0].variables
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        ["InputValue", "SharedValue", "NativeFunction", "OutputValue"]
    );
    assert_eq!(p[1].variables.len(), 2);
    assert_eq!(p[0].return_type.as_deref(), Some("BOOL"));
}
#[test]
fn creates_round_trips_and_preserves_scan_programs() {
    for kind in [UserPouKind::Function, UserPouKind::FunctionBlock] {
        let mut d = doc();
        let before = d.program().list();
        let r = request(kind);
        d.user_pou_mut().create(&r).unwrap();
        let d = XgwxDocument::parse(&d.to_verified_bytes().unwrap()).unwrap();
        let p = &d.user_pou().list()[0];
        assert!(p.editable, "{:?}", p);
        assert_eq!(p.source.as_deref(), Some(r.source.as_str()));
        assert_eq!(p.comment, r.comment);
        assert_eq!(d.program().list(), before);
        assert_eq!(
            p.variables
                .iter()
                .filter(|v| v.scope == UserPouScope::Return)
                .count(),
            usize::from(kind == UserPouKind::Function)
        );
    }
}
#[test]
fn creation_matches_native_symbol_bytes_and_mirrored_interfaces() {
    let n = native();
    for p in n.user_pou().list() {
        let mut d = doc();
        let mut r = request(p.kind.unwrap());
        r.name = p.name.clone();
        r.variables = p
            .variables
            .iter()
            .filter(|v| v.scope != UserPouScope::Return)
            .cloned()
            .collect();
        r.source = String::new();
        r.return_type = p.return_type.clone();
        d.user_pou_mut().create(&r).unwrap();
        let a = roxmltree::Document::parse(d.container().xml()).unwrap();
        let b = roxmltree::Document::parse(n.container().xml()).unwrap();
        let a = a
            .descendants()
            .find(|n| n.has_tag_name("UserFunction"))
            .unwrap();
        let b = b
            .descendants()
            .filter(|n| n.has_tag_name("UserFunction"))
            .nth(p.index)
            .unwrap();
        let symbols = |n: roxmltree::Node<'_, '_>| {
            let s = n.descendants().find(|n| n.has_tag_name("Symbols")).unwrap();
            decode_base64_payload(
                s.text().unwrap_or(""),
                s.attribute("Compressed") == Some("1"),
            )
            .unwrap()
        };
        assert_eq!(symbols(a), symbols(b));
        assert_eq!(a.attribute("Size"), b.attribute("Size"));
        for tag in ["InputVariables", "OutputVariables"] {
            let read = |node: roxmltree::Node<'_, '_>| {
                node.children()
                    .find(|n| n.has_tag_name(tag))
                    .unwrap()
                    .children()
                    .filter(|n| n.is_element())
                    .map(|n| {
                        (
                            n.text().unwrap().to_owned(),
                            n.attribute("Type").unwrap().to_owned(),
                            n.attribute("DataType").unwrap().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(read(a), read(b));
        }
    }
}
#[test]
fn stale_source_and_identity_are_atomic() {
    let mut d = doc();
    d.user_pou_mut()
        .create(&request(UserPouKind::Function))
        .unwrap();
    let old = d.user_pou().list()[0].clone();
    d.user_pou_mut()
        .get(0, &old.object_id)
        .unwrap()
        .set_source(&old, "Scale := FALSE;")
        .unwrap();
    let before = d.to_bytes().unwrap();
    assert!(
        d.user_pou_mut()
            .get(0, &old.object_id)
            .unwrap()
            .set_source(&old, "wrong")
            .is_err()
    );
    assert!(d.user_pou_mut().get(0, "wrong").is_err());
    assert_eq!(d.to_bytes().unwrap(), before);
}
#[test]
fn duplicate_names_ids_and_invalid_parameters_are_atomic() {
    let mut d = doc();
    d.user_pou_mut()
        .create(&request(UserPouKind::Function))
        .unwrap();
    let before = d.to_bytes().unwrap();
    for mut r in [
        request(UserPouKind::Function),
        request(UserPouKind::FunctionBlock),
    ] {
        assert!(d.user_pou_mut().create(&r).is_err());
        r.object_id = "f0000000-0000-4000-8000-000000000003".into();
        r.symbol_id = "f0000000-0000-4000-8000-000000000004".into();
        r.variables
            .push(var("inputvalue", UserPouScope::Input, "DINT"));
        assert!(d.user_pou_mut().create(&r).is_err());
        assert_eq!(d.to_bytes().unwrap(), before);
    }
}
#[test]
fn replace_interface_and_delete_unused_definition() {
    let mut d = doc();
    d.user_pou_mut()
        .create(&request(UserPouKind::FunctionBlock))
        .unwrap();
    let p = d.user_pou().list()[0].clone();
    let mut vars = p.variables.clone();
    vars.push(var("Memory", UserPouScope::Local, "DINT"));
    d.user_pou_mut()
        .get(0, &p.object_id)
        .unwrap()
        .set_variables(&p, &vars)
        .unwrap();
    let p = d.user_pou().list()[0].clone();
    assert_eq!(p.variables.len(), 3);
    d.user_pou_mut()
        .get(0, &p.object_id)
        .unwrap()
        .delete(&p)
        .unwrap();
    assert!(d.user_pou().list().is_empty());
}
#[test]
fn calls_protect_definition_deletion_and_interface_changes() {
    let mut d = doc();
    d.user_pou_mut()
        .create(&request(UserPouKind::Function))
        .unwrap();
    let p = d.user_pou().list()[0].clone();
    let text = d.program().get(0).unwrap().text().unwrap();
    d.program_mut()
        .get(0)
        .unwrap()
        .text()
        .unwrap()
        .update(&TextSourcePatch {
            expected_source: text.source.unwrap(),
            source: "Scale(InputValue := 1);".into(),
        })
        .unwrap();
    let before = d.to_bytes().unwrap();
    assert!(
        d.user_pou_mut()
            .get(0, &p.object_id)
            .unwrap()
            .delete(&p)
            .is_err()
    );
    let mut vars = p.variables.clone();
    vars.push(var("More", UserPouScope::Local, "INT"));
    assert!(
        d.user_pou_mut()
            .get(0, &p.object_id)
            .unwrap()
            .set_variables(&p, &vars)
            .is_err()
    );
    assert_eq!(d.to_bytes().unwrap(), before);
}

#[test]
fn native_check_and_save_as_preserve_complete_function_and_fb_project() {
    let generated =
        XgwxDocument::parse(include_bytes!("../fixtures/user-pous/generated.xgwx")).unwrap();
    let saved = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/native-roundtrip.xgwx"
    ))
    .unwrap();
    let a = generated.user_pou().list();
    let b = saved.user_pou().list();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 2);
    for (a, b) in a.iter().zip(&b) {
        assert!(b.editable, "{:?}", b);
        assert_eq!(a.name, b.name);
        assert_eq!(a.object_id, b.object_id);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.source, b.source);
        assert_eq!(a.variables, b.variables);
        assert_eq!(a.return_type, b.return_type);
    }
    assert_eq!(
        generated.program().get(0).unwrap().text().unwrap().source,
        saved.program().get(0).unwrap().text().unwrap().source
    );
    let vars = saved.program().get(0).unwrap().declarations().unwrap();
    assert!(
        vars.iter()
            .any(|v| v.name == "Counter" && v.data_type == "UserAccumulator")
    );
    let before = saved.to_bytes().unwrap();
    let mut edited = saved.clone();
    let p = edited.user_pou().list()[1].clone();
    assert!(
        edited
            .user_pou_mut()
            .get(1, &p.object_id)
            .unwrap()
            .delete(&p)
            .is_err()
    );
    assert_eq!(edited.to_bytes().unwrap(), before);
}

#[test]
fn all_scalar_allocations_match_native_save_as_byte_for_byte() {
    let generated = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/scalars-generated.xgwx"
    ))
    .unwrap();
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/scalars-native-roundtrip.xgwx"
    ))
    .unwrap();
    let expected = generated.user_pou().list();
    let actual = native.user_pou().list();
    assert_eq!(expected.len(), 22);
    assert_eq!(actual.len(), 22);
    let a = roxmltree::Document::parse(generated.container().xml()).unwrap();
    let b = roxmltree::Document::parse(native.container().xml()).unwrap();
    for ((a, b), (expected, actual)) in a
        .descendants()
        .filter(|n| n.has_tag_name("UserFunction"))
        .zip(b.descendants().filter(|n| n.has_tag_name("UserFunction")))
        .zip(expected.iter().zip(&actual))
    {
        assert!(
            actual.editable,
            "{} became read only: {:?}",
            actual.name, actual.reason
        );
        assert_eq!(
            expected.variables, actual.variables,
            "{} variables",
            expected.name
        );
        assert_eq!(
            a.attribute("Size"),
            b.attribute("Size"),
            "{} native size",
            expected.name
        );
        let decoded = |n: roxmltree::Node<'_, '_>| {
            let s = n.descendants().find(|n| n.has_tag_name("Symbols")).unwrap();
            decode_base64_payload(
                s.text().unwrap_or(""),
                s.attribute("Compressed") == Some("1"),
            )
            .unwrap()
        };
        assert_eq!(
            decoded(a),
            decoded(b),
            "{} native symbol allocation",
            expected.name
        );
    }
}

#[test]
fn editing_compiled_definitions_invalidates_and_regenerates_native_temporaries() {
    let mut d = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/native-roundtrip.xgwx"
    ))
    .unwrap();
    let sources = [
        "UserDouble := (Value * 2) + 1;\r\n",
        "IF Reset THEN\r\n    Total := 0;\r\nELSE\r\n    Total := Total + Increment * 2;\r\nEND_IF;\r\n",
    ];
    for (index, source) in sources.iter().enumerate() {
        let p = d.user_pou().list()[index].clone();
        d.user_pou_mut()
            .get(index, &p.object_id)
            .unwrap()
            .set_source(&p, source)
            .unwrap();
    }
    let parsed = roxmltree::Document::parse(d.container().xml()).unwrap();
    assert!(
        parsed
            .descendants()
            .filter(|n| n.has_tag_name("UserFunction"))
            .all(|n| n
                .descendants()
                .find(|n| n.has_tag_name("TempVar"))
                .unwrap()
                .attribute("Count")
                == Some("0"))
    );
    let native = XgwxDocument::parse(include_bytes!(
        "../fixtures/user-pous/edited-native-roundtrip.xgwx"
    ))
    .unwrap();
    for (a, b) in d.user_pou().list().iter().zip(native.user_pou().list()) {
        assert!(b.editable);
        assert_eq!(a.source, b.source);
        assert_eq!(a.variables, b.variables);
    }
}

#[test]
fn unsupported_interfaces_and_referenced_local_changes_remain_atomic() {
    let mut d = doc();
    d.user_pou_mut()
        .create(&request(UserPouKind::FunctionBlock))
        .unwrap();
    let p = d.user_pou().list()[0].clone();
    let before = d.to_bytes().unwrap();
    let changed = p
        .variables
        .iter()
        .filter(|v| v.name != "InputValue")
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        d.user_pou_mut()
            .get(0, &p.object_id)
            .unwrap()
            .set_variables(&p, &changed)
            .is_err()
    );
    assert_eq!(d.to_bytes().unwrap(), before);
    let xml = d
        .container()
        .xml()
        .replacen("InstMemory=\"0\"", "InstMemory=\"1\"", 1);
    d.container_mut().replace_xml(xml).unwrap();
    let p = d.user_pou().list()[0].clone();
    assert!(!p.editable);
    assert!(d.user_pou_mut().get(0, &p.object_id).is_err());
    let mut d = doc();
    let xml = d
        .container()
        .xml()
        .replace("<UserFunctions></UserFunctions>", "<UserFunctions/>");
    d.container_mut().replace_xml(xml).unwrap();
    d.user_pou_mut()
        .create(&request(UserPouKind::FunctionBlock))
        .unwrap();
    assert_eq!(d.user_pou().list().len(), 1);
}
