//! Compare program 3 L57 Delete Line with the native XG5000 capture.
use std::{env, error::Error, fs};
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [source, native, output] = args.as_slice() else {
        return Err("usage: iec_x3_terminal_contact_branch_acceptance SOURCE NATIVE OUTPUT".into());
    };
    let mut document = XgwxDocument::from_path(source)?;
    document.edit_iec_ld_branch_segment(3, 20, 56, 57, 3, true, false)?;
    fs::write(output, document.to_bytes()?)?;
    let generated = XgwxDocument::from_path(output)?;
    let native = XgwxDocument::from_path(native)?;
    for (index, (generated, native)) in generated
        .ladder_programs()
        .into_iter()
        .zip(native.ladder_programs())
        .enumerate()
    {
        let generated = generated?;
        let native = native?;
        let mut generated_bytes = generated.data.clone();
        let mut native_bytes = native.data.clone();
        if index == 3 {
            // XG5000 refreshes unrelated row display-cache bytes on Save.
            for offset in [0x77, 0x1bc, 0x252, 0x397, 0x1f5e, 0x20ad, 0x212c] {
                generated_bytes[offset] = 0;
                native_bytes[offset] = 0;
            }
        }
        if generated_bytes != native_bytes {
            return Err(format!("program {index} differs from native Delete Line").into());
        }
    }
    println!("PASS native program 3 L57 terminal branch deletion");
    Ok(())
}
