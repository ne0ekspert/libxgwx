use xgwx::{SfcRow, SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/new-xgi-sfc.xgwx")?;
    let row = |kind: &str, title: String, initial, action, action_qualifier, action_time| SfcRow {
        kind: kind.into(),
        title,
        comment: String::new(),
        initial,
        action,
        action_qualifier,
        action_time,
    };
    let mut rows = vec![row("label", "Cycle".into(), false, None, None, None)];
    for (i, q) in ["N", "R", "S", "L", "D", "P", "SD", "DS", "SL"]
        .iter()
        .enumerate()
    {
        rows.push(row(
            "step",
            format!("Q{q}"),
            i == 0,
            Some(format!("%MX{}", 10 + i)),
            (*q != "N").then(|| q.to_string()),
            ["L", "D", "SD", "DS", "SL"]
                .contains(q)
                .then(|| "T#2s".into()),
        ));
        rows.push(row(
            "transition",
            format!("%MX{i}"),
            false,
            None,
            None,
            None,
        ));
    }
    rows.push(row("jump", "Cycle".into(), false, None, None, None));
    doc.replace_sfc_sequence(&SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: doc.sfc_programs()[0].blocks[0].entities.clone(),
        rows,
    })?;
    std::fs::write("/tmp/SFCACTS.xgwx", doc.to_verified_bytes()?)?;
    Ok(())
}
