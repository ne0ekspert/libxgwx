//! Audit IEC function deletion and optional refill preflight without modifying the source.
//! Accepted writer preflight is not native acceptance.
use std::{env, error::Error};
use xgwx::{IecFunctionBlock, IecFunctionPinDirection, LadderProgramData, XgwxDocument, XgwxError};
type Delete = fn(&mut XgwxDocument, usize, usize, &str) -> Result<(), XgwxError>;

fn operands(program: &LadderProgramData, block: &IecFunctionBlock) -> Option<Vec<String>> {
    let records = program.iec_record_frames()?;
    let links = program.iec_function_operand_links()?;
    let mut result = block
        .instance
        .iter()
        .map(|s| s.value.clone())
        .collect::<Vec<_>>();
    let mut pins = block.pins.iter().collect::<Vec<_>>();
    pins.sort_by_key(|pin| {
        (
            pin.direction == IecFunctionPinDirection::Output,
            pin.row_index,
            pin.raw_x,
        )
    });
    for pin in pins {
        let Some(link) = links.iter().find(|l| {
            l.target_record_offset == block.record_offset
                && Some(l.ordinal) == pin.reference_ordinal
        }) else {
            continue;
        };
        let record = records.iter().find(|r| r.offset == link.record_offset)?;
        // Use the framed expression record directly: the heuristic source
        // string inventory can omit short Unicode operands.
        let text = program.data.get(record.offset + 15..record.end)?;
        if text.get(..3)? != [0xff, 0xfe, 0xff] || text.len() != 4 + usize::from(*text.get(3)?) * 2
        {
            return None;
        }
        result.push(
            String::from_utf16(
                &text[4..]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .ok()?,
        );
    }
    Some(result)
}

// Exercise the insertion routes exposed by the blank-cell instruction editor.
fn refill(
    doc: &mut XgwxDocument,
    p: usize,
    block: &IecFunctionBlock,
    args: &[String],
) -> Result<(), XgwxError> {
    let program = doc.ladder_programs().remove(p)?;
    let at = |row, x| row == block.row_index && x == block.raw_x;
    if block.name.value == "MOVE" && args.len() == 2 {
        if let Some(site) = program
            .iec_terminal_function_insertion_sites()
            .unwrap_or_default()
            .into_iter()
            .find(|s| at(s.row_index, s.raw_x))
        {
            return doc.insert_iec_ld_terminal_move(p, site.contact_offset, &args[0], &args[1]);
        }
    }
    if block.name.value == "WORD_TO_UDINT" && args.len() == 2 {
        if let Some(site) = program
            .iec_standalone_function_insertion_sites()
            .unwrap_or_default()
            .into_iter()
            .find(|s| at(s.row_index, s.raw_x))
        {
            return doc.insert_iec_ld_standalone_function(
                p,
                site.insertion_offset,
                &block.name.value,
                &args[0],
                &args[1],
            );
        }
    }
    if let Some(instance) = &block.instance {
        if let Some(site) = program
            .iec_function_cell_insertion_sites()
            .unwrap_or_default()
            .into_iter()
            .find(|s| at(s.row_index, s.raw_x) && s.function_name == block.name.value)
        {
            return doc.insert_iec_ld_function_cell(
                p,
                site.insertion_offset,
                &block.name.value,
                &instance.value,
            );
        }
    }
    if block.name.value == "TON" && args.len() == 3 {
        if let Some(site) = program
            .iec_terminal_timer_insertion_sites()
            .unwrap_or_default()
            .into_iter()
            .find(|s| at(s.row_index, s.raw_x))
        {
            return doc.insert_iec_ld_terminal_timer(
                p,
                site.contact_offset,
                &args[0],
                &args[1],
                &args[2],
            );
        }
    }
    doc.insert_iec_ld_function(p, block.row_index, block.raw_x, &block.name.value, args)
}
fn main() -> Result<(), Box<dyn Error>> {
    let source = env::args()
        .nth(1)
        .ok_or("usage: iec_function_edit_coverage SOURCE [--refill] [--output DIRECTORY] [--only PROGRAM,ROW,X;...]")?;
    let audit_refill = env::args().any(|a| a == "--refill");
    let options = env::args().skip(2).collect::<Vec<_>>();
    let option = |name| {
        options
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| &pair[1])
    };
    let output = option("--output").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path)?;
    }
    let only = option("--only")
        .map(|filter| {
            filter
                .split(';')
                .map(|item| {
                    let fields = item
                        .split(',')
                        .map(str::parse::<usize>)
                        .collect::<Result<Vec<_>, _>>()?;
                    if fields.len() != 3 {
                        return Err("--only requires program,row,x triples".into());
                    }
                    Ok((fields[0], fields[1], fields[2]))
                })
                .collect::<Result<Vec<_>, Box<dyn Error>>>()
        })
        .transpose()?;
    let doc = XgwxDocument::from_path(source)?;
    let original_payloads = doc
        .ladder_programs()
        .into_iter()
        .map(|p| p.map(|p| p.data))
        .collect::<Result<Vec<_>, _>>()?;
    let writers: &[(&str, Delete)] = &[
        ("branch_scalar", XgwxDocument::delete_iec_ld_branch_function),
        (
            "scalar_chain",
            XgwxDocument::delete_iec_ld_scalar_chain_function,
        ),
        (
            "branched_arithmetic",
            XgwxDocument::delete_iec_ld_branched_arithmetic,
        ),
        (
            "terminal_function",
            XgwxDocument::delete_iec_ld_terminal_function,
        ),
        (
            "standalone_function",
            XgwxDocument::delete_iec_ld_standalone_function,
        ),
        ("function_cell", XgwxDocument::delete_iec_ld_function_cell),
        (
            "connected_arithmetic",
            XgwxDocument::delete_iec_ld_connected_arithmetic,
        ),
        ("eq_chain_head", XgwxDocument::delete_iec_ld_eq_chain_head),
        (
            "heating_chain_head",
            XgwxDocument::delete_iec_ld_heating_chain_head,
        ),
        (
            "heating_chain_middle",
            XgwxDocument::delete_iec_ld_heating_chain_middle,
        ),
        (
            "heating_chain_x3_eq_repaired",
            XgwxDocument::delete_iec_ld_heating_chain_x3_eq_repaired,
        ),
        (
            "heating_chain_contact_eq",
            XgwxDocument::delete_iec_ld_heating_chain_contact_eq,
        ),
        (
            "heating_chain_x15_eq",
            XgwxDocument::delete_iec_ld_heating_chain_x15_eq,
        ),
    ];
    let mut total = 0;
    let mut accepted = 0;
    let mut refilled = 0;
    let mut exact = 0;
    for (p, program) in doc.ladder_programs().into_iter().enumerate() {
        let program = program?;
        if program.project_type != Some(2) {
            continue;
        }
        for block in program
            .iec_function_blocks()
            .ok_or("invalid IEC functions")?
        {
            if only.as_ref().is_some_and(|filter| {
                !filter.contains(&(p, usize::from(block.row_index), usize::from(block.raw_x)))
            }) {
                continue;
            }
            total += 1;
            let accepted_by = writers.iter().find_map(|(name, remove)| {
                let mut candidate = doc.clone();
                remove(&mut candidate, p, block.record_offset, &block.name.value)
                    .ok()
                    .map(|_| (*name, candidate))
            });
            if accepted_by.is_some() {
                accepted += 1;
            }
            if let (Some(path), Some((_, deleted))) = (&output, &accepted_by) {
                std::fs::write(
                    path.join(format!("FD{p}R{}X{}.xgwx", block.row_index, block.raw_x)),
                    deleted.to_bytes()?,
                )?;
            }
            let mut status = accepted_by
                .as_ref()
                .map_or("REJECTED".to_owned(), |(name, _)| name.to_string());
            if audit_refill {
                if let Some((_, mut deleted)) = accepted_by {
                    let Some(args) = operands(&program, &block) else {
                        println!(
                            "program {p}, group {}, L{}, x{}, {}: {status}; refill UNRESOLVED operands",
                            block.group_index, block.row_index, block.raw_x, block.name.value
                        );
                        continue;
                    };
                    match refill(&mut deleted, p, &block, &args) {
                        Ok(()) => {
                            refilled += 1;
                            if let Some(path) = &output {
                                std::fs::write(
                                    path.join(format!(
                                        "FR{p}R{}X{}.xgwx",
                                        block.row_index, block.raw_x
                                    )),
                                    deleted.to_bytes()?,
                                )?;
                            }
                            let payloads = deleted
                                .ladder_programs()
                                .into_iter()
                                .map(|p| p.map(|p| p.data))
                                .collect::<Result<Vec<_>, _>>()?;
                            if payloads == original_payloads {
                                exact += 1;
                                status.push_str("; refill EXACT");
                            } else {
                                status.push_str("; refill ACCEPTED, payload differs");
                            }
                        }
                        Err(error) => {
                            status.push_str(&format!("; refill REJECTED {args:?}: {error}"))
                        }
                    }
                }
            }
            println!(
                "program {p}, group {}, L{}, x{}, {}: {}",
                block.group_index, block.row_index, block.raw_x, block.name.value, status
            );
        }
    }
    println!("TOTAL {accepted}/{total} function deletions pass writer preflight");
    if audit_refill {
        println!(
            "REFILL {refilled}/{total} accepted, {exact}/{total} exact across all program payloads"
        );
    }
    Ok(())
}
