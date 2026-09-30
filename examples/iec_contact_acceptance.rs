//! Generate and verify one IEC contact insertion for native XG5000 acceptance.
use std::{error::Error, path::Path};
use xgwx::{IecRecordKind, LadderProgramData, XgwxDocument};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn code(kind: &str) -> Result<u8> {
    match kind {
        "NO" => Ok(0x06),
        "NC" => Ok(0x07),
        "RISING" => Ok(0x08),
        "FALLING" => Ok(0x09),
        "NEGATED_RISING" => Ok(0x0a),
        "NEGATED_FALLING" => Ok(0x0b),
        _ => Err(
            "contact kind must be NO, NC, RISING, FALLING, NEGATED_RISING, or NEGATED_FALLING"
                .into(),
        ),
    }
}

fn target(
    program: &LadderProgramData,
    record_offset: usize,
    raw_x: u8,
    kind: &str,
) -> Result<String> {
    let code = code(kind)?;
    let record = program
        .iec_record_frames()
        .ok_or("IEC record framing failed")?
        .into_iter()
        .find(|record| {
            record.offset == record_offset
                && record.kind == IecRecordKind::Contact(code)
                && program.data.get(record.offset + 5) == Some(&raw_x)
        })
        .ok_or_else(|| format!("no {kind} contact at record {record_offset} and x={raw_x}"))?;
    let strings = program
        .strings
        .iter()
        .filter(|item| item.offset >= record.offset && item.end_offset <= record.end)
        .collect::<Vec<_>>();
    match strings.as_slice() {
        [item] => Ok(item.value.clone()),
        _ => Err(format!("contact record contains {} strings", strings.len()).into()),
    }
}

#[allow(clippy::too_many_arguments)]
fn generate(
    source: &Path,
    output: &Path,
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    start_x: u8,
    end_x: u8,
    kind: &str,
    variable: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_contact(
        program_index,
        wire_offset,
        raw_x,
        start_x,
        end_x,
        kind,
        variable,
    )?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let actual = target(&program, wire_offset + 19, raw_x, kind)?;
    if actual != variable {
        return Err(format!("generated contact is {actual}, expected {variable}").into());
    }
    println!("PASS generated program {program_index} {kind} contact {variable} at x={raw_x}");
    Ok(())
}

fn verify(
    generated: &Path,
    resaved: &Path,
    program_index: usize,
    record_offset: usize,
    raw_x: u8,
    kind: &str,
    variable: &str,
) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
    let program = resaved
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let actual = target(&program, record_offset, raw_x, kind)?;
    if actual != variable {
        return Err(format!("resaved contact is {actual}, expected {variable}").into());
    }
    let generated_programs = generated.ladder_programs();
    let resaved_programs = resaved.ladder_programs();
    if generated_programs.len() != resaved_programs.len() {
        return Err("resaved program count changed".into());
    }
    for (index, (generated, resaved)) in generated_programs
        .into_iter()
        .zip(resaved_programs)
        .enumerate()
    {
        if generated?.data != resaved?.data {
            return Err(format!("decoded ProgramData {index} changed during Save As").into());
        }
    }
    println!(
        "PASS native Save As preserved program {program_index} {kind} contact {variable} at x={raw_x} and every decoded ProgramData payload"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn generate_cell_delete(
    source: &Path,
    output: &Path,
    program_index: usize,
    wire_offset: usize,
    raw_x: u8,
    start_x: u8,
    end_x: u8,
    kind: &str,
    variable: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_contact(
        program_index,
        wire_offset,
        raw_x,
        start_x,
        end_x,
        kind,
        variable,
    )?;
    let contact_offset = wire_offset + 19;
    document.delete_iec_ld_contact_cell(program_index, contact_offset, raw_x, kind, variable)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    program
        .iec_circuit_graph()
        .ok_or("generated deletion has an invalid circuit graph")?;
    let replacement = program
        .iec_record_frames()
        .ok_or("IEC record framing failed")?
        .into_iter()
        .find(|record| record.offset == contact_offset)
        .ok_or("record following deleted contact is missing")?;
    if replacement.kind != IecRecordKind::LongWire
        || program.data.get(replacement.offset + 5) != Some(&raw_x)
    {
        return Err("Cell Delete did not close the contact cell with the right wire".into());
    }
    println!(
        "PASS generated Cell Delete of program {program_index} {kind} contact {variable} at x={raw_x}"
    );
    Ok(())
}

fn verify_cell_delete(generated: &Path, resaved: &Path, program_index: usize) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
    resaved
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??
        .iec_circuit_graph()
        .ok_or("native-resaved deletion has an invalid circuit graph")?;
    let generated_programs = generated.ladder_programs();
    let resaved_programs = resaved.ladder_programs();
    if generated_programs.len() != resaved_programs.len() {
        return Err("resaved program count changed".into());
    }
    for (index, (generated, resaved)) in generated_programs
        .into_iter()
        .zip(resaved_programs)
        .enumerate()
    {
        if generated?.data != resaved?.data {
            return Err(format!("decoded ProgramData {index} changed during Save As").into());
        }
    }
    println!(
        "PASS native Save As preserved the generated Cell Delete and every decoded ProgramData payload"
    );
    Ok(())
}

