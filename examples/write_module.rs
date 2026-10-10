//! Change a supported digital input module's filter and save a workspace.
//!
//! Run: `cargo run --features write --example write-module -- input.xgwx
//! output.xgwx 0 2 5` (base 0, slot 2, raw filter value 5).
//! Requires a uniquely identified, supported module. The output path is written
//! or overwritten; use a different path to preserve the original workspace.
use std::env;
use std::error::Error;
use std::io;

use xgwx::{ModuleInputFilter, XgwxDocument};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let input = required_arg(&mut args, "input .xgwx path")?;
    let output = required_arg(&mut args, "output .xgwx path")?;
    let base = parse_u32(required_arg(&mut args, "base")?, "base")?;
    let slot = parse_u32(required_arg(&mut args, "slot")?, "slot")?;
    let raw_filter = parse_u8(
        required_arg(&mut args, "raw input filter")?,
        "raw input filter",
    )?;
    if args.next().is_some() {
        return Err(invalid_input("too many arguments").into());
    }

    let mut doc = XgwxDocument::from_path(&input)?;
    doc.set_module_input_filter(base, slot, ModuleInputFilter::from_raw(raw_filter))?;
    doc.write_to(&output)?;

    println!("wrote {output}");
    Ok(())
}

fn required_arg(args: &mut impl Iterator<Item = String>, name: &str) -> io::Result<String> {
    args.next().ok_or_else(|| {
        invalid_input(format!(
            "missing {name}; usage: write-module <input> <output> <base> <slot> <raw-filter>"
        ))
    })
}

fn parse_u32(value: String, name: &str) -> io::Result<u32> {
    value
        .parse()
        .map_err(|_| invalid_input(format!("{name} must be an unsigned integer")))
}

fn parse_u8(value: String, name: &str) -> io::Result<u8> {
    value
        .parse()
        .map_err(|_| invalid_input(format!("{name} must be between 0 and 255")))
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
