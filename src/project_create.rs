//! Project generation using library-owned native defaults, never a workspace file.
use crate::{NewProgram, XgwxDocument, XgwxError};

// The fixed XG5000 4.82.1 header, without derived checksum/size fields.
const HEADER: &[u8] = &[
    88, 71, 0, 4, 2, 0, 255, 254, 255, 21, 88, 0, 71, 0, 53, 0, 48, 0, 48, 0, 48, 0, 32, 0, 87, 0,
    79, 0, 82, 0, 75, 0, 83, 0, 80, 0, 65, 0, 67, 0, 69, 0, 32, 0, 70, 0, 73, 0, 76, 0, 69, 0, 1,
    0, 0, 0, 255, 254, 255, 0, 255, 254, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Create a blank workspace for a supported CPU and LD/SFC/ST/IL language.
///
/// Native parameter/security defaults and existing seed identities are retained.
/// Containers are generated using the same compression, checksum and alignment
/// rules as edits. No filesystem access or embedded `.xgwx` container is used.
/// Unsupported CPU/language combinations fail before a document is returned.
pub fn create_project(cpu_model: &str, language: &str) -> Result<XgwxDocument, XgwxError> {
    let fail = |reason: &str| XgwxError::ProgramCreation(reason.into());
    let cpu = crate::cpu_catalog()
        .iter()
        .find(|cpu| cpu.model.eq_ignore_ascii_case(cpu_model))
        .ok_or_else(|| fail("unknown CPU model"))?;
    let language = language.to_ascii_uppercase();
    if !matches!(language.as_str(), "LD" | "SFC" | "ST" | "IL") {
        return Err(fail("unknown program language"));
    }
    let profile = match (cpu.family, cpu.type_code, language.as_str()) {
        ("XGK", 3, "ST") => "xgk-auto",
        ("XGK", _, "LD" | "IL") => "xgk",
        ("XGI", _, "LD" | "ST" | "IL") => "xgi",
        ("XGI", 100 | 102 | 104 | 106 | 107 | 111, "SFC") => "xgi-sfc",
        ("XGB", 109, "ST" | "IL") => "xece",
        ("XGB", 103, "ST" | "IL") => "xech",
        ("XGB", 108, "ST" | "IL") => "xecs",
        ("XGB", 112, "ST" | "IL") => "xecu",
        ("XGB", 116, "ST" | "IL") => "xemh2",
        ("XGB", 115, "ST" | "IL") => "xemhp",
        ("XGB", 114, "ST" | "IL") => "gipam",
        ("XGB", 113, "ST" | "IL") => "kl",
        ("XGR", 101, "ST" | "IL") => "xgr",
        _ => return Err(fail("no native defaults for this CPU and program language")),
    };
    let (xml, security): (&str, &[u8]) = match profile {
        "xgk" => (
            include_str!("project_defaults/xgk.xml"),
            include_bytes!("project_defaults/xgk.security"),
        ),
        "xgi" => (
            include_str!("project_defaults/xgi.xml"),
            include_bytes!("project_defaults/xgi.security"),
        ),
        "xgi-sfc" => (
            include_str!("project_defaults/xgi-sfc.xml"),
            include_bytes!("project_defaults/xgi-sfc.security"),
        ),
        "xgk-auto" => (
            include_str!("project_defaults/xgk-auto.xml"),
            include_bytes!("project_defaults/xgk-auto.security"),
        ),
        "xece" => (
            include_str!("project_defaults/xece.xml"),
            include_bytes!("project_defaults/xece.security"),
        ),
        "xech" => (
            include_str!("project_defaults/xech.xml"),
            include_bytes!("project_defaults/xech.security"),
        ),
        "xecs" => (
            include_str!("project_defaults/xecs.xml"),
            include_bytes!("project_defaults/xecs.security"),
        ),
        "xecu" => (
            include_str!("project_defaults/xecu.xml"),
            include_bytes!("project_defaults/xecu.security"),
        ),
        "xemh2" => (
            include_str!("project_defaults/xemh2.xml"),
            include_bytes!("project_defaults/xemh2.security"),
        ),
        "xemhp" => (
            include_str!("project_defaults/xemhp.xml"),
            include_bytes!("project_defaults/xemhp.security"),
        ),
        "gipam" => (
            include_str!("project_defaults/gipam.xml"),
            include_bytes!("project_defaults/gipam.security"),
        ),
        "kl" => (
            include_str!("project_defaults/kl.xml"),
            include_bytes!("project_defaults/kl.security"),
        ),
        "xgr" => (
            include_str!("project_defaults/xgr.xml"),
            include_bytes!("project_defaults/xgr.security"),
        ),
        _ => unreachable!(),
    };
    let bytes = crate::writer::serialize_new_workspace(HEADER, xml.as_bytes(), security)?;
    let mut doc = XgwxDocument::parse(&bytes)?;
    // Keep migration checks in the public writer; do not overwrite type codes.
    doc.select_cpu(cpu.model)?;
    let program = doc
        .programs()
        .into_iter()
        .next()
        .ok_or_else(|| fail("missing default program"))?;
    let native_language = match program.kind {
        Some(0) => "LD",
        Some(3) => "SFC",
        Some(4) => "ST",
        Some(9) => "IL",
        _ => return Err(fail("unknown default program kind")),
    };
    // Vendor IL is a view of the same XGK ladder records.
    if native_language != language && !(cpu.family == "XGK" && language == "IL") {
        doc.delete_program(
            0,
            program
                .object_id
                .as_deref()
                .ok_or_else(|| fail("missing default program identity"))?,
        )?;
        doc.create_program(&NewProgram {
            name: "NewProgram".into(),
            language,
            object_id: "b9b4302a-9406-4011-9801-3d8ad7330bde".into(),
            symbol_id: "1c7e5d5a-8839-42dd-9ae2-699217f83952".into(),
        })?;
    }
    // Return the normalized container, not pre-edit compression/header state.
    XgwxDocument::parse(&doc.to_verified_bytes()?)
}
