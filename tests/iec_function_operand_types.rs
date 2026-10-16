#![cfg(feature = "write")]
use xgwx::XgwxDocument;

#[test]
fn unresolved_function_operands_are_rejected_atomically() {
    let source = XgwxDocument::parse(include_bytes!(
        "../fixtures/function-output-wires/base.xgwx"
    ))
    .unwrap();
    let program = source.program().ladder_programs().remove(0).unwrap();
    let blocks = program.iec_function_blocks().unwrap();
    let links = program.iec_function_operand_links().unwrap();
    for name in ["ADD", "SUB", "DIV", "EQ"] {
        let block = blocks
            .iter()
            .find(|block| block.name.value == name)
            .unwrap();
        for link in links
            .iter()
            .filter(|link| link.target_record_offset == block.record_offset)
        {
            let offset = link.record_offset + 15;
            let value = &program
                .strings
                .iter()
                .find(|s| s.offset == offset)
                .unwrap()
                .value;
            for replacement in ["asdf", "MissingVariable", "MissingInstance.OUT"] {
                let mut document = source.clone();
                let before = document.to_bytes().unwrap();
                let error = document
                    .program_mut()
                    .get(0)
                    .and_then(|mut selected| {
                        selected
                            .iec_ld()?
                            .update_function_operand(offset, value, replacement)
                    })
                    .unwrap_err();
                assert!(
                    error.to_string().contains("known scalar expression"),
                    "{name}: {error}"
                );
                assert_eq!(
                    document.to_bytes().unwrap(),
                    before,
                    "{name}: {replacement}"
                );
            }
            // A resolved operand of the original type still succeeds.
            let mut document = source.clone();
            document
                .program_mut()
                .get(0)
                .and_then(|mut selected| {
                    selected
                        .iec_ld()?
                        .update_function_operand(offset, value, value)
                })
                .unwrap();
            if !link.is_output {
                for replacement in ["2", "%MW8"] {
                    let mut document = source.clone();
                    document
                        .program_mut()
                        .get(0)
                        .and_then(|mut selected| {
                            selected
                                .iec_ld()?
                                .update_function_operand(offset, value, replacement)
                        })
                        .unwrap();
                }
                let mut document = source.clone();
                let before = document.to_bytes().unwrap();
                assert!(
                    document
                        .program_mut()
                        .get(0)
                        .and_then(|mut selected| {
                            selected
                                .iec_ld()?
                                .update_function_operand(offset, value, "TRUE")
                        })
                        .is_err()
                );
                assert_eq!(document.to_bytes().unwrap(), before);
            }
        }
    }
}
