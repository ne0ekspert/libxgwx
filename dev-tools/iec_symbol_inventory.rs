//! Show local IEC symbol allocation fields from a captured workspace.
use std::error::Error;
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: iec_symbol_inventory <file.xgwx>")?;
    let document = XgwxDocument::from_path(path)?;
    if let Some(other_path) = std::env::args().nth(2) {
        let other = XgwxDocument::from_path(other_path)?;
        let a = document.iec_local_symbols();
        let b = other.iec_local_symbols();
        if a.len() != b.len() {
            return Err("program counts differ".into());
        }
        for (program, (a, b)) in a.into_iter().zip(b).enumerate() {
            let (a, b) = (a?, b?);
            if a.len() != b.len() {
                println!(
                    "program {program}: symbol counts {} -> {}",
                    a.len(),
                    b.len()
                );
            }
            for (index, (a, b)) in a.iter().zip(&b).enumerate() {
                if a != b {
                    println!("program {program} symbol {index}:\nsource {a:?}\nnative {b:?}");
                }
            }
        }
        return Ok(());
    }
    for (program_index, symbols) in document.iec_local_symbols().into_iter().enumerate() {
        for (symbol_index, symbol) in symbols?.into_iter().enumerate() {
            println!(
                "program {program_index} symbol {symbol_index}: {} {:?} type={:?} class={} allocation={:?}/{:?}",
                symbol.name,
                symbol.address,
                symbol.data_type,
                symbol.storage_class,
                symbol.allocation_number,
                symbol.allocation_width,
            );
        }
    }
    Ok(())
}
