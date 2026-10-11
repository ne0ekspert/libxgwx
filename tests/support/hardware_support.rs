//! Translate captured wire-format regression patches to selected editors.
#![allow(dead_code)]
use xgwx::*;

pub fn edit_browser_hardware(
    doc: &mut XgwxDocument,
    patch: &BrowserHardwarePatch,
) -> Result<(), XgwxError> {
    if patch.operation == "slotCount" {
        doc.base_mut()
            .by_number(patch.base)?
            .edit_slot_count(&patch.expected_value, &patch.replacement)
    } else {
        let slot = patch
            .slot
            .ok_or_else(|| XgwxError::BrowserHardwareEdit("missing module slot".into()))?;
        doc.module_mut()
            .at(patch.base, slot)?
            .edit_field(&ModuleFieldEdit {
                operation: patch.operation.clone(),
                expected_value: patch.expected_value.clone(),
                replacement: patch.replacement.clone(),
                key: patch.key.clone(),
                index: patch.index,
            })
    }
}

pub fn edit_browser_network(
    doc: &mut XgwxDocument,
    patch: &BrowserNetworkPatch,
) -> Result<(), XgwxError> {
    let edit = NetworkFieldEdit {
        field: patch.field.clone(),
        expected_value: patch.expected_value.clone(),
        replacement: patch.replacement.clone(),
    };
    if patch.module {
        let (Some(base), Some(slot)) = (patch.base, patch.slot) else {
            return Err(XgwxError::BrowserNetworkEdit(
                "missing membership coordinates".into(),
            ));
        };
        let member = doc.network().module_at(base, slot)?;
        if member.network().map(|network| network.index()) != Some(patch.network_index) {
            return Err(XgwxError::BrowserNetworkEdit(
                "membership belongs to another network".into(),
            ));
        }
        let index = member.index();
        doc.network_mut().module(index)?.edit_field(&edit)
    } else {
        doc.network_mut()
            .get(patch.network_index)?
            .edit_field(&edit)
    }
}

pub fn edit_fenet_field(doc: &mut XgwxDocument, patch: &FenetFieldPatch) -> Result<(), XgwxError> {
    doc.network_mut()
        .module_at(patch.base, patch.slot)?
        .fenet()?
        .edit_field(&NetworkFieldEdit {
            field: patch.field.clone(),
            expected_value: patch.expected_value.clone(),
            replacement: patch.replacement.clone(),
        })
}

pub fn edit_cnet_settings(
    doc: &mut XgwxDocument,
    patch: &CnetSettingsPatch,
) -> Result<(), XgwxError> {
    doc.network_mut()
        .module_at(patch.base, patch.slot)?
        .cnet()?
        .edit_settings(&patch.changes)
}
