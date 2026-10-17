//! Generate a complete ST user-function/FB project for native XG5000 acceptance.
use xgwx::*;
fn variable(name: &str, scope: UserPouScope, ty: &str) -> UserPouVariable {
    UserPouVariable {
        name: name.into(),
        scope,
        data_type: ty.into(),
        description: String::new(),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: user_pou_acceptance OUTPUT.xgwx")?,
    );
    let args: Vec<_> = std::env::args().collect();
    if args.get(2).is_some_and(|s| s == "--edit-native") {
        let mut d =
            XgwxDocument::from_path(args.get(3).ok_or("--edit-native requires an input path")?)?;
        for (index, source) in [
            (0, "UserDouble := (Value * 2) + 1;\r\n"),
            (
                1,
                "IF Reset THEN\r\n    Total := 0;\r\nELSE\r\n    Total := Total + Increment * 2;\r\nEND_IF;\r\n",
            ),
        ] {
            let p = d.user_pou().list()[index].clone();
            d.user_pou_mut()
                .get(index, &p.object_id)?
                .set_source(&p, source)?;
        }
        std::fs::write(out, d.to_verified_bytes()?)?;
        println!("Edited both native compiled definitions and invalidated their temporary caches.");
        return Ok(());
    }
    let mut d = create_project("XGI-CPUE", "ST")?;
    d.user_pou_mut().create(&NewUserPou {
        name: "UserDouble".into(),
        kind: UserPouKind::Function,
        object_id: "11111111-1111-4111-8111-111111111111".into(),
        symbol_id: "22222222-2222-4222-8222-222222222222".into(),
        return_type: Some("DINT".into()),
        variables: vec![variable("Value", UserPouScope::Input, "DINT")],
        source: "UserDouble := Value * 2;\r\n".into(),
        comment: "Doubles an integer".into(),
    })?;
    d.user_pou_mut().create(&NewUserPou{name:"UserAccumulator".into(),kind:UserPouKind::FunctionBlock,object_id:"33333333-3333-4333-8333-333333333333".into(),symbol_id:"44444444-4444-4444-8444-444444444444".into(),return_type:None,variables:vec![variable("Reset",UserPouScope::Input,"BOOL"),variable("Increment",UserPouScope::Input,"DINT"),variable("Total",UserPouScope::Output,"DINT")],source:"IF Reset THEN\r\n    Total := 0;\r\nELSE\r\n    Total := Total + Increment;\r\nEND_IF;\r\n".into(),comment:"State retained by each instance between calls".into()})?;
    if std::env::args().any(|s| s == "--all-types") {
        let types = [
            "BOOL",
            "BYTE",
            "WORD",
            "DWORD",
            "LWORD",
            "SINT",
            "INT",
            "DINT",
            "LINT",
            "USINT",
            "UINT",
            "UDINT",
            "ULINT",
            "REAL",
            "LREAL",
            "TIME",
            "DATE",
            "TIME_OF_DAY",
            "DATE_AND_TIME",
        ];
        for (i, ty) in types.iter().enumerate() {
            let name = format!("UserCopy{ty}");
            d.user_pou_mut().create(&NewUserPou {
                name: name.clone(),
                kind: UserPouKind::Function,
                object_id: format!("aaaa0000-0000-4000-8000-{:012}", i * 2 + 1),
                symbol_id: format!("aaaa0000-0000-4000-8000-{:012}", i * 2 + 2),
                return_type: Some((*ty).into()),
                variables: vec![
                    variable("Value", UserPouScope::Input, ty),
                    variable("Second", UserPouScope::Input, ty),
                    variable("Shared", UserPouScope::InOut, ty),
                ],
                source: format!("{name} := Value;\r\n"),
                comment: String::new(),
            })?;
        }
        let mut variables: Vec<_> = types
            .iter()
            .map(|ty| variable(&format!("Value{ty}"), UserPouScope::Input, ty))
            .collect();
        for ty in types {
            variables.push(variable(&format!("Shared{ty}"), UserPouScope::InOut, ty));
            variables.push(variable(&format!("Output{ty}"), UserPouScope::Output, ty));
        }
        variables.push(variable("ExtraBool", UserPouScope::Input, "BOOL"));
        variables.push(variable("Out", UserPouScope::Output, "BOOL"));
        variables.push(variable("Private", UserPouScope::Local, "INT"));
        d.user_pou_mut().create(&NewUserPou {
            name: "UserAllScalars".into(),
            kind: UserPouKind::FunctionBlock,
            object_id: "bbbb0000-0000-4000-8000-000000000001".into(),
            symbol_id: "bbbb0000-0000-4000-8000-000000000002".into(),
            return_type: None,
            variables,
            source: "Private := 1;\r\nOut := ValueBOOL;\r\n".into(),
            comment: String::new(),
        })?;
    }
    for (name, ty) in [
        ("Result", "DINT"),
        ("Observed", "DINT"),
        ("Counter", "UserAccumulator"),
    ] {
        let mut p = d.program_mut().get(0)?;
        let expected_variables = p.view().declarations()?;
        p.variable_mut().edit_declaration(&ProgramVariablePatch {
            expected_variables,
            name: name.into(),
            data_type: ty.into(),
            description: String::new(),
            remove: false,
            update: false,
            declaration: None,
        })?;
    }
    d.program_mut().get(0)?.text()?.update(&TextSourcePatch{expected_source:String::new(),source:"Result := UserDouble(Value := 4);\r\nCounter(Reset := FALSE, Increment := Result);\r\nObserved := Counter.Total;\r\n".into()})?;
    std::fs::write(out, d.to_verified_bytes()?)?;
    println!("Created a function, a function block, a custom FB instance, and ST calls.");
    Ok(())
}
