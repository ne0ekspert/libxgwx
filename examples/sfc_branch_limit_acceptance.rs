use xgwx::{SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/sfc-native-limits".into());
    std::fs::create_dir_all(&out)?;
    for paths in [16, 511] {
        let mut doc = XgwxDocument::from_path("fixtures/sfc/multi-action-branch-native.xgwx")?;
        let b = doc.sfc_programs()[0].blocks[0].clone();
        let mut rows = b.editable_rows.clone().ok_or("read-only chart")?;
        let start = rows
            .iter()
            .find(|r| r.kind == "parallel_split")
            .unwrap()
            .position
            .as_ref()
            .unwrap()
            .row;
        let end = rows
            .iter()
            .find(|r| r.kind == "parallel_join")
            .unwrap()
            .position
            .as_ref()
            .unwrap()
            .row;
        let templates: Vec<_> = rows
            .iter()
            .filter(|r| {
                let p = r.position.as_ref().unwrap();
                p.column == 0 && p.row > start && p.row < end
            })
            .cloned()
            .collect();
        for path in 2..paths {
            for mut row in templates.clone() {
                row.position.as_mut().unwrap().column = path * 2;
                row.title = if row.kind == "step" {
                    format!("Path{path}")
                } else {
                    String::new()
                };
                row.comment.clear();
                row.initial = false;
                row.action = None;
                row.action_code = None;
                row.action_qualifier = None;
                row.action_time = None;
                rows.push(row);
            }
        }
        for row in &mut rows {
            if row.branch_end.is_some() {
                row.branch_end = Some((paths - 1) * 2);
            }
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
        std::fs::write(format!("{out}/LIM{paths}.xgwx"), doc.to_verified_bytes()?)?;
    }
    Ok(())
}
