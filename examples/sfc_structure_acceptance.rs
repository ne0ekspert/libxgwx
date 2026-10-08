use xgwx::{SfcRow, SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/new-xgi-sfc.xgwx")?;
    let row = |kind: &str, title: &str, initial, action: Option<&str>| SfcRow {
        kind: kind.into(),
        title: title.into(),
        comment: if title == "Run" {
            "Cycle & ready".into()
        } else {
            String::new()
        },
        initial,
        action: action.map(String::from),
        action_qualifier: None,
        action_code: None,
        transition_code: None,
        action_time: None,
    };
    let rows = vec![
        row("label", "Cycle", false, None),
        row("step", "Idle", true, Some("%MX10")),
        row("transition", "%MX2", false, None),
        row("step", "Run", false, None),
        row("transition", "%MX3", false, None),
        row("step", "Done", false, Some("%MX11")),
        row("transition", "%MX4", false, None),
        row("jump", "Cycle", false, None),
    ];
    let apply = |doc: &mut XgwxDocument, rows| {
        let expected = doc.sfc_programs()[0].blocks[0].entities.clone();
        doc.replace_sfc_sequence(&SfcSequencePatch {
            expected_rows: None,
            program_index: 0,
            block_index: 0,
            expected_entities: expected,
            rows,
        })
    };
    apply(&mut doc, rows.clone())?;
    std::fs::create_dir_all("/tmp/sfc-structure")?;
    std::fs::write("/tmp/sfc-structure/SFCBUILD.xgwx", doc.to_verified_bytes()?)?;
    let mut shorter = rows;
    shorter.drain(3..5);
    apply(&mut doc, shorter)?;
    std::fs::write(
        "/tmp/sfc-structure/SFCDELETE.xgwx",
        doc.to_verified_bytes()?,
    )?;
    let mut native = XgwxDocument::from_path("fixtures/sfc/native-action.xgwx")?;
    let mut rows = native.sfc_programs()[0].blocks[0]
        .editable_rows
        .clone()
        .ok_or("native action not editable")?;
    rows[1].title = "Boot".into();
    rows[1].action = Some("%MX12".into());
    rows[3].initial = true;
    rows[1].initial = false;
    rows[2].title = "%MX5".into();
    apply(&mut native, rows)?;
    std::fs::write(
        "/tmp/sfc-structure/SFCRENAME.xgwx",
        native.to_verified_bytes()?,
    )?;
    println!("Generated SFCBUILD, SFCDELETE, and SFCRENAME native acceptance cases");
    Ok(())
}
