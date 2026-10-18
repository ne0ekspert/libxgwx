use base64::Engine;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use xgwx::{XgwxDocument, XgwxError};

fn bytes(xml: &str) -> Vec<u8> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(xml.as_bytes()).unwrap();
    let mut bytes = b"XG".to_vec();
    bytes.extend(gzip.finish().unwrap());
    bytes
}

fn source_block(name: &str, kind: u32, source: &str) -> String {
    let raw: Vec<_> = source.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder.write_all(&raw).unwrap();
    let payload = base64::engine::general_purpose::STANDARD.encode(encoder.finish().unwrap());
    format!(
        "<SFC_ProgramProperty LanguageType='{kind}' Language='4' PragramName='{name}' Comment='' MainBlock='0'><SFC_ProgramData UploadProgramSize='0' DocRungTableOffset='0' DocRungTableSize='0' PreDocStepCount='0' PostDocStepCount='0' Variable='' InitVariable='' NFE='' NFE1=''/>{}<ST_Program Version='Ver 1.1' StepCount='0'><CodeList CodeCount='{}' Compressed='1'>{payload}</CodeList><Breakpoints/><Bookmarks/></ST_Program></SFC_ProgramProperty>",
        if kind == 1 {
            "<SFC_ProgramDataAction ActionPostScan='0'/>"
        } else {
            ""
        },
        raw.len() / 2
    )
}

fn step(title: &str, initial: bool, program: bool) -> String {
    format!(
        "<EntityStep Title='{title}' InnerVariableName='' Comment='' Bookmark='0' BreakPoint='0' PropertyProgram='{}' InitialStep='{}' StepVariable='0'/>",
        u8::from(program),
        u8::from(initial)
    )
}

fn chart(rows: usize, source: &str) -> String {
    let entities: String = (0..rows).map(|row| format!(
        "<EntityProperty Type='0' Row='{row}' Col='0'><EntityBasic Virtural='0'/>{}</EntityProperty><EntityProperty Type='2' Row='{row}' Col='1'><EntityBasic Virtural='0'/>{}<EntityAction FBInstanceName='' FBInstanceOut='' ResetVarName='' TimeIndex='0' Qualifier='1' Time=''/></EntityProperty>",
        step(&format!("Step{row}"), row==0, false), step("Shared", false, true))).collect();
    format!(
        "<Program><SFC_ProgramList><SFC_ProgramProperty LanguageType='3' Language='2' PragramName='Main' MainBlock='1'><EntityContainer BookmarkCount='0'><EntityGrid RowSize='{rows}' ColSize='2'><EntityPropertyList>{entities}</EntityPropertyList></EntityGrid></EntityContainer></SFC_ProgramProperty>{}</SFC_ProgramList></Program>",
        source_block("Shared", 1, source)
    )
}

#[test]
fn compressed_repeated_sfc_source_is_rejected_before_row_cloning() {
    let input = bytes(&format!(
        "<Project>{}</Project>",
        chart(128, &"\u{0800}".repeat(65536))
    ));
    assert!(input.len() < 4096);
    match XgwxDocument::parse(&input) {
        Err(XgwxError::ResourceLimitExceeded { .. }) => {}
        Err(error) => panic!("unexpected error: {error}"),
        Ok(doc) => {
            let programs = doc.program().sfc_programs();
            let rows = programs[0].blocks[0]
                .editable_rows
                .as_ref()
                .expect("valid chart");
            let copies: usize = rows
                .iter()
                .filter_map(|r| r.action_code.as_ref())
                .map(String::len)
                .sum();
            panic!("accepted compressed chart and cloned {copies} source bytes");
        }
    }
}

#[test]
fn ordinary_shared_actions_keep_complete_source_in_every_row() {
    let doc = XgwxDocument::parse(&bytes(&format!(
        "<Project>{}</Project>",
        chart(8, "Count := 1;")
    )))
    .unwrap();
    let programs = doc.program().sfc_programs();
    let rows = programs[0].blocks[0].editable_rows.as_ref().unwrap();
    assert_eq!(rows.len(), 8);
    assert!(
        rows.iter()
            .all(|r| r.action_code.as_deref() == Some("Count := 1;"))
    );
}

