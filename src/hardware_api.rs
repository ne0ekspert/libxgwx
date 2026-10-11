#![deny(missing_docs)]
//! Document-bound installed hardware selection and guarded editing.
use crate::{Base, HardwareModule, Module, ModuleSummary, XgwxDocument, XgwxError};

/// Installed hardware in document order, excluding additional network memberships.
#[derive(Debug, Clone, Copy)]
pub struct ModuleView<'a> {
    pub(crate) doc: &'a XgwxDocument,
}
/// Borrowed installed hardware record. Its index is an installed-record index.
#[derive(Debug, Clone, Copy)]
pub struct ModuleRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}

impl XgwxDocument {
    /// Borrow installed hardware for selection by record index or native coordinates.
    pub fn module(&self) -> ModuleView<'_> {
        ModuleView { doc: self }
    }
    /// Exclusively borrow installed hardware for guarded edits.
    /// Root mutation bridges are private; select a module first.
    /// ```compile_fail
    /// # use xgwx::{XgwxDocument, ModuleInputFilter};
    /// fn old_api(doc: &mut XgwxDocument) {
    ///     doc.set_module_input_filter(0, 0, ModuleInputFilter::from_raw(0)).unwrap();
    /// }
    /// ```
    #[cfg(feature = "write")]
    pub fn module_mut(&mut self) -> ModuleCollectionEditor<'_> {
        ModuleCollectionEditor { doc: self }
    }
}
impl<'a> ModuleView<'a> {
    /// Detached installed hardware summaries in document order, including
    /// unknown module identities. Network memberships are listed separately
    /// through [`crate::NetworkView::module_summaries`].
    pub fn summaries(self) -> Vec<ModuleSummary> {
        self.doc.module_summaries()
    }
    /// List borrowed installed records, retaining unknown module identities.
    pub fn list(self) -> Vec<ModuleRef<'a>> {
        (0..self.doc.module_summaries().len())
            .map(|index| ModuleRef {
                doc: self.doc,
                index,
            })
            .collect()
    }
    /// Select by installed-record index, not a network membership index.
    pub fn get(self, index: usize) -> Result<ModuleRef<'a>, XgwxError> {
        self.doc
            .module_summaries()
            .get(index)
            .ok_or(XgwxError::ModuleIndexNotFound { index })?;
        Ok(ModuleRef {
            doc: self.doc,
            index,
        })
    }
    /// Select a unique installed record by native base number and starting slot.
    /// Repeated coordinates across configurations are rejected.
    pub fn at(self, base: u32, slot: u32) -> Result<ModuleRef<'a>, XgwxError> {
        let indices = self
            .doc
            .module_summaries()
            .iter()
            .enumerate()
            .filter(|(_, m)| m.base == Some(base) && m.slot == Some(slot))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        match indices.as_slice() {
            [] => Err(XgwxError::ModuleNotFound { base, slot }),
            [index] => self.get(*index),
            _ => Err(XgwxError::AmbiguousModule { base, slot }),
        }
    }
}
impl<'a> ModuleRef<'a> {
    /// Installed hardware record index in document order.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Read detached hardware metadata with its configuration scope.
    pub fn hardware(&self) -> HardwareModule {
        self.doc.hardware_modules()[self.index].clone()
    }
    /// Read detached native attributes and hardware metadata.
    pub fn summary(&self) -> ModuleSummary {
        self.hardware().hardware
    }
    /// Owning PLC configuration index, if present.
    pub fn configuration_index(&self) -> Option<usize> {
        self.hardware().configuration_index
    }
    /// Read the same classification and network association as the document inventory.
    pub fn typed(&self) -> Module {
        self.doc.modules()[self.index].clone()
    }
    /// Borrow the owning base. Unplaced or ambiguous base associations return `None`.
    pub fn base(&self) -> Option<Base<'a>> {
        let index = crate::modules::installed_base_index(self.doc, self.index)?;
        self.doc.base().get(index).ok()
    }
    /// Read verified catalog option values at this record's native coordinates.
    #[cfg(feature = "write")]
    pub fn option_values(&self) -> Result<Vec<crate::ModuleOptionSelection>, XgwxError> {
        let (base, slot) = unique_coordinates(self.doc, self.index)?;
        self.doc.module_option_values(base, slot)
    }
}
impl<'a> Base<'a> {
    /// List installed records owned by this base, without duplicate network memberships.
    pub fn installed_modules(&self) -> Vec<ModuleRef<'a>> {
        let owners = crate::modules::installed_base_indices(self.document());
        self.document()
            .module()
            .list()
            .into_iter()
            .zip(owners)
            .filter_map(|(module, owner)| (owner == Some(self.index())).then_some(module))
            .collect()
    }
    /// Select an installed module by its starting physical slot within this base.
    pub fn module(&self, slot: u32) -> Result<ModuleRef<'a>, XgwxError> {
        let found = self
            .installed_modules()
            .into_iter()
            .filter(|m| m.summary().slot == Some(slot))
            .collect::<Vec<_>>();
        if let [module] = found.as_slice() {
            return Ok(*module);
        }
        let base = self
            .number()
            .ok_or_else(|| XgwxError::HardwareEdit("base number is missing".into()))?;
        match found.as_slice() {
            [] => Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => Ok(*module),
            _ => Err(XgwxError::AmbiguousModule { base, slot }),
        }
    }
}

