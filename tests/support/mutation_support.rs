//! Translate captured regression fixture patches to the selected public API.
#![allow(dead_code)]
use xgwx::*;

pub fn edit_text_program(
    mut program: Program<'_>,
    patch: &TextProgramPatch,
) -> Result<(), XgwxError> {
    if patch.expected_object_id.is_empty() || program.object_id() != patch.expected_object_id {
        return Err(XgwxError::TextProgramEdit(
            "program identity changed".into(),
        ));
    }
    if program.view().text()?.language != patch.expected_language {
        return Err(XgwxError::TextProgramEdit(
            "program language changed".into(),
        ));
    }
    program.text()?.update(&TextSourcePatch {
        expected_source: patch.expected_source.clone(),
        source: patch.source.clone(),
    })
}

#[cfg(feature = "il")]
pub fn edit_vendor_il(mut program: Program<'_>, patch: &VendorIlPatch) -> Result<(), XgwxError> {
    if patch.expected_object_id.is_empty() || program.object_id() != patch.expected_object_id {
        return Err(XgwxError::TextProgramEdit(
            "program identity changed".into(),
        ));
    }
    program.vendor_il()?.update(&VendorIlSourcePatch {
        expected_source: patch.expected_source.clone(),
        source: patch.source.clone(),
    })
}

pub fn edit_sfc_entity(mut program: Program<'_>, patch: &SfcEntityPatch) -> Result<(), XgwxError> {
    program.sfc()?.edit_entity(&SfcEntityEdit {
        block_index: patch.block_index,
        entity_index: patch.entity_index,
        expected_type: patch.expected_type,
        expected_row: patch.expected_row,
        expected_column: patch.expected_column,
        field: patch.field.clone(),
        expected_value: patch.expected_value.clone(),
        replacement: patch.replacement.clone(),
    })
}

pub fn replace_sfc_sequence(
    mut program: Program<'_>,
    patch: &SfcSequencePatch,
) -> Result<(), XgwxError> {
    program.sfc()?.replace_sequence(&SfcSequenceEdit {
        block_index: patch.block_index,
        expected_entities: patch.expected_entities.clone(),
        rows: patch.rows.clone(),
        expected_rows: patch.expected_rows.clone(),
    })
}

pub fn edit_sfc_variable(
    mut program: Program<'_>,
    patch: &SfcVariablePatch,
) -> Result<(), XgwxError> {
    program
        .variable_mut()
        .edit_declaration(&ProgramVariablePatch {
            expected_variables: patch.expected_variables.clone(),
            name: patch.name.clone(),
            data_type: patch.data_type.clone(),
            description: patch.description.clone(),
            remove: patch.remove,
            declaration: patch.declaration.clone(),
            update: patch.update,
        })
}

pub fn edit_text_variable(program: Program<'_>, patch: &SfcVariablePatch) -> Result<(), XgwxError> {
    program.view().text()?;
    edit_sfc_variable(program, patch)
}

pub fn update_cell_text(
    mut program: Program<'_>,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<(), XgwxError> {
    match program.view().ladder()?.project_type {
        Some(2) => program
            .iec_ld()?
            .update_cell_text(offset, expected, replacement),
        _ => program
            .ladder()?
            .update_cell_text(offset, expected, replacement),
    }
}