fn rejects(xml: &str) {
    match XgwxDocument::parse(&bytes(xml)) {
        Err(XgwxError::ResourceLimitExceeded {
            resource: "SFC expanded source bytes",
            ..
        }) => {}
        Err(error) => panic!("unexpected rejection: {error}"),
        Ok(_) => panic!("expected aggregate SFC source rejection"),
    }
}

#[test]
fn sibling_charts_share_one_source_budget() {
    let program = chart(32, &"\u{0800}".repeat(65536));
    XgwxDocument::parse(&bytes(&format!("<Project>{program}</Project>"))).unwrap();
    rejects(&format!("<Project>{}</Project>", program.repeat(3)));
}

#[test]
fn nested_programs_cannot_reuse_one_physical_list_without_being_charged() {
    let program = chart(32, &"\u{0800}".repeat(65536));
    rejects(&format!(
        "<Project><Program><Program>{program}</Program></Program></Project>"
    ));
}

#[test]
fn namespace_collisions_use_the_same_last_attribute_values_as_row_properties() {
    let program = chart(128, &"\u{0800}".repeat(65536))
        .replace(
            "Title='Shared'",
            "Title='Unused' xmlns:p='urn:test' p:Title='Shared'",
        )
        .replace(
            "PropertyProgram='1'",
            "PropertyProgram='0' p:PropertyProgram='1'",
        );
    rejects(&format!("<Project>{program}</Project>"));
}

#[test]
fn unsupported_source_metadata_remains_inspectable_without_editable_rows() {
    let xml = format!(
        "<Project>{}</Project>",
        chart(128, &"\u{0800}".repeat(65536))
    )
    .replace("Version='Ver 1.1'", "Version='Unknown'");
    let doc = XgwxDocument::parse(&bytes(&xml)).unwrap();
    assert!(
        doc.program().sfc_programs()[0].blocks[0]
            .editable_rows
            .is_none()
    );
}

#[test]
fn unreferenced_definitions_are_charged_before_source_map_decoding() {
    let template = source_block("SOURCE_NAME", 1, &"\u{0800}".repeat(65536));
    let definitions: String = (0..86)
        .map(|i| template.replace("SOURCE_NAME", &format!("Source{i}")))
        .collect();
    rejects(&format!(
        "<Project><Program><SFC_ProgramList>{definitions}</SFC_ProgramList></Program></Project>"
    ));
}

#[cfg(feature = "write")]
#[test]
fn excessive_xml_replacement_leaves_the_original_document_unchanged() {
    let mut doc = XgwxDocument::parse(&bytes(&format!(
        "<Project>{}</Project>",
        chart(8, "Count := 1;")
    )))
    .unwrap();
    let original = doc.clone();
    assert!(matches!(
        doc.container_mut().replace_xml(format!(
            "<Project>{}</Project>",
            chart(128, &"\u{0800}".repeat(65536))
        )),
        Err(XgwxError::ResourceLimitExceeded { .. })
    ));
    assert_eq!(doc, original);
}

#[cfg(all(feature = "ffi", not(target_arch = "wasm32")))]
#[test]
fn ffi_parse_reports_resource_failure_without_returning_a_handle() {
    use xgwx::ffi::*;
    let input = bytes(&format!(
        "<Project>{}</Project>",
        chart(128, &"\u{0800}".repeat(65536))
    ));
    let mut handle = std::ptr::null_mut();
    assert_eq!(
        unsafe { xgwx_document_parse(input.as_ptr(), input.len(), &mut handle) },
        XGWX_PARSE_ERROR
    );
    assert!(handle.is_null());
}

#[test]
fn source_budget_boundary_is_checked_across_all_repeated_references() {
    let accepted = format!(
        "<Project>{}</Project>",
        chart(255, &"\u{0800}".repeat(21845))
    );
    XgwxDocument::parse(&bytes(&accepted)).unwrap(); // 256 * 21,845 * 3 = 16 MiB - 256
    rejects(&format!(
        "<Project>{}</Project>",
        chart(255, &"\u{0800}".repeat(21846))
    ));
}

