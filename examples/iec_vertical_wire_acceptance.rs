//! Generate a row-preserving vertical-wire deletion and compare its native capture.
use std::{env, error::Error};
use xgwx::{LadderProgramData, XgwxDocument};
fn without_display_cache(program: &LadderProgramData) -> Vec<u8> {
    let mut bytes = program.data.clone();
    for row in program.iec_row_frames().unwrap() {
        bytes[row.start + 17..row.start + 21].fill(0);
    }
    bytes
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: iec_vertical_wire_acceptance SOURCE OUTPUT [NATIVE_DELETE]".into());
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let before = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, true, false)?;
    let after = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(after[0].data.len() + 36, before[0].data.len());
    assert_eq!(
        after[0].iec_row_frames().unwrap().len(),
        before[0].iec_row_frames().unwrap().len()
    );
    let layout = after[0]
        .iec_circuit_layout()
        .ok_or("invalid circuit layout")?;
    assert_eq!(layout.open_branch_endpoints.len(), 2);
    assert_eq!(
        layout.function_bindings.len(),
        before[0]
            .iec_circuit_graph()
            .unwrap()
            .function_bindings
            .len()
    );
    for p in 1..before.len() {
        assert_eq!(after[p].data, before[p].data);
    }
    if let Some(native) = args.get(2) {
        let native = XgwxDocument::from_path(native)?
            .ladder_programs()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        for p in 0..native.len() {
            assert_eq!(
                without_display_cache(&after[p]),
                without_display_cache(&native[p]),
                "program {p}"
            );
        }
    }
    doc.write_to(&args[1])?;
    doc.edit_iec_ld_vertical_wire(0, 33, 69, 70, 12, false, true)?;
    for (before, restored) in before.iter().zip(doc.ladder_programs()) {
        assert_eq!(before.data, restored?.data);
    }
    println!("PASS row-preserving vertical deletion, native comparison and exact restoration");
    Ok(())
}
