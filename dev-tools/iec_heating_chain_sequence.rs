//! Apply successive guarded EQ deletions in the smart-home heating chain.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(
            "usage: iec_heating_chain_sequence SOURCE OUTPUT ROW:head|middle|x3|x3raw|contact|x15 ..."
                .into(),
        );
    }
    let mut document = XgwxDocument::from_path(&args[0])?;
    for step in &args[2..] {
        if step == "delete-group" {
            document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| selected_program.iec_ld()?.delete_group(13, 46))?;
            println!("deleted complete heating group 13");
            continue;
        }
        let (row, kind) = step.split_once(':').ok_or("step must be ROW:kind")?;
        let row = row.parse::<u16>()?;
        let block = document
            .program()
            .ladder_programs()
            .remove(6)?
            .iec_function_blocks()
            .ok_or("invalid IEC blocks")?
            .into_iter()
            .find(|block| {
                block.group_index == 13 && block.row_index == row && block.name.value == "EQ"
            })
            .ok_or_else(|| format!("EQ missing at L{row}"))?;
        match kind {
            "head" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_head(block.record_offset, "EQ")
                })?,
            "middle" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_middle(block.record_offset, "EQ")
                })?,
            "contact" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_contact_eq(block.record_offset, "EQ")
                })?,
            "x3" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_x3_eq_repaired(block.record_offset, "EQ")
                })?,
            "x3raw" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_x3_eq(block.record_offset, "EQ")
                })?,
            "x15" => document
                .program_mut()
                .get(6)
                .and_then(|mut selected_program| {
                    selected_program
                        .iec_ld()?
                        .delete_heating_chain_x15_eq(block.record_offset, "EQ")
                })?,
            _ => return Err(format!("unknown strategy: {kind}").into()),
        }
        println!("deleted {kind} EQ at L{row}");
    }
    document.write_to(&args[1])?;
    Ok(())
}
