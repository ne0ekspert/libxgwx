use xgwx::{SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).unwrap_or("target/sfc-multi".into());
    std::fs::create_dir_all(&out)?;
    for (fixture, stem) in [
        ("multi-action-three-native", "MULTGEN"),
        ("multi-action-branch-native", "BMULGEN"),
    ] {
        let mut doc = XgwxDocument::from_path(format!("fixtures/sfc/{fixture}.xgwx"))?;
        let block = doc.sfc_programs().remove(0).blocks.remove(0);
        let mut rows = block.editable_rows.clone().ok_or("actions not decoded")?;
        let row = rows
            .iter_mut()
            .rev()
            .find(|r| r.kind == "continuation" && r.action.is_some())
            .unwrap();
        row.action = Some("PulseUpdate".into());
        row.action_qualifier = Some("P".into());
        row.action_time = None;
        row.action_code = Some("Count := Count + 3;".into());
        doc.replace_sfc_sequence(&SfcSequencePatch {
            program_index: 0,
            block_index: 0,
            expected_entities: block.entities,
            expected_rows: block.editable_rows,
            rows: rows.clone(),
        })?;
        assert_eq!(
            doc.sfc_programs()[0].blocks[0].editable_rows.as_ref(),
            Some(&rows)
        );
        std::fs::write(format!("{out}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
    }
    Ok(())
}
