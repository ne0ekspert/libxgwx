#[cfg(feature = "write")]
#[path = "fixture_patches.rs"]
mod mutation_support;
use xgwx::{SfcSequencePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or("target/sfc-branches".into());
    std::fs::create_dir_all(&out)?;
    for (fixture, stem) in [("alternative", "ALTGEN"), ("parallel", "PARGEN")] {
        let mut doc =
            XgwxDocument::from_path(format!("fixtures/sfc/branch-{fixture}-native.xgwx"))?;
        let block = doc.program().sfc_programs().remove(0).blocks.remove(0);
        let expected_rows = block.editable_rows.ok_or("branch chart not decoded")?;
        let mut rows = expected_rows.clone();
        for row in &mut rows {
            if row.position.as_ref().unwrap().column == 2 {
                if row.kind == "transition" {
                    row.title = "%MX3".into();
                }
                if row.kind == "step" {
                    row.title = "SideStep".into();
                    row.action = Some("%MX12".into());
                }
            }
        }
        doc.program_mut().get(0).and_then(|selected_program| {
            mutation_support::replace_sfc_sequence(
                selected_program,
                &SfcSequencePatch {
                    program_index: 0,
                    block_index: 0,
                    expected_entities: block.entities,
                    expected_rows: Some(expected_rows),
                    rows: rows.clone(),
                },
            )
        })?;
        assert_eq!(
            doc.program().sfc_programs()[0].blocks[0]
                .editable_rows
                .as_ref(),
            Some(&rows)
        );
        std::fs::write(format!("{out}/{stem}.xgwx"), doc.to_verified_bytes()?)?;
    }
    Ok(())
}