#[cfg(feature = "write")]
fn unique_coordinates(doc: &XgwxDocument, index: usize) -> Result<(u32, u32), XgwxError> {
    let module = doc.module().get(index)?.summary();
    let base = module
        .base
        .ok_or_else(|| XgwxError::HardwareEdit("module base number is missing".into()))?;
    let slot = module
        .slot
        .ok_or_else(|| XgwxError::HardwareEdit("module starting slot is missing".into()))?;
    if doc.module().at(base, slot)?.index() != index {
        return Err(XgwxError::AmbiguousModule { base, slot });
    }
    if let Some(owner) = doc.module().get(index)?.base() {
        if owner.number() != Some(base) {
            return Err(XgwxError::HardwareEdit(
                "module coordinates conflict with its owning base".into(),
            ));
        }
    }
    Ok((base, slot))
}

/// Exclusively borrowed collection of installed hardware records.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct ModuleCollectionEditor<'a> {
    doc: &'a mut XgwxDocument,
}
/// Guarded editor bound to one installed module. Deletion consumes the editor.
///
/// ```compile_fail
/// fn delete_once(module: xgwx::ModuleEditor<'_>) {
///     module.delete().unwrap();
///     let _ = module.summary();
/// }
/// ```
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct ModuleEditor<'a> {
    doc: &'a mut XgwxDocument,
    index: usize,
    expected: HardwareModule,
}
#[cfg(feature = "write")]
impl<'a> ModuleCollectionEditor<'a> {
    /// Bind an installed record by document index. Coordinate validity is checked before mutation.
    pub fn get(self, index: usize) -> Result<ModuleEditor<'a>, XgwxError> {
        let expected = self.doc.module().get(index)?.hardware();
        Ok(ModuleEditor {
            doc: self.doc,
            index,
            expected,
        })
    }
    /// Bind a unique installed record by native base number and starting slot.
    pub fn at(self, base: u32, slot: u32) -> Result<ModuleEditor<'a>, XgwxError> {
        let index = self.doc.module().at(base, slot)?.index();
        self.get(index)
    }
}
#[cfg(feature = "write")]
impl ModuleEditor<'_> {
    /// Installed record index bound to this editor.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Borrow the current record without permitting document mutation.
    pub fn view(&self) -> ModuleRef<'_> {
        ModuleRef {
            doc: self.doc,
            index: self.index,
        }
    }
    /// Read current native attributes and hardware metadata.
    pub fn summary(&self) -> ModuleSummary {
        self.view().summary()
    }
    fn coordinates(&self) -> Result<(u32, u32), XgwxError> {
        ensure_consistent_document(self.doc)?;
        if self.doc.module().get(self.index)?.hardware() != self.expected {
            return Err(XgwxError::HardwareEdit(
                "module identity changed; select it again".into(),
            ));
        }
        unique_coordinates(self.doc, self.index)
    }
    fn refresh(&mut self) {
        self.expected = self.view().hardware();
    }
    /// Update this module's metadata using the bounded native writer.
    pub fn update(&mut self, patch: &crate::ModulePatch) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.update_module(base, slot, patch)?;
        self.refresh();
        Ok(())
    }
    /// Replace the installed model with a verified catalog model, preserving placement guards.
    pub fn select_model(&mut self, model: &str) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.select_module(base, slot, model)?;
        self.refresh();
        Ok(())
    }
    /// Delete this installed module and its supported network records atomically.
    pub fn delete(self) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.delete_module(base, slot)
    }
    /// Read verified option values for this selected module.
    pub fn option_values(&self) -> Result<Vec<crate::ModuleOptionSelection>, XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.module_option_values(base, slot)
    }
    /// Set one verified module, channel, or group option.
    pub fn set_option(&mut self, key: &str, index: u32, value: u32) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.set_module_option(base, slot, key, index, value)?;
        self.refresh();
        Ok(())
    }
    /// Set the native digital input filter, retaining CPU and module eligibility checks.
    pub fn set_input_filter(&mut self, filter: crate::ModuleInputFilter) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc.set_module_input_filter(base, slot, filter)?;
        self.refresh();
        Ok(())
    }
}

