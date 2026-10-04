//! Verify that native-rejected IEC compound operands leave a workspace unchanged.
use std::{env, error::Error};
use xgwx::XgwxDocument;
fn set(doc: &mut XgwxDocument, row: u16, output: bool, value: &str) -> Result<(), Box<dyn Error>> {
    let program = doc.ladder_programs().remove(3)?;
    let block = program
        .iec_function_blocks()
        .ok_or("invalid blocks")?
        .into_iter()
        .find(|b| b.row_index == row && b.name.value == "MOVE")
        .ok_or("missing MOVE")?;
    let link = program
        .iec_function_operand_links()
        .ok_or("invalid operands")?
        .into_iter()
        .find(|l| l.target_record_offset == block.record_offset && l.is_output == output)
        .ok_or("missing pin")?;
    let record = program
        .iec_record_frames()
        .ok_or("invalid records")?
        .into_iter()
        .find(|r| r.offset == link.record_offset)
        .ok_or("missing expression record")?;
    let text = program
        .strings
        .iter()
        .find(|s| s.offset >= record.offset && s.end_offset <= record.end)
        .ok_or("missing expression text")?;
    doc.update_iec_ld_function_operand(3, text.offset, &text.value, value)?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_expression_guard_coverage SOURCE")?;
    let mut doc = XgwxDocument::from_path(source)?;
    let original = doc.to_bytes()?;
    for (row, expression) in [
        (1, "%MW0 MOD 7 + 1"),
        (4, "%MW0 >= 2"),
        (7, "%MX0 OR NOT %MX1 AND (%MW2 >= 2)"),
        (10, "%MW0 XOR (%MW1 AND NOT %MW2)"),
        (13, "TRUE XOR FALSE"),
    ] {
        let error = set(&mut doc, row, false, expression)
            .expect_err("native-rejected expression must fail");
        assert!(
            error
                .to_string()
                .contains("native-valid expression construction"),
            "{error}"
        );
        assert_eq!(doc.to_bytes()?, original, "rejection mutated the document");
    }
    println!("PASS all five native-rejected expression forms leave the document unchanged");
    Ok(())
}
