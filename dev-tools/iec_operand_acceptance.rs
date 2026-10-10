//! Generate and verify one typed IEC function-operand edit for native XG5000 acceptance.
use std::{error::Error, path::Path};
use xgwx::{IecFunctionBinding, LadderProgramData, XgwxDocument};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn binding(program: &LadderProgramData, function: &str, pin: &str) -> Result<IecFunctionBinding> {
    let matches = program
        .iec_circuit_graph()
        .ok_or("IEC circuit graph validation failed")?
        .function_bindings
        .into_iter()
        .filter(|item| {
            item.function_name == function
                && item.pin_name == pin
                && item.expression_record_offset.is_some()
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [item] => Ok(item.clone()),
        [] => Err(format!("no visible {function}.{pin} binding").into()),
        _ => Err(format!("{function}.{pin} is ambiguous: {} matches", matches.len()).into()),
    }
}

fn expression(
    program: &LadderProgramData,
    binding: &IecFunctionBinding,
) -> Result<(usize, String)> {
    let record_offset = binding
        .expression_record_offset
        .ok_or("binding has no expression record")?;
    let record = program
        .iec_record_frames()
        .ok_or("IEC record framing failed")?
        .into_iter()
        .find(|record| record.offset == record_offset)
        .ok_or("expression record is missing")?;
    let matches = program
        .strings
        .iter()
        .filter(|item| item.offset >= record.offset && item.end_offset <= record.end)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [item] => Ok((item.offset, item.value.clone())),
        _ => Err(format!(
            "expression record at {record_offset} contains {} strings",
            matches.len()
        )
        .into()),
    }
}

fn target(
    document: &XgwxDocument,
    program_index: usize,
    function: &str,
    pin: &str,
) -> Result<(usize, String)> {
    let program = document
        .ladder_programs()
        .into_iter()
        .nth(program_index)
        .ok_or("program index is out of range")??;
    let binding = binding(&program, function, pin)?;
    expression(&program, &binding)
}

fn generate(
    source: &Path,
    output: &Path,
    program_index: usize,
    function: &str,
    pin: &str,
    expected: &str,
    replacement: &str,
) -> Result<()> {
    let mut document = XgwxDocument::from_path(source)?;
    let (offset, actual) = target(&document, program_index, function, pin)?;
    if actual != expected {
        return Err(format!("expected {function}.{pin}={expected}, found {actual}").into());
    }
    document.update_iec_ld_function_operand(program_index, offset, expected, replacement)?;
    document.write_to(output)?;

    let reparsed = XgwxDocument::from_path(output)?;
    let (_, actual) = target(&reparsed, program_index, function, pin)?;
    if actual != replacement {
        return Err(format!("generated target is {actual}, expected {replacement}").into());
    }
    println!(
        "PASS generated program {program_index} {function}.{pin}: {expected} -> {replacement} at byte {offset}"
    );
    Ok(())
}

fn verify(
    generated: &Path,
    resaved: &Path,
    program_index: usize,
    function: &str,
    pin: &str,
    expected: &str,
) -> Result<()> {
    let generated = XgwxDocument::from_path(generated)?;
    let resaved = XgwxDocument::from_path(resaved)?;
    let (_, actual) = target(&resaved, program_index, function, pin)?;
    if actual != expected {
        return Err(format!("resaved target is {actual}, expected {expected}").into());
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
        "PASS native Save As preserved {function}.{pin}={expected} and every decoded ProgramData payload"
    );
    Ok(())
}

fn parse_index(value: &str) -> Result<usize> {
    Ok(value.parse()?)
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, source, output, program, function, pin, expected, replacement]
            if mode == "generate" =>
        {
            generate(
                Path::new(source),
                Path::new(output),
                parse_index(program)?,
                function,
                pin,
                expected,
                replacement,
            )
        }
        [mode, generated, resaved, program, function, pin, expected] if mode == "verify" => {
            verify(
                Path::new(generated),
                Path::new(resaved),
                parse_index(program)?,
                function,
                pin,
                expected,
            )
        }
        _ => Err("usage: iec-operand-acceptance generate SOURCE OUTPUT PROGRAM FUNCTION PIN EXPECTED REPLACEMENT | verify GENERATED RESAVED PROGRAM FUNCTION PIN EXPECTED".into()),
    }
}
