//! Generate controlled edits and check their target values after XG5000 Save As.
//! A successful host check alone does not establish XG5000 acceptance.
use std::{error::Error, fs, path::Path};
use xgwx::{
    ModuleInputFilter, ModulePatch, NetworkModulePatch, NetworkPatch, ProgramPatch, VariablePatch,
    XgwxDocument,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const CASES: &[(&str, &str)] = &[
    (
        "noop",
        "Unchanged elements-io workspace; establish XG5000 baseline",
    ),
    ("filter", "Base 0 slot 2 input filter = 5 ms"),
    (
        "module-comment",
        "Base 0 slot 2 comment = Acceptance & XML <test>",
    ),
    (
        "module-replace",
        "Base 0 slot 2 = XGF-RD8A catalog defaults",
    ),
    ("module-delete", "Base 0 slot 2 absent; module count = 47"),
    (
        "module-insert",
        "Insert XGF-RD8A into prepared empty base 0 slot 2",
    ),
    (
        "network-replace",
        "Base 0 slot 2 = XGL-EDMF with companion network configuration",
    ),
    ("network-name", "Network 0 name = AcceptanceNetwork"),
    (
        "network-alias",
        "Base 1 slot 11 network alias = AcceptanceAlias",
    ),
    (
        "cpu-same-family",
        "XGK-CPUSN -> XGK-CPUHN; retained configuration requires build validation",
    ),
    (
        "compact-comment",
        "XBM-DR16S built-in I/O comment = CompactAcceptance",
    ),
    ("program-name", "Program 0 name = AcceptanceProgram"),
    ("ladder-operand", "Program 0 operand M00000 -> M00042"),
    ("variable-name", "Variable 0 name = _0000_DI00"),
];

fn fixture(case: &str) -> &'static str {
    if case == "compact-comment" {
        return "fixtures/XGB_Enet01.xgwx";
    }
    if matches!(
        case,
        "program-name" | "ladder-operand" | "variable-name" | "cpu-same-family"
    ) {
        "fixtures/elements.xgwx"
    } else {
        "fixtures/elements-io.xgwx"
    }
}

fn documents(case: &str) -> Result<(XgwxDocument, XgwxDocument)> {
    if !CASES.iter().any(|(id, _)| *id == case) {
        return Err("unknown case".into());
    }
    let mut source = XgwxDocument::from_path(fixture(case))?;
    // This derived baseline must itself pass XG5000 acceptance before insertion
    // can be attributed independently to insert_module.
    if case == "module-insert" {
        source.delete_module(0, 2)?;
    }
    let mut edited = source.clone();
    match case {
        "noop" => {}
        "filter" => edited.set_module_input_filter(0, 2, ModuleInputFilter::Ms5)?,
        "module-comment" => edited.update_module(
            0,
            2,
            &ModulePatch {
                comment: Some("Acceptance & XML <test>".into()),
                ..Default::default()
            },
        )?,
        "module-replace" => edited.select_module(0, 2, "XGF-RD8A")?,
        "module-delete" => edited.delete_module(0, 2)?,
        "module-insert" => edited.insert_module(0, 2, "XGF-RD8A")?,
        "network-replace" => edited.select_module(0, 2, "XGL-EDMF")?,
        "network-name" => edited.update_network(
            0,
            &NetworkPatch {
                name: Some("AcceptanceNetwork".into()),
                ..Default::default()
            },
        )?,
        "network-alias" => edited.update_network_module(
            1,
            11,
            &NetworkModulePatch {
                alias: Some("AcceptanceAlias".into()),
                ..Default::default()
            },
        )?,
        "cpu-same-family" => edited.select_cpu("XGK-CPUHN")?,
        "compact-comment" => edited.update_module(
            0,
            0,
            &ModulePatch {
                comment: Some("CompactAcceptance".into()),
                ..ModulePatch::default()
            },
        )?,
        "program-name" => edited.update_program(
            0,
            &ProgramPatch {
                name: Some("AcceptanceProgram".into()),
                ..Default::default()
            },
        )?,
        "ladder-operand" => {
            let program = edited.ladder_programs().remove(0)?;
            let cell = program
                .strings
                .iter()
                .find(|cell| cell.value == "M00000")
                .ok_or("baseline operand missing")?;
            edited.update_ladder_cell_text(0, cell.offset, "M00000", "M00042")?;
        }
        "variable-name" => edited.update_variable(
            0,
            &VariablePatch {
                name: Some("_0000_DI00".into()),
                ..Default::default()
            },
        )?,
        _ => unreachable!(),
    }
    Ok((source, edited))
}

