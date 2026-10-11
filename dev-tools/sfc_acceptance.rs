#[cfg(feature = "write")]
#[path = "fixture_patches.rs"]
mod mutation_support;
use xgwx::{SfcEntityPatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-loop.xgwx")?;
    for (entity_index, row, field, expected, replacement) in [
        (2, 2, "condition", "%MX0", "%MX2"),
        (3, 3, "comment", "Ready for cycle", "Cycle active & ready"),
    ] {
        doc.program_mut().get(0).and_then(|selected_program| {
            mutation_support::edit_sfc_entity(
                selected_program,
                &SfcEntityPatch {
                    program_index: 0,
                    block_index: 0,
                    entity_index,
                    expected_type: if field == "condition" { 1 } else { 0 },
                    expected_row: row,
                    expected_column: 0,
                    field: field.into(),
                    expected_value: expected.into(),
                    replacement: replacement.into(),
                },
            )
        })?;
    }
    std::fs::write("/tmp/SFCEDIT.xgwx", doc.to_verified_bytes()?)?;
    println!("SFC edits generated: /tmp/SFCEDIT.xgwx");
    Ok(())
}
