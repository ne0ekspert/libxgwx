//! Strict C JSON requests, kept independent of the WASM camelCase wire format.
use super::{Result, invalid, operation};
use crate::XgwxDocument;
use serde::Deserialize;
mod extended;
#[derive(Deserialize)]
#[serde(untagged)]
enum RequestEdit {
    Core(Edit),
    Extended(extended::Edit),
}

// Missing/null fields retain the native value. Explicit field conversion also
// makes additions to the Rust patch types visible at compile time.
macro_rules! patch {
    ($name:ident => $native:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $name { $( $field: Option<$ty>, )* }
        impl From<$name> for crate::$native {
            fn from(p: $name) -> Self { Self { $( $field: p.$field, )* } }
        }
    };
}

patch!(ModulePatch => ModulePatch {
    id: u32, sub_type: u32, name: String, comment: String, details: String,
});
patch!(NetworkPatch => NetworkPatch {
    name: String, type_name: String, network_type: String,
});
patch!(NetworkModulePatch => NetworkModulePatch {
    config_name: String, alias: String, description: String,
});
patch!(ProgramPatch => ProgramPatch {
    name: String, task: String, version: u32, kind: u32, instance_name: String, comment: String,
});
patch!(VariablePatch => VariablePatch {
    name: String, address_area: String, address_number: u32, data_type: String, description: String,
});

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    schema_version: u32,
    edits: Vec<RequestEdit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BasicParameterChange {
    name: String,
    expected: String,
    replacement: u32,
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Edit {
    CreateTask {
        configuration_index: usize,
        settings: crate::TaskSettings,
    },
    CreateTaskProgram {
        task_index: usize,
        expected: crate::TaskSnapshot,
        name: String,
        language: String,
        object_id: String,
        symbol_id: String,
    },
    UpdateTask {
        task_index: usize,
        expected: crate::TaskSnapshot,
        settings: crate::TaskSettings,
    },
    RenameTask {
        task_index: usize,
        expected: crate::TaskSnapshot,
        name: String,
    },
    DeleteTask {
        task_index: usize,
        expected: crate::TaskSnapshot,
    },
    AssignTaskProgram {
        task_index: usize,
        expected: crate::TaskSnapshot,
        program_index: usize,
        expected_object_id: String,
        expected_task: String,
    },
    EditBasicOperation {
        parameter_index: usize,
        changes: Vec<BasicParameterChange>,
    },
    EditBasicTiming {
        parameter_index: usize,
        changes: Vec<BasicParameterChange>,
    },
    InsertIecLdFunction {
        program_index: usize,
        expected_object_id: String,
        row_index: u16,
        raw_x: u8,
        function_name: String,
        operands: Vec<String>,
    },
    InsertIecLdFunctionOutputWire {
        program_index: usize,
        expected_object_id: String,
        block_offset: usize,
        expected_name: String,
        pin_name: String,
        start_x: u8,
    },
    CreateUserPou {
        patch: super::user_pou::NewUserPou,
    },
    EditUserPouSource {
        expected: super::user_pou::UserPou,
        source: String,
    },
    EditUserPouVariables {
        expected: super::user_pou::UserPou,
        variables: Vec<super::user_pou::UserPouVariable>,
    },
    DeleteUserPou {
        expected: super::user_pou::UserPou,
    },
    UpdateModule {
        base: u32,
        slot: u32,
        expected_name: String,
        patch: ModulePatch,
    },
    SelectModule {
        base: u32,
        slot: u32,
        expected_name: String,
        model: String,
    },
    InsertModule {
        base: u32,
        slot: u32,
        model: String,
    },
    DeleteModule {
        base: u32,
        slot: u32,
        expected_name: String,
    },
    SetModuleOption {
        base: u32,
        slot: u32,
        expected_name: String,
        key: String,
        index: u32,
        expected_value: u32,
        value: u32,
    },
    SetModuleInputFilter {
        base: u32,
        slot: u32,
        expected_name: String,
        expected_value: u8,
        value: u8,
    },
    UpdateNetwork {
        network_index: usize,
        expected_name: String,
        patch: NetworkPatch,
    },
    UpdateNetworkModule {
        base: u32,
        slot: u32,
        expected_config_name: String,
        patch: NetworkModulePatch,
    },
    UpdateProgram {
        program_index: usize,
        expected_object_id: String,
        patch: ProgramPatch,
    },
    CreateProgram {
        name: String,
        language: String,
        object_id: String,
        symbol_id: String,
    },
    DeleteProgram {
        program_index: usize,
        expected_object_id: String,
    },
    MoveProgram {
        from: usize,
        to: usize,
        expected_object_id: String,
        expected_target_id: String,
    },
    InsertVariable {
        name: String,
        data_type: String,
        address_area: String,
        address_number: u32,
        description: String,
    },
    DeleteVariable {
        variable_index: usize,
        expected_name: String,
    },
    UpdateVariable {
        variable_index: usize,
        expected_name: String,
        patch: VariablePatch,
    },
    EditTextProgram {
        program_index: usize,
        expected_object_id: String,
        expected_language: String,
        expected_source: String,
        source: String,
    },
}

fn check_module(doc: &XgwxDocument, base: u32, slot: u32, expected: &str) -> Result<()> {
    let matches = doc
        .module_summaries()
        .into_iter()
        .filter(|m| m.base == Some(base) && m.slot == Some(slot))
        .collect::<Vec<_>>();
    if matches.len() != 1 || matches[0].name.as_deref() != Some(expected) {
        return Err(operation(
            "module is missing, ambiguous or changed; reload the module list",
        ));
    }
    Ok(())
}

fn check_program(doc: &XgwxDocument, index: usize, expected: &str) -> Result<()> {
    if expected.is_empty()
        || doc
            .programs()
            .get(index)
            .and_then(|p| p.object_id.as_deref())
            != Some(expected)
    {
        return Err(operation(
            "program is missing or changed; reload the program list",
        ));
    }
    Ok(())
}

pub(super) fn apply(doc: &mut XgwxDocument, request: &str) -> Result<()> {
    #[cfg(not(feature = "il"))]
    {
        let value: serde_json::Value =
            serde_json::from_str(request).map_err(|e| invalid(&e.to_string()))?;
        if value
            .get("edits")
            .and_then(|v| v.as_array())
            .is_some_and(|edits| {
                edits
                    .iter()
                    .any(|e| e.get("operation").and_then(|v| v.as_str()) == Some("edit_vendor_il"))
            })
        {
            return Err(super::Failure(
                super::XGWX_FEATURE_UNAVAILABLE,
                "enable the il feature".into(),
            ));
        }
    }
    let transaction: Transaction = serde_json::from_str(request)
        .map_err(|e| invalid(&format!("invalid edit request: {e}")))?;
    if transaction.schema_version != 1 {
        return Err(invalid("unsupported edit schema_version"));
    }
    let mut candidate = doc.clone();
    for (index, edit) in transaction.edits.into_iter().enumerate() {
        match edit {
            RequestEdit::Core(edit) => apply_one(&mut candidate, edit),
            RequestEdit::Extended(edit) => edit.apply(&mut candidate),
        }
        .map_err(|super::Failure(status, message)| {
            super::Failure(status, format!("edit {index}: {message}"))
        })?;
    }
    candidate.to_verified_bytes().map_err(operation)?;
    *doc = candidate;
    Ok(())
}

fn apply_one(doc: &mut XgwxDocument, edit: Edit) -> Result<()> {
    match edit {
        Edit::CreateTask {
            configuration_index,
            settings,
        } => doc
            .task_mut()
            .create(configuration_index, &settings)
            .map_err(operation),
        Edit::CreateTaskProgram {
            task_index,
            expected,
            name,
            language,
            object_id,
            symbol_id,
        } => doc
            .task_mut()
            .get_if(task_index, &expected)
            .and_then(|mut t| {
                t.create_program(&crate::NewProgram {
                    name,
                    language,
                    object_id,
                    symbol_id,
                })
            })
            .map_err(operation),
        Edit::UpdateTask {
            task_index,
            expected,
            settings,
        } => doc
            .task_mut()
            .get_if(task_index, &expected)
            .and_then(|mut t| t.update(&settings))
            .map_err(operation),
        Edit::RenameTask {
            task_index,
            expected,
            name,
        } => doc
            .task_mut()
            .get_if(task_index, &expected)
            .and_then(|mut t| t.rename(&name))
            .map_err(operation),
        Edit::DeleteTask {
            task_index,
            expected,
        } => doc
            .task_mut()
            .get_if(task_index, &expected)
            .and_then(|t| t.delete())
            .map_err(operation),
        Edit::AssignTaskProgram {
            task_index,
            expected,
            program_index,
            expected_object_id,
            expected_task,
        } => doc
            .task_mut()
            .get_if(task_index, &expected)
            .and_then(|mut t| t.assign_program(program_index, &expected_object_id, &expected_task))
            .map_err(operation),
        Edit::EditBasicOperation {
            parameter_index,
            changes,
        } => {
            let changes: Vec<_> = changes
                .iter()
                .map(|c| (c.name.as_str(), c.expected.as_str(), c.replacement))
                .collect();
            doc.parameter_mut()
                .get(parameter_index)
                .and_then(|mut p| p.edit_basic_operation(&changes))
                .map_err(operation)
        }
        Edit::EditBasicTiming {
            parameter_index,
            changes,
        } => {
            let changes: Vec<_> = changes
                .iter()
                .map(|c| (c.name.as_str(), c.expected.as_str(), c.replacement))
                .collect();
            doc.parameter_mut()
                .get(parameter_index)
                .and_then(|mut p| p.edit_basic_timing(&changes))
                .map_err(operation)
        }
        Edit::InsertIecLdFunction {
            program_index,
            expected_object_id,
            row_index,
            raw_x,
            function_name,
            operands,
        } => {
            check_program(doc, program_index, &expected_object_id)?;
            doc.program_mut()
                .get(program_index)
                .and_then(|mut p| {
                    p.iec_ld()?
                        .insert_function(row_index, raw_x, &function_name, &operands)
                })
                .map_err(operation)
        }
        Edit::InsertIecLdFunctionOutputWire {
            program_index,
            expected_object_id,
            block_offset,
            expected_name,
            pin_name,
            start_x,
        } => {
            check_program(doc, program_index, &expected_object_id)?;
            doc.program_mut()
                .get(program_index)
                .and_then(|mut p| {
                    p.iec_ld()?.insert_function_output_wire(
                        block_offset,
                        &expected_name,
                        &pin_name,
                        start_x,
                    )
                })
                .map_err(operation)
        }
        Edit::CreateUserPou { patch } => {
            doc.user_pou_mut().create(&patch.into()).map_err(operation)
        }
        Edit::EditUserPouSource { expected, source } => {
            let expected: crate::UserPou = expected.into();
            doc.user_pou_mut()
                .get(expected.index, &expected.object_id)
                .and_then(|mut p| p.set_source(&expected, &source))
                .map_err(operation)
        }
        Edit::EditUserPouVariables {
            expected,
            variables,
        } => {
            let expected: crate::UserPou = expected.into();
            let variables = variables.into_iter().map(Into::into).collect::<Vec<_>>();
            doc.user_pou_mut()
                .get(expected.index, &expected.object_id)
                .and_then(|mut p| p.set_variables(&expected, &variables))
                .map_err(operation)
        }
        Edit::DeleteUserPou { expected } => {
            let expected: crate::UserPou = expected.into();
            doc.user_pou_mut()
                .get(expected.index, &expected.object_id)
                .and_then(|p| p.delete(&expected))
                .map_err(operation)
        }
        Edit::UpdateModule {
            base,
            slot,
            expected_name,
            patch,
        } => {
            check_module(doc, base, slot, &expected_name)?;
            doc.update_module(base, slot, &patch.into())
                .map_err(operation)
        }
        Edit::SelectModule {
            base,
            slot,
            expected_name,
            model,
        } => {
            check_module(doc, base, slot, &expected_name)?;
            doc.select_module(base, slot, &model).map_err(operation)
        }
        Edit::InsertModule { base, slot, model } => {
            doc.insert_module(base, slot, &model).map_err(operation)
        }
        Edit::DeleteModule {
            base,
            slot,
            expected_name,
        } => {
            check_module(doc, base, slot, &expected_name)?;
            doc.delete_module(base, slot).map_err(operation)
        }
        Edit::SetModuleOption {
            base,
            slot,
            expected_name,
            key,
            index,
            expected_value,
            value,
        } => {
            check_module(doc, base, slot, &expected_name)?;
            if !doc
                .module_option_values(base, slot)
                .map_err(operation)?
                .iter()
                .any(|o| o.key == key && o.index == index && o.value == expected_value)
            {
                return Err(operation(
                    "module option changed; reload the module options",
                ));
            }
            doc.set_module_option(base, slot, &key, index, value)
                .map_err(operation)
        }
        Edit::SetModuleInputFilter {
            base,
            slot,
            expected_name,
            expected_value,
            value,
        } => {
            check_module(doc, base, slot, &expected_name)?;
            if !doc.module_summaries().iter().any(|m| {
                m.base == Some(base)
                    && m.slot == Some(slot)
                    && m.input_filter_raw == Some(expected_value)
            }) {
                return Err(operation("module input filter changed or is unsupported"));
            }
            doc.set_module_input_filter(base, slot, crate::ModuleInputFilter::from_raw(value))
                .map_err(operation)
        }
        Edit::UpdateNetwork {
            network_index,
            expected_name,
            patch,
        } => {
            if doc
                .networks()
                .get(network_index)
                .and_then(|n| n.name.as_deref())
                != Some(expected_name.as_str())
            {
                return Err(operation(
                    "network is missing or changed; reload the network list",
                ));
            }
            doc.update_network(network_index, &patch.into())
                .map_err(operation)
        }
        Edit::UpdateNetworkModule {
            base,
            slot,
            expected_config_name,
            patch,
        } => {
            let matches = doc
                .network_modules()
                .into_iter()
                .filter(|m| m.base == Some(base) && m.slot == Some(slot))
                .collect::<Vec<_>>();
            if matches.len() != 1
                || matches[0].config_name.as_deref() != Some(expected_config_name.as_str())
            {
                return Err(operation(
                    "network module is missing, ambiguous or changed; reload its list",
                ));
            }
            doc.update_network_module(base, slot, &patch.into())
                .map_err(operation)
        }
        Edit::UpdateProgram {
            program_index,
            expected_object_id,
            patch,
        } => {
            check_program(doc, program_index, &expected_object_id)?;
            doc.update_program(program_index, &patch.into())
                .map_err(operation)
        }
        Edit::CreateProgram {
            name,
            language,
            object_id,
            symbol_id,
        } => doc
            .create_program(&crate::NewProgram {
                name,
                language,
                object_id,
                symbol_id,
            })
            .map_err(operation),
        Edit::DeleteProgram {
            program_index,
            expected_object_id,
        } => doc
            .delete_program(program_index, &expected_object_id)
            .map_err(operation),
        Edit::MoveProgram {
            from,
            to,
            expected_object_id,
            expected_target_id,
        } => doc
            .move_program(from, to, &expected_object_id, &expected_target_id)
            .map_err(operation),
        Edit::InsertVariable {
            name,
            data_type,
            address_area,
            address_number,
            description,
        } => doc
            .variable_mut()
            .insert(&crate::NewGlobalVariable {
                name,
                data_type,
                address_area,
                address_number,
                description,
            })
            .map(|_| ())
            .map_err(operation),
        Edit::DeleteVariable {
            variable_index,
            expected_name,
        } => {
            let variable = doc.variable_mut().get(variable_index).map_err(operation)?;
            if variable.summary().name.as_deref() != Some(expected_name.as_str()) {
                return Err(operation("stale global variable selection"));
            }
            variable.delete().map_err(operation)
        }
        Edit::UpdateVariable {
            variable_index,
            expected_name,
            patch,
        } => {
            let variables = doc.variables().map_err(operation)?;
            if variables
                .get(variable_index)
                .and_then(|v| v.name.as_deref())
                != Some(expected_name.as_str())
            {
                return Err(operation(
                    "variable is missing or changed; reload the global variable list",
                ));
            }
            doc.update_variable(variable_index, &patch.into())
                .map_err(operation)
        }
        Edit::EditTextProgram {
            program_index,
            expected_object_id,
            expected_language,
            expected_source,
            source,
        } => doc
            .edit_text_program(&crate::TextProgramPatch {
                program_index,
                expected_object_id,
                expected_language,
                expected_source,
                source,
            })
            .map_err(operation),
    }
}
