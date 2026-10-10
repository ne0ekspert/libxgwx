//! Generate a native-captured IEC vertical extension into an implicit blank row.
use std::{env, error::Error};
use xgwx::{LadderProgramData, XgwxDocument};
fn normalized(program: &LadderProgramData) -> Vec<u8> {
    let mut data = program.data.clone();
    for row in program.iec_row_frames().unwrap() {
        data[row.start + 17..row.start + 21].fill(0);
    }
    data
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=4).contains(&args.len()) {
        return Err(
            "usage: iec_new_branch_row_acceptance SOURCE OUTPUT [NATIVE] [NATIVE_DELETE]".into(),
        );
    }
    let mut doc = XgwxDocument::from_path(&args[0])?;
    let original = doc
        .ladder_programs()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    doc.extend_iec_ld_vertical_wire(0, 3, 4, 3)?;
    let changed = doc.ladder_programs().remove(0)?;
    assert_eq!(changed.data.len(), original[0].data.len() + 71);
    assert_eq!(
        changed.iec_row_frames().unwrap().len(),
        original[0].iec_row_frames().unwrap().len() + 1
    );
    assert_eq!(
        changed.iec_record_frames().unwrap().len(),
        original[0].iec_record_frames().unwrap().len() + 2
    );
    for (p, old) in original.iter().enumerate().skip(1) {
        assert!(doc.ladder_programs().remove(p)?.data == old.data);
    }
    if let Some(path) = args.get(2) {
        let native = XgwxDocument::from_path(path)?;
        for (p, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            assert!(
                normalized(&a?) == normalized(&b?),
                "native program {p} payload differs"
            );
        }
    }
    doc.write_to(&args[1])?;
    doc.edit_iec_ld_branch_segment(0, 3, 4, 5, 3, true, false)?;
    for (p, old) in original.iter().enumerate() {
        assert!(
            doc.ladder_programs().remove(p)?.data == old.data,
            "program {p} restoration differs"
        );
    }
    if let Some(path) = args.get(3) {
        let native = XgwxDocument::from_path(path)?;
        for (p, (a, b)) in doc
            .ladder_programs()
            .into_iter()
            .zip(native.ladder_programs())
            .enumerate()
        {
            assert!(
                normalized(&a?) == normalized(&b?),
                "native deletion program {p} differs"
            );
        }
    }
    doc.write_to(format!("{}.removed.xgwx", &args[1]))?;
    println!("PASS native blank-row extension, preserved rows and function bindings");
    Ok(())
}
