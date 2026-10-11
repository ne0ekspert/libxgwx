//! Run: cargo run --example program_handles --features write
//! Optionally pass an output directory to save the generated ST and LD projects.
use std::error::Error;
use xgwx::{NewProgram, ProgramVariablePatch, TextSourcePatch, create_project};

fn main() -> Result<(), Box<dyn Error>> {
    let mut text_doc = create_project("XGI-CPUE", "ST")?;
    text_doc.program_mut().create(&NewProgram {
        name: "Temporary".into(),
        language: "ST".into(),
        object_id: "aaaaaaaa-1111-2222-3333-444444444444".into(),
        symbol_id: "bbbbbbbb-1111-2222-3333-444444444444".into(),
    })?;
    let controller_id = text_doc
        .program()
        .get(0)?
        .summary()
        .object_id
        .ok_or("missing identity")?;
    let temporary_id = text_doc
        .program()
        .get(1)?
        .summary()
        .object_id
        .ok_or("missing identity")?;
    {
        // The selected handle owns the document borrow, so it can be stored.
        let mut program = text_doc.program_mut().get_if(0, &controller_id)?;
        program.rename("Controller")?;
        let expected_variables = program.view().declarations()?;
        program
            .variable_mut()
            .edit_declaration(&ProgramVariablePatch {
                expected_variables,
                name: "Enabled".into(),
                data_type: "BOOL".into(),
                description: "Local controller flag".into(),
                remove: false,
                declaration: None,
                update: false,
            })?;
        let mut text = program.text()?;
        let patch = TextSourcePatch {
            expected_source: text.source()?.ok_or("source unavailable")?,
            source: "Enabled := TRUE;\r\n".into(),
        };
        text.update(&patch)?;
        // Existing stale-source checks still reject replaying this edit.
        assert!(text.update(&patch).is_err());
    }
    {
        let mut controller = text_doc.program_mut().get_if(0, &controller_id)?;
        assert!(controller.move_to_if(1, "stale destination").is_err());
        controller.move_to_if(1, &temporary_id)?;
        assert_eq!(controller.index(), 1);
    }
    // Consuming delete prevents using the handle afterwards. Temporary moved
    // to index 0, so verify its captured identity before deleting it.
    text_doc.program_mut().get_if(0, &temporary_id)?.delete()?;
    let selected = text_doc.program().get(0)?;
    println!(
        "{}: {:?}",
        selected.summary().name.unwrap(),
        selected.text()?.source
    );

    let mut ladder_doc = create_project("XGI-CPUE", "LD")?;
    {
        let mut program = ladder_doc.program_mut().get(0)?;
        program.rename("Interlock")?;
        let mut ladder = program.iec_ld()?;
        // No program index argument: this editor is bound to Interlock.
        ladder.insert_single_element(0, 1, "contact", "NO", "%MX0")?;
        ladder.insert_single_element(0, 94, "coil", "OUTPUT", "%MX1")?;
        println!(
            "IEC records: {}",
            ladder
                .data()?
                .iec_record_frames()
                .ok_or("invalid IEC records")?
                .len()
        );
    }
    let text_bytes = text_doc.to_verified_bytes()?;
    let ladder_bytes = ladder_doc.to_verified_bytes()?;
    if let Some(output) = std::env::args_os().nth(1) {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output)?;
        std::fs::write(output.join("controller.xgwx"), text_bytes)?;
        std::fs::write(output.join("interlock.xgwx"), ladder_bytes)?;
        println!("Saved projects to {}", output.display());
    }
    Ok(())
}