/// Exclusively borrowed collection of native hardware bases.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct BaseCollectionEditor<'a> {
    doc: &'a mut XgwxDocument,
}
/// Guarded editor bound to one native I/O base record.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct BaseEditor<'a> {
    doc: &'a mut XgwxDocument,
    index: usize,
    expected: crate::BaseSummary,
    configuration_index: Option<usize>,
}
#[cfg(feature = "write")]
impl XgwxDocument {
    /// Exclusively borrow hardware bases for selection and guarded edits.
    pub fn base_mut(&mut self) -> BaseCollectionEditor<'_> {
        BaseCollectionEditor { doc: self }
    }
}
#[cfg(feature = "write")]
fn native_base_indices(doc: &XgwxDocument) -> Result<Vec<usize>, XgwxError> {
    let xml = roxmltree::Document::parse(&doc.xml).map_err(XgwxError::Xml)?;
    Ok(xml
        .descendants()
        .filter(|n| n.has_tag_name("Base"))
        .enumerate()
        .filter(|(_, n)| {
            n.parent().is_some_and(|p| p.has_tag_name("BaseInfo"))
                && n.ancestors().any(|p| {
                    p.has_tag_name("Parameter") && p.attribute("Type") == Some("IO PARAMETER")
                })
        })
        .map(|(i, _)| i)
        .collect())
}
#[cfg(feature = "write")]
impl<'a> BaseCollectionEditor<'a> {
    /// Bind a base by document index. Unsupported base layouts are rejected.
    pub fn get(self, index: usize) -> Result<BaseEditor<'a>, XgwxError> {
        let base = self.doc.base().get(index)?;
        let expected = base.summary();
        let configuration_index = base.configuration_index();
        if !native_base_indices(self.doc)?.contains(&index) {
            return Err(XgwxError::HardwareEdit(
                "select a native IO PARAMETER base".into(),
            ));
        }
        Ok(BaseEditor {
            doc: self.doc,
            index,
            expected,
            configuration_index,
        })
    }
    /// Bind a native I/O base by its number, rejecting duplicate numbers.
    pub fn by_number(self, number: u32) -> Result<BaseEditor<'a>, XgwxError> {
        let indices = native_base_indices(self.doc)?
            .into_iter()
            .filter(|&i| {
                self.doc
                    .base()
                    .get(i)
                    .is_ok_and(|b| b.number() == Some(number))
            })
            .collect::<Vec<_>>();
        match indices.as_slice() {
            [] => Err(XgwxError::BaseNotFound { base: number }),
            [index] => self.get(*index),
            _ => Err(XgwxError::AmbiguousBase { base: number }),
        }
    }
}
#[cfg(feature = "write")]
impl<'a> BaseEditor<'a> {
    /// Base record index bound to this editor.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Borrow the current base metadata and its scoped modules.
    pub fn view(&self) -> Base<'_> {
        self.doc.base().get(self.index).expect("bound base exists")
    }
    /// Read detached native base metadata.
    pub fn summary(&self) -> crate::BaseSummary {
        self.view().summary()
    }
    fn number(&self) -> Result<u32, XgwxError> {
        ensure_consistent_document(self.doc)?;
        let base = self.doc.base().get(self.index)?;
        if base.summary() != self.expected || base.configuration_index() != self.configuration_index
        {
            return Err(XgwxError::HardwareEdit(
                "base identity changed; select it again".into(),
            ));
        }
        let number = base
            .number()
            .ok_or_else(|| XgwxError::HardwareEdit("base number is missing".into()))?;
        let candidates = native_base_indices(self.doc)?
            .into_iter()
            .filter(|&i| {
                self.doc
                    .base()
                    .get(i)
                    .is_ok_and(|b| b.number() == Some(number))
            })
            .collect::<Vec<_>>();
        if candidates.as_slice() != [self.index] {
            return Err(XgwxError::AmbiguousBase { base: number });
        }
        Ok(number)
    }
    /// Set the physical slot count, retaining catalog span and CPU capacity checks.
    pub fn set_slot_count(&mut self, count: u32) -> Result<(), XgwxError> {
        let base = self.number()?;
        self.doc.set_base_slot_count(base, count)?;
        self.expected = self.view().summary();
        Ok(())
    }
    /// Insert a verified catalog model into an empty starting slot on this base.
    pub fn insert_module(&mut self, slot: u32, model: &str) -> Result<(), XgwxError> {
        let base = self.number()?;
        self.doc.insert_module(base, slot, model)
    }
    /// Bind an installed module on this base by starting slot. Consumes the base editor.
    pub fn module(self, slot: u32) -> Result<ModuleEditor<'a>, XgwxError> {
        let number = self.number()?;
        let selected = self.view().module(slot)?;
        let index = selected.index();
        if selected.summary().base != Some(number) {
            return Err(XgwxError::HardwareEdit(
                "module coordinates conflict with its owning base".into(),
            ));
        }
        self.doc.module_mut().get(index)
    }
}

