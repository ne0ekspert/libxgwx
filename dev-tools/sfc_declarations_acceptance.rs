use xgwx::{SfcArrayBound, SfcDeclaration, SfcSequencePatch, SfcVariablePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/declarations-string-native.xgwx")?;
    for (name, ty, init, retain, bounds, update) in [
        ("Count", "DINT", "11", false, vec![], true),
        ("StatusText", "STRING", "'Ready'", false, vec![], true),
        ("Samples", "DINT", "4(2)", true, vec![(0, 3)], false),
        (
            "Matrix",
            "WORD",
            "1,2,3,4,5,6",
            false,
            vec![(0, 1), (0, 2)],
            false,
        ),
        (
            "Cube",
            "BOOL",
            "8(FALSE)",
            false,
            vec![(0, 1), (0, 1), (0, 1)],
            false,
        ),
        ("RetainedTimer", "TON", "", true, vec![], false),
        ("Setpoint", "REAL", "1.5", false, vec![], false),
        ("Interval", "TIME", "T#2s", false, vec![], false),
    ] {
        doc.edit_sfc_variable(&SfcVariablePatch {
            program_index: 0,
            expected_variables: doc.sfc_variables(0)?,
            name: name.into(),
            data_type: ty.into(),
            description: "Advanced declaration".into(),
            remove: false,
            update,
            declaration: Some(SfcDeclaration {
                initial_value: init.into(),
                retain,
                dimensions: bounds
                    .into_iter()
                    .map(|(lower, upper)| SfcArrayBound { lower, upper })
                    .collect(),
            }),
        })?;
    }
    let block = doc.sfc_programs().remove(0).blocks.remove(0);
    let mut rows = block.editable_rows.clone().ok_or("editable")?;
    for row in &mut rows {
        if row.action.as_deref() == Some("UpdateValues") {
            row.action_code=Some("Count := Samples[0] + 1;\r\nStatusText := 'Ready';\r\nMatrix[0,0] := WORD#7;\r\nCube[0,0,0] := TRUE;\r\nRetainedTimer(IN := TRUE, PT := Interval);".into());
        }
        if row.title == "Ready" {
            row.transition_code = Some("TRANS := Samples[0] > 0;".into());
        }
    }
    doc.replace_sfc_sequence(&SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: block.entities,
        expected_rows: block.editable_rows,
        rows,
    })?;
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/sfc-declarations/ADVGEN.xgwx".into());
    std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap())?;
    std::fs::write(out, doc.to_verified_bytes()?)?;
    Ok(())
}
