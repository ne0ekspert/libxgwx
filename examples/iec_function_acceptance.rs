//! Generate and verify one terminal IEC function deletion for XG5000 acceptance.
use std::{error::Error, path::Path};
use xgwx::XgwxDocument;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn number<T: std::str::FromStr>(value: &str) -> Result<T>
where
    T::Err: Error + 'static,
{
    Ok(value.parse()?)
}

fn generate(
    source: &Path,
    output: &Path,
    program_index: usize,
    block_offset: usize,
    name: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_terminal_function(program_index, block_offset, name)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    if program
        .iec_function_blocks()
        .ok_or("function framing failed")?
        .iter()
        .any(|block| block.record_offset == block_offset && block.name.value == name)
    {
        return Err("deleted function remains in generated output".into());
    }
    program
        .iec_circuit_graph()
        .ok_or("generated deletion has an invalid circuit graph")?;
    println!("PASS generated deletion of program {program_index} {name} at 0x{block_offset:X}");
    Ok(())
}

fn generate_standalone(
    source: &Path,
    output: &Path,
    program_index: usize,
    block_offset: usize,
    name: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_standalone_function(program_index, block_offset, name)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    if program
        .iec_function_blocks()
        .ok_or("function framing failed")?
        .iter()
        .any(|block| block.record_offset == block_offset && block.name.value == name)
    {
        return Err("deleted standalone function remains in generated output".into());
    }
    program
        .iec_circuit_graph()
        .ok_or("generated deletion has an invalid circuit graph")?;
    println!(
        "PASS generated standalone deletion of program {program_index} {name} at 0x{block_offset:X}"
    );
    Ok(())
}

fn generate_standalone_insert(
    source: &Path,
    output: &Path,
    program_index: usize,
    insertion_offset: usize,
    name: &str,
    input: &str,
    output_operand: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_standalone_function(
        program_index,
        insertion_offset,
        name,
        input,
        output_operand,
    )?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    if !program
        .iec_function_blocks()
        .ok_or("function framing failed")?
        .iter()
        .any(|block| block.record_offset == insertion_offset + 64 && block.name.value == name)
    {
        return Err("inserted standalone function is missing".into());
    }
    program
        .iec_circuit_graph()
        .ok_or("generated insertion has an invalid circuit graph")?;
    println!("PASS generated standalone {name} insertion at 0x{insertion_offset:X}");
    Ok(())
}

fn generate_terminal_move_insert(
    source: &Path,
    output: &Path,
    program_index: usize,
    contact_offset: usize,
    input: &str,
    output_operand: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_terminal_move(program_index, contact_offset, input, output_operand)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    reparsed.ladder_programs()[program_index]
        .as_ref()
        .map_err(|error| error.to_string())?
        .iec_circuit_graph()
        .ok_or("inserted terminal MOVE has an invalid circuit graph")?;
    println!("PASS generated terminal MOVE insertion at contact 0x{contact_offset:X}");
    Ok(())
}

fn generate_standalone_insert_with_symbol(
    source: &Path,
    output: &Path,
    program_index: usize,
    insertion_offset: usize,
    input: &str,
    output_symbol: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_local_symbol(program_index, output_symbol, "UDINT", "")?;
    document.insert_iec_ld_standalone_function(
        program_index,
        insertion_offset,
        "WORD_TO_UDINT",
        input,
        output_symbol,
    )?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let symbols = reparsed.iec_local_symbols();
    let symbol = symbols[program_index]
        .as_ref()
        .map_err(|error| error.to_string())?
        .iter()
        .find(|symbol| symbol.name == output_symbol)
        .ok_or("inserted UDINT symbol is missing")?;
    if symbol.data_type.as_deref() != Some("UDINT") {
        return Err("inserted symbol has the wrong type".into());
    }
    println!("PASS generated WORD_TO_UDINT with new UDINT output {output_symbol}");
    Ok(())
}

fn generate_cell(
    source: &Path,
    output: &Path,
    program_index: usize,
    block_offset: usize,
    name: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_function_cell(program_index, block_offset, name)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    if program
        .iec_function_blocks()
        .ok_or("function framing failed")?
        .iter()
        .any(|block| block.record_offset == block_offset && block.name.value == name)
    {
        return Err("deleted function cell remains in generated output".into());
    }
    program
        .iec_circuit_graph()
        .ok_or("generated function-cell deletion has an invalid circuit graph")?;
    println!(
        "PASS generated function-cell deletion of program {program_index} {name} at 0x{block_offset:X}"
    );
    Ok(())
}

