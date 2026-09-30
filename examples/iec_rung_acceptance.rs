//! Generate and verify one native-shaped IEC linear rung for XG5000 acceptance.
use std::{env, error::Error, path::Path};
use xgwx::{IecRecordKind, XgwxDocument};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn generate(
    source: &Path,
    output: &Path,
    program_index: usize,
    row_index: u16,
    contact: &str,
    coil: &str,
) -> Result<()> {
    generate_kind(
        source,
        output,
        program_index,
        row_index,
        "NO",
        contact,
        "OUTPUT",
        coil,
    )
}

#[allow(clippy::too_many_arguments)]
fn generate_kind(
    source: &Path,
    output: &Path,
    program_index: usize,
    row_index: u16,
    contact_kind: &str,
    contact: &str,
    coil_kind: &str,
    coil: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_rung(
        program_index,
        row_index,
        contact_kind,
        contact,
        coil_kind,
        coil,
    )?;
    document.write_to(output)?;

    let reparsed = XgwxDocument::from_path(output)?;
    let programs = reparsed.ladder_programs();
    let program = programs[program_index]
        .as_ref()
        .map_err(|error| error.to_string())?;
    let row = program
        .iec_row_frames()
        .ok_or("IEC row framing failed")?
        .into_iter()
        .find(|row| row.row_index == row_index)
        .ok_or("created row is missing")?;
    let kinds = program
        .iec_record_frames()
        .ok_or("IEC record framing failed")?
        .into_iter()
        .filter(|record| record.group_index == row.group_index && record.row_index == row.row_index)
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    if kinds.len() != 3
        || !matches!(kinds[0], IecRecordKind::Contact(_))
        || kinds[1] != IecRecordKind::LongWire
        || !matches!(kinds[2], IecRecordKind::Coil(_))
    {
        return Err(format!("unexpected created rung records: {kinds:?}").into());
    }
    let graph = program
        .iec_circuit_graph()
        .ok_or("IEC circuit graph validation failed")?;
    if !graph.power_components.iter().any(|component| {
        component.group_index == row.group_index
            && component.touches_left_rail
            && component.touches_right_rail
    }) {
        return Err("created rung does not join both rails".into());
    }
    println!(
        "PASS generated program {program_index} L{row_index}: {contact_kind} {contact} -> {coil_kind} {coil} in group {}",
        row.group_index
    );
    Ok(())
}

fn verify(generated: &Path, resaved: &Path) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
    let generated_programs = generated.ladder_programs();
    let resaved_programs = resaved.ladder_programs();
    let program_count = generated_programs.len();
    if generated_programs.len() != resaved_programs.len() {
        return Err("program count changed during Save As".into());
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
    let generated_symbols = generated
        .iec_local_symbols()
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let resaved_symbols = resaved
        .iec_local_symbols()
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if generated_symbols != resaved_symbols {
        return Err("IEC local symbol tables changed during Save As".into());
    }
    println!(
        "PASS native Save As preserved all {program_count} ProgramData payloads and parsed IEC local symbols"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, source, output] if command == "append-all" => {
            let mut document = XgwxDocument::from_path(source)?;
            let targets = document.ladder_programs().into_iter().enumerate()
                .map(|(index, program)| {
                    let program = program?;
                    let rows = program.iec_row_frames().ok_or("IEC row framing failed")?;
                    Ok((index, rows.last().ok_or("empty IEC program")?.row_index + 1))
                }).collect::<Result<Vec<_>>>()?;
            for (index, row) in targets {
                document.insert_iec_ld_rung(index, row, "NO", &format!("%MX{}", 1000 + index * 2),
                    "OUTPUT", &format!("%MX{}", 1001 + index * 2))?;
                println!("PASS appended program {index} L{row}");
            }
            document.write_to(output)?;
            Ok(())
        }
        [command, source, output, program, row, contact, coil] if command == "generate" => {
            generate(
                Path::new(source),
                Path::new(output),
                program.parse()?,
                row.parse()?,
                contact,
                coil,
            )
        }
        [
            command,
            source,
            output,
            program,
            row,
            contact_kind,
            contact,
            coil_kind,
            coil,
        ] if command == "generate-kind" => generate_kind(
            Path::new(source),
            Path::new(output),
            program.parse()?,
            row.parse()?,
            contact_kind,
            contact,
            coil_kind,
            coil,
        ),
        [command, generated, resaved] if command == "verify" => {
            verify(Path::new(generated), Path::new(resaved))
        }
        [command, source, output, program, row, contact_kind, contact, coil_kind, coil]
            if command == "delete-row" =>
        {
            let mut document = XgwxDocument::from_path(source)?;
            document.delete_iec_ld_simple_row(
                program.parse()?,
                row.parse()?,
                contact_kind,
                contact,
                coil_kind,
                coil,
            )?;
            document.write_to(output)?;
            Ok(())
        }
        [command, source, output, program, group, row] if command == "delete-branch-top-row" => {
            let mut document = XgwxDocument::from_path(source)?;
            document.delete_iec_ld_branch_top_row(program.parse()?, group.parse()?, row.parse()?)?;
            document.write_to(output)?;
            Ok(())
        }
        _ => Err("usage: iec_rung_acceptance append-all SOURCE OUTPUT | generate SOURCE OUTPUT PROGRAM ROW CONTACT COIL | generate-kind SOURCE OUTPUT PROGRAM ROW CONTACT_KIND CONTACT COIL_KIND COIL | delete-row SOURCE OUTPUT PROGRAM ROW CONTACT_KIND CONTACT COIL_KIND COIL | verify GENERATED RESAVED".into()),
    }
}