// Check the edited section's semantic values, without demanding identical
// compressed bytes or volatile project timestamps after XG5000 serialization.
fn observation(case: &str, doc: &XgwxDocument) -> Result<String> {
    Ok(match case {
        "compact-comment" => format!("{}\n{}", compact_module_values(doc), network_values(doc)),
        "cpu-same-family" => format!(
            "{}\n{}\n{:?}",
            module_values(doc),
            network_values(doc),
            doc.configurations()
                .iter()
                .map(|c| c.type_code)
                .collect::<Vec<_>>()
        ),
        "program-name" => format!(
            "{:?}",
            doc.programs()
                .iter()
                .map(|p| (&p.name, &p.task, &p.comment))
                .collect::<Vec<_>>()
        ),
        "ladder-operand" => {
            let mut values = Vec::new();
            for program in doc.ladder_programs() {
                values.push(
                    program?
                        .strings
                        .into_iter()
                        .map(|s| s.value)
                        .collect::<Vec<_>>(),
                );
            }
            format!("{values:?}")
        }
        "variable-name" => format!(
            "{:?}",
            doc.variables()?
                .iter()
                .map(|v| (
                    &v.name,
                    &v.address_area,
                    v.address_number,
                    &v.data_type,
                    &v.description
                ))
                .collect::<Vec<_>>()
        ),
        "network-name" | "network-alias" => network_values(doc),
        "network-replace" => format!("{}\n{}\n{:#?}", module_values(doc), network_values(doc), {
            let mut configs = doc.xgpd_config_infos();
            configs.sort_by_key(|c| (c.kind.clone(), c.base, c.slot));
            for config in &mut configs {
                config.attributes.sort_by(|a, b| a.name.cmp(&b.name));
            }
            configs
        }),
        _ => module_values(doc),
    })
}

