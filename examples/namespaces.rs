//! Inspect through borrowed program/variable namespaces (no optional features).
//! Run: `cargo run --example namespaces -- project.xgwx`.
use std::error::Error;
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args_os()
        .nth(1)
        .ok_or("usage: namespaces <file.xgwx>")?;
    let doc = XgwxDocument::from_path(input)?;
    let programs = doc.program();
    println!("Languages: {:?}", programs.languages());
    for (index, program) in programs.list().iter().enumerate() {
        println!("Program #{index}: {program:#?}");
    }
    for text in programs.text_programs() {
        println!(
            "{} source #{}: {:?}",
            text.language, text.program_index, text.source
        );
    }
    let variables = doc.variable();
    for global in variables.globals()? {
        println!("Global: {global:#?}");
    }
    for (index, locals) in variables.locals().into_iter().enumerate() {
        match locals {
            Ok(locals) => println!("Program #{index} local symbols: {locals:#?}"),
            Err(error) => eprintln!("Program #{index} local symbols: {error}"),
        }
    }
    Ok(())
}
