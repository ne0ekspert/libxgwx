//! Generate a row-shifted IEC fixture for subsequent editing audits.
//! Successful local serialization does not establish native acceptance.
use std::{env, error::Error};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, program, after_row, output] = args.as_slice() else {
        return Err("usage: iec_shifted_function_probe SOURCE PROGRAM AFTER_ROW OUTPUT".into());
    };
    let program = program.parse::<usize>()?;
    let after_row = after_row.parse::<u16>()?;
    let mut document = XgwxDocument::from_path(source)?;
    let before = document.ladder_programs();
    let locals = document.iec_local_symbols();
    document.insert_iec_ld_blank_row(program, after_row)?;
    let after = document.ladder_programs();
    assert_eq!(before.len(), after.len());
    for (p, (a, b)) in before.into_iter().zip(after).enumerate() {
        let a = a?;
        let b = b?;
        if p != program {
            assert_eq!(a.data, b.data, "unrelated program {p}");
            continue;
        }
        let old = a
            .iec_function_blocks()
            .ok_or("source function decode failed")?;
        let new = b
            .iec_function_blocks()
            .ok_or("shifted function decode failed")?;
        assert_eq!(old.len(), new.len());
        for (a, b) in old.iter().zip(&new) {
            assert_eq!(a.name.value, b.name.value);
            assert_eq!(a.raw_x, b.raw_x);
            assert_eq!(
                a.row_index + u16::from(a.row_index > after_row),
                b.row_index
            );
        }
    }
    for (a, b) in locals.into_iter().zip(document.iec_local_symbols()) {
        assert_eq!(a?, b?);
    }
    document.write_to(output)?;
    println!(
        "PASS row shift in program {program} after L{after_row}; locals and other programs unchanged"
    );
    Ok(())
}