fn network_values(doc: &XgwxDocument) -> String {
    let mut networks = doc.networks();
    for network in &mut networks {
        network.modules.sort_by_key(|m| (m.base, m.slot));
        network.attributes.sort_by(|a, b| a.name.cmp(&b.name));
        for module in &mut network.modules {
            // The unchanged XG5000 baseline resave regenerates these addresses.
            // Preserve the model prefix and all non-pointer names verbatim.
            if let Some(name) = &mut module.name {
                normalize_runtime_name(name);
            }
            for attribute in &mut module.attributes {
                if attribute.name == "Name" {
                    normalize_runtime_name(&mut attribute.value);
                }
            }
            module.attributes.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }
    format!("{networks:#?}")
}

fn normalize_runtime_name(name: &mut String) {
    if let Some((prefix, address)) = name.rsplit_once("@0x")
        && !address.is_empty()
        && address.bytes().all(|b| b.is_ascii_hexdigit())
    {
        name.truncate(prefix.len());
    }
}

fn compact_module_values(doc: &XgwxDocument) -> String {
    let mut modules = doc.modules();
    if let Some(profile) = doc.cpu_hardware_profile() {
        // The unchanged 2026-09-09 native control changes only this built-in
        // SubType from 1 to 0. Never normalize other positions or variants.
        for module in &mut modules {
            if module.base == Some(profile.builtin_io_base)
                && module.slot == Some(profile.builtin_io_slot)
            {
                module.sub_type = Some(0);
            }
        }
    }
    format!(
        "{:?}",
        modules
            .iter()
            .map(|m| (
                m.base, m.slot, m.id, m.sub_type, &m.name, &m.comment, &m.details
            ))
            .collect::<Vec<_>>()
    )
}

fn module_values(doc: &XgwxDocument) -> String {
    format!(
        "{:?}",
        doc.modules()
            .iter()
            .map(|m| (
                m.base, m.slot, m.id, m.sub_type, &m.name, &m.comment, &m.details
            ))
            .collect::<Vec<_>>()
    )
}

fn generate(directory: &Path) -> Result<()> {
    // Refuse reuse so a previous run's evidence cannot be overwritten.
    fs::create_dir(directory)?;
    let mut manifest = String::from(
        "case\tfixture\tintent\tlocal\topen\tinspect\tprogram_check\tsave_as\treparse\n",
    );
    for (case, intent) in CASES {
        let (source, edited) = documents(case)?;
        let bytes = edited.to_bytes()?;
        if *case == "filter" {
            let mut catalog_edit = source.clone();
            catalog_edit.set_module_option(0, 2, "inputFilter", 0, 5)?;
            if catalog_edit.to_bytes()? != bytes {
                return Err(
                    "typed and catalog input-filter writers produced different bytes".into(),
                );
            }
        }
        let reparsed = XgwxDocument::parse(&bytes)?;
        if observation(case, &edited)? != observation(case, &reparsed)? {
            return Err(format!("{case}: local target mismatch").into());
        }
        if *case == "noop" && bytes != fs::read(fixture(case))? {
            return Err("no-op byte identity failed".into());
        }
        source.write_to(directory.join(format!("{case}-source.xgwx")))?;
        fs::write(directory.join(format!("{case}.xgwx")), bytes)?;
        fs::write(
            directory.join(format!("{case}-expected.txt")),
            observation(case, &edited)?,
        )?;
        manifest.push_str(&format!(
            "{case}\t{}\t{intent}\tPASS\tPENDING\tPENDING\tPENDING\tPENDING\tPENDING\n",
            fixture(case)
        ));
    }
    fs::write(directory.join("matrix.tsv"), manifest)?;
    println!(
        "Generated {} cases in {}. XG5000 stages remain PENDING.",
        CASES.len(),
        directory.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, directory] if mode == "generate" => generate(Path::new(directory)),
        [mode, case, generated, resaved] if mode == "verify" => {
            if !CASES.iter().any(|(id, _)| id == case) {
                return Err("unknown case".into());
            }
            let expected = XgwxDocument::from_path(generated)?;
            let actual = XgwxDocument::from_path(resaved)?;
            let expected = observation(case, &expected)?;
            let actual = observation(case, &actual)?;
            if expected != actual {
                let expected_lines: Vec<_> = expected.lines().collect();
                let actual_lines: Vec<_> = actual.lines().collect();
                let line = (0..expected_lines.len().max(actual_lines.len()))
                    .find(|&i| expected_lines.get(i) != actual_lines.get(i))
                    .unwrap_or(0);
                let excerpt = |lines: &[&str]| {
                    lines
                        .get(line)
                        .unwrap_or(&"<end>")
                        .chars()
                        .take(240)
                        .collect::<String>()
                };
                eprintln!(
                    "First difference at observation line {}:\nExpected: {}\nActual: {}",
                    line + 1,
                    excerpt(&expected_lines),
                    excerpt(&actual_lines)
                );
                return Err(format!("{case}: resaved section mismatch").into());
            }
            println!("PASS {case}: resaved section matches. Record UI/build evidence separately.");
            Ok(())
        }
        _ => Err(
            "usage: xg5000-acceptance generate NEW_DIRECTORY | verify CASE GENERATED RESAVED"
                .into(),
        ),
    }
}
