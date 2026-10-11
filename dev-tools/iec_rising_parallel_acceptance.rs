//! Add a lower contact to the original smart home program 0 L2 rising-edge rung.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_rising_parallel_acceptance <source.xgwx> <output.xgwx>".into());
    };
    let source_bytes = fs::read(source)?;
    let mut document = XgwxDocument::parse(&source_bytes)?;
    document
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program.iec_ld()?.insert_parallel_contact_kind(
                2,
                "스위치_1",
                "시작",
                "NO",
                "ON",
            )
        })?;
    fs::write(output, document.to_bytes()?)?;

    let mut restored = XgwxDocument::from_path(output)?;
    let top = restored
        .program()
        .ladder_programs()
        .remove(0)?
        .iec_row_frames()
        .ok_or("IEC row framing failed")?
        .into_iter()
        .find(|row| row.row_index == 2)
        .ok_or("L2 missing")?;
    restored
        .program_mut()
        .get(0)
        .and_then(|mut selected_program| {
            selected_program
                .iec_ld()?
                .edit_branch_segment(top.group_index, 2, 3, 3, true, false)
        })?;
    let original = XgwxDocument::parse(&source_bytes)?;
    let payloads = |document: &XgwxDocument| -> Result<Vec<Vec<u8>>, xgwx::XgwxError> {
        document
            .program()
            .ladder_programs()
            .into_iter()
            .map(|program| program.map(|program| program.data))
            .collect()
    };
    if payloads(&restored)? != payloads(&original)?
        || restored
            .variable()
            .iec_local_symbols()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            != original
                .variable()
                .iec_local_symbols()
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
    {
        return Err(
            "removing the new branch did not restore all program and local-symbol payloads".into(),
        );
    }
    println!("PASS program 0 L2 rising-edge contact with lower NO contact and payload inverse");
    Ok(())
}