fn generate_connected_arithmetic(
    source: &Path,
    output: &Path,
    program_index: usize,
    block_offset: usize,
    name: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.delete_iec_ld_connected_arithmetic(program_index, block_offset, name)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    program
        .iec_circuit_graph()
        .ok_or("generated arithmetic deletion has an invalid circuit graph")?;
    println!(
        "PASS generated connected {name} deletion in program {program_index} at 0x{block_offset:X}"
    );
    Ok(())
}

fn generate_cell_insert(
    source: &Path,
    output: &Path,
    program_index: usize,
    insertion_offset: usize,
    name: &str,
    instance: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    document.insert_iec_ld_function_cell(program_index, insertion_offset, name, instance)?;
    document.write_to(output)?;
    let reparsed = XgwxDocument::from_path(output)?;
    let program = reparsed
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let inserted = program
        .iec_function_blocks()
        .ok_or("function framing failed")?
        .into_iter()
        .find(|block| block.record_offset == insertion_offset)
        .ok_or("inserted function cell is missing")?;
    if inserted.name.value != name
        || inserted.instance.as_ref().map(|value| value.value.as_str()) != Some(instance)
    {
        return Err("inserted function identity changed".into());
    }
    program
        .iec_circuit_graph()
        .ok_or("generated function-cell insertion has an invalid circuit graph")?;
    println!(
        "PASS generated function-cell insertion of program {program_index} {name}/{instance} at 0x{insertion_offset:X}"
    );
    Ok(())
}

fn verify(generated: &Path, resaved: &Path) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
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
    let generated_symbols = generated
        .iec_local_symbols()
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let resaved_symbols = resaved
        .iec_local_symbols()
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if generated_symbols != resaved_symbols {
        return Err("native Save As changed IEC local symbol summaries".into());
    }
    println!(
        "PASS native Save As preserved every ProgramData payload and IEC local symbol summary"
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, source, output, program, block, name] if mode == "generate" => generate(
            Path::new(source),
            Path::new(output),
            number(program)?,
            number(block)?,
            name,
        ),
        [mode, source, output, program, block, name] if mode == "generate-standalone" => {
            generate_standalone(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(block)?,
                name,
            )
        }
        [mode, source, output, program, insertion, name, input, output_operand]
            if mode == "generate-standalone-insert" =>
        {
            generate_standalone_insert(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(insertion)?,
                name,
                input,
                output_operand,
            )
        }
        [mode, source, output, program, contact, input, output_operand]
            if mode == "generate-terminal-move-insert" =>
        {
            generate_terminal_move_insert(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(contact)?,
                input,
                output_operand,
            )
        }
        [mode, source, output, program, insertion, input, output_symbol]
            if mode == "generate-standalone-insert-with-symbol" =>
        {
            generate_standalone_insert_with_symbol(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(insertion)?,
                input,
                output_symbol,
            )
        }
        [mode, source, output, program, block, name] if mode == "generate-cell" => generate_cell(
            Path::new(source),
            Path::new(output),
            number(program)?,
            number(block)?,
            name,
        ),
        [mode, source, output, program, block, name] if mode == "generate-connected-arithmetic" =>
            generate_connected_arithmetic(Path::new(source), Path::new(output), number(program)?, number(block)?, name),
        [mode, source, output, program, insertion, name, instance]
            if mode == "generate-cell-insert" =>
        {
            generate_cell_insert(
                Path::new(source),
                Path::new(output),
                number(program)?,
                number(insertion)?,
                name,
                instance,
            )
        }
        [mode, generated, resaved] if mode == "verify" => {
            verify(Path::new(generated), Path::new(resaved))
        }
        _ => Err("usage: iec-function-acceptance generate SOURCE OUTPUT PROGRAM BLOCK NAME | generate-standalone SOURCE OUTPUT PROGRAM BLOCK NAME | generate-standalone-insert SOURCE OUTPUT PROGRAM INSERTION NAME INPUT OUTPUT_OPERAND | generate-standalone-insert-with-symbol SOURCE OUTPUT PROGRAM INSERTION INPUT OUTPUT_SYMBOL | generate-cell SOURCE OUTPUT PROGRAM BLOCK NAME | generate-cell-insert SOURCE OUTPUT PROGRAM INSERTION NAME INSTANCE | verify GENERATED RESAVED".into()),
    }
}