fn generate_delete(
    source: &Path,
    output: &Path,
    program_index: usize,
    contact_offset: usize,
    raw_x: u8,
    kind: &str,
    variable: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_contact(program_index, contact_offset, raw_x, kind, variable)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??
        .iec_circuit_graph()
        .ok_or("generated deletion has an invalid circuit graph")?;
    println!(
        "PASS generated Delete of program {program_index} {kind} contact {variable} at x={raw_x}"
    );
    Ok(())
}

fn generate_repair(
    source: &Path,
    output: &Path,
    program_index: usize,
    insertion_offset: usize,
    raw_x: u8,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.repair_iec_ld_horizontal_wire(program_index, insertion_offset, raw_x)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??
        .iec_circuit_graph()
        .ok_or("generated repair has an invalid circuit graph")?;
    println!("PASS generated F5 repair in program {program_index} at x={raw_x}");
    Ok(())
}

fn number<T: std::str::FromStr>(value: &str) -> Result<T>
where
    T::Err: Error + 'static,
{
    Ok(value.parse()?)
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, source, output, program, wire, x, start, end, kind, variable]
            if mode == "generate" =>
        {
            generate(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(wire)?,
                number(x)?,
                number(start)?,
                number(end)?,
                kind,
                variable,
            )
        }
        [mode, source, output, program, wire, x, start, end, kind, variable]
            if mode == "generate-cell-delete" =>
        {
            generate_cell_delete(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(wire)?,
                number(x)?,
                number(start)?,
                number(end)?,
                kind,
                variable,
            )
        }
        [mode, generated, resaved, program, record, x, kind, variable] if mode == "verify" => verify(
            Path::new(generated),
            Path::new(resaved),
            number(program)?,
            number(record)?,
            number(x)?,
            kind,
            variable,
        ),
        [mode, generated, resaved, program] if mode == "verify-cell-delete" => {
            verify_cell_delete(Path::new(generated), Path::new(resaved), number(program)?)
        }
        [mode, source, output, program, offset, kind, variable]
            if mode == "generate-leading-insert" =>
        {
            let mut document = XgwxDocument::from_path(source)?;
            document.insert_iec_ld_leading_contact(
                number(program)?, number(offset)?, kind, variable,
            )?;
            document.write_to(output)?;
            Ok(())
        }
        [mode, source, output, program, wire, x, kind, variable]
            if mode == "generate-short-wire-insert" =>
        {
            let mut document = XgwxDocument::from_path(source)?;
            document.insert_iec_ld_short_wire_contact(
                number(program)?, number(wire)?, number(x)?, kind, variable,
            )?;
            document.write_to(output)?;
            Ok(())
        }
        [mode, source, output, program, record, x, kind, variable]
            if mode == "generate-cell-delete-direct" =>
        {
            let mut document = XgwxDocument::from_path(source)?;
            document.delete_iec_ld_contact_cell(
                number(program)?, number(record)?, number(x)?, kind, variable,
            )?;
            document.write_to(output)?;
            Ok(())
        }
        [mode, source, output, program, record, x, kind, variable]
            if mode == "generate-delete" =>
        {
            generate_delete(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(record)?,
                number(x)?,
                kind,
                variable,
            )
        }
        [mode, source, output, program, record, x] if mode == "generate-repair" => {
            generate_repair(
                Path::new(source), Path::new(output), number(program)?,
                number(record)?, number(x)?,
            )
        }
        _ => Err("usage: iec-contact-acceptance generate SOURCE OUTPUT PROGRAM WIRE X START END KIND VARIABLE | generate-cell-delete SOURCE OUTPUT PROGRAM WIRE X START END KIND VARIABLE | generate-delete SOURCE OUTPUT PROGRAM RECORD X KIND VARIABLE | generate-repair SOURCE OUTPUT PROGRAM RECORD X | verify GENERATED RESAVED PROGRAM RECORD X KIND VARIABLE | verify-cell-delete GENERATED RESAVED PROGRAM".into()),
    }
}
