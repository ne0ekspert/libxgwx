use xgwx::{
    NewProgram, SfcArrayBound, SfcDeclaration, SfcVariablePatch, TextProgramPatch, XgwxDocument,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/text-programs/native-source.xgwx"
    ))?;
    // Keep only the captured ST and IL programs; the capture project's SFC was blank.
    for _ in 0..2 {
        let id = doc.programs()[0].object_id.clone().unwrap();
        doc.delete_program(0, &id)?;
    }
    for (i, language) in ["ST", "IL"].iter().enumerate() {
        doc.create_program(&NewProgram {
            name: format!("Created{language}"),
            language: (*language).into(),
            object_id: format!("abcd000{}-1234-4567-89ab-0123456789ab", i * 2 + 1),
            symbol_id: format!("abcd000{}-1234-4567-89ab-0123456789ab", i * 2 + 2),
        })?;
    }
    for index in 0..4 {
        let text = doc
            .text_programs()
            .into_iter()
            .find(|p| p.program_index == index)
            .unwrap();
        let declarations = if text.language == "ST" {
            vec![
                (
                    "Count",
                    "INT",
                    Some(SfcDeclaration {
                        initial_value: "7".into(),
                        retain: true,
                        dimensions: vec![],
                    }),
                ),
                ("Delay", "TON", None),
                (
                    "Samples",
                    "INT",
                    Some(SfcDeclaration {
                        dimensions: vec![SfcArrayBound { lower: 0, upper: 3 }],
                        initial_value: String::new(),
                        retain: false,
                    }),
                ),
                (
                    "Message",
                    "STRING",
                    Some(SfcDeclaration {
                        initial_value: "'Ready'".into(),
                        ..Default::default()
                    }),
                ),
            ]
        } else {
            vec![("Count", "INT", None)]
        };
        for (name, ty, declaration) in declarations {
            let current = doc
                .text_programs()
                .into_iter()
                .find(|p| p.program_index == index)
                .unwrap();
            doc.edit_text_variable(
                &current.object_id,
                &SfcVariablePatch {
                    program_index: index,
                    expected_variables: current.variables,
                    name: name.into(),
                    data_type: ty.into(),
                    description: String::new(),
                    remove: false,
                    update: false,
                    declaration,
                },
            )?;
        }
        let source = if text.language == "ST" {
            "Count := Count + 1;\r\nSamples[0] := Count;\r\nDelay(IN := TRUE, PT := T#1s);\r\n%MX0 := Delay.Q;\r\nMessage := 'Running';\r\n"
        } else {
            "LD 2\r\nST Count\r\n"
        };
        doc.edit_text_program(&TextProgramPatch {
            program_index: index,
            expected_object_id: text.object_id,
            expected_language: text.language,
            expected_source: text.source.unwrap(),
            source: source.into(),
        })?;
    }
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/text-programs/TEXTGEN.xgwx".into());
    std::fs::write(output, doc.to_verified_bytes()?)?;
    println!(
        "Generated four ST/IL programs with declarations for native Check Program and Save As."
    );
    Ok(())
}
