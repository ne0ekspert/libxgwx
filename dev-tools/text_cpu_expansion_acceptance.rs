use xgwx::{NewProgram, SfcVariablePatch, TextProgramPatch, VendorIlPatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = "target/text-other-cpus/generated";
    std::fs::create_dir_all(out)?;
    for (index, stem) in [
        "xgk-auto", "xece", "xech", "xecs", "xecu", "xemh2", "xemhp", "gipam", "kl", "xgr",
    ]
    .iter()
    .enumerate()
    {
        let mut doc =
            XgwxDocument::from_path(format!("fixtures/text-cpus/{stem}-native-blank.xgwx"))?;
        for language in doc
            .program_languages()
            .into_iter()
            .filter(|l| ["ST", "IL"].contains(l))
        {
            let program_index = doc.programs().len();
            doc.create_program(&NewProgram {
                name: format!("Added{language}"),
                language: language.into(),
                object_id: format!(
                    "abcd{:04x}-1234-4567-89ab-0123456789ab",
                    index * 4 + program_index * 2 + 1
                ),
                symbol_id: format!(
                    "abcd{:04x}-1234-4567-89ab-0123456789ab",
                    index * 4 + program_index * 2 + 2
                ),
            })?;
        }
        for program in doc.text_programs() {
            doc.edit_text_variable(
                &program.object_id,
                &SfcVariablePatch {
                    program_index: program.program_index,
                    expected_variables: program.variables.clone(),
                    name: "Count".into(),
                    data_type: "INT".into(),
                    description: "Source preservation check".into(),
                    remove: false,
                    update: false,
                    declaration: None,
                },
            )?;
            doc.edit_text_program(&TextProgramPatch {
                program_index: program.program_index,
                expected_object_id: program.object_id,
                expected_language: program.language.clone(),
                expected_source: program.source.unwrap(),
                source: if program.language == "ST" {
                    "Count := Count + 1;\r\n".into()
                } else {
                    "LD Count\r\nADD 1\r\nST Count\r\n".into()
                },
            })?;
        }
        std::fs::write(format!("{out}/TX{index:02}.xgwx"), doc.to_verified_bytes()?)?;
    }
    let mut doc = XgwxDocument::from_path("fixtures/empty-projects/new-xgk.xgwx")?;
    let p = doc.vendor_il_programs().remove(0);
    doc.edit_vendor_il(&VendorIlPatch{program_index:0,expected_object_id:p.object_id,expected_source:p.source.unwrap(),source:"LOAD M00000\nAND NOT M00001\nOUT M00002\nLOADP M00003\nOUTP M00004\nLOADN M00005\nRST M00006\nLOAD M00007\nSET M00008\nLOAD M00009\nMOV 1 D00100".into()})?;
    std::fs::write(format!("{out}/TXGKIL.xgwx"), doc.to_verified_bytes()?)?;
    Ok(())
}
