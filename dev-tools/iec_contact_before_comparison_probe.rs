//! Preflight contact-first deletion without overwriting the source project.
//! Successful local writes still require native validation.
use std::{env, error::Error};
use xgwx::{IecRecordKind, XgwxDocument, XgwxError};

type Delete = fn(&mut XgwxDocument, usize, usize, &str) -> Result<(), XgwxError>;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, program, row, x, sites] = args.as_slice() else {
        return Err("usage: iec_contact_before_comparison_probe SOURCE PROGRAM COMPARISON_ROW COMPARISON_X CONTACT_ROW,CONTACT_X[;...]".into());
    };
    let p: usize = program.parse()?;
    let row: u16 = row.parse()?;
    let x: u8 = x.parse()?;
    let source = XgwxDocument::from_path(source)?;
    let routes: [(&str, Delete); 10] = [
        ("head", XgwxDocument::delete_iec_ld_heating_chain_head),
        ("middle", XgwxDocument::delete_iec_ld_heating_chain_middle),
        (
            "x3",
            XgwxDocument::delete_iec_ld_heating_chain_x3_eq_repaired,
        ),
        (
            "contact",
            XgwxDocument::delete_iec_ld_heating_chain_contact_eq,
        ),
        ("x15", XgwxDocument::delete_iec_ld_heating_chain_x15_eq),
        ("scalar", XgwxDocument::delete_iec_ld_scalar_chain_function),
        ("terminal", XgwxDocument::delete_iec_ld_terminal_function),
        (
            "standalone",
            XgwxDocument::delete_iec_ld_standalone_function,
        ),
        ("branch", XgwxDocument::delete_iec_ld_branch_function),
        ("cell", XgwxDocument::delete_iec_ld_function_cell),
    ];
    for site in sites.split(';') {
        let (r, cx) = site.split_once(',').ok_or("invalid contact site")?;
        let r: u16 = r.parse()?;
        let cx: u8 = cx.parse()?;
        let mut edited = source.clone();
        let program = edited.ladder_programs().remove(p)?;
        let contact = program
            .iec_record_frames()
            .ok_or("records")?
            .into_iter()
            .find(|v| {
                v.row_index == r
                    && matches!(v.kind, IecRecordKind::Contact(6..=11))
                    && program.data[v.offset + 5] == cx
            })
            .ok_or("contact")?;
        let IecRecordKind::Contact(code) = contact.kind else {
            unreachable!()
        };
        let kind = [
            "NO",
            "NC",
            "RISING",
            "FALLING",
            "NEGATED_RISING",
            "NEGATED_FALLING",
        ][usize::from(code - 6)];
        let variable = String::from_utf16(
            &program.data[contact.offset + 19..contact.end]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )?;
        edited.delete_iec_ld_contact(p, contact.offset, cx, kind, &variable)?;
        let block = edited
            .ladder_programs()
            .remove(p)?
            .iec_function_blocks()
            .ok_or("blocks")?
            .into_iter()
            .find(|v| v.row_index == row && v.raw_x == x)
            .ok_or("comparison removed with contact")?;
        let before = edited.to_bytes()?;
        let mut accepted = Vec::new();
        for (name, delete) in routes {
            let mut candidate = edited.clone();
            if delete(&mut candidate, p, block.record_offset, &block.name.value).is_ok() {
                accepted.push(name);
            } else {
                assert_eq!(candidate.to_bytes()?, before, "atomic {name} rejection");
            }
        }
        println!("contact L{r} x{cx} Delete accepted; comparison deletion routes: {accepted:?}");
    }
    Ok(())
}
