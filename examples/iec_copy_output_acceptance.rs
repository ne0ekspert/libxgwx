//! Give the copied R_TRIG/ADD/MOVE network distinct output destinations.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, output] = args.as_slice() else {
        return Err("usage: iec_copy_output_acceptance <instance-copy.xgwx> <output.xgwx>".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    // These are the captured FF46 text offsets in GCI.XGWX. All replacements
    // have equal UTF-16 length; each call guards the expected original text.
    document.update_iec_ld_function_operand(0, 11232, "%MW700", "%MW701")?; // ADD.OUT
    document.update_iec_ld_function_operand(0, 11624, "%MW700", "%MW701")?; // MOVE.IN
    document.update_iec_ld_function_operand(0, 11664, "%MW200", "%MW201")?; // MOVE.OUT
    fs::write(output, document.to_bytes()?)?;
    Ok(())
}