#[test]
fn branch_sources_share_the_document_budget() {
    let native = XgwxDocument::from_path("fixtures/sfc/multi-action-branch-native.xgwx").unwrap();
    let mut xml = native.container().xml().to_owned();
    let replacement_block = source_block("Shared", 1, &"\u{0800}".repeat(65536));
    let replacement_tree = roxmltree::Document::parse(&replacement_block).unwrap();
    let replacement = replacement_tree
        .descendants()
        .find(|n| n.has_tag_name("CodeList"))
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let ranges: Vec<_> = tree
        .descendants()
        .filter(|n| n.has_tag_name("CodeList"))
        .map(|n| n.range())
        .collect();
    for range in ranges.into_iter().rev() {
        xml.replace_range(range, &replacement_block[replacement.range()]);
    }
    let doc = XgwxDocument::parse(&bytes(&xml)).unwrap();
    let programs = doc.program().sfc_programs();
    let rows = programs[0].blocks[0].editable_rows.as_ref().unwrap();
    assert!(rows.iter().any(|r| r.position.is_some()));
    assert!(rows.iter().any(|r| r.kind == "continuation"));
    assert!(rows.iter().any(|r| r.action_code.is_some()));
    let tree = roxmltree::Document::parse(&xml).unwrap();
    let program = tree
        .descendants()
        .find(|n| n.has_tag_name("Program"))
        .unwrap();
    rejects(&format!(
        "<Project>{}</Project>",
        xml[program.range()].repeat(32)
    ));
}

fn continuation_chart(rows: usize, source: &str) -> String {
    let mut program = chart(rows, source);
    for row in 1..rows {
        program = program
            .replace(
                &format!("Type='0' Row='{row}'"),
                &format!("Type='7' Row='{row}'"),
            )
            .replace(&step(&format!("Step{row}"), false, false), "");
    }
    program
}

#[test]
fn continuation_actions_are_charged_even_without_ordinary_step_growth() {
    let accepted = continuation_chart(8, "Count := 1;");
    let doc = XgwxDocument::parse(&bytes(&format!("<Project>{accepted}</Project>"))).unwrap();
    let programs = doc.program().sfc_programs();
    let rows = programs[0].blocks[0].editable_rows.as_ref().unwrap();
    assert!(
        rows[1..]
            .iter()
            .all(|r| r.kind == "continuation" && r.action_code.as_deref() == Some("Count := 1;"))
    );
    rejects(&format!(
        "<Project>{}</Project>",
        continuation_chart(128, &"\u{0800}".repeat(65536))
    ));
}

fn transition_chart(rows: usize, source: &str) -> String {
    let mut program = chart(rows, source)
        .replace("LanguageType='1'", "LanguageType='2'")
        .replace("<SFC_ProgramDataAction ActionPostScan='0'/>", "");
    let tree = roxmltree::Document::parse(&program).unwrap();
    let replacements: Vec<_> = tree.descendants().filter(|n| n.has_tag_name("EntityProperty")).map(|entity| {
        let column = entity.attribute("Col").unwrap();
        let row = entity.attribute("Row").unwrap();
        let kind = if column == "0" { 1 } else { 9 };
        (entity.range(), format!("<EntityProperty Type='{kind}' Row='{row}' Col='{column}'><EntityBasic Virtural='0'/>{}</EntityProperty>", step("Shared", false, column == "0")))
    }).collect();
    for (range, replacement) in replacements.into_iter().rev() {
        program.replace_range(range, &replacement);
    }
    program
}

#[test]
fn transition_source_references_are_also_charged() {
    let accepted = transition_chart(8, "TRANS := TRUE;");
    let doc = XgwxDocument::parse(&bytes(&format!("<Project>{accepted}</Project>"))).unwrap();
    let programs = doc.program().sfc_programs();
    assert!(
        programs[0].blocks[0]
            .editable_rows
            .as_ref()
            .unwrap()
            .iter()
            .all(|r| r.transition_code.as_deref() == Some("TRANS := TRUE;"))
    );
    rejects(&format!(
        "<Project>{}</Project>",
        transition_chart(128, &"\u{0800}".repeat(65536))
    ));
}

#[test]
fn repeated_list_traversal_is_bounded_even_for_empty_sources() {
    let xml = format!(
        "<Project>{}{}{}</Project>",
        "<Program>".repeat(160),
        chart(4000, ""),
        "</Program>".repeat(160)
    );
    match XgwxDocument::parse(&bytes(&xml)) {
        Err(XgwxError::ResourceLimitExceeded {
            resource: "SFC source inspection work",
            ..
        }) => {}
        Err(error) => panic!("unexpected rejection: {error}"),
        Ok(_) => panic!("unbounded repeated list traversal"),
    }
}
