use xgwx::{SfcPosition, SfcRow, SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/sfc-paths/PATHGEN.xgwx".into());
    let mut doc = XgwxDocument::from_path("fixtures/sfc/multi-action-branch-native.xgwx")?;
    let b = doc.sfc_programs()[0].blocks[0].clone();
    let mut rows = b.editable_rows.clone().ok_or("read-only chart")?;
    let at = rows
        .iter()
        .find(|r| r.kind == "parallel_join")
        .unwrap()
        .position
        .as_ref()
        .unwrap()
        .row;
    for row in &mut rows {
        if row.position.as_ref().unwrap().row >= at {
            row.position.as_mut().unwrap().row += 2;
        }
    }
    for (offset, column, kind, title) in [
        (0, 0, "continuation", ""),
        (1, 0, "continuation", ""),
        (0, 2, "transition", "%MX20"),
        (1, 2, "step", "SideNext"),
    ] {
        rows.push(SfcRow {
            kind: kind.into(),
            title: title.into(),
            comment: String::new(),
            initial: false,
            action: None,
            action_code: None,
            transition_code: None,
            action_qualifier: None,
            action_time: None,
            position: Some(SfcPosition {
                row: at + offset,
                column,
            }),
            branch_end: None,
        });
    }
    rows.sort_by_key(|r| {
        let p = r.position.as_ref().unwrap();
        (p.row, p.column)
    });
    doc.replace_sfc_sequence(&SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: b.entities,
        expected_rows: b.editable_rows,
        rows,
    })?;
    if let Some(parent) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, doc.to_verified_bytes()?)?;
    Ok(())
}