/// Expected-value field edit for a selected installed module, without coordinates.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
pub struct ModuleFieldEdit {
    /// Supported browser hardware field: comment, inputFilter, or option.
    pub operation: String,
    /// Exact previously read native field value.
    pub expected_value: String,
    /// Replacement field value in its native representation.
    pub replacement: String,
    /// Catalog option key when editing an option.
    pub key: Option<String>,
    /// Channel or group index when editing an option.
    pub index: Option<u32>,
}
#[cfg(feature = "write")]
impl ModuleEditor<'_> {
    /// Apply a field edit using the same stale-field and transaction guards as the browser.
    pub fn edit_field(&mut self, edit: &ModuleFieldEdit) -> Result<(), XgwxError> {
        let (base, slot) = self.coordinates()?;
        self.doc
            .edit_browser_hardware(&crate::BrowserHardwarePatch {
                operation: edit.operation.clone(),
                base,
                slot: Some(slot),
                expected_value: edit.expected_value.clone(),
                replacement: edit.replacement.clone(),
                key: edit.key.clone(),
                index: edit.index,
            })?;
        self.refresh();
        Ok(())
    }
}
#[cfg(feature = "write")]
impl BaseEditor<'_> {
    /// Edit the slot count using exact expected/replacement native strings.
    pub fn edit_slot_count(&mut self, expected: &str, replacement: &str) -> Result<(), XgwxError> {
        let base = self.number()?;
        self.doc
            .edit_browser_hardware(&crate::BrowserHardwarePatch {
                operation: "slotCount".into(),
                base,
                slot: None,
                expected_value: expected.into(),
                replacement: replacement.into(),
                key: None,
                index: None,
            })?;
        self.expected = self.view().summary();
        Ok(())
    }
}
#[cfg(feature = "write")]
fn ensure_consistent_document(doc: &XgwxDocument) -> Result<(), XgwxError> {
    if crate::parse_xml(&doc.xml)? != doc.root {
        return Err(XgwxError::HardwareEdit(
            "XML and parsed hardware tree disagree; reparse the document".into(),
        ));
    }
    Ok(())
}
