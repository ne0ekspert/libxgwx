//! Audit comparison deletion after changing a feeding contact's kind.
//! Local preflight is not native acceptance; the source is never overwritten.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 && args.len() != 8 {
        return Err("usage: iec_contact_comparison_kind_probe SOURCE PROGRAM ROW CONTACT_X [OUTPUT IN1 IN2 DESTINATION]".into());
    }
    let [source, program, row, x] = &args[..4] else {
        unreachable!()
    };
    let output = args.get(4).map(std::path::PathBuf::from);
    if let Some(output) = &output {
        std::fs::create_dir_all(output)?;
    }
    let p = program.parse::<usize>()?;
    let row = row.parse::<u16>()?;
    let x = x.parse::<u8>()?;
    let source = XgwxDocument::from_path(source)?;
    let program = source.ladder_programs().remove(p)?;
    let record = program
        .iec_record_frames()
        .ok_or("record decode failed")?
        .into_iter()
        .find(|r| {
            r.row_index == row
                && matches!(r.kind, IecRecordKind::Contact(6..=11))
                && program.data[r.offset + 5] == x
        })
        .ok_or("contact not found")?;
    let IecRecordKind::Contact(original_code) = record.kind else {
        unreachable!()
    };
    let kinds = [
        "NO",
        "NC",
        "RISING",
        "FALLING",
        "NEGATED_RISING",
        "NEGATED_FALLING",
    ];
    for (index, kind) in kinds.into_iter().enumerate() {
        let code = index + 6;
        let mut candidate = source.clone();
        candidate.update_iec_ld_contact_kind(
            p,
            record.offset + 15,
            kinds[usize::from(original_code - 6)],
            kind,
        )?;
        let before = candidate.to_bytes()?;
        if let Some(output) = &output {
            candidate.write_to(output.join(format!("K{code}C.xgwx")))?;
        }
        let block = candidate
            .ladder_programs()
            .remove(p)?
            .iec_function_blocks()
            .ok_or("function decode failed")?
            .into_iter()
            .find(|b| b.row_index == row && b.raw_x == 19)
            .ok_or("comparison not found")?;
        let result = candidate.delete_iec_ld_heating_chain_contact_eq(
            p,
            block.record_offset,
            &block.name.value,
        );
        if result.is_err() {
            assert_eq!(candidate.to_bytes()?, before, "atomic rejection for {kind}");
        }
        println!("{kind}: contact kind edit accepted; comparison Delete {result:?}");
        if result.is_ok() {
            if let Some(output) = &output {
                candidate.write_to(output.join(format!("K{code}D.xgwx")))?;
                candidate.insert_iec_ld_function(p, row, 19, &block.name.value, &args[5..8])?;
                candidate.write_to(output.join(format!("K{code}R.xgwx")))?;
                println!("{kind}: comparison refill accepted");
            }
        }
    }
    Ok(())
}
