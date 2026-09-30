use crate::*;
use base64::Engine;
use bzip2::Compression as Bzip2Compression;
use bzip2::write::BzEncoder;
use std::cmp::Reverse;
use std::io::Write;
use std::ops::Range;

use flate2::Compression;
use flate2::write::GzEncoder;

const SUPPORTED_HEADER_LEN: usize = 138;
const HEADER_CHECKSUM_OFFSET: usize = 64;
const HEADER_CHECKSUM_INPUT_START: usize = 68;
const COMPRESSED_SIZE_OFFSET: usize = 134;
const MAIN_GZIP_ALIGNMENT: usize = 4;
const XG_FRAME_HEADER_LEN: usize = 8;
const XG_FRAME_TAG_LEN: usize = 8;
const XG_FRAME_FOOTER_LEN: usize = 4;
const XG_FRAME_MAGIC: &[u8; 4] = b"HEAD";
const XG_FRAME_FOOTER: &[u8; 4] = b"FOOT";
const XG_CRC64_POLYNOMIAL: u64 = 0x1b;

/// Changes to ordinary attributes on one hardware `<Module>` record.
///
/// `Base` and `Slot` identify the record and are intentionally not editable in
/// this first writer API. A `None` field leaves the corresponding attribute
/// byte-for-byte unchanged. Module `Details` must be an even-length hexadecimal
/// string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct ModulePatch {
    pub id: Option<u32>,
    pub sub_type: Option<u32>,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub details: Option<String>,
}

/// Current value of one verified module option at one module/channel/group index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct ModuleOptionSelection {
    pub key: &'static str,
    pub index: u32,
    pub value: u32,
}

/// Changes to editable metadata on one `<Program>` record.
///
/// Programs are selected by their stable document order because their name is
/// itself editable. A `None` field leaves the corresponding XML byte range
/// untouched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct ProgramPatch {
    pub name: Option<String>,
    pub task: Option<String>,
    pub version: Option<u32>,
    pub kind: Option<u32>,
    pub instance_name: Option<String>,
    pub comment: Option<String>,
}

/// Changes to one decoded global variable symbol record.
///
/// Variables are selected by document order. String replacements must retain
/// their existing UTF-16 length so opaque record offsets remain stable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct VariablePatch {
    pub name: Option<String>,
    pub address_area: Option<String>,
    pub address_number: Option<u32>,
    pub data_type: Option<String>,
    pub description: Option<String>,
}

/// Changes to the editable metadata on one `<Network>` record.
///
/// Networks are selected by document order because their name is editable.
/// Protocol identity and member modules are deliberately outside this bounded
/// writer API.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct NetworkPatch {
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub network_type: Option<String>,
}

/// Changes to user-facing metadata on one `<NetworkModule>` record.
///
/// The linked hardware module, base/slot, type, and option type are immutable
/// identity fields. `ConfigName`, `Alias`, and `Description` are the fields
/// XG5000 exposes as editable metadata for a configured network module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct NetworkModulePatch {
    pub config_name: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
}

impl XgwxDocument {
    /// Select the CPU model stored by the primary project configuration.
    ///
    /// This updates the authoritative `<Configuration Type>` value while
    /// preserving the existing basic parameters, programs, hardware, and
    /// network configuration. Only changes between XGK models are supported;
    /// retained hardware must fit the target CPU. Other model changes require
    /// a migration and are rejected. Selecting the current type is a no-op.
    pub fn select_cpu(&mut self, model: &str) -> Result<(), XgwxError> {
        let entry = crate::cpu::find_cpu(model)?;
        let current = self.hardware_cpu()?;
        if current.type_code == entry.type_code {
            return Ok(());
        }
        if current.family != "XGK" || entry.family != "XGK" {
            return Err(XgwxError::UnsupportedCpuChange {
                from: current.model.into(),
                to: entry.model.into(),
            });
        }
        for base in self.bases() {
            let number = base.base.ok_or(XgwxError::UnsupportedCpuHardware {
                type_code: current.type_code,
            })?;
            let count = base.slot_count.ok_or(XgwxError::UnsupportedCpuHardware {
                type_code: current.type_code,
            })?;
            if number >= entry.max_base || count > entry.max_slot {
                return Err(XgwxError::CpuHardwareLimit {
                    model: entry.model.into(),
                    base: number,
                    slot: count.saturating_sub(1),
                });
            }
        }
        for module in self.modules() {
            let base = module.base.ok_or(XgwxError::UnsupportedCpuHardware {
                type_code: current.type_code,
            })?;
            let slot = module.slot.ok_or(XgwxError::UnsupportedCpuHardware {
                type_code: current.type_code,
            })?;
            let span = crate::xgk_module_catalog()
                .iter()
                .find(|item| module.id == Some(item.id) && module.sub_type == Some(item.sub_type))
                .ok_or(XgwxError::ModuleCatalogMismatch { base, slot })?
                .slot_span;
            Self::validate_cpu_position(entry, base, slot, span)?;
        }
        let mut changed = self.clone();
        let document = roxmltree::Document::parse(&changed.xml).map_err(XgwxError::Xml)?;
        let configuration = document
            .descendants()
            .find(|node| node.has_tag_name("Configuration"))
            .ok_or(XgwxError::MissingConfiguration)?;
        let range = configuration
            .attributes()
            .find(|attribute| attribute.name() == "Type")
            .map(|attribute| attribute.range_value())
            .ok_or(XgwxError::MissingConfigurationAttribute { attribute: "Type" })?;
        changed.apply_xml_replacements(vec![(range, entry.type_code.to_string())])?;
        *self = changed;
        Ok(())
    }

    /// Change an existing XGK base's physical slot count without removing modules.
    /// Native XG5000 choices are 4, 6, 8, 10 and 12. CPU limits, ambiguous
    /// bases and modules extending beyond the requested size are rejected.
    pub fn set_base_slot_count(&mut self, base: u32, slot_count: u32) -> Result<(), XgwxError> {
        let cpu = self.require_xgk_hardware()?;
        if ![4, 6, 8, 10, 12].contains(&slot_count) {
            return Err(XgwxError::InvalidBaseSlotCount { base, slot_count });
        }
        Self::validate_cpu_position(cpu, base, slot_count - 1, 1)?;
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let mut bases = document.descendants().filter(|node| {
            node.has_tag_name("Base")
                && node.attribute("Base").and_then(|v| v.parse::<u32>().ok()) == Some(base)
                && node.parent().is_some_and(|p| p.has_tag_name("BaseInfo"))
                && node.ancestors().any(|p| {
                    p.has_tag_name("Parameter") && p.attribute("Type") == Some("IO PARAMETER")
                })
        });
        let target = bases.next().ok_or(XgwxError::BaseNotFound { base })?;
        if bases.next().is_some() {
            return Err(XgwxError::AmbiguousBase { base });
        }
        let attribute = target
            .attributes()
            .find(|a| a.name() == "SlotCount")
            .ok_or(XgwxError::MissingBaseSlotCount { base })?;
        for module in self.modules().into_iter().filter(|m| m.base == Some(base)) {
            let slot = module.slot.ok_or(XgwxError::UnsupportedCpuHardware {
                type_code: cpu.type_code,
            })?;
            let mut entries = crate::xgk_module_catalog()
                .iter()
                .filter(|e| module.id == Some(e.id) && module.sub_type == Some(e.sub_type));
            let entry = entries
                .next()
                .ok_or(XgwxError::ModuleCatalogMismatch { base, slot })?;
            if entries.any(|e| e.slot_span != entry.slot_span) {
                return Err(XgwxError::ModuleCatalogMismatch { base, slot });
            }
            if slot
                .checked_add(entry.slot_span)
                .is_none_or(|end| end > slot_count)
            {
                return Err(XgwxError::ModulePlacementExceedsBase {
                    base,
                    slot,
                    slot_span: entry.slot_span,
                    slot_count,
                });
            }
        }
        if attribute.value().parse::<u32>().ok() == Some(slot_count) {
            return Ok(());
        }
        self.apply_xml_replacements(vec![(attribute.range_value(), slot_count.to_string())])
    }

    /// Insert a catalog module into an empty physical base slot.
    ///
    /// The module receives the latest-stable catalog defaults and an empty
    /// comment. The operation fails without mutating the document when the
    /// base is absent, the module does not fit, or any occupied slot overlaps.
    pub fn insert_module(&mut self, base: u32, slot: u32, model: &str) -> Result<(), XgwxError> {
        self.require_xgk_hardware()?;
        let entry = crate::catalog::find_xgk_module(model)?;
        let mut changed = self.clone();
        changed.validate_empty_module_placement(base, slot, entry.slot_span)?;
        let configuration_name = changed.primary_configuration_name();
        let document = roxmltree::Document::parse(&changed.xml).map_err(XgwxError::Xml)?;
        let container = document
            .descendants()
            .find(|node| {
                node.has_tag_name("Parameter") && node.attribute("Type") == Some("IO PARAMETER")
            })
            .ok_or(XgwxError::MissingModuleContainer)?;
        let anchor = container
            .children()
            .filter(|node| node.is_element() && node.has_tag_name("Module"))
            .find(|node| {
                let node_base = node.attribute("Base").and_then(|value| value.parse().ok());
                let node_slot = node.attribute("Slot").and_then(|value| value.parse().ok());
                node_base
                    .zip(node_slot)
                    .is_some_and(|position| position > (base, slot))
            })
            .or_else(|| {
                container
                    .children()
                    .find(|node| node.is_element() && node.has_tag_name("BaseInfo"))
            })
            .ok_or(XgwxError::MissingModuleContainer)?;
        let insertion_offset = anchor.range().start;
        let indentation_start = changed.xml[..insertion_offset]
            .rfind('\n')
            .map_or(insertion_offset, |offset| offset + 1);
        let indentation = &changed.xml[indentation_start..insertion_offset];
        let newline = xml_newline(&changed.xml);
        let module = format!(
            "<Module Base=\"{base}\" Slot=\"{slot}\" Id=\"{}\" SubType=\"{}\" Name=\"{}\" Comment=\"\" Details=\"{}\"></Module>{newline}{indentation}",
            entry.id,
            entry.sub_type,
            escape_xml_attribute(entry.name),
            entry.details,
        );
        let mut replacements = vec![(insertion_offset..insertion_offset, module)];
        push_network_configuration_additions(
            &mut replacements,
            &changed.xml,
            &document,
            &configuration_name,
            base,
            slot,
            entry,
        )?;
        changed.apply_xml_replacements(replacements)?;
        *self = changed;
        Ok(())
    }

    /// Delete one module selected by its unique base and slot.
    ///
    /// The containing base and every other XML node remain unchanged. The
    /// operation fails without mutating the document when the target is absent
    /// or ambiguous.
    pub fn delete_module(&mut self, base: u32, slot: u32) -> Result<(), XgwxError> {
        self.validate_hardware_identity_edit(base, slot)?;
        let mut changed = self.clone();
        let document = roxmltree::Document::parse(&changed.xml).map_err(XgwxError::Xml)?;
        let matches = document
            .descendants()
            .filter(|node| node.has_tag_name("Module"))
            .filter(|node| {
                node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .collect::<Vec<_>>();

        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => *module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };
        let mut replacements = vec![(module.range(), String::new())];
        push_network_configuration_removals(&mut replacements, &document, base, slot);
        changed.apply_xml_replacements(replacements)?;
        *self = changed;
        Ok(())
    }

    /// Select a module model from the embedded latest-stable XGK catalog.
    ///
    /// The target base, slot, and comment are preserved. `Id`, `SubType`,
    /// `Name`, and `Details` are replaced atomically with the catalog defaults.
    /// Captured network-module companion records are synchronized at the same
    /// time for supported communication models.
    pub fn select_module(&mut self, base: u32, slot: u32, model: &str) -> Result<(), XgwxError> {
        self.require_xgk_hardware()?;
        let entry = crate::catalog::find_xgk_module(model)?;
        let mut changed = self.clone();
        changed.validate_module_placement(base, slot, entry.slot_span)?;
        let configuration_name = changed.primary_configuration_name();
        let document = roxmltree::Document::parse(&changed.xml).map_err(XgwxError::Xml)?;
        let matches = document
            .descendants()
            .filter(|node| node.has_tag_name("Module"))
            .filter(|node| {
                node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .collect::<Vec<_>>();
        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => *module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };
        let mut replacements = Vec::new();
        push_u32_replacement(&mut replacements, module, base, slot, "Id", Some(entry.id))?;
        push_u32_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "SubType",
            Some(entry.sub_type),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Name",
            Some(entry.name),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Details",
            Some(entry.details),
        )?;
        push_network_configuration_removals(&mut replacements, &document, base, slot);
        push_network_configuration_additions(
            &mut replacements,
            &changed.xml,
            &document,
            &configuration_name,
            base,
            slot,
            entry,
        )?;
        changed.apply_xml_replacements(replacements)?;
        *self = changed;
        Ok(())
    }

    fn primary_configuration_name(&self) -> String {
        self.configurations()
            .into_iter()
            .next()
            .and_then(|configuration| configuration.name)
            .unwrap_or_else(|| "PLC".to_owned())
    }

    fn hardware_cpu(&self) -> Result<&'static CpuCatalogEntry, XgwxError> {
        let configurations = self.configurations();
        let configuration = match configurations.as_slice() {
            [] => return Err(XgwxError::MissingConfiguration),
            [configuration] => configuration,
            _ => return Err(XgwxError::AmbiguousConfiguration),
        };
        let type_code = configuration
            .type_code
            .ok_or(XgwxError::MissingConfigurationAttribute { attribute: "Type" })?;
        crate::cpu::cpu_for_type(type_code).ok_or(XgwxError::UnsupportedCpuHardware { type_code })
    }

    fn require_xgk_hardware(&self) -> Result<&'static CpuCatalogEntry, XgwxError> {
        let cpu = self.hardware_cpu()?;
        if cpu.family != "XGK" {
            return Err(XgwxError::UnsupportedCpuHardware {
                type_code: cpu.type_code,
            });
        }
        Ok(cpu)
    }

    fn validate_hardware_identity_edit(&self, base: u32, slot: u32) -> Result<(), XgwxError> {
        if self.cpu_hardware_profile().is_some_and(|profile| {
            base == profile.builtin_io_base && slot == profile.builtin_io_slot
        }) {
            return Err(XgwxError::FixedCpuModule { base, slot });
        }
        self.require_xgk_hardware()?;
        Ok(())
    }

    fn validate_cpu_position(
        cpu: &CpuCatalogEntry,
        base: u32,
        slot: u32,
        span: u32,
    ) -> Result<(), XgwxError> {
        if base >= cpu.max_base || slot.checked_add(span).is_none_or(|end| end > cpu.max_slot) {
            return Err(XgwxError::CpuHardwareLimit {
                model: cpu.model.into(),
                base,
                slot,
            });
        }
        Ok(())
    }

    fn validate_module_placement(
        &self,
        base: u32,
        slot: u32,
        slot_span: u32,
    ) -> Result<(), XgwxError> {
        match self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base) && module.slot == Some(slot))
            .count()
        {
            0 => return Err(XgwxError::ModuleNotFound { base, slot }),
            1 => {}
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        }
        let end = slot
            .checked_add(slot_span)
            .ok_or(XgwxError::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count: 0,
            })?;
        if let Some(slot_count) = self
            .bases()
            .into_iter()
            .find(|item| item.base == Some(base))
            .and_then(|item| item.slot_count)
            && end > slot_count
        {
            return Err(XgwxError::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count,
            });
        }
        if let Some(conflicting_slot) = self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base))
            .filter_map(|module| module.slot)
            .find(|other_slot| *other_slot > slot && *other_slot < end)
        {
            return Err(XgwxError::ModulePlacementConflict {
                base,
                slot,
                slot_span,
                conflicting_slot,
            });
        }
        Self::validate_cpu_position(self.require_xgk_hardware()?, base, slot, slot_span)?;
        Ok(())
    }

    fn validate_empty_module_placement(
        &self,
        base: u32,
        slot: u32,
        slot_span: u32,
    ) -> Result<(), XgwxError> {
        let slot_count = self
            .bases()
            .into_iter()
            .find(|item| item.base == Some(base))
            .ok_or(XgwxError::BaseNotFound { base })?
            .slot_count
            .ok_or(XgwxError::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count: 0,
            })?;
        let end = slot
            .checked_add(slot_span)
            .ok_or(XgwxError::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count,
            })?;
        if end > slot_count {
            return Err(XgwxError::ModulePlacementExceedsBase {
                base,
                slot,
                slot_span,
                slot_count,
            });
        }

        if let Some(conflicting_slot) = self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base))
            .filter_map(|module| {
                let other_slot = module.slot?;
                let other_span = crate::xgk_module_catalog()
                    .iter()
                    .filter(|entry| {
                        module.id == Some(entry.id) && module.sub_type == Some(entry.sub_type)
                    })
                    .map(|entry| entry.slot_span)
                    .next()
                    .unwrap_or(1);
                let other_end = other_slot.saturating_add(other_span);
                (slot < other_end && other_slot < end).then_some(other_slot)
            })
            .next()
        {
            return Err(XgwxError::ModulePlacementConflict {
                base,
                slot,
                slot_span,
                conflicting_slot,
            });
        }
        Self::validate_cpu_position(self.require_xgk_hardware()?, base, slot, slot_span)?;
        Ok(())
    }

    /// Return all currently selected values for the module's verified options.
    pub fn module_option_values(
        &self,
        base: u32,
        slot: u32,
    ) -> Result<Vec<ModuleOptionSelection>, XgwxError> {
        let (entry, details) = self.catalog_module_details(base, slot)?;
        let bytes = decode_hex_ascii_payload(&details, "Module", "Details")?;
        let mut selections = Vec::new();
        for option in entry.options {
            for index in 0..option.count {
                selections.push(ModuleOptionSelection {
                    key: option.key,
                    index,
                    value: read_module_option(&bytes, option, index)?,
                });
            }
        }
        Ok(selections)
    }

    /// Set one verified dropdown option in a module's `Details` payload.
    ///
    /// `index` is zero for module-wide options and selects the channel or group
    /// for scoped options. Values not listed in the catalog are rejected.
    pub fn set_module_option(
        &mut self,
        base: u32,
        slot: u32,
        key: &str,
        index: u32,
        value: u32,
    ) -> Result<(), XgwxError> {
        let (entry, details) = self.catalog_module_details(base, slot)?;
        let option = crate::catalog::find_xgk_module_option(entry, key)?;
        if index >= option.count {
            return Err(XgwxError::ModuleOptionIndexOutOfRange {
                key: key.to_owned(),
                index,
                count: option.count,
            });
        }
        if !option.values.iter().any(|item| item.value == value) {
            return Err(XgwxError::InvalidModuleOptionValue {
                key: key.to_owned(),
                value,
            });
        }

        let mut bytes = decode_hex_ascii_payload(&details, "Module", "Details")?;
        write_module_option(&mut bytes, option, index, value)?;
        let details = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>();
        self.update_module(
            base,
            slot,
            &ModulePatch {
                details: Some(details),
                ..ModulePatch::default()
            },
        )
    }

    fn catalog_module_details(
        &self,
        base: u32,
        slot: u32,
    ) -> Result<(&'static ModuleCatalogEntry, String), XgwxError> {
        self.require_xgk_hardware()?;
        let matches = self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base) && module.slot == Some(slot))
            .collect::<Vec<_>>();
        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };
        let entries = crate::xgk_module_catalog()
            .iter()
            .filter(|entry| module.id == Some(entry.id) && module.sub_type == Some(entry.sub_type))
            .collect::<Vec<_>>();
        let entry = match entries.as_slice() {
            [entry] => *entry,
            _ => return Err(XgwxError::ModuleCatalogMismatch { base, slot }),
        };
        let details = module
            .details
            .clone()
            .ok_or(XgwxError::MissingModuleAttribute {
                base,
                slot,
                attribute: "Details",
            })?;
        Ok((entry, details))
    }

    /// Update one module selected by its unique base and slot.
    ///
    /// Only the requested XML attribute values are replaced. Element order,
    /// whitespace, unknown elements, binary payloads, and the workspace trailer
    /// remain untouched.
    pub fn update_module(
        &mut self,
        base: u32,
        slot: u32,
        patch: &ModulePatch,
    ) -> Result<(), XgwxError> {
        if patch.id.is_some() || patch.sub_type.is_some() || patch.name.is_some() {
            self.validate_hardware_identity_edit(base, slot)?;
        }
        if patch.details.is_some() {
            self.require_xgk_hardware()?;
        }
        if let Some(details) = &patch.details {
            validate_module_details(details)?;
        }

        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let matches = document
            .descendants()
            .filter(|node| node.has_tag_name("Module"))
            .filter(|node| {
                node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .collect::<Vec<_>>();

        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => *module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };

        if patch.id.is_some() || patch.sub_type.is_some() {
            let id = patch
                .id
                .or_else(|| module.attribute("Id").and_then(|v| v.parse().ok()));
            let sub_type = patch
                .sub_type
                .or_else(|| module.attribute("SubType").and_then(|v| v.parse().ok()));
            let entry = crate::xgk_module_catalog()
                .iter()
                .find(|entry| id == Some(entry.id) && sub_type == Some(entry.sub_type))
                .ok_or(XgwxError::ModuleCatalogMismatch { base, slot })?;
            self.validate_module_placement(base, slot, entry.slot_span)?;
        }

        let mut replacements = Vec::new();
        push_u32_replacement(&mut replacements, module, base, slot, "Id", patch.id)?;
        push_u32_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "SubType",
            patch.sub_type,
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Name",
            patch.name.as_deref(),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Comment",
            patch.comment.as_deref(),
        )?;
        push_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Details",
            patch.details.as_deref(),
        )?;

        replacements.sort_by_key(|replacement| Reverse(replacement.0.start));
        let mut xml = self.xml.clone();
        for (range, value) in replacements {
            xml.replace_range(range, &value);
        }

        let root = parse_xml(&xml)?;
        self.xml = xml;
        self.root = root;
        Ok(())
    }

    /// Set the first `Details` byte for an XGI-D24A/B digital input module.
    pub fn set_module_input_filter(
        &mut self,
        base: u32,
        slot: u32,
        filter: ModuleInputFilter,
    ) -> Result<(), XgwxError> {
        self.require_xgk_hardware()?;
        let matches = self
            .modules()
            .into_iter()
            .filter(|module| module.base == Some(base) && module.slot == Some(slot))
            .collect::<Vec<_>>();
        let module = match matches.as_slice() {
            [] => return Err(XgwxError::ModuleNotFound { base, slot }),
            [module] => module,
            _ => return Err(XgwxError::AmbiguousModule { base, slot }),
        };
        if !module
            .name
            .as_deref()
            .is_some_and(|name| name.contains("XGI-D24A/B"))
        {
            return Err(XgwxError::InvalidModuleInputFilterTarget { base, slot });
        }

        let mut details = module
            .details
            .clone()
            .ok_or(XgwxError::MissingModuleAttribute {
                base,
                slot,
                attribute: "Details",
            })?;
        validate_module_details(&details)?;
        if details.len() < 2 {
            return Err(XgwxError::InvalidHexPayload {
                element: "Module".to_owned(),
                attribute: "Details".to_owned(),
            });
        }
        details.replace_range(..2, &format!("{:02X}", filter.raw()));
        self.update_module(
            base,
            slot,
            &ModulePatch {
                details: Some(details),
                ..ModulePatch::default()
            },
        )
    }

    /// Update editable XML metadata for one program selected by document order.
    pub fn update_program(
        &mut self,
        program_index: usize,
        patch: &ProgramPatch,
    ) -> Result<(), XgwxError> {
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;

        let mut replacements = Vec::new();
        push_program_string_replacement(
            &mut replacements,
            program,
            program_index,
            "Task",
            patch.task.as_deref(),
        )?;
        push_program_u32_replacement(
            &mut replacements,
            program,
            program_index,
            "Version",
            patch.version,
        )?;
        push_program_u32_replacement(
            &mut replacements,
            program,
            program_index,
            "Kind",
            patch.kind,
        )?;
        push_program_string_replacement(
            &mut replacements,
            program,
            program_index,
            "InstanceName",
            patch.instance_name.as_deref(),
        )?;
        push_program_string_replacement(
            &mut replacements,
            program,
            program_index,
            "Comment",
            patch.comment.as_deref(),
        )?;

        if let Some(name) = &patch.name {
            let name_node = program.children().find(|node| node.is_text()).ok_or(
                XgwxError::MissingProgramName {
                    index: program_index,
                },
            )?;
            replacements.push((name_node.range(), escape_xml_text(name)));
        }

        self.apply_xml_replacements(replacements)
    }

    /// Update editable metadata for one network selected by document order.
    pub fn update_network(
        &mut self,
        network_index: usize,
        patch: &NetworkPatch,
    ) -> Result<(), XgwxError> {
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let network = document
            .descendants()
            .filter(|node| node.has_tag_name("Network"))
            .nth(network_index)
            .ok_or(XgwxError::NetworkNotFound {
                index: network_index,
            })?;
        let mut replacements = Vec::new();
        push_network_string_replacement(
            &mut replacements,
            network,
            network_index,
            "Name",
            patch.name.as_deref(),
        )?;
        push_network_string_replacement(
            &mut replacements,
            network,
            network_index,
            "Type",
            patch.type_name.as_deref(),
        )?;
        push_network_string_replacement(
            &mut replacements,
            network,
            network_index,
            "NetworkType",
            patch.network_type.as_deref(),
        )?;
        self.apply_xml_replacements(replacements)
    }

    /// Update user-facing metadata for the unique network module at a base and
    /// slot. Hardware identity and network protocol fields remain unchanged.
    pub fn update_network_module(
        &mut self,
        base: u32,
        slot: u32,
        patch: &NetworkModulePatch,
    ) -> Result<(), XgwxError> {
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let matches = document
            .descendants()
            .filter(|node| node.has_tag_name("NetworkModule"))
            .filter(|node| {
                node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .collect::<Vec<_>>();
        let module = match matches.as_slice() {
            [] => return Err(XgwxError::NetworkModuleNotFound { base, slot }),
            [module] => *module,
            _ => return Err(XgwxError::AmbiguousNetworkModule { base, slot }),
        };
        let mut replacements = Vec::new();
        push_network_module_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "ConfigName",
            patch.config_name.as_deref(),
        )?;
        push_network_module_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Alias",
            patch.alias.as_deref(),
        )?;
        push_network_module_string_replacement(
            &mut replacements,
            module,
            base,
            slot,
            "Description",
            patch.description.as_deref(),
        )?;
        self.apply_xml_replacements(replacements)
    }

    /// Update one global variable while preserving the binary symbol layout.
    pub fn update_variable(
        &mut self,
        variable_index: usize,
        patch: &VariablePatch,
    ) -> Result<(), XgwxError> {
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let symbols = document
            .descendants()
            .find(|node| node.has_tag_name("Symbols"))
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = symbols
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = symbols
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let mut payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, string)| (string.value == "SV5.0").then_some(index))
            .collect::<Vec<_>>();
        let start = starts
            .get(variable_index)
            .copied()
            .ok_or(XgwxError::VariableNotFound {
                index: variable_index,
            })?;
        let end = starts
            .get(variable_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| record.len() >= 7)
            .ok_or(XgwxError::InvalidVariableRecord {
                index: variable_index,
            })?;

        patch_variable_string(
            &mut payload,
            variable_index,
            "name",
            &record[1],
            patch.name.as_deref(),
        )?;
        patch_variable_string(
            &mut payload,
            variable_index,
            "address area",
            &record[2],
            patch.address_area.as_deref(),
        )?;
        patch_variable_string(
            &mut payload,
            variable_index,
            "data type",
            &record[3],
            patch.data_type.as_deref(),
        )?;
        patch_variable_string(
            &mut payload,
            variable_index,
            "description",
            &record[4],
            patch.description.as_deref(),
        )?;

        if let Some(address_number) = patch.address_number {
            let range = record[2].end_offset..record[2].end_offset + 4;
            let bytes = payload
                .get_mut(range)
                .ok_or(XgwxError::InvalidVariableRecord {
                    index: variable_index,
                })?;
            bytes.copy_from_slice(&address_number.to_le_bytes());
        }

        let replacement_text = encode_payload_text(original_text, compressed, &payload)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
    }

    /// Change the mapped address of a captured IEC program-local BOOL symbol.
    /// The PB50 record stores both a display string and a binary bit number.
    /// Existing mappings keep their address area. Clearing or assigning a mapping
    /// also updates its storage class and allocation metadata.
    pub fn update_iec_local_symbol_address(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_address: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbol = symbols
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        if symbol.name != expected_name
            || symbol.address.as_deref().unwrap_or("") != expected_address
            || symbol.data_type_code != 1
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let area = if expected_address.is_empty() {
            replacement.get(..3).unwrap_or("")
        } else {
            expected_address.get(..3).unwrap_or("")
        };
        let parse_bit = |value: &str| -> Option<u32> {
            let components = value.get(3..)?.split('.').collect::<Vec<_>>();
            match components.as_slice() {
                [bit] if area == "%MX" || area == "%QX" => bit.parse().ok(),
                [base, slot, bit] if area == "%IX" || area == "%QX" => {
                    // The captured dotted addresses use base 0 and a 64-bit
                    // allocation per slot. Other layouts need native captures.
                    let (base, slot, bit) = (
                        base.parse::<u32>().ok()?,
                        slot.parse::<u32>().ok()?,
                        bit.parse::<u32>().ok()?,
                    );
                    (base == 0 && bit < 64).then_some(slot.checked_mul(64)?.checked_add(bit)?)
                }
                _ => None,
            }
        };
        let old_bit = parse_bit(expected_address);
        let new_bit = parse_bit(replacement);
        if !matches!(area, "%MX" | "%IX" | "%QX")
            || replacement == expected_address
            || (!expected_address.is_empty() && old_bit.is_none())
            || (!replacement.is_empty()
                && (!replacement.starts_with(area)
                    || new_bit.is_none()
                    || old_bit == new_bit
                    || symbols.iter().enumerate().any(|(index, other)| {
                        index != symbol_index
                            && other.address.as_deref().is_some_and(|address| {
                                address.starts_with(area) && parse_bit(address) == new_bit
                            })
                    })))
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local BOOL address must be a unique supported bit address, or empty to clear its mapping",
            });
        }
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;
        let table = program
            .descendants()
            .find(|node| node.has_tag_name("LocalVar"))
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let mut payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, string)| (string.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        let start = *starts
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        let end = starts
            .get(symbol_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| record.len() == 8)
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        if record[1].value != expected_name || record[2].value != expected_address {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let class = &record[record.len() - 3];
        let numeric_offset = class.end_offset;
        let next_offset = record[record.len() - 2].offset;
        let old_numeric = payload
            .get(numeric_offset..numeric_offset + 4)
            .filter(|_| next_offset >= numeric_offset + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes);
        let allocation = payload
            .get(numeric_offset..next_offset)
            .filter(|bytes| bytes.len() == 36)
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        let old_width = u32::from_le_bytes(allocation[4..8].try_into().unwrap());
        let mapped_class = match area {
            "%MX" => "M",
            "%IX" => "I",
            "%QX" => "Q",
            _ => unreachable!("validated IEC BOOL address area"),
        };
        let expected_class = if expected_address.is_empty() {
            ""
        } else {
            mapped_class
        };
        if old_numeric != Some(old_bit.unwrap_or(u32::MAX))
            || old_width != u32::from(!expected_address.is_empty())
            || class.value != expected_class
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        payload[numeric_offset..numeric_offset + 4]
            .copy_from_slice(&new_bit.unwrap_or(u32::MAX).to_le_bytes());
        payload[numeric_offset + 4..numeric_offset + 8]
            .copy_from_slice(&u32::from(!replacement.is_empty()).to_le_bytes());
        let replacement_class = if replacement.is_empty() {
            ""
        } else {
            mapped_class
        };
        let with_class = replace_iec_ld_text_bytes(
            &payload,
            program_index,
            class.offset,
            expected_class,
            replacement_class,
        )?;
        let updated = replace_iec_ld_text_bytes(
            &with_class,
            program_index,
            record[2].offset,
            expected_address,
            replacement,
        )?;
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
    }

    /// Rename one IEC program-local symbol and every captured LD operand or
    /// matching function-instance marker. Reject unknown reference records so
    /// the edit stays atomic.
    pub fn rename_iec_local_symbol(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbol = symbols
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        if symbol.name != expected_name {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        if replacement.is_empty()
            || replacement == expected_name
            || replacement.encode_utf16().count() > u8::MAX as usize
            || !replacement
                .chars()
                .next()
                .is_some_and(|character| character.is_alphabetic() || character == '_')
            || !replacement
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
            || symbols
                .iter()
                .enumerate()
                .any(|(index, other)| index != symbol_index && other.name == replacement)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local symbol name must be a unique identifier of at most 255 UTF-16 units",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut known_offsets = crate::iec_ld::element_operands(&program)
            .into_iter()
            .map(|item| item.string.offset)
            .chain(
                crate::iec_ld::function_operands(&program)
                    .into_iter()
                    .map(|item| item.offset),
            )
            .collect::<std::collections::HashSet<_>>();
        let mut function_name_offsets = std::collections::HashSet::new();
        if symbol.is_instance {
            let expected_type =
                symbol
                    .type_reference
                    .as_deref()
                    .ok_or(XgwxError::InvalidVariableRecord {
                        index: symbol_index,
                    })?;
            let blocks = program
                .iec_function_blocks()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            for block in blocks {
                function_name_offsets.insert(block.name.offset);
                if let Some(instance) = block.instance.filter(|item| item.value == expected_name) {
                    if block.name.value != expected_type {
                        return Err(XgwxError::InvalidLadderEdit {
                            reason: "IEC function instance type does not match its local symbol",
                        });
                    }
                    known_offsets.insert(instance.offset);
                }
            }
        }
        let mut references = extract_utf16_marker_strings(&program.data, false, false)
            .iter()
            .filter(|item| {
                item.value == expected_name && !function_name_offsets.contains(&item.offset)
            })
            .map(|item| item.offset)
            .collect::<Vec<_>>();
        if references
            .iter()
            .any(|offset| !known_offsets.contains(offset))
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC symbol has an unclassified program reference",
            });
        }
        references.sort_unstable_by(|left, right| right.cmp(left));
        let mut edited = self.clone();
        for offset in references {
            edited.replace_iec_ld_text(program_index, offset, expected_name, replacement)?;
        }
        let document = roxmltree::Document::parse(&edited.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        let start = *starts
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        let end = starts
            .get(symbol_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| matches!(record.len(), 8 | 9))
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        if record[1].value != expected_name {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let updated = replace_iec_ld_text_bytes(
            &payload,
            program_index,
            record[1].offset,
            expected_name,
            replacement,
        )?;
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        edited.apply_xml_replacements(vec![(text_node.range(), replacement_text)])?;
        *self = edited;
        Ok(())
    }

    /// Give one captured IEC function block a separate instance by cloning its
    /// PB50 instance declaration, assigning a new automatic-memory range, and changing only that block's
    /// instance marker. This is useful after copying a network containing a
    /// stateful function such as R_TRIG.
    pub fn duplicate_iec_ld_function_instance(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_instance: &str,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        if new_instance.is_empty()
            || new_instance.encode_utf16().count() > u8::MAX as usize
            || !new_instance
                .chars()
                .next()
                .is_some_and(|character| character.is_alphabetic() || character == '_')
            || !new_instance
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC function instance name must be a valid identifier of at most 255 UTF-16 units",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let block = program
            .iec_function_blocks()
            .and_then(|blocks| {
                blocks
                    .into_iter()
                    .find(|block| block.record_offset == block_offset)
            })
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        let instance = block
            .instance
            .as_ref()
            .filter(|instance| instance.value == expected_instance)
            .ok_or(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            })?;
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let template = symbols
            .iter()
            .find(|symbol| symbol.name == expected_instance)
            .filter(|symbol| {
                symbol.is_instance
                    && symbol.type_reference.as_deref() == Some(block.name.value.as_str())
                    && symbol.storage_class == "A"
                    && symbol.address.is_none()
                    && symbol.allocation_number.is_some()
                    && symbol.allocation_width == Some(8)
            })
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "IEC function instance must match a captured eight-byte automatic declaration of the same type",
            })?;
        let next_allocation = symbols
            .iter()
            .filter(|symbol| symbol.storage_class == "A")
            .map(|symbol| {
                symbol
                    .allocation_number
                    .zip(symbol.allocation_width)
                    .and_then(|(start, width)| start.checked_add(width.checked_mul(8)?))
                    .ok_or(XgwxError::UnsupportedLadderLayout)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .checked_add(63)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            / 64
            * 64;
        let folded_name = new_instance.to_lowercase();
        if symbols
            .iter()
            .any(|symbol| symbol.name.to_lowercase() == folded_name)
            || symbols
                .windows(2)
                .any(|pair| pair[0].name.to_lowercase() > pair[1].name.to_lowercase())
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC function instance name must be unique in the sorted local table",
            });
        }
        let insertion_index = symbols
            .iter()
            .position(|symbol| symbol.name.to_lowercase() > folded_name)
            .unwrap_or(symbols.len());
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let count_attribute = table
            .attributes()
            .find(|attribute| attribute.name() == "Count")
            .ok_or(XgwxError::MissingSymbols)?;
        if count_attribute.value().parse::<usize>().ok() != Some(symbols.len()) {
            return Err(XgwxError::InvalidVariableRecord {
                index: insertion_index,
            });
        }
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let template_index = symbols
            .iter()
            .position(|symbol| symbol.record_offset == template.record_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let template_end = symbols
            .get(template_index + 1)
            .map_or(payload.len(), |symbol| symbol.record_offset);
        let insertion_offset = symbols
            .get(insertion_index)
            .map_or(payload.len(), |symbol| symbol.record_offset);
        let raw_template = payload
            .get(template.record_offset..template_end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let fields = extract_utf16_marker_strings(raw_template, false, true);
        if fields.len() != 9 || fields[0].value != "PB50" || fields[1].value != expected_instance {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut cloned_record = replace_iec_ld_text_bytes(
            raw_template,
            program_index,
            fields[1].offset,
            expected_instance,
            new_instance,
        )?;
        let cloned_fields = extract_utf16_marker_strings(&cloned_record, false, true);
        let allocation_start = cloned_fields
            .get(6)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .end_offset;
        if cloned_record.get(allocation_start..allocation_start + 4)
            != Some(&template.allocation_number.unwrap().to_le_bytes())
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        cloned_record[allocation_start..allocation_start + 4]
            .copy_from_slice(&next_allocation.to_le_bytes());
        let mut updated = Vec::with_capacity(payload.len() + cloned_record.len());
        updated.extend_from_slice(&payload[..insertion_offset]);
        updated.extend_from_slice(&cloned_record);
        updated.extend_from_slice(&payload[insertion_offset..]);
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        let mut edited = self.clone();
        edited.apply_xml_replacements(vec![
            (
                count_attribute.range_value(),
                (symbols.len() + 1).to_string(),
            ),
            (text_node.range(), replacement_text),
        ])?;
        edited.replace_iec_ld_text(
            program_index,
            instance.offset,
            expected_instance,
            new_instance,
        )?;
        let verified_symbols = edited
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        let verified_program = edited
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        let verified_blocks = verified_program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if verified_symbols.len() != symbols.len() + 1
            || verified_symbols.get(insertion_index).is_none_or(|symbol| {
                symbol.name != new_instance
                    || symbol.type_reference != template.type_reference
                    || !symbol.is_instance
                    || symbol.allocation_number != Some(next_allocation)
            })
            || verified_blocks.len()
                != program
                    .iec_function_blocks()
                    .ok_or(XgwxError::UnsupportedLadderLayout)?
                    .len()
            || verified_blocks
                .iter()
                .find(|block| block.record_offset == block_offset)
                .and_then(|block| block.instance.as_ref())
                .is_none_or(|instance| instance.value != new_instance)
            || verified_program.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        *self = edited;
        Ok(())
    }

    /// Clone one captured IEC function-instance declaration into another
    /// program, assigning an unused automatic-memory range in that program.
    /// The source must contain a block using the declaration's function type.
    pub fn copy_iec_instance_declaration_to_program(
        &mut self,
        source_program_index: usize,
        source_instance: &str,
        destination_program_index: usize,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        if source_program_index == destination_program_index
            || new_instance.is_empty()
            || new_instance.encode_utf16().count() > u8::MAX as usize
            || !new_instance
                .chars()
                .next()
                .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
            || !new_instance
                .chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_')
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "cross-program IEC instance requires distinct programs and a valid new name",
            });
        }
        let programs = self.ladder_programs();
        let source = programs
            .get(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })?
            .as_ref()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        let destination = programs
            .get(destination_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: destination_program_index,
            })?
            .as_ref()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        if source.project_type != Some(2)
            || destination.project_type != Some(2)
            || source.version.as_deref() != Some("LD VER 1.1")
            || destination.version.as_deref() != Some("LD VER 1.1")
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let source_symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })??;
        let destination_symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(destination_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: destination_program_index,
            })??;
        let (source_index, template) = source_symbols
            .iter()
            .enumerate()
            .find(|(_, symbol)| symbol.name == source_instance && symbol.is_instance)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "source IEC function instance declaration is missing",
            })?;
        let function_type = template
            .type_reference
            .as_deref()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let width = template
            .allocation_width
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if template.storage_class != "A"
            || template.address.is_some()
            || !matches!(width, 8 | 192)
            || template.allocation_number.is_none()
            || !source
                .iec_function_blocks()
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .iter()
                .any(|block| {
                    block.name.value == function_type
                        && block
                            .instance
                            .as_ref()
                            .is_some_and(|instance| instance.value == source_instance)
                })
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "source IEC instance is not a captured automatic function declaration",
            });
        }
        let folded_name = new_instance.to_lowercase();
        if destination_symbols
            .iter()
            .any(|symbol| symbol.name.to_lowercase() == folded_name)
            || destination_symbols
                .windows(2)
                .any(|pair| pair[0].name.to_lowercase() > pair[1].name.to_lowercase())
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "destination IEC function instance name must be unique in a sorted table",
            });
        }
        let next_allocation = destination_symbols
            .iter()
            .filter(|symbol| symbol.storage_class == "A")
            .map(|symbol| {
                symbol
                    .allocation_number
                    .zip(symbol.allocation_width)
                    .and_then(|(start, width)| start.checked_add(width.checked_mul(8)?))
                    .ok_or(XgwxError::UnsupportedLadderLayout)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(0)
            .checked_add(63)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            / 64
            * 64;
        let source_document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let source_table = source_document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(source_program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let source_text = source_table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let source_compressed = source_table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let source_payload =
            decode_base64_payload(source_text.text().unwrap_or_default(), source_compressed)?.data;
        let template_end = source_symbols
            .get(source_index + 1)
            .map_or(source_payload.len(), |symbol| symbol.record_offset);
        let raw_template = source_payload
            .get(template.record_offset..template_end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let fields = extract_utf16_marker_strings(raw_template, false, true);
        if fields.len() != 9 || fields[0].value != "PB50" || fields[1].value != source_instance {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut record = replace_iec_ld_text_bytes(
            raw_template,
            source_program_index,
            fields[1].offset,
            source_instance,
            new_instance,
        )?;
        let record_fields = extract_utf16_marker_strings(&record, false, true);
        let allocation_start = record_fields
            .get(6)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .end_offset;
        if record.get(allocation_start..allocation_start + 4)
            != Some(&template.allocation_number.unwrap().to_le_bytes())
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        record[allocation_start..allocation_start + 4]
            .copy_from_slice(&next_allocation.to_le_bytes());
        let destination_table = source_document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(destination_program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let count = destination_table
            .attributes()
            .find(|attribute| attribute.name() == "Count")
            .ok_or(XgwxError::MissingSymbols)?;
        if count.value().parse::<usize>().ok() != Some(destination_symbols.len()) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let text_node = destination_table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = destination_table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let insertion_index = destination_symbols
            .iter()
            .position(|symbol| symbol.name.to_lowercase() > folded_name)
            .unwrap_or(destination_symbols.len());
        let insertion_offset = destination_symbols
            .get(insertion_index)
            .map_or(payload.len(), |symbol| symbol.record_offset);
        let mut updated_payload = Vec::with_capacity(payload.len() + record.len());
        updated_payload.extend_from_slice(&payload[..insertion_offset]);
        updated_payload.extend_from_slice(&record);
        updated_payload.extend_from_slice(&payload[insertion_offset..]);
        let replacement_text = encode_payload_text(original_text, compressed, &updated_payload)?;
        let mut updated = self.clone();
        updated.apply_xml_replacements(vec![
            (
                count.range_value(),
                (destination_symbols.len() + 1).to_string(),
            ),
            (text_node.range(), replacement_text),
        ])?;
        let verified = updated
            .iec_local_symbols()
            .into_iter()
            .nth(destination_program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        if verified.len() != destination_symbols.len() + 1
            || verified.get(insertion_index).is_none_or(|symbol| {
                symbol.name != new_instance
                    || !symbol.is_instance
                    || symbol.type_reference.as_deref() != Some(function_type)
                    || symbol.allocation_number != Some(next_allocation)
                    || symbol.allocation_width != Some(width)
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        *self = updated;
        Ok(())
    }

    /// Edit one captured IEC program-local symbol description. The PB50
    /// description is a length-prefixed UTF-16 field, independent of program
    /// operands and allocation metadata.
    pub fn update_iec_local_symbol_description(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_description: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbol = symbols
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        if symbol.name != expected_name
            || symbol.description.as_deref().unwrap_or("") != expected_description
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        if replacement == expected_description
            || replacement.encode_utf16().count() > u8::MAX as usize
            || replacement.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local description must change, have at most 255 UTF-16 units, and contain no control characters",
            });
        }
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        let start = *starts
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        let end = starts
            .get(symbol_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| matches!(record.len(), 8 | 9))
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        let description = &record[record.len() - 4];
        if record[0].offset != symbol.record_offset
            || record[1].value != expected_name
            || description.value != expected_description
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let updated = replace_iec_ld_text_bytes(
            &payload,
            program_index,
            description.offset,
            expected_description,
            replacement,
        )?;
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
    }

    /// Insert an unallocated primitive variable in a captured IEC program's
    /// local PB50 table. XG5000 sorts this table by case-insensitive name.
    pub fn insert_iec_local_symbol(
        &mut self,
        program_index: usize,
        name: &str,
        data_type: &str,
        description: &str,
    ) -> Result<(), XgwxError> {
        let valid_name = !name.is_empty()
            && name.encode_utf16().count() <= u8::MAX as usize
            && name
                .chars()
                .next()
                .is_some_and(|character| character.is_alphabetic() || character == '_')
            && name
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_');
        if !valid_name
            || description.encode_utf16().count() > u8::MAX as usize
            || description.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local symbol requires a valid name and a description of at most 255 UTF-16 units without control characters",
            });
        }
        let type_code = crate::iec_symbols::iec_primitive_type_code(data_type).ok_or(
            XgwxError::InvalidLadderEdit {
                reason: "IEC local symbol type is not a supported primitive",
            },
        )?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let folded_name = name.to_lowercase();
        if symbols
            .iter()
            .any(|symbol| symbol.name.to_lowercase() == folded_name)
            || symbols
                .windows(2)
                .any(|pair| pair[0].name.to_lowercase() > pair[1].name.to_lowercase())
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local symbol name must be unique and the existing table sorted",
            });
        }
        let insertion_index = symbols
            .iter()
            .position(|symbol| symbol.name.to_lowercase() > folded_name)
            .unwrap_or(symbols.len());
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let count_attribute = table
            .attributes()
            .find(|attribute| attribute.name() == "Count")
            .ok_or(XgwxError::MissingSymbols)?;
        if count_attribute.value().parse::<usize>().ok() != Some(symbols.len()) {
            return Err(XgwxError::InvalidVariableRecord {
                index: insertion_index,
            });
        }
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let insertion_offset = symbols
            .get(insertion_index)
            .map_or(payload.len(), |symbol| symbol.record_offset);
        if symbols.iter().any(|symbol| {
            payload.get(symbol.record_offset..symbol.record_offset + 12)
                != Some(&[0xff, 0xfe, 0xff, 4, b'P', 0, b'B', 0, b'5', 0, b'0', 0][..])
        }) {
            return Err(XgwxError::InvalidVariableRecord {
                index: insertion_index,
            });
        }
        fn field(record: &mut Vec<u8>, value: &str) {
            record.extend_from_slice(UTF16_MARKER);
            record.push(value.encode_utf16().count() as u8);
            for unit in value.encode_utf16() {
                record.extend_from_slice(&unit.to_le_bytes());
            }
        }
        let mut record = Vec::new();
        field(&mut record, "PB50");
        field(&mut record, name);
        record.extend_from_slice(&1_u32.to_le_bytes());
        record.extend_from_slice(&type_code.to_le_bytes());
        record.extend_from_slice(&0_u32.to_le_bytes());
        field(&mut record, ""); // type reference or mapped address
        field(&mut record, ""); // reserved metadata
        field(&mut record, description);
        field(&mut record, ""); // no allocation class yet
        record.extend_from_slice(&u32::MAX.to_le_bytes());
        record.extend_from_slice(&0_u32.to_le_bytes());
        record.extend_from_slice(&u32::MAX.to_le_bytes());
        record.extend_from_slice(&[0; 24]);
        field(&mut record, "");
        field(&mut record, "");
        let mut updated = Vec::with_capacity(payload.len() + record.len());
        updated.extend_from_slice(&payload[..insertion_offset]);
        updated.extend_from_slice(&record);
        updated.extend_from_slice(&payload[insertion_offset..]);
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        self.apply_xml_replacements(vec![
            (
                count_attribute.range_value(),
                (symbols.len() + 1).to_string(),
            ),
            (text_node.range(), replacement_text),
        ])
    }

    /// Allocate a newly inserted primitive local using the width of a
    /// captured automatic source declaration. The source width remains
    /// opaque; reserve a conservatively aligned range after existing locals.
    fn allocate_inserted_iec_local_symbol(
        &mut self,
        program_index: usize,
        name: &str,
        data_type_code: u32,
        width: u32,
    ) -> Result<(), XgwxError> {
        if width == 0 || width > 256 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let (symbol_index, symbol) = symbols
            .iter()
            .enumerate()
            .find(|(_, symbol)| symbol.name == name)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if symbol.is_instance
            || symbol.address.is_some()
            || symbol.data_type_code != data_type_code
            || !symbol.storage_class.is_empty()
            || symbol.allocation_number.is_some()
            || symbol.allocation_width.is_some()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let next_allocation = symbols
            .iter()
            .filter(|item| item.storage_class == "A")
            .map(|item| {
                item.allocation_number
                    .zip(item.allocation_width)
                    .and_then(|(start, width)| start.checked_add(width.checked_mul(8)?))
                    .ok_or(XgwxError::UnsupportedLadderLayout)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or(0)
            .checked_add(63)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            / 64
            * 64;
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let mut payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        let start = *starts
            .get(symbol_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let end = starts
            .get(symbol_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| record.len() == 8 && record[1].value == name)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let class = &record[5];
        let allocation_start = class.end_offset;
        if !class.value.is_empty()
            || payload.get(allocation_start..allocation_start + 8)
                != Some(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0][..])
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        payload[allocation_start..allocation_start + 4]
            .copy_from_slice(&next_allocation.to_le_bytes());
        payload[allocation_start + 4..allocation_start + 8].copy_from_slice(&width.to_le_bytes());
        let updated = replace_iec_ld_text_bytes(&payload, program_index, class.offset, "", "A")?;
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        let mut edited = self.clone();
        edited.apply_xml_replacements(vec![(text_node.range(), replacement_text)])?;
        let verified = edited
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        if verified.get(symbol_index).is_none_or(|item| {
            item.name != name
                || item.storage_class != "A"
                || item.allocation_number != Some(next_allocation)
                || item.allocation_width != Some(width)
        }) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        *self = edited;
        Ok(())
    }

    /// Remove an IEC program-local PB50 record when the program does not
    /// contain a marker string referring to that symbol.
    pub fn delete_iec_local_symbol(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbol = symbols
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        if symbol.name != expected_name {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        if extract_utf16_marker_strings(&program.data, false, false)
            .iter()
            .any(|item| item.value == expected_name)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC local symbol is referenced by its program",
            });
        }
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let count_attribute = table
            .attributes()
            .find(|attribute| attribute.name() == "Count")
            .ok_or(XgwxError::MissingSymbols)?;
        if count_attribute.value().parse::<usize>().ok() != Some(symbols.len()) {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original_text, compressed)?.data;
        let start = symbol.record_offset;
        let end = symbols
            .get(symbol_index + 1)
            .map_or(payload.len(), |next| next.record_offset);
        if start >= end
            || end > payload.len()
            || payload.get(start..start + 12)
                != Some(&[0xff, 0xfe, 0xff, 4, b'P', 0, b'B', 0, b'5', 0, b'0', 0][..])
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let mut updated = Vec::with_capacity(payload.len() - (end - start));
        updated.extend_from_slice(&payload[..start]);
        updated.extend_from_slice(&payload[end..]);
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        self.apply_xml_replacements(vec![
            (
                count_attribute.range_value(),
                (symbols.len() - 1).to_string(),
            ),
            (text_node.range(), replacement_text),
        ])
    }

    /// Change a program-local automatic variable's primitive IEC type.
    /// XG5000 clears its allocation on a type change, even when the new
    /// type has the same storage width. Program references are retained.
    pub fn update_iec_local_symbol_type(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_type: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbol = symbols
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        let Some(new_code) = crate::iec_symbols::iec_primitive_type_code(replacement) else {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        };
        if symbol.name != expected_name
            || symbol.data_type.as_deref() != Some(expected_type)
            || replacement == expected_type
            || symbol.is_instance
            || symbol.address.is_some()
            || !matches!(symbol.storage_class.as_str(), "A" | "")
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let table = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .and_then(|program| {
                program
                    .descendants()
                    .find(|node| node.has_tag_name("LocalVar"))
            })
            .and_then(|local| {
                local
                    .descendants()
                    .find(|node| node.has_tag_name("Symbols"))
            })
            .ok_or(XgwxError::MissingSymbols)?;
        let text_node = table
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingSymbols)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = table
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let mut payload = decode_base64_payload(original_text, compressed)?.data;
        let strings = extract_utf16_marker_strings(&payload, false, true);
        let starts = strings
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.value == "PB50").then_some(index))
            .collect::<Vec<_>>();
        let start = *starts
            .get(symbol_index)
            .ok_or(XgwxError::VariableNotFound {
                index: symbol_index,
            })?;
        let end = starts
            .get(symbol_index + 1)
            .copied()
            .unwrap_or(strings.len());
        let record = strings
            .get(start..end)
            .filter(|record| record.len() == 8)
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        let type_start = record[1].end_offset;
        let type_header = payload
            .get(type_start..record[2].offset)
            .filter(|bytes| bytes.len() == 12 && bytes[..4] == 1_u32.to_le_bytes())
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        if u32::from_le_bytes(type_header[4..8].try_into().unwrap()) != symbol.data_type_code {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        let class = &record[record.len() - 3];
        let allocation = payload
            .get(class.end_offset..record[record.len() - 2].offset)
            .filter(|bytes| bytes.len() == 36)
            .ok_or(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            })?;
        if !matches!(class.value.as_str(), "A" | "")
            || (class.value.is_empty()
                && (allocation[..4] != u32::MAX.to_le_bytes() || allocation[4..8] != [0; 4]))
        {
            return Err(XgwxError::InvalidVariableRecord {
                index: symbol_index,
            });
        }
        payload[type_start + 4..type_start + 8].copy_from_slice(&new_code.to_le_bytes());
        payload[class.end_offset..class.end_offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        payload[class.end_offset + 4..class.end_offset + 8].copy_from_slice(&0_u32.to_le_bytes());
        let updated = if class.value.is_empty() {
            payload
        } else {
            replace_iec_ld_text_bytes(&payload, program_index, class.offset, "A", "")?
        };
        let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
    }

    /// Replace decoded ladder text. Recognized application instructions allow
    /// variable-length operands and catalog-backed mnemonic changes, synchronizing
    /// opcodes, decomposed strings, occupied cells and wires. Other records retain the
    /// same-length restriction to preserve their undecoded binary layout.
    pub fn update_ladder_cell_text(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let expected_units = expected.encode_utf16().count();
        let replacement_units = replacement.encode_utf16().count();

        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;
        let program_data = program
            .descendants()
            .find(|node| node.has_tag_name("ProgramData"))
            .ok_or(XgwxError::MissingProgramData)?;
        let text_node = program_data
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingProgramData)?;
        let original_text = text_node.text().unwrap_or_default();
        let compressed = program_data
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let mut payload = decode_base64_payload(original_text, compressed)?.data;

        let Some(length_byte) = payload.get(offset + UTF16_MARKER.len()).copied() else {
            return Err(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            });
        };
        if payload.get(offset..offset + UTF16_MARKER.len()) != Some(UTF16_MARKER)
            || usize::from(length_byte) != expected_units
        {
            return Err(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            });
        }

        let text_start = offset + UTF16_MARKER.len() + 1;
        let text_end = text_start + expected_units * 2;
        let Some(encoded) = payload.get(text_start..text_end) else {
            return Err(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            });
        };
        let actual = decode_utf16_bytes(encoded).ok_or(XgwxError::LadderCellNotFound {
            program_index,
            offset,
        })?;
        if actual != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }

        if program_data.attribute("Version") == Some("LD VER 1.1")
            && program_data.attribute("ProjectType") == Some("1")
            && let Some(updated) = crate::ladder_write::update_instruction_text(
                &payload,
                offset,
                expected,
                replacement,
            )?
        {
            let replacement_text = encode_payload_text(original_text, compressed, &updated)?;
            return self.apply_xml_replacements(vec![(text_node.range(), replacement_text)]);
        }

        if replacement_units != expected_units {
            return Err(XgwxError::LadderCellLengthChanged {
                expected_utf16_units: expected_units,
                actual_utf16_units: replacement_units,
            });
        }

        for (chunk, unit) in payload[text_start..text_end]
            .chunks_exact_mut(2)
            .zip(replacement.encode_utf16())
        {
            chunk.copy_from_slice(&unit.to_le_bytes());
        }

        let replacement_text = encode_payload_text(original_text, compressed, &payload)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
    }

    /// Replace the text of a captured IEC LD comment. XG5000 stores its UTF-16
    /// length in the marker byte; the surrounding record has no length field.
    /// Symbol and operand strings are deliberately excluded from this operation.
    pub fn update_iec_ld_comment(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let comment = crate::iec_ld::comments(&program)
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if comment.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        if replacement.is_empty() || replacement.chars().any(char::is_control) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC comment text must be nonempty and contain no control characters",
            });
        }
        self.replace_iec_ld_text(program_index, offset, expected, replacement)
    }

    /// Replace the variable reference of a captured IEC LD rising-edge
    /// contact (`FF 08`).
    pub fn update_iec_ld_rising_contact_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let operand = crate::iec_ld::rising_contact_operands(&program)
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if operand.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        if replacement.is_empty() || replacement.chars().any(char::is_control) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact variable must be nonempty and contain no control characters",
            });
        }
        self.replace_iec_ld_text(program_index, offset, expected, replacement)
    }

    /// Replace an operand in a captured IEC LD contact or output coil record.
    /// This accepts only the record codes identified by `iec_ld::element_operands`.
    /// Classified replacements must be BOOL; coil destinations must be writable.
    /// XG5000 must still validate unclassified expressions.
    pub fn update_iec_ld_element_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let operand = crate::iec_ld::element_operands(&program)
            .into_iter()
            .find(|item| item.string.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if operand.string.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        if replacement.is_empty() || replacement.chars().any(char::is_control) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC element variable must be nonempty and contain no control characters",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let classified = classify_iec_bool_expression(replacement, &symbols, &program);
        if let Some(expression) = classified {
            if expression.data_type_mask & 1 == 0 {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC contact or coil operand must resolve to BOOL",
                });
            }
            if operand.record_code >= 0x0e && !expression.writable {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC output coil requires a writable BOOL destination",
                });
            }
        } else if replacement.trim().starts_with('%') {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact or coil device address must use a BOOL X address",
            });
        }
        self.replace_iec_ld_text(program_index, offset, expected, replacement)
    }

    /// Change a captured IEC contact among the six addressed contact kinds.
    pub fn update_iec_ld_contact_kind(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let contact = crate::iec_ld::element_operands(&program)
            .into_iter()
            .find(|item| item.string.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        let expected_code = iec_rung_contact_code(expected)?;
        let replacement_code = iec_rung_contact_code(replacement)?;
        if contact.record_code != expected_code {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        let code_offset = offset - 14;
        self.edit_program_payload(program_index, "2", |payload| {
            let mut updated = payload.to_vec();
            if updated.get(code_offset) != Some(&expected_code) {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset,
                });
            }
            updated[code_offset] = replacement_code;
            Ok(updated)
        })
    }

    /// Change a captured IEC coil among output, inverse, set, reset, rising,
    /// and falling kinds while preserving its operand and position.
    pub fn update_iec_ld_coil_kind(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let coil = crate::iec_ld::element_operands(&program)
            .into_iter()
            .find(|item| item.string.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        let expected_code = iec_rung_coil_code(expected)?;
        let replacement_code = iec_rung_coil_code(replacement)?;
        if coil.record_code != expected_code {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        let code_offset = offset
            .checked_sub(14)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        self.edit_program_payload(program_index, "2", |payload| {
            let mut updated = payload.to_vec();
            if updated.get(code_offset) != Some(&expected_code) {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset,
                });
            }
            updated[code_offset] = replacement_code;
            Ok(updated)
        })
    }

    /// Add or remove one captured IEC LD vertical branch segment between two
    /// adjacent rows that already belong to the same native row group.
    ///
    /// Removal is accepted only while another segment still connects the same
    /// row pair, because splitting and rebuilding row groups is not decoded yet.
    /// Addition is limited to rows without comments or function records and
    /// preserves the group's existing row envelope.
    #[allow(clippy::too_many_arguments)]
    pub fn edit_iec_ld_branch_segment(
        &mut self,
        program_index: usize,
        group_index: usize,
        start_row_index: u16,
        end_row_index: u16,
        x: u8,
        expected: bool,
        present: bool,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = edit_iec_ld_branch_segment_bytes(
            &program,
            group_index,
            start_row_index,
            end_row_index,
            x,
            expected,
            present,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Insert one implicit blank IEC LD row after `after_row_index`, matching
    /// XG5000 Ctrl+L on a blank row boundary. The payload does not gain a row
    /// record; later row envelopes and every decoded Y coordinate move down by
    /// one four-unit grid row.
    pub fn insert_iec_ld_blank_row(
        &mut self,
        program_index: usize,
        after_row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = insert_iec_ld_blank_row_bytes(&program, after_row_index)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Compatibility wrapper for a normally open parallel contact.
    pub fn insert_iec_ld_parallel_contact(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        self.insert_iec_ld_parallel_contact_kind(
            program_index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            "NO",
            parallel_variable,
        )
    }

    /// Add an addressed contact in parallel with the leading contact of a
    /// captured one-row addressed contact/wire/output-coil rung. The operation creates
    /// a second row, connects it at the contact's right edge, and shifts later
    /// rows down. The complete two-row network is validated before writing.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_iec_ld_parallel_contact_kind(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_kind: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        let parallel_code = iec_rung_contact_code(parallel_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let units = parallel_variable.encode_utf16().count();
        if units == 0 || units > u8::MAX as usize || parallel_variable.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC parallel contact variable must be 1 to 255 UTF-16 units without controls",
            });
        }
        if !classify_iec_bool_expression(parallel_variable, &symbols, &program)
            .is_some_and(|expression| expression.data_type_mask & 1 != 0)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC parallel contact variable must resolve to BOOL",
            });
        }
        let updated = insert_iec_ld_parallel_contact_bytes(
            &program,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            parallel_code,
            parallel_variable,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete one implicit IEC LD blank row and move every later decoded Y
    /// coordinate up by one row, matching XG5000 Ctrl+D on an empty line.
    pub fn delete_iec_ld_blank_row(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = delete_iec_ld_blank_row_bytes(&program, blank_row_index)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Create the compatibility normally-open-contact to output-coil IEC LD
    /// rung. See [`Self::insert_iec_ld_rung`] for the other decoded kinds.
    pub fn insert_iec_ld_linear_rung(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
        contact_variable: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.insert_iec_ld_rung(
            program_index,
            blank_row_index,
            "NO",
            contact_variable,
            "OUTPUT",
            coil_variable,
        )
    }

    /// Create a native three-record IEC LD rung in one implicit blank row.
    /// The captured shape is an addressed BOOL contact at the left rail, a
    /// long wire, and a BOOL coil at the right rail. A new single-row group is
    /// inserted without moving any existing row, including after the last
    /// stored network, extending the stored row range by one when necessary.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_iec_ld_rung(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
        contact_kind: &str,
        contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        let contact_code = iec_rung_contact_code(contact_kind)?;
        let coil_code = iec_rung_coil_code(coil_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        for (index, variable) in [contact_variable, coil_variable].into_iter().enumerate() {
            let units = variable.encode_utf16().count();
            if units == 0 || units > u8::MAX as usize || variable.chars().any(char::is_control) {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC rung variables must be 1 to 255 UTF-16 units without controls",
                });
            }
            if !classify_iec_bool_expression(variable, &symbols, &program).is_some_and(
                |expression| {
                    expression.data_type_mask & 1 != 0 && (index == 0 || expression.writable)
                },
            ) {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC rung contact and coil operands must resolve to BOOL symbols or device addresses",
                });
            }
        }
        let updated = insert_iec_ld_linear_rung_bytes(
            &program,
            blank_row_index,
            contact_code,
            contact_variable,
            coil_code,
            coil_variable,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete the compatibility normally-open-contact to output-coil rung.
    /// See [`Self::delete_iec_ld_rung`] for the other decoded kinds.
    pub fn delete_iec_ld_linear_rung(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.delete_iec_ld_rung(
            program_index,
            row_index,
            "NO",
            expected_contact_variable,
            "OUTPUT",
            expected_coil_variable,
        )
    }

    /// Delete one exact addressed-contact, wire, and coil single-row group,
    /// restoring its row to an implicit blank gap without moving other rows.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_iec_ld_rung(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        let expected_contact_code = iec_rung_contact_code(expected_contact_kind)?;
        let expected_coil_code = iec_rung_coil_code(expected_coil_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = delete_iec_ld_linear_rung_bytes(
            &program,
            row_index,
            expected_contact_code,
            expected_contact_variable,
            expected_coil_code,
            expected_coil_variable,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete the terminal wire and coil from a captured one-row IEC rung.
    /// XG5000 keeps the leading contact as the row's sole record.
    pub fn delete_iec_ld_terminal_coil(
        &mut self,
        program_index: usize,
        coil_record_offset: usize,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let coil = records
            .iter()
            .find(|record| record.offset == coil_record_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: coil_record_offset,
            })?;
        if !matches!(coil.kind, IecRecordKind::Coil(0x0e..=0x13)) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let row = rows
            .iter()
            .find(|row| row.group_index == coil.group_index && row.row_index == coil.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = rows
            .iter()
            .filter(|candidate| candidate.group_index == row.group_index)
            .count();
        let row_records = records
            .iter()
            .filter(|record| {
                record.group_index == row.group_index && record.row_index == row.row_index
            })
            .collect::<Vec<_>>();
        if group_rows != 1
            || row.record_count != 3
            || row_records.len() != 3
            || !matches!(row_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
            || row_records[1].kind != IecRecordKind::LongWire
            || row_records[2].offset != coil_record_offset
            || program.data.get(row_records[0].offset + 5) != Some(&1)
            || program.data.get(row_records[1].offset + 5) != Some(&4)
            || program.data.get(row_records[1].offset + 15) != Some(&91)
            || program.data.get(coil.offset + 5) != Some(&94)
            || row_records[0].end != row_records[1].offset
            || row_records[1].end != coil.offset
            || coil.end != row.end
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let operand = crate::iec_ld::element_operands(&program)
            .into_iter()
            .find(|item| item.string.offset == coil.offset + 15)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if operand.string.value != expected_variable {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: coil_record_offset,
            });
        }
        let mut updated = program.data.clone();
        updated[row.start + 33..row.start + 35].copy_from_slice(&1u16.to_le_bytes());
        updated.drain(row_records[1].offset..coil.end);
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        let verified_rows = verified
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let verified_records = verified
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if verified_rows.len() != rows.len()
            || verified_records.len() != records.len() - 2
            || verified_rows
                .iter()
                .find(|candidate| {
                    candidate.group_index == row.group_index && candidate.row_index == row.row_index
                })
                .is_none_or(|candidate| candidate.record_count != 1)
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: coil_record_offset,
                });
            }
            Ok(updated)
        })
    }

    /// Finish a contact-only one-row IEC rung with a native long wire and coil.
    /// The row must have the exact shape left by terminal-coil deletion.
    pub fn insert_iec_ld_terminal_coil(
        &mut self,
        program_index: usize,
        contact_record_offset: usize,
        expected_contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        let coil_code = iec_rung_coil_code(coil_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let contact = records
            .iter()
            .find(|record| record.offset == contact_record_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: contact_record_offset,
            })?;
        if !matches!(contact.kind, IecRecordKind::Contact(0x06..=0x0b)) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let row = rows
            .iter()
            .find(|row| {
                row.group_index == contact.group_index && row.row_index == contact.row_index
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if row.record_count != 1
            || contact.end != row.end
            || rows
                .iter()
                .filter(|candidate| candidate.group_index == row.group_index)
                .count()
                != 1
            || program.data.get(contact.offset + 5) != Some(&1)
            || records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .count()
                != 1
            || crate::iec_ld::element_operands(&program)
                .into_iter()
                .find(|item| item.string.offset == contact.offset + 15)
                .is_none_or(|item| item.string.value != expected_contact_variable)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let units = coil_variable.encode_utf16().collect::<Vec<_>>();
        if units.is_empty()
            || units.len() > u8::MAX as usize
            || coil_variable.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC coil variable must be 1 to 255 UTF-16 units without controls",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if !classify_iec_bool_expression(coil_variable, &symbols, &program)
            .is_some_and(|expression| expression.data_type_mask & 1 != 0 && expression.writable)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC coil operand must resolve to a BOOL symbol or device address",
            });
        }
        let y = row
            .row_index
            .checked_mul(4)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes();
        let mut inserted = vec![
            0xff, 0x02, 0, 0, 0, 4, y[0], y[1], 0, 0, 0, 4, 0, 0, 0, 91, y[0], y[1], 0,
        ];
        inserted.extend_from_slice(&[
            0xff,
            coil_code,
            0,
            0,
            0,
            94,
            y[0],
            y[1],
            0,
            1,
            0,
            0x24,
            0,
            0,
            0,
            0xff,
            0xfe,
            0xff,
            units.len() as u8,
        ]);
        for unit in units {
            inserted.extend_from_slice(&unit.to_le_bytes());
        }
        let mut updated = program.data.clone();
        updated[row.start + 33..row.start + 35].copy_from_slice(&3u16.to_le_bytes());
        updated.splice(contact.end..contact.end, inserted);
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        let verified_rows = verified
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let verified_records = verified
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if verified_rows.len() != rows.len()
            || verified_records.len() != records.len() + 2
            || verified_rows
                .iter()
                .find(|candidate| {
                    candidate.group_index == row.group_index && candidate.row_index == row.row_index
                })
                .is_none_or(|candidate| candidate.record_count != 3)
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: contact_record_offset,
                });
            }
            Ok(updated)
        })
    }

    /// Delete one exact simple IEC LD rung and close its row, matching
    /// XG5000 Ctrl+D on an occupied line. Both edits are checked before the
    /// document is changed.
    #[allow(clippy::too_many_arguments)]
    pub fn delete_iec_ld_simple_row(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        let expected_contact_code = iec_rung_contact_code(expected_contact_kind)?;
        let expected_coil_code = iec_rung_coil_code(expected_coil_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let without_rung = delete_iec_ld_linear_rung_bytes(
            &program,
            row_index,
            expected_contact_code,
            expected_contact_variable,
            expected_coil_code,
            expected_coil_variable,
        )?;
        let mut intermediate = program.clone();
        intermediate.decoded_len = without_rung.len();
        intermediate.data = without_rung;
        let updated = delete_iec_ld_blank_row_bytes(&intermediate, row_index)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete the upper line of a captured two-row IEC branch. The lower
    /// contact-only line remains and all later rows move up, as in XG5000
    /// Ctrl+D on the upper branch line.
    pub fn delete_iec_ld_branch_top_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = delete_iec_ld_branch_top_row_bytes(&program, group_index, row_index)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete a contact-only middle line from a nested IEC branch, reconnecting
    /// the outer x3 branch and removing the two branches local to that line.
    pub fn delete_iec_ld_nested_contact_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let geometry = program
            .iec_geometry()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = rows
            .iter()
            .filter(|row| row.group_index == group_index)
            .collect::<Vec<_>>();
        let connection = geometry
            .vertical
            .iter()
            .find(|branch| {
                branch.group_index == group_index
                    && branch.end_row_index == row_index
                    && branch.x == 3
            })
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "the selected IEC row has no nested x3 branch",
            })?;
        let updated = remove_iec_ld_middle_nested_contact_branch_row(
            &program,
            &rows,
            &records,
            &geometry,
            connection,
            &group_rows,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete a contact-only middle line in a chain of x3 branches. The
    /// adjacent vertical segments become one segment across the removed line.
    pub fn delete_iec_ld_chained_contact_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated =
            delete_iec_ld_chained_contact_branch_row_bytes(&program, group_index, row_index)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Delete a middle IEC row that only carries an incoming and outgoing
    /// vertical branch. The two segments are joined across the row gap.
    pub fn delete_iec_ld_empty_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated =
            delete_iec_ld_chained_branch_row_bytes(&program, group_index, row_index, false)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Reproduce XG5000 Delete Line on the lower output row of the captured
    /// two-row FF branch. Native deletion also removes the connected FF block
    /// and its output reference from the upper row.
    pub fn delete_iec_ld_ff_branch_output_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let updated = delete_iec_ld_ff_branch_output_row_bytes(
            &program,
            program_index,
            group_index,
            row_index,
        )?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Insert a normally open contact into a captured linear IEC LD wire.
    /// This compatibility wrapper delegates to [`Self::insert_iec_ld_contact`].
    pub fn insert_iec_ld_no_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.insert_iec_ld_contact(
            program_index,
            wire_offset,
            raw_x,
            expected_wire_start_x,
            expected_wire_end_x,
            "NO",
            variable,
        )
    }

    /// Insert an addressed contact into the empty x1 cell of a captured
    /// two-row IEC branch upper line, matching XG5000 F3/F4 insertion.
    pub fn insert_iec_ld_leading_contact(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        let contact_code = iec_rung_contact_code(contact_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let site = program
            .iec_leading_contact_insertion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.insertion_offset == insertion_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: insertion_offset,
            })?;
        let row = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let utf16 = variable.encode_utf16().collect::<Vec<_>>();
        if utf16.is_empty()
            || utf16.len() > u8::MAX as usize
            || variable.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact variable must be 1 to 255 UTF-16 units without controls",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if !classify_iec_bool_expression(variable, &symbols, &program)
            .is_some_and(|expression| expression.data_type_mask & 1 != 0)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact operand must resolve to a BOOL symbol or device address",
            });
        }
        let y = site
            .row_index
            .checked_mul(4)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes();
        let mut contact = vec![
            0xff,
            contact_code,
            0,
            0,
            0,
            1,
            y[0],
            y[1],
            0,
            1,
            0,
            4,
            0,
            0,
            0,
            0xff,
            0xfe,
            0xff,
            utf16.len() as u8,
        ];
        for unit in utf16 {
            contact.extend_from_slice(&unit.to_le_bytes());
        }
        let mut updated = program.data.clone();
        updated.splice(insertion_offset..insertion_offset, contact);
        updated[row.start + 33..row.start + 35].copy_from_slice(
            &row.record_count
                .checked_add(1)
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .to_le_bytes(),
        );
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        if verified.iec_row_frames().is_none()
            || verified.iec_record_frames().is_none()
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: insertion_offset,
                });
            }
            Ok(updated)
        })
    }

    /// Insert one of the six addressed IEC contact kinds into a captured long
    /// wire. The record shape follows native XG5000 insertion and contact-kind
    /// captures. The operand must resolve to a BOOL symbol or device address.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_iec_ld_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        let contact_code = iec_rung_contact_code(contact_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let wire = records
            .iter()
            .find(|record| record.offset == wire_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: wire_offset,
            })?;
        if wire.kind != crate::IecRecordKind::LongWire {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC insertion requires a long wire record",
            });
        }
        let row = rows
            .iter()
            .find(|row| row.group_index == wire.group_index && row.row_index == wire.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let row_records = records
            .iter()
            .filter(|record| {
                record.group_index == row.group_index && record.row_index == row.row_index
            })
            .collect::<Vec<_>>();
        if row_records.len() != row.record_count as usize
            || !row_records
                .iter()
                .any(|record| record.offset == wire_offset)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC insertion requires a framed long wire in its row",
            });
        }
        let old = program
            .data
            .get(wire.offset..wire.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if old.len() != 19 || old[5] != expected_wire_start_x || old[15] != expected_wire_end_x {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: wire_offset,
            });
        }
        if !matches!(&old[9..15], [0, 0, 4, 0, 0, 0] | [0, 0, 0, 0, 0, 0])
            || raw_x < old[5].saturating_add(3)
            || raw_x.saturating_add(3) > old[15]
            || !(raw_x - old[5]).is_multiple_of(3)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact position must split the captured long wire on its grid",
            });
        }
        let utf16 = variable.encode_utf16().collect::<Vec<_>>();
        if utf16.is_empty()
            || utf16.len() > u8::MAX as usize
            || variable.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact variable must be 1 to 255 UTF-16 units without controls",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if !classify_iec_bool_expression(variable, &symbols, &program)
            .is_some_and(|expression| expression.data_type_mask & 1 != 0)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact operand must resolve to a BOOL symbol or device address",
            });
        }
        let row_start = row.start;
        let row_count = row.record_count;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(wire_offset..wire.end) != Some(old)
                || payload.get(row_start + 33..row_start + 35)
                    != Some(row_count.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: wire_offset,
                });
            }
            let mut left = old.to_vec();
            left[15] = raw_x - 3;
            let mut right = old.to_vec();
            right[5] = raw_x + 3;
            let y = (row.row_index * 4).to_le_bytes();
            let mut contact = vec![
                0xff,
                contact_code,
                0,
                0,
                0,
                raw_x,
                y[0],
                y[1],
                0,
                1,
                0,
                4,
                0,
                0,
                0,
                0xff,
                0xfe,
                0xff,
                utf16.len() as u8,
            ];
            for unit in &utf16 {
                contact.extend_from_slice(&unit.to_le_bytes());
            }
            let mut updated = Vec::with_capacity(payload.len() + contact.len() + right.len());
            updated.extend_from_slice(&payload[..wire_offset]);
            updated.extend_from_slice(&left);
            updated.extend_from_slice(&contact);
            updated.extend_from_slice(&right);
            updated.extend_from_slice(&payload[wire.end..]);
            updated[row_start + 29] = updated[row_start + 29].max(raw_x);
            updated[row_start + 33..row_start + 35].copy_from_slice(&(row_count + 2).to_le_bytes());
            Ok(updated)
        })
    }

    /// Replace one framed IEC short wire with an addressed contact. XG5000 F3
    /// preserves the row count and surrounding records for this cell edit.
    pub fn insert_iec_ld_short_wire_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        expected_raw_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        let contact_code = iec_rung_contact_code(contact_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_short_wire_contact_insertion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.wire_offset == wire_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: wire_offset,
            })?;
        if site.raw_x != expected_raw_x {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: wire_offset,
            });
        }
        let row = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let wire = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|record| record.offset == wire_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let old = program
            .data
            .get(wire.offset..wire.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let utf16 = variable.encode_utf16().collect::<Vec<_>>();
        if utf16.is_empty()
            || utf16.len() > u8::MAX as usize
            || variable.chars().any(char::is_control)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact variable must be 1 to 255 UTF-16 units without controls",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if !classify_iec_bool_expression(variable, &symbols, &program)
            .is_some_and(|expression| expression.data_type_mask & 1 != 0)
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC contact operand must resolve to a BOOL symbol or device address",
            });
        }
        let mut contact = vec![
            0xff,
            contact_code,
            0,
            0,
            0,
            site.raw_x,
            old[6],
            old[7],
            old[8],
            1,
            0,
            4,
            0,
            0,
            0,
            0xff,
            0xfe,
            0xff,
            utf16.len() as u8,
        ];
        for unit in utf16 {
            contact.extend_from_slice(&unit.to_le_bytes());
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(wire.offset..wire.end) != Some(old)
                || payload.get(row.start + 33..row.start + 35)
                    != Some(row.record_count.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: wire_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated.splice(wire.offset..wire.end, contact);
            Ok(updated)
        })
    }

    /// Delete any decoded contact between two captured wire fragments, checking
    /// its expected kind before applying native XG5000 Delete semantics.
    pub fn delete_iec_ld_contact(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.verify_iec_ld_contact_kind(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
        )?;
        self.delete_iec_ld_no_contact(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }

    /// Delete a decoded contact between two captured wire fragments.
    /// Native XG5000 Delete leaves both fragments in place and decrements the
    /// row record count. Retained for compatibility with the original NO/NC API.
    pub fn delete_iec_ld_no_contact(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_no_contact_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.contact_offset == contact_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: contact_offset,
            })?;
        if site.raw_x != expected_raw_x {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: contact_offset,
            });
        }
        let row = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let contact = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|record| record.offset == contact_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let bytes = program
            .data
            .get(contact.offset..contact.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let expected_utf16 = expected_variable.encode_utf16().collect::<Vec<_>>();
        if expected_utf16.is_empty()
            || expected_utf16.len() > u8::MAX as usize
            || bytes.len() != 19 + expected_utf16.len() * 2
            || bytes[18] as usize != expected_utf16.len()
            || bytes[19..]
                .chunks_exact(2)
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                .collect::<Vec<_>>()
                != expected_utf16
        {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: contact_offset,
            });
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(contact_offset..contact.end) != Some(bytes)
                || payload.get(row.start + 33..row.start + 35)
                    != Some(row.record_count.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: contact_offset,
                });
            }
            let mut updated = Vec::with_capacity(payload.len() - bytes.len());
            updated.extend_from_slice(&payload[..contact_offset]);
            updated.extend_from_slice(&payload[contact.end..]);
            updated[row.start + 33..row.start + 35]
                .copy_from_slice(&(row.record_count - 1).to_le_bytes());
            Ok(updated)
        })
    }

    /// Delete any decoded contact with native XG5000 Cell Delete semantics,
    /// checking its expected kind before applying the edit.
    pub fn delete_iec_ld_contact_cell(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.verify_iec_ld_contact_kind(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
        )?;
        self.delete_iec_ld_no_contact_cell(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }

    /// Delete a decoded contact with native XG5000 Cell Delete semantics.
    /// A leading branch contact moves the next contact to x1 and puts a short
    /// wire at x4. Contacts between long wires shift later cells left while
    /// the output coil stays fixed. Retained for API compatibility.
    pub fn delete_iec_ld_no_contact_cell(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_no_contact_cell_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.contact_offset == contact_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: contact_offset,
            })?;
        if site.raw_x != expected_raw_x {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: contact_offset,
            });
        }
        let row = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let row_records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .filter(|record| {
                record.group_index == row.group_index && record.row_index == row.row_index
            })
            .collect::<Vec<_>>();
        let contact_index = row_records
            .iter()
            .position(|record| record.offset == contact_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let contact = row_records[contact_index];
        let bytes = program
            .data
            .get(contact.offset..contact.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let expected_utf16 = expected_variable.encode_utf16().collect::<Vec<_>>();
        if expected_utf16.is_empty()
            || expected_utf16.len() > u8::MAX as usize
            || bytes.len() != 19 + expected_utf16.len() * 2
            || bytes[18] as usize != expected_utf16.len()
            || bytes[19..]
                .chunks_exact(2)
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                .collect::<Vec<_>>()
                != expected_utf16
        {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: contact_offset,
            });
        }
        let contact_bytes = bytes.to_vec();
        let row_start = row.start;
        let row_count = row.record_count;
        if site.raw_x == 1 {
            let next = row_records
                .get(1)
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let next_bytes = program
                .data
                .get(next.offset..next.end)
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .to_vec();
            if contact_index != 0
                || !matches!(next.kind, IecRecordKind::Contact(6..=11))
                || next_bytes.len() < 21
                || next_bytes[5] != 4
                || next_bytes[6..9] != contact_bytes[6..9]
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            let mut moved_contact = next_bytes.clone();
            moved_contact[5] = 1;
            let short_wire = [
                0xff,
                0x01,
                0,
                0,
                0,
                4,
                contact_bytes[6],
                contact_bytes[7],
                contact_bytes[8],
                0,
                0,
                4,
                0,
                0,
                0,
            ];
            return self.edit_program_payload(program_index, "2", |payload| {
                if payload.get(contact.offset..contact.end) != Some(contact_bytes.as_slice())
                    || payload.get(next.offset..next.end) != Some(next_bytes.as_slice())
                    || payload.get(row_start + 33..row_start + 35)
                        != Some(row_count.to_le_bytes().as_slice())
                {
                    return Err(XgwxError::LadderCellChanged {
                        program_index,
                        offset: contact_offset,
                    });
                }
                let mut updated = payload.to_vec();
                updated.splice(
                    contact.offset..next.end,
                    moved_contact.into_iter().chain(short_wire),
                );
                Ok(updated)
            });
        }
        let mut coordinate_edits = Vec::new();
        for (index, record) in row_records.iter().enumerate().skip(contact_index + 1) {
            match record.kind {
                IecRecordKind::Contact(_) => {
                    coordinate_edits.push(record.offset + 5);
                }
                IecRecordKind::LongWire => {
                    coordinate_edits.push(record.offset + 5);
                    if row_records
                        .get(index + 1)
                        .is_some_and(|next| matches!(next.kind, IecRecordKind::Contact(_)))
                    {
                        coordinate_edits.push(record.offset + 15);
                    }
                }
                IecRecordKind::Coil(_) => {}
                _ => return Err(XgwxError::UnsupportedLadderLayout),
            }
        }
        let last_contact_x = row_records
            .iter()
            .filter(|record| {
                record.offset != contact_offset && matches!(record.kind, IecRecordKind::Contact(_))
            })
            .map(|record| {
                let x = program.data[record.offset + 5];
                if record.offset > contact_offset {
                    x.checked_sub(3)
                } else {
                    Some(x)
                }
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .max()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(contact.offset..contact.end) != Some(contact_bytes.as_slice())
                || payload.get(row_start + 33..row_start + 35)
                    != Some(row_count.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: contact_offset,
                });
            }
            let mut updated = payload.to_vec();
            for &offset in &coordinate_edits {
                updated[offset] = updated[offset]
                    .checked_sub(3)
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
            }
            updated.drain(contact.offset..contact.end);
            updated[row_start + 29] = last_contact_x;
            updated[row_start + 33..row_start + 35].copy_from_slice(&(row_count - 1).to_le_bytes());
            Ok(updated)
        })
    }

    fn verify_iec_ld_contact_kind(
        &self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
    ) -> Result<(), XgwxError> {
        let expected_code = iec_rung_contact_code(expected_contact_kind)?;
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let record = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|record| record.offset == contact_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: contact_offset,
            })?;
        if record.kind != IecRecordKind::Contact(expected_code)
            || program.data.get(contact_offset + 5) != Some(&expected_raw_x)
        {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: contact_offset,
            });
        }
        Ok(())
    }

    /// Remove a captured one-cell `FF 01` wire between two long wires.
    /// The resulting gap must be recognized by the guarded wire repair path.
    pub fn delete_iec_ld_horizontal_wire(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let record_index = records
            .iter()
            .position(|record| record.offset == wire_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: wire_offset,
            })?;
        let wire = &records[record_index];
        let left = record_index
            .checked_sub(1)
            .and_then(|index| records.get(index))
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let right = records
            .get(record_index + 1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let row = rows
            .iter()
            .find(|row| row.group_index == wire.group_index && row.row_index == wire.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let wire_bytes = program
            .data
            .get(wire.offset..wire.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let left_bytes = program
            .data
            .get(left.offset..left.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let right_bytes = program
            .data
            .get(right.offset..right.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if wire.kind != IecRecordKind::ShortWire
            || left.kind != IecRecordKind::LongWire
            || right.kind != IecRecordKind::LongWire
            || left.group_index != wire.group_index
            || right.group_index != wire.group_index
            || left.row_index != wire.row_index
            || right.row_index != wire.row_index
            || left.end != wire.offset
            || wire.end != right.offset
            || wire_bytes.len() != 15
            || left_bytes.len() != 19
            || right_bytes.len() != 19
            || wire_bytes[5] != expected_raw_x
            || left_bytes[15].checked_add(3) != Some(expected_raw_x)
            || expected_raw_x.checked_add(3) != Some(right_bytes[5])
            || wire_bytes[6..9] != left_bytes[6..9]
            || wire_bytes[6..9] != right_bytes[6..9]
            || wire_bytes[9..15] != left_bytes[9..15]
            || wire_bytes[9..15] != right_bytes[9..15]
            || row.record_count < 3
            || !records.iter().any(|record| {
                record.group_index == row.group_index
                    && record.row_index == row.row_index
                    && record.offset < left.offset
                    && matches!(record.kind, IecRecordKind::Contact(_))
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut updated = Vec::with_capacity(program.data.len() - wire_bytes.len());
        updated.extend_from_slice(&program.data[..wire_offset]);
        updated.extend_from_slice(&program.data[wire.end..]);
        updated[row.start + 29] = expected_raw_x;
        updated[row.start + 33..row.start + 35]
            .copy_from_slice(&(row.record_count - 1).to_le_bytes());
        let mut preview = program.clone();
        preview.decoded_len = updated.len();
        preview.data = updated.clone();
        if !preview
            .iec_horizontal_wire_repair_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .iter()
            .any(|site| site.insertion_offset == wire_offset && site.raw_x == expected_raw_x)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: wire_offset,
                });
            }
            Ok(updated)
        })
    }

    /// Reconnect the one-cell gap left by the captured IEC contact deletion.
    /// Native XG5000 F5 inserts an `FF 01` short-wire record, increments the
    /// row count, and resets the row header coordinate to the first contact.
    pub fn repair_iec_ld_horizontal_wire(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_horizontal_wire_repair_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.insertion_offset == insertion_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: insertion_offset,
            })?;
        if site.raw_x != expected_raw_x {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: insertion_offset,
            });
        }
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let row = rows
            .iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let header_x = records
            .iter()
            .filter(|record| {
                record.group_index == row.group_index
                    && record.row_index == row.row_index
                    && matches!(
                        record.kind,
                        IecRecordKind::Contact(_) | IecRecordKind::FunctionBlock
                    )
            })
            .filter_map(|record| program.data.get(record.offset + 5).copied())
            .max()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let left = records
            .iter()
            .find(|record| record.end == insertion_offset && record.kind == IecRecordKind::LongWire)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let left_bytes = program
            .data
            .get(left.offset..left.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let updated_row_count = row
            .record_count
            .checked_add(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let y = (row.row_index * 4).to_le_bytes();
        let mut short_wire = [
            0xff, 0x01, 0, 0, 0, site.raw_x, y[0], y[1], 0, 0, 0, 0, 0, 0, 0,
        ];
        short_wire[9..15].copy_from_slice(&left_bytes[9..15]);
        let row_start = row.start;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(row_start + 29) != Some(&expected_raw_x)
                || payload.get(row_start + 33..row_start + 35)
                    != Some(row.record_count.to_le_bytes().as_slice())
                || insertion_offset > payload.len()
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: insertion_offset,
                });
            }
            let mut updated = Vec::with_capacity(payload.len() + short_wire.len());
            updated.extend_from_slice(&payload[..insertion_offset]);
            updated.extend_from_slice(&short_wire);
            updated.extend_from_slice(&payload[insertion_offset..]);
            updated[row_start + 29] = header_x;
            updated[row_start + 33..row_start + 35]
                .copy_from_slice(&updated_row_count.to_le_bytes());
            Ok(updated)
        })
    }

    /// Replace a captured IEC LD function input/output expression (`FF 46`).
    /// Classified symbols and literals are checked against the decoded pin type;
    /// output pins also require a writable destination.
    pub fn update_iec_ld_function_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let operand = crate::iec_ld::function_operands(&program)
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if operand.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        if replacement.is_empty() || replacement.chars().any(char::is_control) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC function operand must be nonempty and contain no control characters",
            });
        }
        let record = program
            .iec_record_frames()
            .and_then(|records| {
                records.into_iter().find(|record| {
                    record.kind == IecRecordKind::FunctionOperand
                        && operand.offset >= record.offset
                        && operand.offset < record.end
                })
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let link = program
            .iec_function_operand_links()
            .and_then(|links| {
                links
                    .into_iter()
                    .find(|link| link.record_offset == record.offset)
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let classified = classify_iec_expression(replacement, &symbols, &program);
        if looks_like_iec_direct_device_address(replacement.trim()) && classified.is_none() {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC direct device address has an unsupported form",
            });
        }
        if looks_like_iec_arithmetic_expression(replacement) && classified.is_none() {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC arithmetic expression has an unsupported operand or type",
            });
        }
        if let Some(expression) = classified {
            if expression.data_type_mask & link.data_type_mask == 0 {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC function operand type is incompatible with the decoded pin type",
                });
            }
            if link.is_output && !expression.writable {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC function output requires a writable variable or device address",
                });
            }
        }
        self.replace_iec_ld_text(program_index, offset, expected, replacement)
    }

    /// Delete a terminal IEC function block using the captured XG5000 Delete
    /// shape. This removes its preceding wire and its otherwise empty pin rows,
    /// while retaining the leading contact as the only record in the group.
    pub fn delete_iec_ld_terminal_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_terminal_function_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.block_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|block| block.record_offset == block_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = rows
            .iter()
            .filter(|row| row.group_index == site.group_index)
            .collect::<Vec<_>>();
        let top_row = *group_rows
            .first()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removal_end = group_rows
            .last()
            .map(|row| row.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let contact = records
            .iter()
            .find(|record| {
                record.group_index == site.group_index
                    && record.row_index == site.row_index
                    && matches!(record.kind, IecRecordKind::Contact(_))
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let contact_x = *program
            .data
            .get(contact.offset + 5)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let row_cache = u32::try_from(contact.end - contact.offset + 10)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        let group_header = top_row
            .start
            .checked_sub(10)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = program
            .data
            .get(site.wire_offset..removal_end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_vec();
        let expected_rows = rows.len() - usize::from(site.pin_count);
        let removed_record_count = records
            .iter()
            .filter(|record| {
                record.group_index == site.group_index && record.offset >= site.wire_offset
            })
            .count();
        let expected_records = records.len() - removed_record_count;
        let expected_blocks = blocks.len() - 1;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(site.wire_offset..removal_end) != Some(removed.as_slice())
                || payload.get(group_header + 8..group_header + 10)
                    != Some((u16::from(site.pin_count) + 1).to_le_bytes().as_slice())
                || payload.get(top_row.start + 33..top_row.start + 35)
                    != Some(3u16.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated[group_header + 8..group_header + 10].copy_from_slice(&1u16.to_le_bytes());
            updated[top_row.start + 17..top_row.start + 21]
                .copy_from_slice(&row_cache.to_le_bytes());
            updated[top_row.start + 29] = contact_x;
            updated[top_row.start + 33..top_row.start + 35].copy_from_slice(&1u16.to_le_bytes());
            updated.drain(site.wire_offset..removal_end);

            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|rows| rows.len()) != Some(expected_rows)
                || verified.iec_record_frames().map(|records| records.len())
                    != Some(expected_records)
                || verified.iec_function_blocks().map(|blocks| blocks.len())
                    != Some(expected_blocks)
                || verified.iec_function_references().is_none()
                || verified.iec_function_operand_links().is_none()
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Restore a native two-pin MOVE after a retained terminal contact.
    /// Operand edits run on a clone, so the operation is atomic.
    pub fn insert_iec_ld_terminal_move(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        if input_operand.parse::<u32>().is_err() {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "terminal MOVE requires an integer input literal",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_terminal_function_insertion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.contact_offset == contact_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: contact_offset,
            })?;
        let elevator = program_index == 3 && (1..=5).contains(&site.group_index);
        let lighting = program_index == 0 && site.group_index == 32 && site.row_index == 63;
        if !elevator && !lighting {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        if elevator
            && !output_operand
                .to_ascii_uppercase()
                .strip_prefix("%MW")
                .is_some_and(|number| !number.is_empty() && number.parse::<u32>().is_ok())
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "elevator MOVE requires a writable %MW output",
            });
        }
        if lighting {
            let symbols = self
                .iec_local_symbols()
                .into_iter()
                .nth(program_index)
                .ok_or(XgwxError::UnsupportedLadderLayout)??;
            if !classify_iec_expression(output_operand, &symbols, &program)
                .is_some_and(|expression| expression.writable)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "lighting MOVE requires a writable local or direct output",
                });
            }
        }
        let (mut template, expected_input, expected_output) = if elevator {
            (
                include_bytes!("iec_terminal_move_tail.bin").to_vec(),
                "1",
                "%MW300",
            )
        } else {
            (
                include_bytes!("iec_terminal_move_p0_tail.bin").to_vec(),
                "0",
                "자기유지2",
            )
        };
        if template.len() != if elevator { 401 } else { 399 }
            || template.get(..5) != Some(&[0xff, 0x02, 0, 0, 0])
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        if elevator {
            // The captured L1 and L4 insertions share a record layout. The
            // original L7, L10 and L13 groups confirm these coordinate shifts.
            const TAIL_START: usize = 74;
            let y_shift = (site.group_index as u32 - 1) * 12;
            for (group_offset, base_y) in [
                (0x50, 4u32),
                (0x5a, 4),
                (0x63, 4),
                (0x78, 4),
                (0xc0, 8),
                (0x144, 12),
                (0x165, 8),
                (0x169, 8),
                (0x16d, 8),
                (0x178, 8),
                (0x18d, 4),
                (0x196, 8),
                (0x1c5, 12),
                (0x1c9, 12),
                (0x1cd, 12),
                (0x1d8, 4),
            ] {
                let offset = group_offset - TAIL_START;
                if template.get(offset).copied() != Some(base_y as u8) {
                    return Err(XgwxError::UnsupportedLadderLayout);
                }
                template[offset] = (base_y + y_shift) as u8;
            }
            for (group_offset, base_row, row_delta) in [(0x14f, 2u16, 1u16), (0x1af, 3, 2)] {
                let offset = group_offset - TAIL_START;
                if template.get(offset..offset + 2) != Some(base_row.to_le_bytes().as_slice()) {
                    return Err(XgwxError::UnsupportedLadderLayout);
                }
                template[offset..offset + 2]
                    .copy_from_slice(&(site.row_index + row_delta).to_le_bytes());
            }
        }
        let row = program
            .iec_row_frames()
            .and_then(|rows| {
                rows.into_iter()
                    .find(|row| row.group_index == site.group_index)
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_start = row
            .start
            .checked_sub(10)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let mut group_header = [0u8; 10];
        group_header[..4].copy_from_slice(&(site.group_index as u32).to_le_bytes());
        group_header[8..10].copy_from_slice(&1u16.to_le_bytes());
        let before_rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .len();
        let before_blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .len();
        let mut candidate = self.clone();
        candidate.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data
                || payload.get(group_start..row.start) != Some(group_header.as_slice())
                || payload.get(row.start + 33..row.start + 35)
                    != Some(1u16.to_le_bytes().as_slice())
            {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: contact_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated[group_start + 8..group_start + 10].copy_from_slice(&3u16.to_le_bytes());
            if lighting {
                updated[row.start + 17..row.start + 21].copy_from_slice(&52u32.to_le_bytes());
            }
            updated[row.start + 29] = site.raw_x;
            updated[row.start + 33..row.start + 35].copy_from_slice(&3u16.to_le_bytes());
            updated.splice(
                site.insertion_offset..site.insertion_offset,
                template.iter().copied(),
            );
            Ok(updated)
        })?;
        let block_offset = site.insertion_offset + 19;
        for (expected, replacement, is_output) in [
            (expected_input, input_operand, false),
            (expected_output, output_operand, true),
        ] {
            if expected == replacement {
                continue;
            }
            let inserted = candidate
                .ladder_programs()
                .into_iter()
                .nth(program_index)
                .ok_or(XgwxError::UnsupportedLadderLayout)??;
            let records = inserted
                .iec_record_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let links = inserted
                .iec_function_operand_links()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let text = crate::iec_ld::function_operands(&inserted);
            let operand_offset = links
                .iter()
                .filter(|link| {
                    link.target_record_offset == block_offset && link.is_output == is_output
                })
                .find_map(|link| {
                    let record = records
                        .iter()
                        .find(|record| record.offset == link.record_offset)?;
                    text.iter()
                        .find(|item| item.offset >= record.offset && item.offset < record.end)
                        .filter(|item| item.value == expected)
                        .map(|item| item.offset)
                })
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            candidate.update_iec_ld_function_operand(
                program_index,
                operand_offset,
                expected,
                replacement,
            )?;
        }
        let inserted = candidate
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        if inserted.iec_row_frames().map(|rows| rows.len()) != Some(before_rows + 2)
            || inserted.iec_function_blocks().map(|blocks| blocks.len()) != Some(before_blocks + 1)
            || !inserted
                .iec_function_blocks()
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .iter()
                .any(|block| {
                    block.record_offset == block_offset
                        && block.name.value == "MOVE"
                        && block.row_index == site.row_index
                        && block.raw_x == site.raw_x
                })
            || inserted.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        *self = candidate;
        Ok(())
    }

    /// Delete a standalone IEC function group using the captured XG5000
    /// Delete shape. The group's stored rows become an implicit blank gap and
    /// all later group ordinals are decremented without changing row indices.
    pub fn delete_iec_ld_standalone_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_standalone_function_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.block_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|block| block.record_offset == block_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }

        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = rows
            .iter()
            .filter(|row| row.group_index == site.group_index)
            .collect::<Vec<_>>();
        let first_row = *group_rows
            .first()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removal_end = group_rows
            .last()
            .map(|row| row.end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_start = first_row
            .start
            .checked_sub(10)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_count = u16::from_le_bytes(
            program
                .data
                .get(6..8)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        if group_count <= 1 || usize::from(group_count) <= site.group_index {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let group_starts = (0..usize::from(group_count))
            .map(|group_index| {
                rows.iter()
                    .find(|row| row.group_index == group_index)
                    .and_then(|row| row.start.checked_sub(10))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        for (group_index, &start) in group_starts.iter().enumerate() {
            if program.data.get(start..start + 4)
                != Some(
                    u32::try_from(group_index)
                        .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                        .to_le_bytes()
                        .as_slice(),
                )
                || program.data.get(start + 4..start + 8) != Some([0; 4].as_slice())
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        if group_starts[site.group_index] != group_start
            || program.data.get(group_start + 8..group_start + 10)
                != Some((u16::from(site.pin_count) + 1).to_le_bytes().as_slice())
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }

        let removed = program
            .data
            .get(group_start..removal_end)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_vec();
        let removed_records = records
            .iter()
            .filter(|record| record.group_index == site.group_index)
            .count();
        let expected_rows = rows.len() - group_rows.len();
        let expected_records = records.len() - removed_records;
        let expected_blocks = blocks.len() - 1;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(group_start..removal_end) != Some(removed.as_slice()) {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated[6..8].copy_from_slice(&(group_count - 1).to_le_bytes());
            for (group_index, &start) in group_starts.iter().enumerate().skip(site.group_index + 1)
            {
                updated[start..start + 4].copy_from_slice(
                    &u32::try_from(group_index - 1)
                        .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                        .to_le_bytes(),
                );
            }
            updated.drain(group_start..removal_end);

            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            let verified_rows = verified
                .iec_row_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let row_shapes_match = rows
                .iter()
                .filter(|row| row.group_index != site.group_index)
                .zip(&verified_rows)
                .all(|(before, after)| {
                    after.group_index
                        == before.group_index - usize::from(before.group_index > site.group_index)
                        && after.row_index == before.row_index
                        && after.record_count == before.record_count
                });
            if verified_rows.len() != expected_rows
                || !row_shapes_match
                || verified.iec_record_frames().map(|records| records.len())
                    != Some(expected_records)
                || verified.iec_function_blocks().map(|blocks| blocks.len())
                    != Some(expected_blocks)
                || verified.iec_function_references().is_none()
                || verified.iec_function_operand_links().is_none()
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Remove one complete decoded IEC LD network while leaving its row range
    /// as an implicit blank gap. Later network ordinals are renumbered; their
    /// rows, records, and coordinates remain unchanged.
    pub fn delete_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if program.iec_function_references().is_none()
            || program.iec_function_operand_links().is_none()
            || program.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let group_count = u16::from_le_bytes(
            program
                .data
                .get(6..8)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        if group_count <= 1 || group_index >= usize::from(group_count) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let group_starts = (0..usize::from(group_count))
            .map(|index| {
                rows.iter()
                    .find(|row| row.group_index == index)
                    .and_then(|row| row.start.checked_sub(10))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        for (index, &start) in group_starts.iter().enumerate() {
            if program.data.get(start..start + 4)
                != Some(
                    u32::try_from(index)
                        .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                        .to_le_bytes()
                        .as_slice(),
                )
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        let group_rows = rows
            .iter()
            .filter(|row| row.group_index == group_index)
            .collect::<Vec<_>>();
        let first = group_rows
            .first()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if first.row_index != expected_first_row_index {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: first.start,
            });
        }
        let start = group_starts[group_index];
        let end = group_rows
            .last()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .end;
        if group_starts
            .get(group_index + 1)
            .copied()
            .unwrap_or(program.data.len())
            != end
            || program.data.get(start + 8..start + 10)
                != Some(
                    u16::try_from(group_rows.len())
                        .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                        .to_le_bytes()
                        .as_slice(),
                )
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let removed = program.data[start..end].to_vec();
        let removed_record_count = records
            .iter()
            .filter(|record| record.group_index == group_index)
            .count();
        let removed_block_count = blocks
            .iter()
            .filter(|block| block.group_index == group_index)
            .count();
        let mut updated = program.data.clone();
        updated[6..8].copy_from_slice(&(group_count - 1).to_le_bytes());
        for (index, &offset) in group_starts.iter().enumerate().skip(group_index + 1) {
            updated[offset..offset + 4].copy_from_slice(
                &u32::try_from(index - 1)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
        }
        updated.drain(start..end);
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        let verified_rows = verified
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if verified_rows.len() != rows.len() - group_rows.len()
            || rows
                .iter()
                .filter(|row| row.group_index != group_index)
                .zip(&verified_rows)
                .any(|(before, after)| {
                    after.group_index
                        != before.group_index - usize::from(before.group_index > group_index)
                        || after.row_index != before.row_index
                        || after.record_count != before.record_count
                })
            || verified.iec_record_frames().map(|items| items.len())
                != Some(records.len() - removed_record_count)
            || verified.iec_function_blocks().map(|items| items.len())
                != Some(blocks.len() - removed_block_count)
            || verified.iec_function_references().is_none()
            || verified.iec_function_operand_links().is_none()
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload.get(start..end) != Some(removed.as_slice()) {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: start,
                });
            }
            Ok(updated)
        })
    }

    /// Replace an occupied IEC LD network with a copy of another network in
    /// the same program. Both edits are checked on a private document, so a
    /// failed copy never leaves the destination deleted.
    pub fn replace_iec_ld_group(
        &mut self,
        program_index: usize,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        if source_group_index == destination_group_index {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC network replacement requires distinct source and destination networks",
            });
        }
        let mut candidate = self.clone();
        candidate.delete_iec_ld_group(
            program_index,
            destination_group_index,
            expected_destination_first_row_index,
        )?;
        let remaining_source_index =
            source_group_index - usize::from(destination_group_index < source_group_index);
        candidate.copy_iec_ld_group(
            program_index,
            remaining_source_index,
            expected_source_first_row_index,
            expected_destination_first_row_index,
        )?;
        *self = candidate;
        Ok(())
    }

    /// Replace a destination network with a network from another IEC program.
    /// Missing source locals and captured function instances can be copied in
    /// the same atomic edit when requested.
    #[allow(clippy::too_many_arguments)]
    pub fn replace_iec_ld_group_from_program(
        &mut self,
        source_program_index: usize,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_program_index: usize,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
        copy_missing_locals: bool,
    ) -> Result<(), XgwxError> {
        if source_program_index == destination_program_index {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "cross-program IEC replacement requires distinct programs",
            });
        }
        let mut candidate = self.clone();
        candidate.delete_iec_ld_group(
            destination_program_index,
            destination_group_index,
            expected_destination_first_row_index,
        )?;
        if copy_missing_locals {
            candidate.copy_iec_ld_group_to_program_with_locals(
                source_program_index,
                source_group_index,
                expected_source_first_row_index,
                destination_program_index,
                expected_destination_first_row_index,
            )?;
        } else {
            candidate.copy_iec_ld_group_to_program(
                source_program_index,
                source_group_index,
                expected_source_first_row_index,
                destination_program_index,
                expected_destination_first_row_index,
            )?;
        }
        *self = candidate;
        Ok(())
    }

    /// Move a complete decoded IEC LD network into an empty row range.
    /// Its internal branches, function pins, and operand links move together.
    pub fn move_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.relocate_iec_ld_group(
            program_index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
            false,
        )
    }

    /// Copy a complete decoded IEC LD network into an empty row range,
    /// retaining the original group. The copied operands and function instance
    /// names can be edited separately after insertion.
    pub fn copy_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.relocate_iec_ld_group(
            program_index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
            true,
        )
    }

    /// Copy a complete IEC network into an empty row range in another
    /// program. Referenced local symbols and function instances must already
    /// exist with compatible types in the destination program.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_iec_ld_group_to_program(
        &mut self,
        source_program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        if source_program_index == destination_program_index {
            return self.copy_iec_ld_group(
                source_program_index,
                group_index,
                expected_first_row_index,
                destination_first_row_index,
            );
        }
        let programs = self.ladder_programs();
        let source = programs
            .get(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })?
            .as_ref()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        let destination = programs
            .get(destination_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: destination_program_index,
            })?
            .as_ref()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        if source.project_type != Some(2)
            || destination.project_type != Some(2)
            || source.version.as_deref() != Some("LD VER 1.1")
            || destination.version.as_deref() != Some("LD VER 1.1")
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let source_rows = source
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let source_records = source
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let source_graph = source
            .iec_circuit_graph()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = source_rows
            .iter()
            .filter(|row| row.group_index == group_index)
            .collect::<Vec<_>>();
        let first = group_rows
            .first()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let last = group_rows
            .last()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if first.row_index != expected_first_row_index {
            return Err(XgwxError::LadderCellChanged {
                program_index: source_program_index,
                offset: first.start,
            });
        }
        let source_start = first
            .start
            .checked_sub(10)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_records = source_records
            .iter()
            .filter(|record| record.group_index == group_index)
            .collect::<Vec<_>>();
        let source_strings = extract_utf16_marker_strings(&source.data, false, false);
        if group_records.is_empty()
            || group_records.iter().any(|record| {
                !matches!(
                    record.kind,
                    IecRecordKind::Contact(_)
                        | IecRecordKind::Coil(_)
                        | IecRecordKind::LongWire
                        | IecRecordKind::ShortWire
                        | IecRecordKind::BranchStart
                        | IecRecordKind::BranchEnd
                        | IecRecordKind::Comment
                        | IecRecordKind::FunctionBlock
                        | IecRecordKind::FunctionOperand
                        | IecRecordKind::LinkReference(_)
                )
            })
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "cross-program IEC copy requires a fully decoded network",
            });
        }
        let source_symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })??;
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(destination_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: destination_program_index,
            })??;
        let group_blocks = source
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .filter(|block| block.group_index == group_index)
            .collect::<Vec<_>>();
        for block in &group_blocks {
            if let Some(instance) = &block.instance {
                if !symbols.iter().any(|symbol| {
                    symbol.name.eq_ignore_ascii_case(&instance.value)
                        && symbol.is_instance
                        && symbol.type_reference.as_deref() == Some(block.name.value.as_str())
                }) {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "copied IEC function instance needs a matching destination declaration",
                    });
                }
            }
        }
        for string in source_strings.iter().filter(|string| {
            group_records.iter().any(|record| {
                matches!(
                    record.kind,
                    IecRecordKind::Contact(_)
                        | IecRecordKind::Coil(_)
                        | IecRecordKind::FunctionOperand
                ) && record.offset <= string.offset
                    && string.end_offset <= record.end
            })
        }) {
            if let Some(local) = source_symbols
                .iter()
                .find(|symbol| symbol.name.eq_ignore_ascii_case(&string.value))
            {
                if !symbols.iter().any(|destination_symbol| {
                    destination_symbol.name.eq_ignore_ascii_case(&local.name)
                        && destination_symbol.data_type_code == local.data_type_code
                        && destination_symbol.is_instance == local.is_instance
                        && destination_symbol.type_reference == local.type_reference
                }) {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "copied IEC network refers to a missing or incompatible destination local symbol",
                    });
                }
            }
        }
        for binding in source_graph
            .function_bindings
            .iter()
            .filter(|binding| binding.group_index == group_index)
        {
            let Some(expression_offset) = binding.expression_record_offset else {
                continue;
            };
            let record = group_records
                .iter()
                .find(|record| record.offset == expression_offset)
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let Some(expression) = source_strings
                .iter()
                .find(|string| string.offset >= record.offset && string.end_offset <= record.end)
            else {
                // A captured FF46 output-link record can have no visible
                // expression cell; its pin is already validated by the graph.
                continue;
            };
            let classified = classify_iec_expression(&expression.value, &symbols, destination)
                .ok_or(XgwxError::InvalidLadderEdit {
                    reason: "copied IEC function operand cannot be typed in the destination program",
                })?;
            if classified.data_type_mask & binding.data_type_mask == 0
                || (binding.direction == IecFunctionPinDirection::Output && !classified.writable)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "copied IEC function operand is incompatible with its destination pin",
                });
            }
        }
        let operands = crate::iec_ld::element_operands(source);
        for record in group_records.iter().filter(|record| {
            matches!(
                record.kind,
                IecRecordKind::Contact(_) | IecRecordKind::Coil(_)
            )
        }) {
            let operand = operands
                .iter()
                .find(|operand| operand.string.offset == record.offset + 15)
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let classified = classify_iec_bool_expression(
                &operand.string.value,
                &symbols,
                destination,
            )
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "copied IEC contact or coil operand is not BOOL in the destination program",
            })?;
            if classified.data_type_mask & 1 == 0
                || (matches!(record.kind, IecRecordKind::Coil(_)) && !classified.writable)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "copied IEC coil requires a writable BOOL destination",
                });
            }
        }
        let target_rows = destination
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let target_records = destination
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let graph = destination
            .iec_circuit_graph()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let max_rows = u16::from_le_bytes(
            destination
                .data
                .get(4..6)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        let group_count = usize::from(u16::from_le_bytes(
            destination
                .data
                .get(6..8)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        ));
        let destination_last = destination_first_row_index
            .checked_add(last.row_index - first.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if group_count >= usize::from(max_rows)
            || destination_last >= max_rows
            || target_rows.iter().any(|row| {
                (destination_first_row_index..=destination_last).contains(&row.row_index)
            })
            || graph.occupied_areas.iter().any(|area| {
                area.start_row_index <= destination_last
                    && area.end_row_index >= destination_first_row_index
            })
            || graph.edges.iter().any(|edge| {
                edge.start.row_index.min(edge.end.row_index) <= destination_last
                    && edge.start.row_index.max(edge.end.row_index) >= destination_first_row_index
            })
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "cross-program IEC destination must be an empty row range",
            });
        }
        if source.data.get(source_start..source_start + 4)
            != Some(
                u32::try_from(group_index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes()
                    .as_slice(),
            )
            || source.data.get(source_start + 8..source_start + 10)
                != Some(
                    u16::try_from(group_rows.len())
                        .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                        .to_le_bytes()
                        .as_slice(),
                )
            || source_rows
                .iter()
                .find(|row| row.group_index == group_index + 1)
                .and_then(|row| row.start.checked_sub(10))
                .unwrap_or(source.data.len())
                != last.end
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let mut relocated = source.data[source_start..last.end].to_vec();
        let delta = i32::from(destination_first_row_index) - i32::from(first.row_index);
        for row in &group_rows {
            let offset = row.start - source_start;
            let new_index = i32::from(row.row_index) + delta;
            relocated[offset..offset + 4].copy_from_slice(
                &u32::try_from(new_index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
            translate_iec_y(&mut relocated, offset + 22, delta)?;
            translate_iec_y(&mut relocated, offset + 30, delta)?;
            let secondary_y = u16::from_le_bytes(
                relocated[offset + 26..offset + 28]
                    .try_into()
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
            );
            if !secondary_y.is_multiple_of(4) {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            if (first.row_index..=last.row_index).contains(&(secondary_y / 4)) {
                translate_iec_y(&mut relocated, offset + 26, delta)?;
            }
        }
        for record in group_records {
            let offset = record.offset - source_start;
            match record.kind {
                IecRecordKind::LongWire => {
                    translate_iec_y(&mut relocated, offset + 6, delta)?;
                    translate_iec_y(&mut relocated, offset + 16, delta)?;
                }
                IecRecordKind::BranchStart => {
                    translate_iec_y(&mut relocated, offset + 8, delta)?;
                    translate_iec_y(&mut relocated, offset + 18, delta)?;
                }
                IecRecordKind::Contact(_)
                | IecRecordKind::Coil(_)
                | IecRecordKind::ShortWire
                | IecRecordKind::BranchEnd
                | IecRecordKind::Comment
                | IecRecordKind::FunctionOperand
                | IecRecordKind::LinkReference(_) => {
                    translate_iec_y(&mut relocated, offset + 6, delta)?;
                }
                IecRecordKind::FunctionBlock => {
                    translate_iec_function_block_rows(
                        &mut relocated,
                        offset,
                        record.end - source_start,
                        record.row_index,
                        delta,
                    )?;
                }
            }
        }
        let grouped_target_rows = (0..group_count)
            .map(|index| {
                target_rows
                    .iter()
                    .filter(|row| row.group_index == index)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let starts = grouped_target_rows
            .iter()
            .map(|group| group.first().and_then(|row| row.start.checked_sub(10)))
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if starts.first().copied() != Some(8) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        for index in 0..group_count {
            let group = &grouped_target_rows[index];
            if group.last().map(|row| row.end)
                != Some(
                    starts
                        .get(index + 1)
                        .copied()
                        .unwrap_or(destination.data.len()),
                )
                || destination.data.get(starts[index]..starts[index] + 4)
                    != Some(
                        u32::try_from(index)
                            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                            .to_le_bytes()
                            .as_slice(),
                    )
                || destination.data.get(starts[index] + 8..starts[index] + 10)
                    != Some(
                        u16::try_from(group.len())
                            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                            .to_le_bytes()
                            .as_slice(),
                    )
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        let insertion_index = grouped_target_rows
            .iter()
            .position(|group| group[0].row_index > destination_first_row_index)
            .unwrap_or(group_count);
        let mut groups = (0..group_count)
            .map(|index| {
                let end = starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(destination.data.len());
                destination.data[starts[index]..end].to_vec()
            })
            .collect::<Vec<_>>();
        groups.insert(insertion_index, relocated);
        let mut updated = destination.data[..8].to_vec();
        updated[6..8].copy_from_slice(
            &u16::try_from(group_count + 1)
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                .to_le_bytes(),
        );
        for (index, mut group) in groups.into_iter().enumerate() {
            group[..4].copy_from_slice(
                &u32::try_from(index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
            updated.extend(group);
        }
        let mut verified = destination.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        if verified.iec_row_frames().map(|rows| rows.len())
            != Some(target_rows.len() + group_rows.len())
            || verified.iec_record_frames().map(|records| records.len())
                != Some(
                    target_records.len()
                        + source_records
                            .iter()
                            .filter(|record| record.group_index == group_index)
                            .count(),
                )
            || verified.iec_function_references().is_none()
            || verified.iec_function_operand_links().is_none()
            || verified.iec_function_blocks().map(|blocks| blocks.len())
                != destination
                    .iec_function_blocks()
                    .map(|blocks| blocks.len() + group_blocks.len())
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(destination_program_index, "2", |payload| {
            if payload != destination.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index: destination_program_index,
                    offset: 0,
                });
            }
            Ok(updated)
        })
    }

    /// Copy a network and create its missing primitive locals and captured
    /// function-instance declarations in the destination. Supported BOOL
    /// mappings retain their source address.
    /// A private document makes declaration insertion and network copying atomic.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_iec_ld_group_to_program_with_locals(
        &mut self,
        source_program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        if source_program_index == destination_program_index {
            return self.copy_iec_ld_group(
                source_program_index,
                group_index,
                expected_first_row_index,
                destination_first_row_index,
            );
        }
        let programs = self.ladder_programs();
        let source = programs
            .get(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })?
            .as_ref()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?;
        let rows = source
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group = rows
            .iter()
            .filter(|row| row.group_index == group_index)
            .collect::<Vec<_>>();
        let first = group.first().ok_or(XgwxError::UnsupportedLadderLayout)?;
        group.last().ok_or(XgwxError::UnsupportedLadderLayout)?;
        if first.row_index != expected_first_row_index {
            return Err(XgwxError::LadderCellChanged {
                program_index: source_program_index,
                offset: first.start,
            });
        }
        let source_records = source
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let source_symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(source_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: source_program_index,
            })??;
        let destination_symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(destination_program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: destination_program_index,
            })??;
        let mut needed = std::collections::BTreeMap::<String, IecLocalSymbol>::new();
        let mut needed_instances = std::collections::BTreeMap::<String, String>::new();
        for block in source
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .filter(|block| group.iter().any(|row| row.row_index == block.row_index))
        {
            let Some(instance) = block.instance else {
                continue;
            };
            let symbol = source_symbols
                .iter()
                .find(|symbol| {
                    symbol.name.eq_ignore_ascii_case(&instance.value)
                        && symbol.is_instance
                        && symbol.type_reference.as_deref() == Some(block.name.value.as_str())
                })
                .ok_or(XgwxError::InvalidLadderEdit {
                    reason: "source IEC function instance declaration is missing or mismatched",
                })?;
            if let Some(existing) = destination_symbols
                .iter()
                .find(|item| item.name.eq_ignore_ascii_case(&symbol.name))
            {
                if !existing.is_instance || existing.type_reference != symbol.type_reference {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "destination IEC function instance has a different type",
                    });
                }
            } else {
                needed_instances.insert(symbol.name.to_lowercase(), symbol.name.clone());
            }
        }
        for string in extract_utf16_marker_strings(&source.data, false, false)
            .into_iter()
            .filter(|string| {
                source_records.iter().any(|record| {
                    record.group_index == group_index
                        && matches!(
                            record.kind,
                            IecRecordKind::Contact(_)
                                | IecRecordKind::Coil(_)
                                | IecRecordKind::FunctionOperand
                        )
                        && record.offset <= string.offset
                        && string.end_offset <= record.end
                })
            })
        {
            let Some(symbol) = source_symbols
                .iter()
                .find(|symbol| symbol.name.eq_ignore_ascii_case(&string.value))
            else {
                continue;
            };
            if let Some(existing) = destination_symbols
                .iter()
                .find(|item| item.name.eq_ignore_ascii_case(&symbol.name))
            {
                if existing.data_type_code != symbol.data_type_code
                    || existing.is_instance != symbol.is_instance
                    || existing.type_reference != symbol.type_reference
                    || existing.address != symbol.address
                {
                    return Err(XgwxError::InvalidLadderEdit {
                        reason: "destination IEC local has a different type or mapped address",
                    });
                }
            } else if symbol.is_instance {
                needed_instances.insert(symbol.name.to_lowercase(), symbol.name.clone());
            } else {
                needed.insert(symbol.name.to_lowercase(), symbol.clone());
            }
        }
        let mut updated = self.clone();
        for symbol in needed.values() {
            let data_type = symbol
                .data_type
                .as_deref()
                .ok_or(XgwxError::InvalidLadderEdit {
                    reason: "copied IEC local has an unsupported primitive type",
                })?;
            if symbol.address.is_some() && data_type != "BOOL" {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "copied IEC mapped local must be BOOL",
                });
            }
            updated.insert_iec_local_symbol(
                destination_program_index,
                &symbol.name,
                data_type,
                symbol.description.as_deref().unwrap_or(""),
            )?;
            if let Some(address) = &symbol.address {
                let index = updated
                    .iec_local_symbols()
                    .into_iter()
                    .nth(destination_program_index)
                    .ok_or(XgwxError::ProgramNotFound {
                        index: destination_program_index,
                    })??
                    .iter()
                    .position(|item| item.name.eq_ignore_ascii_case(&symbol.name))
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
                updated.update_iec_local_symbol_address(
                    destination_program_index,
                    index,
                    &symbol.name,
                    "",
                    address,
                )?;
            } else if symbol.storage_class == "A" {
                updated.allocate_inserted_iec_local_symbol(
                    destination_program_index,
                    &symbol.name,
                    symbol.data_type_code,
                    symbol
                        .allocation_width
                        .ok_or(XgwxError::UnsupportedLadderLayout)?,
                )?;
            } else if !symbol.storage_class.is_empty() {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        for instance in needed_instances.values() {
            updated.copy_iec_instance_declaration_to_program(
                source_program_index,
                instance,
                destination_program_index,
                instance,
            )?;
        }
        updated.copy_iec_ld_group_to_program(
            source_program_index,
            group_index,
            expected_first_row_index,
            destination_program_index,
            destination_first_row_index,
        )?;
        *self = updated;
        Ok(())
    }

    /// Create a standalone IEC comment in an empty row by copying a captured
    /// one-record comment group from the same program and replacing its text.
    /// The source group remains untouched; failures leave the document intact.
    pub fn insert_iec_ld_comment(
        &mut self,
        program_index: usize,
        destination_row_index: u16,
        text: &str,
    ) -> Result<(), XgwxError> {
        if text.is_empty() || text.chars().any(char::is_control) {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC comment text must be nonempty and contain no control characters",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let source = rows
            .iter()
            .find(|row| {
                row.record_count == 1
                    && rows
                        .iter()
                        .filter(|other| other.group_index == row.group_index)
                        .count()
                        == 1
                    && records.iter().any(|record| {
                        record.group_index == row.group_index
                            && record.row_index == row.row_index
                            && record.kind == IecRecordKind::Comment
                    })
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let mut edited = self.clone();
        edited.copy_iec_ld_group(
            program_index,
            source.group_index,
            source.row_index,
            destination_row_index,
        )?;
        let copied_program = edited
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)??;
        let copied_record = copied_program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|record| {
                record.row_index == destination_row_index && record.kind == IecRecordKind::Comment
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let copied_text = crate::iec_ld::comments(&copied_program)
            .into_iter()
            .find(|item| item.offset == copied_record.offset + 15)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        edited.update_iec_ld_comment(
            program_index,
            copied_text.offset,
            &copied_text.value,
            text,
        )?;
        *self = edited;
        Ok(())
    }

    fn relocate_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
        keep_source: bool,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let graph = program
            .iec_circuit_graph()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if program.iec_function_references().is_none()
            || program.iec_function_operand_links().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let max_rows = u16::from_le_bytes(
            program
                .data
                .get(4..6)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        let group_count = usize::from(u16::from_le_bytes(
            program
                .data
                .get(6..8)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        ));
        if group_index >= group_count || (keep_source && group_count >= usize::from(max_rows)) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let grouped_rows = (0..group_count)
            .map(|index| {
                rows.iter()
                    .filter(|row| row.group_index == index)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let source_rows = &grouped_rows[group_index];
        let first = source_rows
            .first()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let source_last = source_rows
            .last()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .row_index;
        if first.row_index != expected_first_row_index {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: first.start,
            });
        }
        let destination_last = destination_first_row_index
            .checked_add(source_last - first.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if destination_last >= max_rows
            || !(destination_last < first.row_index || destination_first_row_index > source_last)
            || rows.iter().any(|row| {
                (keep_source || row.group_index != group_index)
                    && (destination_first_row_index..=destination_last).contains(&row.row_index)
            })
            || graph.occupied_areas.iter().any(|area| {
                (keep_source || area.group_index != group_index)
                    && area.start_row_index <= destination_last
                    && area.end_row_index >= destination_first_row_index
            })
            || graph.edges.iter().any(|edge| {
                (keep_source || edge.start.group_index != group_index)
                    && edge.start.row_index.min(edge.end.row_index) <= destination_last
                    && edge.start.row_index.max(edge.end.row_index) >= destination_first_row_index
            })
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC network destination must be a separate empty row range within the program",
            });
        }
        let starts = grouped_rows
            .iter()
            .map(|group| group.first().and_then(|row| row.start.checked_sub(10)))
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if starts[0] != 8 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        for index in 0..group_count {
            let end = grouped_rows[index]
                .last()
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .end;
            if end != starts.get(index + 1).copied().unwrap_or(program.data.len())
                || program.data.get(starts[index]..starts[index] + 4)
                    != Some(
                        u32::try_from(index)
                            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                            .to_le_bytes()
                            .as_slice(),
                    )
                || program.data.get(starts[index] + 8..starts[index] + 10)
                    != Some(
                        u16::try_from(grouped_rows[index].len())
                            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                            .to_le_bytes()
                            .as_slice(),
                    )
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        let delta = i32::from(destination_first_row_index) - i32::from(first.row_index);
        let mut shifted = program.data.clone();
        for row in source_rows {
            let new_index = i32::from(row.row_index) + delta;
            shifted[row.start..row.start + 4].copy_from_slice(
                &u32::try_from(new_index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
            translate_iec_y(&mut shifted, row.start + 22, delta)?;
            translate_iec_y(&mut shifted, row.start + 30, delta)?;
            let secondary_y = u16::from_le_bytes(
                shifted[row.start + 26..row.start + 28]
                    .try_into()
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
            );
            if !secondary_y.is_multiple_of(4) {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            if (first.row_index..=source_last).contains(&(secondary_y / 4)) {
                translate_iec_y(&mut shifted, row.start + 26, delta)?;
            }
        }
        for record in records
            .iter()
            .filter(|record| record.group_index == group_index)
        {
            match record.kind {
                IecRecordKind::LongWire => {
                    translate_iec_y(&mut shifted, record.offset + 6, delta)?;
                    translate_iec_y(&mut shifted, record.offset + 16, delta)?;
                }
                IecRecordKind::BranchStart => {
                    translate_iec_y(&mut shifted, record.offset + 8, delta)?;
                    translate_iec_y(&mut shifted, record.offset + 18, delta)?;
                }
                IecRecordKind::BranchEnd | IecRecordKind::LinkReference(_) => {
                    translate_iec_y(&mut shifted, record.offset + 6, delta)?;
                }
                IecRecordKind::FunctionBlock => {
                    translate_iec_function_block_rows(
                        &mut shifted,
                        record.offset,
                        record.end,
                        record.row_index,
                        delta,
                    )?;
                }
                IecRecordKind::ShortWire
                | IecRecordKind::Contact(_)
                | IecRecordKind::Coil(_)
                | IecRecordKind::Comment
                | IecRecordKind::FunctionOperand => {
                    translate_iec_y(&mut shifted, record.offset + 6, delta)?;
                }
            }
        }
        let mut groups = (0..group_count)
            .map(|index| {
                let end = starts.get(index + 1).copied().unwrap_or(program.data.len());
                program.data[starts[index]..end].to_vec()
            })
            .collect::<Vec<_>>();
        let source_end = starts
            .get(group_index + 1)
            .copied()
            .unwrap_or(program.data.len());
        let relocated = shifted[starts[group_index]..source_end].to_vec();
        if !keep_source {
            groups.remove(group_index);
        }
        let mut expected_groups = grouped_rows
            .iter()
            .enumerate()
            .filter(|(index, _)| keep_source || *index != group_index)
            .map(|(_, group)| group.iter().map(|row| row.row_index).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let new_rows = source_rows
            .iter()
            .map(|row| {
                u16::try_from(i32::from(row.row_index) + delta)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let insertion_index = expected_groups
            .iter()
            .position(|group| group[0] > destination_first_row_index)
            .unwrap_or(expected_groups.len());
        expected_groups.insert(insertion_index, new_rows);
        groups.insert(insertion_index, relocated);
        let mut updated = program.data[..8].to_vec();
        if keep_source {
            updated[6..8].copy_from_slice(
                &u16::try_from(group_count + 1)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
        }
        for (index, mut group) in groups.into_iter().enumerate() {
            group[..4].copy_from_slice(
                &u32::try_from(index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes(),
            );
            updated.extend(group);
        }
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        let verified_rows = verified
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let copied_record_count = records
            .iter()
            .filter(|record| record.group_index == group_index)
            .count();
        let copied_block_count = blocks
            .iter()
            .filter(|block| block.group_index == group_index)
            .count();
        if verified_rows.len() != rows.len() + usize::from(keep_source) * source_rows.len()
            || verified_rows
                .iter()
                .zip(
                    expected_groups
                        .iter()
                        .enumerate()
                        .flat_map(|(index, group)| group.iter().map(move |&row| (index, row))),
                )
                .any(|(row, (group_index, row_index))| {
                    row.group_index != group_index || row.row_index != row_index
                })
            || verified.iec_record_frames().map(|items| items.len())
                != Some(records.len() + usize::from(keep_source) * copied_record_count)
            || verified.iec_function_blocks().map(|items| items.len())
                != Some(blocks.len() + usize::from(keep_source) * copied_block_count)
            || verified.iec_function_references().is_none()
            || verified.iec_function_operand_links().is_none()
            || verified.iec_circuit_graph().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: starts[group_index],
                });
            }
            Ok(updated)
        })
    }

    /// Insert the captured standalone WORD_TO_UDINT shape at a three-row group gap.
    pub fn insert_iec_ld_standalone_function(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        function_name: &str,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        if function_name != "WORD_TO_UDINT" {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "only the captured standalone WORD_TO_UDINT function shape is verified",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let upper_input = input_operand.to_ascii_uppercase();
        let direct_word = ["%MW", "%IW", "%QW"].iter().any(|prefix| {
            upper_input.strip_prefix(prefix).is_some_and(|number| {
                !number.is_empty()
                    && number.bytes().all(|byte| byte.is_ascii_digit())
                    && number.parse::<u32>().is_ok()
            })
        });
        let local_word = symbols.iter().any(|symbol| {
            symbol.name == input_operand
                && !symbol.is_instance
                && symbol.data_type.as_deref() == Some("WORD")
        });
        if !direct_word && !local_word {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "WORD_TO_UDINT input requires a WORD device address or local WORD symbol",
            });
        }
        if symbols
            .iter()
            .filter(|symbol| {
                symbol.name == output_operand
                    && !symbol.is_instance
                    && symbol.storage_class != "I"
                    && symbol.data_type.as_deref() == Some("UDINT")
            })
            .count()
            != 1
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "WORD_TO_UDINT output requires one writable UDINT local symbol",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_standalone_function_insertion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.insertion_offset == insertion_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: insertion_offset,
            })?;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_count = u16::from_le_bytes(
            program
                .data
                .get(6..8)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        let next_count = group_count
            .checked_add(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let following = rows
            .iter()
            .filter(|row| row.group_index >= site.group_index)
            .filter(|row| {
                rows.iter()
                    .find(|candidate| candidate.group_index == row.group_index)
                    .is_some_and(|first| first.start == row.start)
            })
            .map(|row| (row.group_index, row.start - 10))
            .collect::<Vec<_>>();
        if following.len() != usize::from(group_count) - site.group_index
            || following.first().map(|item| item.1) != Some(insertion_offset)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let captured_template = include_bytes!("iec_word_to_udint_group.bin");
        let positioned_template = position_iec_standalone_word_to_udint_group(
            captured_template,
            site.group_index,
            site.row_index,
        )?;
        // Replace the later marker first so the earlier marker keeps its
        // captured offset when the output's UTF-16 length changes.
        let template = replace_iec_ld_text_bytes(
            &positioned_template,
            program_index,
            338,
            "변환",
            output_operand,
        )?;
        let template =
            replace_iec_ld_text_bytes(&template, program_index, 298, "%MW301", input_operand)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: insertion_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated[6..8].copy_from_slice(&next_count.to_le_bytes());
            for &(group_index, start) in &following {
                if updated.get(start..start + 4)
                    != Some((group_index as u32).to_le_bytes().as_slice())
                {
                    return Err(XgwxError::UnsupportedLadderLayout);
                }
                updated[start..start + 4]
                    .copy_from_slice(&((group_index + 1) as u32).to_le_bytes());
            }
            updated.splice(insertion_offset..insertion_offset, template.iter().copied());
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            let verified_rows = verified
                .iec_row_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let new_block_offset = insertion_offset + 64;
            let inserted = verified
                .iec_function_blocks()
                .and_then(|blocks| {
                    blocks
                        .into_iter()
                        .find(|block| block.record_offset == new_block_offset)
                })
                .filter(|block| {
                    block.name.value == function_name
                        && block.row_index == site.row_index
                        && block.group_index == site.group_index
                        && block.raw_x == site.raw_x
                        && block.pin_count == 2
                });
            let inserted_links = verified
                .iec_function_operand_links()
                .ok_or(XgwxError::UnsupportedLadderLayout)?
                .into_iter()
                .filter(|link| link.target_record_offset == new_block_offset)
                .collect::<Vec<_>>();
            let inserted_records = verified
                .iec_record_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let inserted_text = crate::iec_ld::function_operands(&verified);
            let binding_matches = inserted_links.len() == 2
                && inserted_links.iter().all(|link| {
                    inserted_records
                        .iter()
                        .find(|record| record.offset == link.record_offset)
                        .and_then(|record| {
                            inserted_text.iter().find(|item| {
                                item.offset >= record.offset && item.offset < record.end
                            })
                        })
                        .is_some_and(|item| {
                            item.value
                                == if link.is_output {
                                    output_operand
                                } else {
                                    input_operand
                                }
                        })
                });
            if verified_rows.len() != rows.len() + 3
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() + 6)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() + 1)
                || verified.iec_function_references().map(|items| items.len())
                    != Some(references.len() + 2)
                || verified
                    .iec_function_operand_links()
                    .map(|items| items.len())
                    != Some(operands.len() + 2)
                || inserted.is_none()
                || !binding_matches
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Insert the captured connected single-output FF function cell into a
    /// guarded two-row gap. Both stored rows and their surrounding records are
    /// retained; one function record and one output-link record are added.
    pub fn insert_iec_ld_function_cell(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        function_name: &str,
        instance_name: &str,
    ) -> Result<(), XgwxError> {
        if function_name != "FF" {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "only the captured connected FF insertion is verified",
            });
        }
        let symbols = self
            .iec_local_symbols()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        if symbols
            .iter()
            .filter(|symbol| {
                symbol.name == instance_name
                    && symbol.is_instance
                    && symbol.type_reference.as_deref() == Some(function_name)
            })
            .count()
            != 1
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "FF instance name is missing, ambiguous, or has another type",
            });
        }
        let instance_units = instance_name.encode_utf16().collect::<Vec<_>>();
        if instance_units.is_empty() || instance_units.len() > u8::MAX as usize {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "FF instance name must contain 1 to 255 UTF-16 units",
            });
        }
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_function_cell_insertion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.insertion_offset == insertion_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: insertion_offset,
            })?;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let top_row = rows
            .iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let bottom_row = rows
            .iter()
            .find(|row| row.group_index == site.group_index && row.row_index == site.row_index + 1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;

        fn marker(output: &mut Vec<u8>, value: &[u16]) {
            output.extend_from_slice(UTF16_MARKER);
            output.push(value.len() as u8);
            for unit in value {
                output.extend_from_slice(&unit.to_le_bytes());
            }
        }
        let y = site
            .row_index
            .checked_mul(4)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes();
        let pin_y = site
            .row_index
            .checked_add(1)
            .and_then(|row| row.checked_mul(4))
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes();
        let mut block = vec![
            0x00, 0x67, 0x00, 0x00, 0x00, site.raw_x, y[0], y[1], 0x00, 0x01, 0x00, site.raw_x,
            0x00, 0x00, 0x00, 0x21, 0x21, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
            site.raw_x, y[0], y[1], 0x00, 0x01, 0x00, site.raw_x, 0x00, 0x00, 0x00, 0x02, 0x00,
        ];
        marker(&mut block, &[]);
        block.extend_from_slice(&[0x01, 0x00, 0x20, 0x00]);
        marker(&mut block, &[b'C' as u16, b'L' as u16, b'K' as u16]);
        block.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00]);
        marker(&mut block, &[]);
        block.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        marker(&mut block, &[b'Q' as u16]);
        block.extend_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x00]);
        marker(&mut block, &[b'F' as u16, b'F' as u16]);
        marker(&mut block, &instance_units);
        block.extend_from_slice(&[
            site.raw_x, pin_y[0], pin_y[1], 0x00, 0x00, 0x00, site.raw_x, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ]);
        let reference = [0x01, 0x69, 0x00, 0x00, 0x00, site.raw_x, y[0], y[1], 0x00];
        let expected_records = records.len() + 2;
        let expected_blocks = blocks.len() + 1;
        let expected_references = references.len() + 1;

        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: insertion_offset,
                });
            }
            let mut updated = payload.to_vec();
            for row in [top_row, bottom_row] {
                let changed = row
                    .record_count
                    .checked_add(1)
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
                updated[row.start + 33..row.start + 35].copy_from_slice(&changed.to_le_bytes());
            }
            updated.splice(site.reference_offset..site.reference_offset, reference);
            updated.splice(site.insertion_offset..site.insertion_offset, block.clone());

            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            let verified_rows = verified
                .iec_row_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let row_shapes_match = rows.iter().zip(&verified_rows).all(|(before, after)| {
                let added = u16::from(before.group_index == site.group_index);
                after.group_index == before.group_index
                    && after.row_index == before.row_index
                    && after.record_count == before.record_count + added
            });
            let inserted = verified
                .iec_function_blocks()
                .and_then(|items| {
                    items
                        .into_iter()
                        .find(|item| item.record_offset == insertion_offset)
                })
                .filter(|item| {
                    item.name.value == function_name
                        && item.instance.as_ref().map(|value| value.value.as_str())
                            == Some(instance_name)
                        && item.row_index == site.row_index
                        && item.raw_x == site.raw_x
                        && item.pin_count == 1
                });
            if verified_rows.len() != rows.len()
                || !row_shapes_match
                || verified.iec_record_frames().map(|items| items.len()) != Some(expected_records)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(expected_blocks)
                || verified.iec_function_references().map(|items| items.len())
                    != Some(expected_references)
                || verified
                    .iec_function_operand_links()
                    .map(|items| items.len())
                    != Some(operands.len())
                || inserted.is_none()
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete a captured IEC function cell while retaining unrelated records
    /// in the same stored rows. The block and all of its pin-link records are
    /// removed and each affected row's record count is decremented.
    pub fn delete_iec_ld_function_cell(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_function_cell_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.block_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|block| block.record_offset == block_offset)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operand_links = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let linked_offsets = references
            .iter()
            .filter(|reference| reference.target_record_offset == block_offset)
            .map(|reference| reference.record_offset)
            .chain(
                operand_links
                    .iter()
                    .filter(|link| link.target_record_offset == block_offset)
                    .map(|link| link.record_offset),
            )
            .collect::<Vec<_>>();
        let mut removed = records
            .iter()
            .filter(|record| {
                record.offset == block_offset || linked_offsets.contains(&record.offset)
            })
            .map(|record| (record.offset, record.end, record.row_index))
            .collect::<Vec<_>>();
        if removed.len() != 1 + linked_offsets.len()
            || removed
                .iter()
                .any(|(_, _, row_index)| *row_index < site.row_index)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        removed.sort_unstable_by_key(|(start, _, _)| Reverse(*start));
        let expected_records = records.len() - removed.len();
        let expected_blocks = blocks.len() - 1;

        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut updated = payload.to_vec();
            for row in rows
                .iter()
                .filter(|row| row.group_index == site.group_index)
            {
                let removed_count = removed
                    .iter()
                    .filter(|(_, _, row_index)| *row_index == row.row_index)
                    .count();
                if removed_count == 0 {
                    continue;
                }
                let changed = row
                    .record_count
                    .checked_sub(
                        u16::try_from(removed_count)
                            .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
                    )
                    .filter(|count| *count > 0)
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
                updated[row.start + 33..row.start + 35].copy_from_slice(&changed.to_le_bytes());
            }
            for &(start, end, _) in &removed {
                updated.drain(start..end);
            }

            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            let verified_rows = verified
                .iec_row_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let row_shapes_match = rows.iter().zip(&verified_rows).all(|(before, after)| {
                let removed_count = removed
                    .iter()
                    .filter(|(_, _, row_index)| *row_index == before.row_index)
                    .count() as u16;
                after.group_index == before.group_index
                    && after.row_index == before.row_index
                    && after.record_count == before.record_count - removed_count
            });
            if verified_rows.len() != rows.len()
                || !row_shapes_match
                || verified.iec_record_frames().map(|records| records.len())
                    != Some(expected_records)
                || verified.iec_function_blocks().map(|blocks| blocks.len())
                    != Some(expected_blocks)
                || verified.iec_function_references().is_none()
                || verified.iec_function_operand_links().is_none()
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete the first EQ in the captured four-block comparison chain.
    /// XG5000 closes the fourth row of that block and joins the x12 feed to
    /// the next EQ; deleting only the function records leaves a stale branch.
    pub fn delete_iec_ld_eq_chain_head(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        let first = group.first().ok_or(XgwxError::UnsupportedLadderLayout)?;
        let y = first.row_index;
        if expected_name != "EQ"
            || program_index != 0
            || block.group_index != 33
            || group.len() != 16
            || block.row_index != y
            || y != 67
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = group
            .iter()
            .take(4)
            .map(|row| {
                records
                    .iter()
                    .filter(|record| {
                        record.group_index == block.group_index && record.row_index == row.row_index
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.len() != 4
            || leading.iter().map(Vec::len).collect::<Vec<_>>() != [7, 7, 4, 3]
            || leading[0][5].kind != IecRecordKind::LongWire
            || leading[0][6].offset != block_offset
            || leading[1][4].kind != IecRecordKind::FunctionOperand
            || leading[1][5].kind != IecRecordKind::LinkReference(104)
            || leading[1][6].kind != IecRecordKind::FunctionOperand
            || leading[2][2].kind != IecRecordKind::FunctionOperand
            || leading[2][3].kind != IecRecordKind::LinkReference(104)
            || leading[3][2].kind != IecRecordKind::LinkReference(105)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][5],
            leading[0][6],
            leading[1][4],
            leading[1][5],
            leading[1][6],
            leading[2][2],
            leading[2][3],
        ];
        if removed
            .iter()
            .filter(|record| record.kind == IecRecordKind::LinkReference(104))
            .any(|record| {
                !references.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !references.iter().any(|item| {
                item.record_offset == leading[3][2].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let next_end = records
            .iter()
            .find(|record| {
                record.group_index == block.group_index
                    && record.row_index == y + 4
                    && record.kind == IecRecordKind::BranchEnd
            })
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let branch_start = leading[2][1];
        if branch_start.kind != IecRecordKind::BranchStart
            || program.data[branch_start.offset + 7] != 12
            || program.data[next_end.offset + 5] != 12
            || u16::from_le_bytes([program.data[first.start - 2], program.data[first.start - 1]])
                != 16
            || [
                program.data[group[0].start + 17],
                program.data[group[0].start + 29],
                program.data[group[1].start + 29],
                program.data[group[2].start + 26],
                program.data[group[2].start + 29],
                program.data[group[4].start + 17],
                program.data[group[5].start + 17],
                program.data[group[8].start + 17],
            ] != [52, 16, 19, 164, 16, 52, 56, 52]
            || program.data[leading[1][3].offset + 13] != 0
            || program.data[leading[1][3].offset + 23] != 0
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = first.start - 10;
            intermediate[group_header + 8..group_header + 10].copy_from_slice(&15u16.to_le_bytes());
            for (row, count, x) in [(group[0], 5u16, 12u8), (group[1], 4, 12), (group[2], 2, 12)] {
                intermediate[row.start + 33..row.start + 35].copy_from_slice(&count.to_le_bytes());
                intermediate[row.start + 29] = x;
            }
            intermediate[group[0].start + 17] = 39;
            intermediate[group[2].start + 26] = 168;
            intermediate[group[4].start + 17] = 39;
            intermediate[group[5].start + 17] = 39;
            intermediate[group[8].start + 17] = 39;
            intermediate[branch_start.offset + 18..branch_start.offset + 20]
                .copy_from_slice(&((y + 4) * 4).to_le_bytes());
            intermediate[next_end.offset + 6..next_end.offset + 8]
                .copy_from_slice(&((y + 2) * 4).to_le_bytes());
            intermediate[leading[1][3].offset + 13] = 4;
            intermediate[leading[1][3].offset + 23] = 4;
            let removed_row = group[3];
            intermediate.drain(removed_row.start..removed_row.end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, y + 3)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 10)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete the first comparison block in the captured heating/cooling
    /// chain. Native Delete Line on its first pin row removes the four-row
    /// block body, joins the two long branch feeds, and shifts later rows.
    pub fn delete_iec_ld_heating_chain_head(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        if program_index != 6
            || block.group_index != 13
            || block.row_index != 46
            || expected_name != "EQ"
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
            || group
                .iter()
                .take(5)
                .map(|row| row.row_index)
                .collect::<Vec<_>>()
                != [46, 47, 48, 49, 50]
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = group
            .iter()
            .take(5)
            .map(|row| {
                records
                    .iter()
                    .filter(|record| {
                        record.group_index == block.group_index && record.row_index == row.row_index
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.iter().take(4).map(Vec::len).collect::<Vec<_>>() != [11, 11, 6, 5]
            || leading[4].len() < 4
            || leading[0][10].offset != block_offset
            || leading[0][9].kind != IecRecordKind::LongWire
            || leading[1][8].kind != IecRecordKind::FunctionOperand
            || leading[1][9].kind != IecRecordKind::LinkReference(104)
            || leading[1][10].kind != IecRecordKind::FunctionOperand
            || leading[2][4].kind != IecRecordKind::FunctionOperand
            || leading[2][5].kind != IecRecordKind::LinkReference(104)
            || leading[3][4].kind != IecRecordKind::LinkReference(105)
            || ![
                (0, 1),
                (0, 3),
                (0, 6),
                (0, 8),
                (1, 1),
                (1, 7),
                (2, 1),
                (2, 3),
                (4, 1),
                (4, 3),
            ]
            .iter()
            .all(|&(row, index)| leading[row][index].kind == IecRecordKind::BranchStart)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][3],
            leading[0][6],
            leading[0][9],
            leading[0][10],
            leading[1][2],
            leading[1][3],
            leading[1][4],
            leading[1][5],
            leading[1][8],
            leading[1][9],
            leading[1][10],
            leading[2][4],
            leading[2][5],
        ];
        if removed
            .iter()
            .filter(|record| matches!(record.kind, IecRecordKind::LinkReference(_)))
            .any(|record| {
                !references.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !references.iter().any(|item| {
                item.record_offset == leading[3][4].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        for (row, start_index, end_index, x) in [(2usize, 1usize, 0usize, 3u8), (2, 3, 2, 15)] {
            let start = leading[row][start_index];
            let end = leading[4][end_index];
            if program.data[start.offset + 7] != x
                || program.data[end.offset + 5] != x
                || program.data[start.offset + 18..start.offset + 20] != (49u16 * 4).to_le_bytes()
                || program.data[end.offset + 6..end.offset + 8] != (49u16 * 4).to_le_bytes()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = group[0].start - 10;
            let remaining_rows =
                u16::try_from(group.len() - 1).map_err(|_| XgwxError::UnsupportedLadderLayout)?;
            intermediate[group_header + 8..group_header + 10]
                .copy_from_slice(&remaining_rows.to_le_bytes());
            for (row, count, rightmost, header_kind) in [
                (group[0], 7u16, 15u8, 39u8),
                (group[1], 4, 15, 39),
                (group[2], 4, 15, 39),
            ] {
                intermediate[row.start + 33..row.start + 35].copy_from_slice(&count.to_le_bytes());
                intermediate[row.start + 29] = rightmost;
                intermediate[row.start + 17] = header_kind;
            }
            for row in [group[1], group[2]] {
                let old_y = u16::from_le_bytes([
                    intermediate[row.start + 26],
                    intermediate[row.start + 27],
                ]);
                intermediate[row.start + 26..row.start + 28]
                    .copy_from_slice(&(old_y + 4).to_le_bytes());
            }
            for start in [leading[2][1], leading[2][3]] {
                intermediate[start.offset + 18..start.offset + 20]
                    .copy_from_slice(&(50u16 * 4).to_le_bytes());
            }
            for end in [leading[4][0], leading[4][2]] {
                intermediate[end.offset + 6..end.offset + 8]
                    .copy_from_slice(&(48u16 * 4).to_le_bytes());
            }
            intermediate.drain(group[3].start..group[3].end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, 49)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 18)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete one of the two captured middle EQ blocks in the heating chain.
    /// Native Delete Line on its first pin row keeps the x3/x15 branch feeds.
    pub fn delete_iec_ld_heating_chain_middle(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        let y = block.row_index;
        if program_index != 6
            || block.group_index != 13
            || expected_name != "EQ"
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = (0..5)
            .map(|delta| {
                records
                    .iter()
                    .filter(|record| {
                        record.group_index == block.group_index && record.row_index == y + delta
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.iter().take(4).map(Vec::len).collect::<Vec<_>>() != [6, 7, 6, 5]
            || leading[4].len() < 3
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        if leading[0][5].offset != block_offset
            || leading[0][4].kind != IecRecordKind::LongWire
            || leading[1][4].kind != IecRecordKind::FunctionOperand
            || leading[1][5].kind != IecRecordKind::LinkReference(104)
            || leading[1][6].kind != IecRecordKind::FunctionOperand
            || leading[2][4].kind != IecRecordKind::FunctionOperand
            || leading[2][5].kind != IecRecordKind::LinkReference(104)
            || leading[3][4].kind != IecRecordKind::LinkReference(105)
            || !(0..4).all(|row| {
                leading[row][0].kind == IecRecordKind::BranchEnd
                    && leading[row][1].kind == IecRecordKind::BranchStart
                    && leading[row][2].kind == IecRecordKind::BranchEnd
                    && leading[row][3].kind == IecRecordKind::BranchStart
                    && program.data[leading[row][0].offset + 5] == 3
                    && program.data[leading[row][2].offset + 5] == 15
            })
            || leading[4][0].kind != IecRecordKind::BranchEnd
            || leading[4][1].kind != IecRecordKind::BranchStart
            || leading[4][2].kind != IecRecordKind::BranchEnd
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][4],
            leading[0][5],
            leading[1][4],
            leading[1][5],
            leading[1][6],
            leading[2][4],
            leading[2][5],
        ];
        if removed
            .iter()
            .filter(|record| matches!(record.kind, IecRecordKind::LinkReference(_)))
            .any(|record| {
                !references.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !references.iter().any(|item| {
                item.record_offset == leading[3][4].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        for (start_index, end_index, x) in [(1usize, 0usize, 3u8), (3, 2, 15)] {
            let start = leading[2][start_index];
            let end = leading[4][end_index];
            if program.data[start.offset + 7] != x
                || program.data[end.offset + 5] != x
                || program.data[start.offset + 18..start.offset + 20] != ((y + 3) * 4).to_le_bytes()
                || program.data[end.offset + 6..end.offset + 8] != ((y + 3) * 4).to_le_bytes()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
        }
        let selected_rows = (0..4)
            .map(|delta| group.iter().find(|row| row.row_index == y + delta).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = group[0].start - 10;
            let remaining_rows =
                u16::try_from(group.len() - 1).map_err(|_| XgwxError::UnsupportedLadderLayout)?;
            intermediate[group_header + 8..group_header + 10]
                .copy_from_slice(&remaining_rows.to_le_bytes());
            for (index, row) in selected_rows.iter().take(3).enumerate() {
                intermediate[row.start + 33..row.start + 35].copy_from_slice(&4u16.to_le_bytes());
                intermediate[row.start + 29] = 15;
                intermediate[row.start + 17] = 39;
                if index > 0 {
                    let old_y = u16::from_le_bytes([
                        intermediate[row.start + 26],
                        intermediate[row.start + 27],
                    ]);
                    intermediate[row.start + 26..row.start + 28]
                        .copy_from_slice(&(old_y + 4).to_le_bytes());
                }
            }
            for start in [leading[2][1], leading[2][3]] {
                intermediate[start.offset + 18..start.offset + 20]
                    .copy_from_slice(&((y + 4) * 4).to_le_bytes());
            }
            for end in [leading[4][0], leading[4][2]] {
                intermediate[end.offset + 6..end.offset + 8]
                    .copy_from_slice(&((y + 2) * 4).to_le_bytes());
            }
            intermediate.drain(selected_rows[3].start..selected_rows[3].end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, y + 3)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 12)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete an x15-fed EQ in the captured heating comparison chain by its
    /// first pin row. The shape guard also accepts the next comparison after
    /// an earlier deletion has shifted its row index.
    pub fn delete_iec_ld_heating_chain_x15_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let y = block.row_index;
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        if program_index != 6
            || block.group_index != 13
            || expected_name != "EQ"
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = (0..5)
            .map(|delta| {
                records
                    .iter()
                    .filter(|record| record.group_index == 13 && record.row_index == y + delta)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.iter().take(4).map(Vec::len).collect::<Vec<_>>() != [4, 5, 4, 3]
            || leading[4].is_empty()
            || leading[0][3].offset != block_offset
            || leading[0][2].kind != IecRecordKind::LongWire
            || leading[1][2].kind != IecRecordKind::FunctionOperand
            || leading[1][3].kind != IecRecordKind::LinkReference(104)
            || leading[1][4].kind != IecRecordKind::FunctionOperand
            || leading[2][2].kind != IecRecordKind::FunctionOperand
            || leading[2][3].kind != IecRecordKind::LinkReference(104)
            || leading[3][2].kind != IecRecordKind::LinkReference(105)
            || !(0..4).all(|row| {
                leading[row][0].kind == IecRecordKind::BranchEnd
                    && leading[row][1].kind == IecRecordKind::BranchStart
                    && program.data[leading[row][0].offset + 5] == 15
                    && program.data[leading[row][1].offset + 7] == 15
            })
            || leading[4][0].kind != IecRecordKind::BranchEnd
            || program.data[leading[4][0].offset + 5] != 15
            || program.data[leading[2][1].offset + 18..leading[2][1].offset + 20]
                != ((y + 3) * 4).to_le_bytes()
            || program.data[leading[4][0].offset + 6..leading[4][0].offset + 8]
                != ((y + 3) * 4).to_le_bytes()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let refs = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][2],
            leading[0][3],
            leading[1][2],
            leading[1][3],
            leading[1][4],
            leading[2][2],
            leading[2][3],
        ];
        if removed
            .iter()
            .filter(|record| matches!(record.kind, IecRecordKind::LinkReference(_)))
            .any(|record| {
                !refs.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !refs.iter().any(|item| {
                item.record_offset == leading[3][2].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let selected = (0..4)
            .map(|delta| group.iter().find(|row| row.row_index == y + delta).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = group[0].start - 10;
            let remaining_rows =
                u16::try_from(group.len() - 1).map_err(|_| XgwxError::UnsupportedLadderLayout)?;
            intermediate[group_header + 8..group_header + 10]
                .copy_from_slice(&remaining_rows.to_le_bytes());
            for (index, row) in selected.iter().take(3).enumerate() {
                intermediate[row.start + 33..row.start + 35].copy_from_slice(&2u16.to_le_bytes());
                intermediate[row.start + 29] = 15;
                intermediate[row.start + 17] = 39;
                if index > 0 {
                    let old_y = u16::from_le_bytes([
                        intermediate[row.start + 26],
                        intermediate[row.start + 27],
                    ]);
                    intermediate[row.start + 26..row.start + 28]
                        .copy_from_slice(&(old_y + 4).to_le_bytes());
                }
            }
            intermediate[leading[2][1].offset + 18..leading[2][1].offset + 20]
                .copy_from_slice(&((y + 4) * 4).to_le_bytes());
            intermediate[leading[4][0].offset + 6..leading[4][0].offset + 8]
                .copy_from_slice(&((y + 2) * 4).to_le_bytes());
            intermediate.drain(selected[3].start..selected[3].end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, y + 3)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 10)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete the captured contact-fed L62 EQ by its first pin row.
    /// XG5000 removes its x6/x12 contact branches and retains the x15 feed.
    pub fn delete_iec_ld_heating_chain_contact_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        let y = block.row_index;
        if program_index != 6
            || block.group_index != 13
            || expected_name != "EQ"
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = (0..5)
            .map(|delta| {
                records
                    .iter()
                    .filter(|record| record.group_index == 13 && record.row_index == y + delta)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.iter().take(4).map(Vec::len).collect::<Vec<_>>() != [10, 9, 4, 3]
            || leading[4].is_empty()
            || leading[0][9].offset != block_offset
            || leading[0][0].kind != IecRecordKind::BranchEnd
            || leading[0][1].kind != IecRecordKind::ShortWire
            || leading[0][2].kind != IecRecordKind::BranchStart
            || leading[0][3].kind != IecRecordKind::Contact(6)
            || leading[0][4].kind != IecRecordKind::Contact(7)
            || leading[0][5].kind != IecRecordKind::BranchStart
            || leading[0][6].kind != IecRecordKind::Contact(7)
            || leading[0][7].kind != IecRecordKind::BranchStart
            || leading[0][8].kind != IecRecordKind::LongWire
            || leading[1][0].kind != IecRecordKind::BranchEnd
            || leading[1][1].kind != IecRecordKind::Contact(6)
            || leading[1][2].kind != IecRecordKind::ShortWire
            || leading[1][3].kind != IecRecordKind::BranchEnd
            || leading[1][4].kind != IecRecordKind::BranchEnd
            || leading[1][5].kind != IecRecordKind::BranchStart
            || leading[1][6].kind != IecRecordKind::FunctionOperand
            || leading[1][7].kind != IecRecordKind::LinkReference(104)
            || leading[1][8].kind != IecRecordKind::FunctionOperand
            || leading[2][0].kind != IecRecordKind::BranchEnd
            || leading[2][1].kind != IecRecordKind::BranchStart
            || leading[2][2].kind != IecRecordKind::FunctionOperand
            || leading[2][3].kind != IecRecordKind::LinkReference(104)
            || leading[3][0].kind != IecRecordKind::BranchEnd
            || leading[3][1].kind != IecRecordKind::BranchStart
            || leading[3][2].kind != IecRecordKind::LinkReference(105)
            || leading[4][0].kind != IecRecordKind::BranchEnd
            || program.data[leading[0][2].offset + 7] != 6
            || program.data[leading[0][5].offset + 7] != 12
            || program.data[leading[0][7].offset + 7] != 15
            || program.data[leading[1][0].offset + 5] != 6
            || program.data[leading[1][3].offset + 5] != 12
            || program.data[leading[1][4].offset + 5] != 15
            || program.data[leading[4][0].offset + 5] != 15
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let refs = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][2],
            leading[0][5],
            leading[0][8],
            leading[0][9],
            leading[1][0],
            leading[1][1],
            leading[1][2],
            leading[1][3],
            leading[1][6],
            leading[1][7],
            leading[1][8],
            leading[2][2],
            leading[2][3],
        ];
        if removed
            .iter()
            .filter(|record| matches!(record.kind, IecRecordKind::LinkReference(_)))
            .any(|record| {
                !refs.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !refs.iter().any(|item| {
                item.record_offset == leading[3][2].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let selected = (0..4)
            .map(|delta| group.iter().find(|row| row.row_index == y + delta).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = group[0].start - 10;
            let remaining_rows =
                u16::try_from(group.len() - 1).map_err(|_| XgwxError::UnsupportedLadderLayout)?;
            intermediate[group_header + 8..group_header + 10]
                .copy_from_slice(&remaining_rows.to_le_bytes());
            for (index, count) in [(0usize, 6u16), (1, 2), (2, 2)] {
                let row = selected[index];
                intermediate[row.start + 33..row.start + 35].copy_from_slice(&count.to_le_bytes());
                intermediate[row.start + 29] = 15;
                intermediate[row.start + 17] = 39;
                if index > 0 {
                    let old_y = u16::from_le_bytes([
                        intermediate[row.start + 26],
                        intermediate[row.start + 27],
                    ]);
                    intermediate[row.start + 26..row.start + 28]
                        .copy_from_slice(&(old_y + 4).to_le_bytes());
                }
            }
            intermediate[leading[2][1].offset + 18..leading[2][1].offset + 20]
                .copy_from_slice(&((y + 4) * 4).to_le_bytes());
            intermediate[leading[4][0].offset + 6..leading[4][0].offset + 8]
                .copy_from_slice(&((y + 2) * 4).to_le_bytes());
            intermediate.drain(selected[3].start..selected[3].end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, y + 3)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 16)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Delete the x3-fed EQ and its orphan x15 feed as one valid edit.
    pub fn delete_iec_ld_heating_chain_x3_eq_repaired(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let mut edited = self.clone();
        edited.delete_iec_ld_heating_chain_x3_eq(program_index, block_offset, expected_name)?;
        edited.repair_iec_ld_heating_chain_x3_eq_deletion(program_index)?;
        *self = edited;
        Ok(())
    }

    /// Reproduce XG5000 Delete Line on the L59 pin row of the L58 EQ.
    /// This native edit leaves one input/output error until the circuit is repaired.
    pub fn delete_iec_ld_heating_chain_x3_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let blocks = program
            .iec_function_blocks()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let block = blocks
            .iter()
            .find(|item| item.record_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if block.name.value != expected_name {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let y = block.row_index;
        let group = rows
            .iter()
            .filter(|row| row.group_index == block.group_index)
            .collect::<Vec<_>>();
        if program_index != 6
            || block.group_index != 13
            || !(55..=58).contains(&y)
            || expected_name != "EQ"
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let leading = (0..5)
            .map(|delta| {
                records
                    .iter()
                    .filter(|record| {
                        record.group_index == block.group_index && record.row_index == y + delta
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if leading.iter().take(4).map(Vec::len).collect::<Vec<_>>() != [5, 5, 4, 3]
            || leading[4].is_empty()
            || leading[0][4].offset != block_offset
            || leading[0][0].kind != IecRecordKind::BranchEnd
            || leading[0][1].kind != IecRecordKind::BranchStart
            || leading[0][2].kind != IecRecordKind::BranchEnd
            || leading[0][3].kind != IecRecordKind::LongWire
            || leading[1][0].kind != IecRecordKind::BranchEnd
            || leading[1][1].kind != IecRecordKind::BranchStart
            || leading[1][2].kind != IecRecordKind::FunctionOperand
            || leading[1][3].kind != IecRecordKind::LinkReference(104)
            || leading[1][4].kind != IecRecordKind::FunctionOperand
            || leading[2][0].kind != IecRecordKind::BranchEnd
            || leading[2][1].kind != IecRecordKind::BranchStart
            || leading[2][2].kind != IecRecordKind::FunctionOperand
            || leading[2][3].kind != IecRecordKind::LinkReference(104)
            || leading[3][0].kind != IecRecordKind::BranchEnd
            || leading[3][1].kind != IecRecordKind::BranchStart
            || leading[3][2].kind != IecRecordKind::LinkReference(105)
            || leading[4][0].kind != IecRecordKind::BranchEnd
            || program.data[leading[4][0].offset + 5] != 3
            || program.data[leading[2][1].offset + 7] != 3
            || program.data[leading[2][1].offset + 18..leading[2][1].offset + 20]
                != ((y + 3) * 4).to_le_bytes()
            || program.data[leading[4][0].offset + 6..leading[4][0].offset + 8]
                != ((y + 3) * 4).to_le_bytes()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let removed = [
            leading[0][3],
            leading[0][4],
            leading[1][2],
            leading[1][3],
            leading[1][4],
            leading[2][2],
            leading[2][3],
        ];
        if removed
            .iter()
            .filter(|record| matches!(record.kind, IecRecordKind::LinkReference(_)))
            .any(|record| {
                !references.iter().any(|item| {
                    item.record_offset == record.offset && item.target_record_offset == block_offset
                })
            })
            || removed
                .iter()
                .filter(|record| record.kind == IecRecordKind::FunctionOperand)
                .any(|record| {
                    !operands.iter().any(|item| {
                        item.record_offset == record.offset
                            && item.target_record_offset == block_offset
                    })
                })
            || !references.iter().any(|item| {
                item.record_offset == leading[3][2].offset
                    && item.target_record_offset == block_offset
            })
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let selected_rows = (0..4)
            .map(|delta| group.iter().find(|row| row.row_index == y + delta).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if selected_rows.len() != 4
            || selected_rows
                .iter()
                .enumerate()
                .any(|(index, row)| row.row_index != y + index as u16)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload_allow_invalid(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut intermediate = payload.to_vec();
            let group_header = group[0].start - 10;
            intermediate[group_header + 8..group_header + 10]
                .copy_from_slice(&((group.len() - 1) as u16).to_le_bytes());
            for (index, row) in selected_rows.iter().take(3).enumerate() {
                intermediate[row.start + 33..row.start + 35]
                    .copy_from_slice(&(if index == 0 { 3u16 } else { 2u16 }).to_le_bytes());
                intermediate[row.start + 29] = if index == 0 { 14 } else { 3 };
                intermediate[row.start + 17] = 39;
                if index > 0 {
                    let old_y = u16::from_le_bytes([
                        intermediate[row.start + 26],
                        intermediate[row.start + 27],
                    ]);
                    intermediate[row.start + 26..row.start + 28]
                        .copy_from_slice(&(old_y + 4).to_le_bytes());
                }
            }
            intermediate[leading[2][1].offset + 18..leading[2][1].offset + 20]
                .copy_from_slice(&((y + 4) * 4).to_le_bytes());
            intermediate[leading[4][0].offset + 6..leading[4][0].offset + 8]
                .copy_from_slice(&((y + 2) * 4).to_le_bytes());
            intermediate.drain(selected_rows[3].start..selected_rows[3].end);
            for record in removed.iter().rev() {
                intermediate.drain(record.offset..record.end);
            }
            let mut temporary = program.clone();
            temporary.decoded_len = intermediate.len();
            temporary.data = intermediate;
            let updated = delete_iec_ld_blank_row_bytes(&temporary, y + 3)?;
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
                || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 10)
                || verified.iec_function_blocks().map(|items| items.len()) != Some(blocks.len() - 1)
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Remove the four dangling x15 feed segments after the captured L59
    /// Delete Line, restoring a valid circuit while retaining the x3 feed.
    pub fn repair_iec_ld_heating_chain_x3_eq_deletion(
        &mut self,
        program_index: usize,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group = rows
            .iter()
            .filter(|row| row.group_index == 13)
            .collect::<Vec<_>>();
        if program_index != 6
            || group.len() < 5
            || group.first().is_none_or(|row| row.row_index != 46)
            || program.iec_circuit_graph().is_some()
            || program.iec_function_blocks().is_none()
            || program.iec_function_references().is_none()
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let candidates = (55..=58u16)
            .filter_map(|last_y| {
                let first_y = last_y - 4;
                let selected_rows = (first_y..=last_y)
                    .map(|y| group.iter().find(|row| row.row_index == y).copied())
                    .collect::<Option<Vec<_>>>()?;
                let leading = (first_y..=last_y)
                    .map(|y| {
                        records
                            .iter()
                            .filter(|record| record.group_index == 13 && record.row_index == y)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                if leading.iter().take(4).any(|row| row.len() < 4)
                    || leading[4].len() != 3
                    || leading[4][2].kind != IecRecordKind::BranchEnd
                    || program.data[leading[4][2].offset + 5] != 15
                    || (0..4).any(|index| {
                        let start = leading[index][3];
                        let end = leading[index + 1][2];
                        start.kind != IecRecordKind::BranchStart
                            || end.kind != IecRecordKind::BranchEnd
                            || program.data[start.offset + 7] != 15
                            || program.data[end.offset + 5] != 15
                            || program.data[start.offset + 18..start.offset + 20]
                                != ((first_y + index as u16 + 1) * 4).to_le_bytes()
                            || program.data[end.offset + 6..end.offset + 8]
                                != ((first_y + index as u16) * 4).to_le_bytes()
                    })
                {
                    return None;
                }
                Some((selected_rows, leading))
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let (selected_rows, _) = &candidates[0];
        let last_y = selected_rows[4].row_index;
        let mut pairs = Vec::new();
        for y in (46..last_y).rev() {
            let starts = records
                .iter()
                .filter(|record| {
                    record.group_index == 13
                        && record.row_index == y
                        && record.kind == IecRecordKind::BranchStart
                        && program.data[record.offset + 7] == 15
                        && program.data[record.offset + 18..record.offset + 20]
                            == ((y + 1) * 4).to_le_bytes()
                })
                .collect::<Vec<_>>();
            let ends = records
                .iter()
                .filter(|record| {
                    record.group_index == 13
                        && record.row_index == y + 1
                        && record.kind == IecRecordKind::BranchEnd
                        && program.data[record.offset + 5] == 15
                        && program.data[record.offset + 6..record.offset + 8]
                            == (y * 4).to_le_bytes()
                })
                .collect::<Vec<_>>();
            if starts.len() != 1 || ends.len() != 1 {
                break;
            }
            pairs.push((*starts[0], *ends[0]));
        }
        if pairs.len() < 4 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        self.edit_program_payload_repair(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            for pair_count in 4..=pairs.len() {
                let mut removed = pairs[..pair_count]
                    .iter()
                    .flat_map(|(start, end)| [*start, *end])
                    .collect::<Vec<_>>();
                removed.sort_by_key(|record| record.offset);
                let mut updated = payload.to_vec();
                for row in group.iter() {
                    let removed_count = removed
                        .iter()
                        .filter(|record| record.row_index == row.row_index)
                        .count();
                    if removed_count > 0 {
                        let count = usize::from(row.record_count) - removed_count;
                        updated[row.start + 33..row.start + 35]
                            .copy_from_slice(&(count as u16).to_le_bytes());
                    }
                }
                updated[selected_rows[4].start + 29] = 3;
                for record in removed.iter().rev() {
                    updated.drain(record.offset..record.end);
                }
                let mut verified = program.clone();
                verified.decoded_len = updated.len();
                verified.data = updated.clone();
                if verified.iec_row_frames().map(|items| items.len()) == Some(rows.len())
                    && verified.iec_record_frames().map(|items| items.len())
                        == Some(records.len() - removed.len())
                    && verified.iec_circuit_graph().is_some()
                {
                    return Ok(updated);
                }
            }
            Err(XgwxError::UnsupportedLadderLayout)
        })
    }

    /// Delete a captured connected ADD/SUB in a five-row R_TRIG/arithmetic/MOVE group.
    /// XG5000 retains the R_TRIG and MOVE circuits as two separate groups.
    pub fn delete_iec_ld_connected_arithmetic(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let site = program
            .iec_connected_arithmetic_deletion_sites()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .into_iter()
            .find(|site| site.block_offset == block_offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset: block_offset,
            })?;
        if program
            .iec_function_blocks()
            .and_then(|blocks| {
                blocks
                    .into_iter()
                    .find(|block| block.record_offset == block_offset)
            })
            .is_none_or(|block| block.name.value != expected_name)
        {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset: block_offset,
            });
        }
        let rows = program
            .iec_row_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let records = program
            .iec_record_frames()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let references = program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let operands = program
            .iec_function_operand_links()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let group_rows = rows
            .iter()
            .filter(|row| row.group_index == site.group_index)
            .collect::<Vec<_>>();
        let mut removed = records
            .iter()
            .filter(|record| {
                record.offset == site.wire_offset
                    || record.offset == site.block_offset
                    || references.iter().any(|item| {
                        item.target_record_offset == block_offset
                            && item.record_offset == record.offset
                    })
                    || operands.iter().any(|item| {
                        item.target_record_offset == block_offset
                            && item.record_offset == record.offset
                    })
            })
            .map(|record| (record.offset, record.end, record.row_index))
            .collect::<Vec<_>>();
        if removed.len() != 8 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        removed.sort_unstable_by_key(|(start, _, _)| Reverse(*start));
        let group_count = u16::from_le_bytes(
            program.data[6..8]
                .try_into()
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
        );
        let next_group_count = group_count
            .checked_add(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let later_headers = rows
            .iter()
            .filter(|row| row.group_index > site.group_index)
            .filter(|row| {
                rows.iter()
                    .find(|candidate| candidate.group_index == row.group_index)
                    .is_some_and(|first| first.start == row.start)
            })
            .map(|row| (row.group_index, row.start - 10))
            .collect::<Vec<_>>();
        let split_at = group_rows[2].start;
        let removed_before_split = removed
            .iter()
            .filter(|(_, end, _)| *end <= split_at)
            .map(|(start, end, _)| end - start)
            .sum::<usize>();
        self.edit_program_payload(program_index, "2", |payload| {
            if payload != program.data {
                return Err(XgwxError::LadderCellChanged {
                    program_index,
                    offset: block_offset,
                });
            }
            let mut updated = payload.to_vec();
            updated[6..8].copy_from_slice(&next_group_count.to_le_bytes());
            let group_header = group_rows[0].start - 10;
            if updated[group_header + 8..group_header + 10] != 5u16.to_le_bytes()
                || group_rows
                    .iter()
                    .take(4)
                    .map(|row| updated[row.start + 29])
                    .collect::<Vec<_>>()
                    != [16, 19, 16, 16]
                || group_rows
                    .iter()
                    .take(4)
                    .map(|row| updated[row.start + 17])
                    .collect::<Vec<_>>()
                    != [52, 39, 52, 39]
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            updated[group_header + 8..group_header + 10].copy_from_slice(&2u16.to_le_bytes());
            for &(group_index, start) in &later_headers {
                if updated[start..start + 4] != (group_index as u32).to_le_bytes() {
                    return Err(XgwxError::UnsupportedLadderLayout);
                }
                updated[start..start + 4]
                    .copy_from_slice(&((group_index + 1) as u32).to_le_bytes());
            }
            for (index, row) in group_rows.iter().enumerate().take(4) {
                let deleted = removed
                    .iter()
                    .filter(|(_, _, row_index)| *row_index == row.row_index)
                    .count() as u16;
                let count = row
                    .record_count
                    .checked_sub(deleted)
                    .filter(|count| *count > 0)
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
                updated[row.start + 33..row.start + 35].copy_from_slice(&count.to_le_bytes());
                updated[row.start + 29] = [7, 7, 4, 7][index];
                if index == 0 {
                    updated[row.start + 17] = 50;
                }
                if index == 2 {
                    updated[row.start + 17] = 39;
                }
            }
            for &(start, end, _) in &removed {
                updated.drain(start..end);
            }
            let mut new_header = [0u8; 10];
            new_header[..4].copy_from_slice(&((site.group_index + 1) as u32).to_le_bytes());
            new_header[8..10].copy_from_slice(&3u16.to_le_bytes());
            updated.splice(
                split_at - removed_before_split..split_at - removed_before_split,
                new_header,
            );
            let mut verified = program.clone();
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            let verified_rows = verified
                .iec_row_frames()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
            let expected_rows = rows.iter().zip(&verified_rows).all(|(before, after)| {
                let removed_count = removed
                    .iter()
                    .filter(|(_, _, row_index)| *row_index == before.row_index)
                    .count() as u16;
                after.row_index == before.row_index
                    && after.group_index
                        == before.group_index
                            + usize::from(before.row_index >= group_rows[2].row_index)
                    && after.record_count == before.record_count - removed_count
            });
            if verified_rows.len() != rows.len()
                || !expected_rows
                || verified.iec_record_frames().map(|records| records.len())
                    != Some(records.len() - 8)
                || verified.iec_function_blocks().map(|blocks| blocks.len())
                    != program.iec_function_blocks().map(|blocks| blocks.len() - 1)
                || verified.iec_function_references().map(|items| items.len())
                    != Some(references.len() - 3)
                || verified
                    .iec_function_operand_links()
                    .map(|items| items.len())
                    != Some(operands.len() - 3)
                || verified.iec_circuit_graph().is_none()
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            Ok(updated)
        })
    }

    /// Change a captured three-operand IEC ADD/SUB/MUL/DIV block.
    /// The native capture shows a paired function opcode and UTF-16 name.
    pub fn update_iec_ld_arithmetic_function(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let function = crate::iec_ld::arithmetic_function_names(&program)
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if function.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        let replacement_opcode = match replacement {
            "ADD" => 0x47,
            "SUB" => 0x7f,
            "MUL" => 0x48,
            "DIV" => 0x63,
            _ => {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "captured IEC arithmetic function must be ADD, SUB, MUL, or DIV",
                });
            }
        };
        self.edit_program_payload(program_index, "2", |payload| {
            let mut updated =
                replace_iec_ld_text_bytes(payload, program_index, offset, expected, replacement)?;
            let opcode_offset = offset - 66;
            updated[opcode_offset] = replacement_opcode;
            Ok(updated)
        })
    }

    /// Change a captured three-operand IEC comparison block among EQ/GT/GE/LT/LE.
    pub fn update_iec_ld_comparison_function(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let program = self
            .ladder_programs()
            .into_iter()
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })??;
        let function = crate::iec_ld::comparison_function_names(&program)
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or(XgwxError::LadderCellNotFound {
                program_index,
                offset,
            })?;
        if function.value != expected {
            return Err(XgwxError::LadderCellChanged {
                program_index,
                offset,
            });
        }
        let replacement_opcode = match replacement {
            "EQ" => 0x4c,
            "GT" => 0x44,
            "GE" => 0x4f,
            "LT" => 0x4d,
            "LE" => 0x50,
            _ => {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "captured IEC comparison function must be EQ, GT, GE, LT, or LE",
                });
            }
        };
        self.edit_program_payload(program_index, "2", |payload| {
            let mut updated =
                replace_iec_ld_text_bytes(payload, program_index, offset, expected, replacement)?;
            updated[offset - 66] = replacement_opcode;
            Ok(updated)
        })
    }

    fn replace_iec_ld_text(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.edit_program_payload(program_index, "2", |payload| {
            replace_iec_ld_text_bytes(payload, program_index, offset, expected, replacement)
        })
    }

    /// Insert, replace, or delete a contact/coil record in the captured
    /// LD format. Unlike text clearing, deletion removes the actual record.
    /// Unsupported topology is rejected without changing the document.
    pub fn edit_ladder_cell(
        &mut self,
        program_index: usize,
        edit: &LadderCellEdit,
    ) -> Result<(), XgwxError> {
        self.edit_ladder_payload(program_index, |payload| {
            crate::ladder_write::edit_ladder_cell(payload, edit)
        })
    }

    /// Create or edit a native rung/output comment record.
    pub fn edit_ladder_comment(
        &mut self,
        program_index: usize,
        edit: &LadderCommentEdit,
    ) -> Result<(), XgwxError> {
        self.edit_ladder_payload(program_index, |payload| {
            crate::ladder_write::edit_ladder_comment(payload, edit)
        })
    }

    /// Delete a native rung-comment row and close the resulting coordinate gap.
    pub fn delete_ladder_rung_comment(
        &mut self,
        program_index: usize,
        raw_y: u8,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.edit_ladder_payload(program_index, |payload| {
            crate::ladder_write::delete_ladder_rung_comment(payload, raw_y, expected)
        })
    }

    /// Add or remove a supported vertical connection between adjacent rows.
    pub fn edit_ladder_branch(
        &mut self,
        program_index: usize,
        edit: &LadderBranchEdit,
    ) -> Result<(), XgwxError> {
        self.edit_ladder_payload(program_index, |payload| {
            crate::ladder_write::edit_ladder_branch(payload, edit)
        })
    }

    /// Insert a physical blank row before the selected row, as native Ctrl+L.
    pub fn insert_ladder_row(&mut self, program_index: usize, raw_y: u8) -> Result<(), XgwxError> {
        self.edit_ladder_payload(program_index, |payload| {
            crate::ladder_write::insert_ladder_row(payload, raw_y)
        })
    }

    fn edit_ladder_payload(
        &mut self,
        program_index: usize,
        update: impl FnOnce(&[u8]) -> Result<Vec<u8>, XgwxError>,
    ) -> Result<(), XgwxError> {
        self.edit_program_payload(program_index, "1", update)
    }

    fn edit_program_payload(
        &mut self,
        program_index: usize,
        project_type: &str,
        update: impl FnOnce(&[u8]) -> Result<Vec<u8>, XgwxError>,
    ) -> Result<(), XgwxError> {
        self.edit_program_payload_with_validation(program_index, project_type, true, true, update)
    }

    fn edit_program_payload_allow_invalid(
        &mut self,
        program_index: usize,
        project_type: &str,
        update: impl FnOnce(&[u8]) -> Result<Vec<u8>, XgwxError>,
    ) -> Result<(), XgwxError> {
        self.edit_program_payload_with_validation(program_index, project_type, true, false, update)
    }

    fn edit_program_payload_repair(
        &mut self,
        program_index: usize,
        project_type: &str,
        update: impl FnOnce(&[u8]) -> Result<Vec<u8>, XgwxError>,
    ) -> Result<(), XgwxError> {
        self.edit_program_payload_with_validation(program_index, project_type, false, true, update)
    }

    fn edit_program_payload_with_validation(
        &mut self,
        program_index: usize,
        project_type: &str,
        require_valid_source: bool,
        require_valid_result: bool,
        update: impl FnOnce(&[u8]) -> Result<Vec<u8>, XgwxError>,
    ) -> Result<(), XgwxError> {
        let original_iec = if project_type == "2" {
            let program = self
                .ladder_programs()
                .into_iter()
                .nth(program_index)
                .ok_or(XgwxError::ProgramNotFound {
                    index: program_index,
                })??;
            if require_valid_source {
                program
                    .iec_circuit_graph()
                    .ok_or(XgwxError::UnsupportedLadderLayout)?;
            }
            Some(program)
        } else {
            None
        };
        let document = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let program = document
            .descendants()
            .filter(|node| node.has_tag_name("Program"))
            .nth(program_index)
            .ok_or(XgwxError::ProgramNotFound {
                index: program_index,
            })?;
        let data = program
            .descendants()
            .find(|node| node.has_tag_name("ProgramData"))
            .ok_or(XgwxError::MissingProgramData)?;
        if data.attribute("Version") != Some("LD VER 1.1")
            || data.attribute("ProjectType") != Some(project_type)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let text = data
            .children()
            .find(|node| node.is_text())
            .ok_or(XgwxError::MissingProgramData)?;
        let original = text.text().unwrap_or_default();
        let compressed = data
            .attribute("Compressed")
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "True"));
        let payload = decode_base64_payload(original, compressed)?.data;
        let updated = update(&payload)?;
        if require_valid_result && let Some(mut verified) = original_iec {
            verified.decoded_len = updated.len();
            verified.data = updated.clone();
            verified
                .iec_circuit_graph()
                .ok_or(XgwxError::UnsupportedLadderLayout)?;
        }
        let replacement = encode_payload_text(original, compressed, &updated)?;
        self.apply_xml_replacements(vec![(text.range(), replacement)])
    }

    fn apply_xml_replacements(
        &mut self,
        mut replacements: Vec<(Range<usize>, String)>,
    ) -> Result<(), XgwxError> {
        replacements.sort_by_key(|replacement| Reverse(replacement.0.start));
        let mut xml = self.xml.clone();
        for (range, value) in replacements {
            xml.replace_range(range, &value);
        }

        let root = parse_xml(&xml)?;
        self.xml = xml;
        self.root = root;
        Ok(())
    }

    /// Serialize the workspace after supported edits.
    ///
    /// An unmodified XML payload reuses the original gzip member and produces
    /// byte-identical output. Modified payloads are recompressed, padded to the
    /// four-byte boundary recorded by XG5000, and covered by the container's
    /// additive header checksum. Existing trailing Security metadata is
    /// validated and preserved byte-for-byte.
    pub fn to_bytes(&self) -> Result<Vec<u8>, XgwxError> {
        let original = parse_gzip_member(&self.main_gzip, 0)?;
        if original.data == self.xml.as_bytes() {
            return Ok(assemble_workspace(
                &self.header.raw,
                &self.main_gzip,
                &self.trailer,
            ));
        }

        let preserved_trailer = self.supported_trailer_after_main_padding()?;
        validate_preserved_security(preserved_trailer)?;

        let main_gzip = gzip_xml(self.xml.as_bytes())?;
        let aligned_len = align_up(main_gzip.len(), MAIN_GZIP_ALIGNMENT)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        let aligned_len_u32 =
            u32::try_from(aligned_len).map_err(|_| XgwxError::AuthenticatedRewriteUnsupported)?;
        let padding_len = aligned_len - main_gzip.len();

        let mut header = self.header.raw.clone();
        header[COMPRESSED_SIZE_OFFSET..SUPPORTED_HEADER_LEN]
            .copy_from_slice(&aligned_len_u32.to_le_bytes());
        let checksum = workspace_checksum(&header, aligned_len_u32, &main_gzip);
        header[HEADER_CHECKSUM_OFFSET..HEADER_CHECKSUM_INPUT_START]
            .copy_from_slice(&checksum.to_le_bytes());

        let mut bytes = Vec::with_capacity(header.len() + aligned_len + preserved_trailer.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(&main_gzip);
        bytes.resize(bytes.len() + padding_len, 0);
        bytes.extend_from_slice(preserved_trailer);
        Ok(bytes)
    }

    /// Serialize and write the workspace to a file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_to(&self, path: impl AsRef<std::path::Path>) -> Result<(), XgwxError> {
        std::fs::write(path, self.to_bytes()?).map_err(XgwxError::Io)
    }

    fn supported_trailer_after_main_padding(&self) -> Result<&[u8], XgwxError> {
        if self.header.gzip_offset != SUPPORTED_HEADER_LEN
            || self.header.raw.len() != SUPPORTED_HEADER_LEN
        {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        let expected_aligned_len = align_up(self.main_gzip.len(), MAIN_GZIP_ALIGNMENT)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        if self.header.compressed_size_hint != u32::try_from(expected_aligned_len).ok() {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        let padding_len = expected_aligned_len - self.main_gzip.len();
        let padding = self
            .trailer
            .get(..padding_len)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }

        self.trailer
            .get(padding_len..)
            .ok_or(XgwxError::AuthenticatedRewriteUnsupported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClassifiedIecExpression {
    data_type_mask: u32,
    writable: bool,
}

fn classify_iec_bool_expression(
    expression: &str,
    symbols: &[IecLocalSymbol],
    program: &LadderProgramData,
) -> Option<ClassifiedIecExpression> {
    if expression.trim().starts_with('%') {
        classify_iec_bool_device_address(expression)
    } else {
        classify_iec_expression(expression, symbols, program).or_else(|| {
            let mut observed = false;
            let mut writable = false;
            for operand in crate::iec_ld::element_operands(program) {
                if operand.string.value.eq_ignore_ascii_case(expression.trim()) {
                    observed = true;
                    writable |= operand.record_code >= 0x0e;
                }
            }
            observed.then_some(ClassifiedIecExpression {
                data_type_mask: 1,
                writable,
            })
        })
    }
}

fn classify_iec_bool_device_address(expression: &str) -> Option<ClassifiedIecExpression> {
    classify_iec_direct_device_address(&expression.trim().to_ascii_uppercase())
        .filter(|address| address.data_type_mask == 1)
}

fn classify_iec_expression(
    expression: &str,
    symbols: &[IecLocalSymbol],
    program: &LadderProgramData,
) -> Option<ClassifiedIecExpression> {
    classify_iec_atomic_expression(expression, symbols, program).or_else(|| {
        let mut parser = IecArithmeticParser {
            input: expression.trim(),
            cursor: 0,
            symbols,
            program,
            operator_seen: false,
        };
        let classified = parser.parse_sum()?;
        parser.skip_space();
        (parser.operator_seen && parser.cursor == parser.input.len()).then_some(classified)
    })
}

fn looks_like_iec_arithmetic_expression(expression: &str) -> bool {
    expression
        .bytes()
        .any(|byte| matches!(byte, b'+' | b'-' | b'*' | b'/' | b'(' | b')'))
}

struct IecArithmeticParser<'a> {
    input: &'a str,
    cursor: usize,
    symbols: &'a [IecLocalSymbol],
    program: &'a LadderProgramData,
    operator_seen: bool,
}

impl IecArithmeticParser<'_> {
    fn skip_space(&mut self) {
        while self
            .input
            .as_bytes()
            .get(self.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.cursor += 1;
        }
    }

    fn consume(&mut self, byte: u8) -> bool {
        self.skip_space();
        if self.input.as_bytes().get(self.cursor) == Some(&byte) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn parse_sum(&mut self) -> Option<ClassifiedIecExpression> {
        let mut left = self.parse_product()?;
        loop {
            if !self.consume(b'+') && !self.consume(b'-') {
                return Some(left);
            }
            self.operator_seen = true;
            let right = self.parse_product()?;
            left = self.combine_numeric(left, right)?;
        }
    }

    fn parse_product(&mut self) -> Option<ClassifiedIecExpression> {
        let mut left = self.parse_unary()?;
        loop {
            if !self.consume(b'*') && !self.consume(b'/') {
                return Some(left);
            }
            self.operator_seen = true;
            let right = self.parse_unary()?;
            left = self.combine_numeric(left, right)?;
        }
    }

    fn parse_unary(&mut self) -> Option<ClassifiedIecExpression> {
        if self.consume(b'+') || self.consume(b'-') {
            self.operator_seen = true;
            let operand = self.parse_unary()?;
            return (operand.data_type_mask & 0x0000_7ffe != 0).then_some(
                ClassifiedIecExpression {
                    data_type_mask: operand.data_type_mask & 0x0000_7ffe,
                    writable: false,
                },
            );
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<ClassifiedIecExpression> {
        if self.consume(b'(') {
            let value = self.parse_sum()?;
            return self.consume(b')').then_some(value);
        }
        self.skip_space();
        let start = self.cursor;
        while let Some(&byte) = self.input.as_bytes().get(self.cursor) {
            if matches!(byte, b'+' | b'-' | b'*' | b'/' | b'(' | b')') {
                break;
            }
            self.cursor += 1;
        }
        classify_iec_atomic_expression(
            self.input[start..self.cursor].trim(),
            self.symbols,
            self.program,
        )
    }

    fn combine_numeric(
        &self,
        left: ClassifiedIecExpression,
        right: ClassifiedIecExpression,
    ) -> Option<ClassifiedIecExpression> {
        let data_type_mask = left.data_type_mask & right.data_type_mask & 0x0000_7ffe;
        (data_type_mask != 0).then_some(ClassifiedIecExpression {
            data_type_mask,
            writable: false,
        })
    }
}

fn classify_iec_atomic_expression(
    expression: &str,
    symbols: &[IecLocalSymbol],
    program: &LadderProgramData,
) -> Option<ClassifiedIecExpression> {
    let expression = expression.trim();
    let folded = expression.to_lowercase();
    if let Some(symbol) = symbols
        .iter()
        .find(|symbol| symbol.name.to_lowercase() == folded)
    {
        let data_type_mask = symbol
            .data_type_code
            .checked_sub(1)
            .filter(|bit| *bit < 20)
            .and_then(|bit| 1_u32.checked_shl(bit))?;
        return Some(ClassifiedIecExpression {
            data_type_mask,
            writable: !symbol.is_instance && symbol.storage_class != "I",
        });
    }
    if let Some((instance, member)) = expression.split_once('.') {
        let instance = instance.to_lowercase();
        let member = member.to_lowercase();
        if symbols
            .iter()
            .any(|symbol| symbol.is_instance && symbol.name.to_lowercase() == instance)
        {
            let block = program.iec_function_blocks()?.into_iter().find(|block| {
                block
                    .instance
                    .as_ref()
                    .is_some_and(|field| field.value.to_lowercase() == instance)
            })?;
            let pin = block
                .pins
                .into_iter()
                .chain([block.control_input, block.control_output])
                .find(|pin| pin.name.value.to_lowercase() == member)?;
            return Some(ClassifiedIecExpression {
                data_type_mask: pin.data_type_mask,
                writable: false,
            });
        }
    }
    let upper = expression.to_ascii_uppercase();
    if upper.starts_with('%') {
        return classify_iec_direct_device_address(&upper);
    }
    if matches!(upper.as_str(), "TRUE" | "FALSE") {
        return Some(ClassifiedIecExpression {
            data_type_mask: 1,
            writable: false,
        });
    }
    for (prefix, bit) in [
        ("T#", 15),
        ("TIME#", 15),
        ("D#", 16),
        ("DATE#", 16),
        ("TOD#", 17),
        ("TIME_OF_DAY#", 17),
        ("DT#", 18),
        ("DATE_AND_TIME#", 18),
    ] {
        if upper.starts_with(prefix) && upper.len() > prefix.len() {
            return Some(ClassifiedIecExpression {
                data_type_mask: 1 << bit,
                writable: false,
            });
        }
    }
    if expression.parse::<i128>().is_ok() {
        return Some(ClassifiedIecExpression {
            data_type_mask: 0x0000_7ffe,
            writable: false,
        });
    }
    if expression.contains('.') && expression.parse::<f64>().is_ok() {
        return Some(ClassifiedIecExpression {
            data_type_mask: 0x0000_6000,
            writable: false,
        });
    }
    None
}

fn classify_iec_direct_device_address(address: &str) -> Option<ClassifiedIecExpression> {
    let rest = address.strip_prefix('%')?;
    let mut chars = rest.chars();
    let area = chars.next()?;
    if !matches!(
        area,
        'I' | 'Q' | 'M' | 'U' | 'W' | 'F' | 'K' | 'L' | 'R' | 'A'
    ) {
        return None;
    }
    let data_type_mask = match chars.next()? {
        'X' => 1_u32 << 0,
        'B' => (1_u32 << 1) | (1_u32 << 5) | (1_u32 << 9),
        'W' => (1_u32 << 2) | (1_u32 << 6) | (1_u32 << 10),
        'D' => (1_u32 << 3) | (1_u32 << 7) | (1_u32 << 11) | (1_u32 << 13),
        'L' => (1_u32 << 4) | (1_u32 << 8) | (1_u32 << 12) | (1_u32 << 14),
        _ => return None,
    };
    let index = chars.as_str();
    let parts = index.split('.').collect::<Vec<_>>();
    if parts.is_empty()
        || parts.len() > 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || part.parse::<u32>().is_err()
        })
        || (matches!(area, 'I' | 'Q' | 'U') && !matches!(parts.len(), 1 | 3))
        || (!matches!(area, 'I' | 'Q' | 'U') && parts.len() != 1)
    {
        return None;
    }
    Some(ClassifiedIecExpression {
        data_type_mask,
        writable: area != 'I',
    })
}

fn looks_like_iec_direct_device_address(expression: &str) -> bool {
    expression.starts_with('%')
        && expression[1..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
}

fn module_option_location(option: &ModuleOptionEntry, index: u32) -> (usize, u32, usize) {
    let encoding = option.encoding;
    match encoding.kind {
        "scalar" => (encoding.offset as usize, 0, (encoding.bits / 8) as usize),
        "bit" => (encoding.offset as usize, encoding.shift_base + index, 4),
        "packed" => (
            encoding.offset as usize,
            encoding.shift_base + encoding.bits * index,
            4,
        ),
        "per-channel" => ((encoding.offset + encoding.stride * index) as usize, 0, 4),
        "banked-packed" => (
            (encoding.offset + encoding.bank_stride * (index / encoding.channels_per_word))
                as usize,
            encoding.bits * (index % encoding.channels_per_word),
            4,
        ),
        "split-packed" => (
            if index < encoding.channels_per_word {
                encoding.offset as usize
            } else {
                encoding.second_offset as usize
            },
            encoding.bits * (index % encoding.channels_per_word),
            4,
        ),
        "packed-bytes" => (
            (encoding.offset + encoding.bank_stride * (index / encoding.channels_per_word))
                as usize,
            encoding.bits * (index % encoding.channels_per_word),
            1,
        ),
        _ => unreachable!("generated module option encoding is normalized"),
    }
}

fn read_module_option(
    details: &[u8],
    option: &ModuleOptionEntry,
    index: u32,
) -> Result<u32, XgwxError> {
    let (offset, shift, width) = module_option_location(option, index);
    let range = details.get(offset..offset + width).ok_or_else(|| {
        XgwxError::ModuleOptionDetailsTooShort {
            key: option.key.to_owned(),
        }
    })?;
    let mut encoded = [0u8; 4];
    encoded[..width].copy_from_slice(range);
    let word = u32::from_le_bytes(encoded);
    let mask = if option.encoding.bits == 32 {
        u32::MAX
    } else {
        (1u32 << option.encoding.bits) - 1
    };
    Ok((word >> shift) & mask)
}

fn write_module_option(
    details: &mut [u8],
    option: &ModuleOptionEntry,
    index: u32,
    value: u32,
) -> Result<(), XgwxError> {
    let (offset, shift, width) = module_option_location(option, index);
    let range = details.get_mut(offset..offset + width).ok_or_else(|| {
        XgwxError::ModuleOptionDetailsTooShort {
            key: option.key.to_owned(),
        }
    })?;
    let mut encoded = [0u8; 4];
    encoded[..width].copy_from_slice(range);
    let mut word = u32::from_le_bytes(encoded);
    let value_mask = if option.encoding.bits == 32 {
        u32::MAX
    } else {
        (1u32 << option.encoding.bits) - 1
    };
    let mask = value_mask << shift;
    word = (word & !mask) | ((value & value_mask) << shift);
    range.copy_from_slice(&word.to_le_bytes()[..width]);
    Ok(())
}

fn gzip_xml(xml: &[u8]) -> Result<Vec<u8>, XgwxError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml).map_err(XgwxError::Io)?;
    encoder.finish().map_err(XgwxError::Io)
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value
        .checked_add(alignment.checked_sub(1)?)
        .map(|value| value & !(alignment - 1))
}

fn workspace_checksum(header: &[u8], aligned_len: u32, main_gzip: &[u8]) -> u32 {
    let header_sum = header[HEADER_CHECKSUM_INPUT_START..COMPRESSED_SIZE_OFFSET]
        .iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(u32::from(*byte)));
    let gzip_sum = main_gzip
        .iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(u32::from(*byte)));
    header_sum.wrapping_add(aligned_len).wrapping_add(gzip_sum)
}

fn validate_preserved_security(trailer: &[u8]) -> Result<(), XgwxError> {
    let mut search_from = 0;
    while let Some(offset) = find_gzip_member(trailer, search_from) {
        search_from = offset + 2;
        // Opaque security bytes can contain the two-byte gzip magic by chance.
        // Only the DEFLATE method identifies a candidate gzip member.
        if trailer.get(offset + 2) != Some(&8) {
            continue;
        }
        let member = parse_gzip_member(trailer, offset)?;
        if member.data.starts_with(XG_FRAME_MAGIC) && !validate_xg_frame(&member.data) {
            return Err(XgwxError::AuthenticatedRewriteUnsupported);
        }
        search_from = member.end_offset;
    }
    Ok(())
}

pub(crate) fn validate_xg_frame(frame: &[u8]) -> bool {
    let Some(payload_len_bytes) = frame.get(4..8).and_then(|bytes| bytes.try_into().ok()) else {
        return false;
    };
    if frame.get(..4) != Some(XG_FRAME_MAGIC.as_slice()) {
        return false;
    }

    let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;
    let Some(tag_start) = XG_FRAME_HEADER_LEN.checked_add(payload_len) else {
        return false;
    };
    let Some(footer_start) = tag_start.checked_add(XG_FRAME_TAG_LEN) else {
        return false;
    };
    let Some(frame_len) = footer_start.checked_add(XG_FRAME_FOOTER_LEN) else {
        return false;
    };
    if frame.len() != frame_len || frame.get(footer_start..) != Some(XG_FRAME_FOOTER.as_slice()) {
        return false;
    }

    let Some(payload) = frame.get(XG_FRAME_HEADER_LEN..tag_start) else {
        return false;
    };
    let Some(expected_tag) = frame.get(tag_start..footer_start) else {
        return false;
    };
    if xg_crc64(payload).to_le_bytes() != expected_tag {
        return false;
    }

    if payload.starts_with(XG_FRAME_MAGIC) {
        let Some(inner_payload_len_bytes) =
            payload.get(4..8).and_then(|bytes| bytes.try_into().ok())
        else {
            return false;
        };
        let inner_payload_len = u32::from_le_bytes(inner_payload_len_bytes) as usize;
        let Some(inner_len) = XG_FRAME_HEADER_LEN
            .checked_add(inner_payload_len)
            .and_then(|len| len.checked_add(XG_FRAME_TAG_LEN + XG_FRAME_FOOTER_LEN))
        else {
            return false;
        };
        let Some(inner) = payload.get(..inner_len) else {
            return false;
        };
        if !validate_xg_frame(inner) {
            return false;
        }
    }

    true
}

pub(crate) fn xg_crc64(payload: &[u8]) -> u64 {
    let padded_len = align_up(payload.len(), 8).unwrap_or(payload.len());
    let mut crc = 0u64;
    for index in 0..padded_len {
        let byte = payload.get(index).copied().unwrap_or(0);
        crc ^= u64::from(byte) << 56;
        for _ in 0..8 {
            crc = if crc & (1 << 63) != 0 {
                (crc << 1) ^ XG_CRC64_POLYNOMIAL
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn push_u32_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: Option<u32>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_replacement(
            replacements,
            module,
            base,
            slot,
            attribute,
            value.to_string(),
        )?;
    }
    Ok(())
}

fn push_program_u32_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    program: roxmltree::Node<'_, '_>,
    program_index: usize,
    attribute: &'static str,
    value: Option<u32>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_program_replacement(
            replacements,
            program,
            program_index,
            attribute,
            value.to_string(),
        )?;
    }
    Ok(())
}

fn push_program_string_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    program: roxmltree::Node<'_, '_>,
    program_index: usize,
    attribute: &'static str,
    value: Option<&str>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_program_replacement(
            replacements,
            program,
            program_index,
            attribute,
            escape_xml_attribute(value),
        )?;
    }
    Ok(())
}

fn push_program_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    program: roxmltree::Node<'_, '_>,
    program_index: usize,
    attribute: &'static str,
    value: String,
) -> Result<(), XgwxError> {
    let range = program
        .attributes()
        .find(|item| item.name() == attribute)
        .map(|item| item.range_value())
        .ok_or(XgwxError::MissingProgramAttribute {
            index: program_index,
            attribute,
        })?;
    replacements.push((range, value));
    Ok(())
}

fn push_network_string_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    network: roxmltree::Node<'_, '_>,
    network_index: usize,
    attribute: &'static str,
    value: Option<&str>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        let range = network
            .attributes()
            .find(|item| item.name() == attribute)
            .map(|item| item.range_value())
            .ok_or(XgwxError::MissingNetworkAttribute {
                index: network_index,
                attribute,
            })?;
        replacements.push((range, escape_xml_attribute(value)));
    }
    Ok(())
}

fn push_network_module_string_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: Option<&str>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        let range = module
            .attributes()
            .find(|item| item.name() == attribute)
            .map(|item| item.range_value())
            .ok_or(XgwxError::MissingNetworkModuleAttribute {
                base,
                slot,
                attribute,
            })?;
        replacements.push((range, escape_xml_attribute(value)));
    }
    Ok(())
}

fn push_string_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: Option<&str>,
) -> Result<(), XgwxError> {
    if let Some(value) = value {
        push_replacement(
            replacements,
            module,
            base,
            slot,
            attribute,
            escape_xml_attribute(value),
        )?;
    }
    Ok(())
}

fn push_replacement(
    replacements: &mut Vec<(Range<usize>, String)>,
    module: roxmltree::Node<'_, '_>,
    base: u32,
    slot: u32,
    attribute: &'static str,
    value: String,
) -> Result<(), XgwxError> {
    let range = module
        .attributes()
        .find(|item| item.name() == attribute)
        .map(|item| item.range_value())
        .ok_or(XgwxError::MissingModuleAttribute {
            base,
            slot,
            attribute,
        })?;
    replacements.push((range, value));
    Ok(())
}

fn validate_module_details(details: &str) -> Result<(), XgwxError> {
    if details.len().is_multiple_of(2) && details.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(XgwxError::InvalidHexPayload {
            element: "Module".to_owned(),
            attribute: "Details".to_owned(),
        })
    }
}

fn push_network_configuration_removals(
    replacements: &mut Vec<(Range<usize>, String)>,
    document: &roxmltree::Document<'_>,
    base: u32,
    slot: u32,
) {
    replacements.extend(
        document
            .descendants()
            .filter(|node| {
                (node.has_tag_name("NetworkModule")
                    || node.tag_name().name().starts_with("XGPD_CONFIG_INFO_"))
                    && node.attribute("Base").and_then(|value| value.parse().ok()) == Some(base)
                    && node.attribute("Slot").and_then(|value| value.parse().ok()) == Some(slot)
            })
            .map(|node| (node.range(), String::new())),
    );
}

fn push_network_configuration_additions(
    replacements: &mut Vec<(Range<usize>, String)>,
    xml: &str,
    document: &roxmltree::Document<'_>,
    configuration_name: &str,
    base: u32,
    slot: u32,
    entry: &ModuleCatalogEntry,
) -> Result<(), XgwxError> {
    let Some(profile) = network_profile(entry.model) else {
        return Ok(());
    };
    let network = document
        .descendants()
        .find(|node| node.has_tag_name("Network"))
        .ok_or(XgwxError::MissingNetworkContainer)?;
    let network_offset = closing_tag_offset(xml, network, "Network")?;
    let network_indentation = line_indentation(xml, network_offset);
    let newline = xml_newline(xml);
    let network_module = format!(
        "<NetworkModule ConfigName=\"{}\" ConfigType=\"1\" Type=\"NETWORK MODULE\" Name=\"{}@0x0\" Base=\"{base}\" Slot=\"{slot}\" Id=\"{}\" ChannelType=\"0\" OptionType=\"{}\" Alias=\"\" Description=\"\"></NetworkModule>{newline}{network_indentation}",
        escape_xml_attribute(configuration_name),
        entry.model,
        entry.id,
        entry.sub_type,
    );
    replacements.push((network_offset..network_offset, network_module));

    if let Some(tag) = profile.xgpd_tag {
        let group = document
            .descendants()
            .find(|node| node.has_tag_name("XGPD_CONFIG_INFO_GROUP"))
            .ok_or(XgwxError::MissingNetworkConfigurationGroup)?;
        let group_offset = closing_tag_offset(xml, group, "XGPD_CONFIG_INFO_GROUP")?;
        let group_indentation = line_indentation(xml, group_offset);
        let config = format!(
            "<XGPD_CONFIG_INFO_{tag} StationNo=\"0\" Type=\"{}\" Base=\"{base}\" Slot=\"{slot}\" SubType=\"{}\"{}></XGPD_CONFIG_INFO_{tag}>{newline}{group_indentation}",
            entry.id, entry.sub_type, profile.attributes,
        );
        replacements.push((group_offset..group_offset, config));
    }
    Ok(())
}

struct NetworkProfile {
    xgpd_tag: Option<&'static str>,
    attributes: &'static str,
}

fn network_profile(model: &str) -> Option<NetworkProfile> {
    match model {
        "XGL-EDMT" => Some(NetworkProfile {
            xgpd_tag: Some("FDENET"),
            attributes: " Media=\"0\" Master=\"0\"",
        }),
        "XGL-EDMF" => Some(NetworkProfile {
            xgpd_tag: Some("FDENET"),
            attributes: " Media=\"6\" Master=\"0\"",
        }),
        "XGL-DMEA/B" => Some(NetworkProfile {
            xgpd_tag: Some("DNET"),
            attributes: "",
        }),
        "XGL-RMEA/B" => Some(NetworkProfile {
            xgpd_tag: Some("RNET"),
            attributes: "",
        }),
        // Captured from an XG5000 FEnet configuration with stable type code
        // 23041. The module model may differ by platform, but XG5000 joins the
        // configuration to NetworkModule through this type code.
        "XGL-EFMT(B)" => Some(NetworkProfile {
            xgpd_tag: Some("FENET"),
            attributes: " Media=\"0\" MediaB=\"0\" Media1=\"0\" Media1_2=\"0\" IpAddr_0=\"192\" IpAddr_1=\"168\" IpAddr_2=\"0\" IpAddr_3=\"100\" Subnet_0=\"255\" Subnet_1=\"255\" Subnet_2=\"255\" Subnet_3=\"0\" Gateway_0=\"192\" Gateway_1=\"168\" Gateway_2=\"0\" Gateway_3=\"1\" Dns_0=\"0\" Dns_1=\"0\" Dns_2=\"0\" Dns_3=\"0\" Dhcp=\"0\" Relay=\"0\" RapienetProtocol=\"0\" DriverType=\"2\" RcvWaitTime=\"100\" ClientWaitTime=\"60\" GlofaSocketCnt=\"3\" HsNo2=\"0\" Media2=\"0\" Media2_2=\"0\" IpAddr2_0=\"0\" IpAddr2_1=\"0\" IpAddr2_2=\"0\" IpAddr2_3=\"0\" Subnet2_0=\"0\" Subnet2_1=\"0\" Subnet2_2=\"0\" Subnet2_3=\"0\" Gateway2_0=\"0\" Gateway2_1=\"0\" Gateway2_2=\"0\" Gateway2_3=\"0\" Dns2_0=\"0\" Dns2_1=\"0\" Dns2_2=\"0\" Dns2_3=\"0\" Dhcp2=\"0\" OneIPSolution=\"0\" DI_DeviceType=\"80\" DI_DataType=\"88\" DI_Size=\"0\" DI_Addr=\"0\" DO_DeviceType=\"80\" DO_DataType=\"88\" DO_Size=\"0\" DO_Addr=\"200\" AI_DeviceType=\"68\" AI_DataType=\"87\" AI_Size=\"0\" AI_Addr=\"0\" AO_DeviceType=\"68\" AO_DataType=\"87\" AO_Size=\"0\" AO_Addr=\"100\" EnableHostTable=\"0\" arHostIp_Count=\"0\" ExtendEnableHostTable=\"0\" SecurityConfigItemCount=\"0\" ServerPortEnable=\"0\" ServerPortIndividualType_0=\"0\" ServerPortIndividualStartPortNo_0=\"0\" ServerPortIndividualPortCount_0=\"0\" ServerPortIndividualType_1=\"0\" ServerPortIndividualStartPortNo_1=\"0\" ServerPortIndividualPortCount_1=\"0\" ServerPortIndividualType_2=\"0\" ServerPortIndividualStartPortNo_2=\"0\" ServerPortIndividualPortCount_2=\"0\" ServerPortIndividualType_3=\"0\" ServerPortIndividualStartPortNo_3=\"0\" ServerPortIndividualPortCount_3=\"0\" ServerPortIndividualType_4=\"0\" ServerPortIndividualStartPortNo_4=\"0\" ServerPortIndividualPortCount_4=\"0\" ServerPortIndividualType_5=\"0\" ServerPortIndividualStartPortNo_5=\"0\" ServerPortIndividualPortCount_5=\"0\" ServerPortIndividualType_6=\"0\" ServerPortIndividualStartPortNo_6=\"0\" ServerPortIndividualPortCount_6=\"0\" ServerPortIndividualType_7=\"0\" ServerPortIndividualStartPortNo_7=\"0\" ServerPortIndividualPortCount_7=\"0\" Used_OPCUA=\"0\" AutoNegotiationSpeedLimit=\"0\"",
        }),
        "XGL-BIPT" | "XGL-EIPT" | "XGL-C22A/B" | "XGL-C42A/B" | "XGL-CH2A/B" | "XGL-EFMF(B)"
        | "XGL-EFMHB" => Some(NetworkProfile {
            xgpd_tag: None,
            attributes: "",
        }),
        _ => None,
    }
}

fn closing_tag_offset(
    xml: &str,
    node: roxmltree::Node<'_, '_>,
    tag: &str,
) -> Result<usize, XgwxError> {
    let range = node.range();
    let closing_tag = format!("</{tag}>");
    xml[range.clone()]
        .rfind(&closing_tag)
        .map(|offset| range.start + offset)
        .ok_or(XgwxError::MissingNetworkContainer)
}

fn line_indentation(xml: &str, offset: usize) -> &str {
    let start = xml[..offset].rfind('\n').map_or(offset, |index| index + 1);
    &xml[start..offset]
}

fn xml_newline(xml: &str) -> &'static str {
    if xml.contains("\r\n") { "\r\n" } else { "\n" }
}

fn escape_xml_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\r' => escaped.push_str("&#xD;"),
            '\n' => escaped.push_str("&#xA;"),
            '\t' => escaped.push_str("&#x9;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn escape_xml_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\r' => escaped.push_str("&#xD;"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn patch_variable_string(
    payload: &mut [u8],
    variable_index: usize,
    field: &'static str,
    original: &LadderString,
    replacement: Option<&str>,
) -> Result<(), XgwxError> {
    let Some(replacement) = replacement else {
        return Ok(());
    };
    let expected_units = original.value.encode_utf16().count();
    let actual_units = replacement.encode_utf16().count();
    if actual_units != expected_units {
        return Err(XgwxError::VariableFieldLengthChanged {
            index: variable_index,
            field,
            expected_utf16_units: expected_units,
            actual_utf16_units: actual_units,
        });
    }

    let text_start = original.offset + UTF16_MARKER.len() + 1;
    let bytes = payload.get_mut(text_start..original.end_offset).ok_or(
        XgwxError::InvalidVariableRecord {
            index: variable_index,
        },
    )?;
    for (chunk, unit) in bytes.chunks_exact_mut(2).zip(replacement.encode_utf16()) {
        chunk.copy_from_slice(&unit.to_le_bytes());
    }
    Ok(())
}

fn insert_iec_ld_blank_row_bytes(
    program: &LadderProgramData,
    after_row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let max_rows = u16::from_le_bytes(
        program
            .data
            .get(4..6)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    if after_row_index >= max_rows
        || max_rows >= u16::MAX / 4
        || rows.iter().any(|row| row.row_index >= u16::MAX / 4)
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC blank row must be inserted inside the decoded row range",
        });
    }

    let mut updated = program.data.clone();
    updated[4..6].copy_from_slice(&(max_rows + 1).to_le_bytes());
    if let Some(boundary) = rows.iter().find(|row| row.row_index == after_row_index) {
        let secondary_y = u16::from_le_bytes(
            updated[boundary.start + 26..boundary.start + 28]
                .try_into()
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
        );
        if secondary_y.checked_add(4) == after_row_index.checked_mul(4) {
            updated[boundary.start + 26..boundary.start + 28]
                .copy_from_slice(&(secondary_y + 4).to_le_bytes());
        }
    }
    for row in rows.iter().filter(|row| row.row_index > after_row_index) {
        let changed = row
            .row_index
            .checked_add(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        updated[row.start..row.start + 4].copy_from_slice(&u32::from(changed).to_le_bytes());
        shift_iec_y_after(&mut updated, row.start + 22, after_row_index, 1)?;
        shift_iec_y_after(&mut updated, row.start + 30, after_row_index, 1)?;
    }
    for record in &records {
        match record.kind {
            IecRecordKind::LongWire => {
                shift_iec_y_after(&mut updated, record.offset + 6, after_row_index, 1)?;
                shift_iec_y_after(&mut updated, record.offset + 16, after_row_index, 1)?;
            }
            IecRecordKind::BranchStart => {
                shift_iec_y_after(&mut updated, record.offset + 8, after_row_index, 1)?;
                shift_iec_y_after(&mut updated, record.offset + 18, after_row_index, 1)?;
            }
            IecRecordKind::BranchEnd | IecRecordKind::LinkReference(_) => {
                shift_iec_y_after(&mut updated, record.offset + 6, after_row_index, 1)?;
            }
            IecRecordKind::FunctionBlock => {
                shift_iec_function_block_rows(
                    &mut updated,
                    record.offset,
                    record.end,
                    record.row_index,
                    after_row_index,
                    1,
                )?;
            }
            IecRecordKind::ShortWire
            | IecRecordKind::Contact(_)
            | IecRecordKind::Coil(_)
            | IecRecordKind::Comment
            | IecRecordKind::FunctionOperand => {
                shift_iec_y_after(&mut updated, record.offset + 6, after_row_index, 1)?;
            }
        }
    }

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC blank-row insertion did not preserve row framing",
        })?;
    if verified_rows.len() != rows.len()
        || rows.iter().zip(&verified_rows).any(|(before, after)| {
            after.group_index != before.group_index
                || after.start != before.start
                || after.end != before.end
                || after.record_count != before.record_count
                || after.row_index
                    != before.row_index + u16::from(before.row_index > after_row_index)
        })
        || verified.iec_record_frames().is_none()
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn insert_iec_ld_parallel_contact_bytes(
    program: &LadderProgramData,
    row_index: u16,
    expected_contact: &str,
    expected_coil: &str,
    parallel_code: u8,
    parallel: &str,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    program
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let top =
        rows.iter()
            .find(|row| row.row_index == row_index)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "IEC parallel contact requires an occupied top row",
            })?;
    let group_rows = rows
        .iter()
        .filter(|row| row.group_index == top.group_index)
        .collect::<Vec<_>>();
    let group_records = records
        .iter()
        .filter(|record| record.group_index == top.group_index)
        .collect::<Vec<_>>();
    if group_rows.len() != 1
        || group_records.len() != 3
        || !matches!(group_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || group_records[1].kind != IecRecordKind::LongWire
        || !matches!(group_records[2].kind, IecRecordKind::Coil(0x0e..=0x13))
        || program.data[group_records[0].offset + 5] != 1
        || program.data[group_records[1].offset + 5] != 4
        || program.data[group_records[1].offset + 15] != 91
        || program.data[group_records[2].offset + 5] != 94
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC parallel contact requires a single-row addressed contact, long wire, and addressed coil",
        });
    }
    let operands = crate::iec_ld::element_operands(program);
    let operand_at = |offset| {
        operands
            .iter()
            .find(|operand| operand.string.offset == offset + 15)
    };
    if operand_at(group_records[0].offset)
        .is_none_or(|operand| operand.string.value != expected_contact)
        || operand_at(group_records[2].offset)
            .is_none_or(|operand| operand.string.value != expected_coil)
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC rung operands changed",
        });
    }
    let bottom_index = row_index
        .checked_add(1)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let shifted = insert_iec_ld_blank_row_bytes(program, row_index)?;
    let mut shifted_program = program.clone();
    shifted_program.data = shifted.clone();
    shifted_program.decoded_len = shifted.len();
    let shifted_rows = shifted_program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let shifted_records = shifted_program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let top = shifted_rows
        .iter()
        .find(|row| row.group_index == top.group_index && row.row_index == row_index)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let contact = shifted_records
        .iter()
        .find(|record| {
            record.group_index == top.group_index && record.kind == group_records[0].kind
        })
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let y_top = row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .to_le_bytes();
    let y_bottom = bottom_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .to_le_bytes();
    let mut branch_start = vec![
        0,
        0,
        0,
        0,
        0,
        2,
        0,
        3,
        y_top[0],
        y_top[1],
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        2,
        y_bottom[0],
        y_bottom[1],
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut bottom = shifted[top.start..top.records_start].to_vec();
    bottom[0..4].copy_from_slice(&u32::from(bottom_index).to_le_bytes());
    bottom[22..24].copy_from_slice(&y_bottom);
    bottom[26..28].copy_from_slice(&y_bottom);
    bottom[29] = 2;
    bottom[30..32].copy_from_slice(&y_bottom);
    bottom[33..35].copy_from_slice(&2u16.to_le_bytes());
    let utf16 = parallel.encode_utf16().collect::<Vec<_>>();
    bottom.extend_from_slice(&[
        0xff,
        parallel_code,
        0,
        0,
        0,
        1,
        y_bottom[0],
        y_bottom[1],
        0,
        1,
        0,
        0,
        0,
        0,
        0,
        0xff,
        0xfe,
        0xff,
        utf16.len() as u8,
    ]);
    for unit in utf16 {
        bottom.extend_from_slice(&unit.to_le_bytes());
    }
    bottom.extend_from_slice(&[1, 0, 0, 0, 0, 3, y_top[0], y_top[1], 0]);

    let mut updated = shifted;
    updated[top.start - 2..top.start].copy_from_slice(&2u16.to_le_bytes());
    updated[top.start + 33..top.start + 35].copy_from_slice(&4u16.to_le_bytes());
    updated.splice(top.end..top.end, bottom);
    updated.splice(contact.end..contact.end, branch_start.drain(..));
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let checked_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let checked_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    verified
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if checked_rows.len() != rows.len() + 1
        || checked_records.len() != records.len() + 3
        || !geometry.vertical.iter().any(|connection| {
            connection.group_index == top.group_index
                && connection.start_row_index == row_index
                && connection.end_row_index == bottom_index
                && connection.x == 3
        })
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn iec_rung_contact_code(kind: &str) -> Result<u8, XgwxError> {
    let contact = match kind {
        "NO" => LadderEditKind::NormallyOpen,
        "NC" => LadderEditKind::NormallyClosed,
        "RISING" => LadderEditKind::AddressedRisingPulse,
        "FALLING" => LadderEditKind::AddressedFallingPulse,
        "NEGATED_RISING" => LadderEditKind::AddressedRisingPulseNot,
        "NEGATED_FALLING" => LadderEditKind::AddressedFallingPulseNot,
        _ => Err(XgwxError::InvalidLadderEdit {
            reason: "IEC rung contact kind must be NO, NC, RISING, FALLING, NEGATED_RISING, or NEGATED_FALLING",
        })?,
    };
    Ok(contact.marker())
}

fn iec_rung_coil_code(kind: &str) -> Result<u8, XgwxError> {
    match kind {
        "OUTPUT" => Ok(0x0e),
        "INVERSE" => Ok(0x0f),
        "SET" => Ok(0x10),
        "RESET" => Ok(0x11),
        "RISING" => Ok(0x12),
        "FALLING" => Ok(0x13),
        _ => Err(XgwxError::InvalidLadderEdit {
            reason: "IEC rung coil kind must be OUTPUT, INVERSE, SET, RESET, RISING, or FALLING",
        }),
    }
}

fn insert_iec_ld_linear_rung_bytes(
    program: &LadderProgramData,
    blank_row_index: u16,
    contact_code: u8,
    contact_variable: &str,
    coil_code: u8,
    coil_variable: &str,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let graph = program
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let max_rows = u16::from_le_bytes(
        program
            .data
            .get(4..6)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    let group_count = u16::from_le_bytes(
        program
            .data
            .get(6..8)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    if blank_row_index > max_rows
        || blank_row_index >= u16::MAX / 4
        || group_count == u16::MAX
        || rows.iter().any(|row| row.row_index == blank_row_index)
        || graph.occupied_areas.iter().any(|area| {
            area.start_row_index <= blank_row_index && blank_row_index <= area.end_row_index
        })
        || graph.edges.iter().any(|edge| {
            edge.start.row_index <= blank_row_index && blank_row_index <= edge.end.row_index
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC rung creation requires an unoccupied implicit row",
        });
    }
    let (insertion_group, insertion_offset) =
        if let Some(next_row) = rows.iter().find(|row| row.row_index > blank_row_index) {
            if rows
                .iter()
                .any(|row| row.group_index == next_row.group_index && row.start < next_row.start)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC rung creation cannot split an existing row group",
                });
            }
            if next_row.group_index >= usize::from(group_count) || next_row.start < 10 {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            (next_row.group_index, next_row.start - 10)
        } else {
            // IEC row framing runs to the end of the program payload. Require
            // the final decoded record to consume that tail before appending
            // a new group, so opaque trailing data is never reinterpreted.
            if rows.last().map(|row| row.end) != Some(program.data.len())
                || records.last().map(|record| record.end) != Some(program.data.len())
            {
                return Err(XgwxError::UnsupportedLadderLayout);
            }
            (usize::from(group_count), program.data.len())
        };
    let group_starts = (0..usize::from(group_count))
        .map(|group_index| {
            rows.iter()
                .find(|row| row.group_index == group_index)
                .and_then(|row| row.start.checked_sub(10))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    for (group_index, &start) in group_starts.iter().enumerate() {
        if program.data.get(start..start + 4)
            != Some(u32::try_from(group_index).unwrap().to_le_bytes().as_slice())
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
    }

    let y = blank_row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .to_le_bytes();
    let mut inserted = Vec::new();
    inserted.extend_from_slice(
        &u32::try_from(insertion_group)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    inserted.extend_from_slice(&0u32.to_le_bytes());
    inserted.extend_from_slice(&1u16.to_le_bytes());
    inserted.extend_from_slice(&u32::from(blank_row_index).to_le_bytes());
    inserted.extend_from_slice(&[
        0xff, 0x43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x27, 0, 0, 0, 94, y[0], y[1], 0, 94, y[0],
        y[1], 0, 1, y[0], y[1], 0, 3, 0,
    ]);
    fn append_element(
        output: &mut Vec<u8>,
        y: [u8; 2],
        code: u8,
        raw_x: u8,
        flags: [u8; 6],
        value: &str,
    ) {
        let utf16 = value.encode_utf16().collect::<Vec<_>>();
        output.extend_from_slice(&[
            0xff,
            code,
            0,
            0,
            0,
            raw_x,
            y[0],
            y[1],
            0,
            flags[0],
            flags[1],
            flags[2],
            flags[3],
            flags[4],
            flags[5],
            0xff,
            0xfe,
            0xff,
            utf16.len() as u8,
        ]);
        for unit in utf16 {
            output.extend_from_slice(&unit.to_le_bytes());
        }
    }
    append_element(
        &mut inserted,
        y,
        contact_code,
        1,
        [1, 0, 0, 0, 0, 0],
        contact_variable,
    );
    inserted.extend_from_slice(&[
        0xff, 0x02, 0, 0, 0, 4, y[0], y[1], 0, 0, 0, 0, 0, 0, 0, 91, y[0], y[1], 0,
    ]);
    append_element(
        &mut inserted,
        y,
        coil_code,
        94,
        [1, 0, 0x20, 0, 0, 0],
        coil_variable,
    );

    let mut updated = program.data.clone();
    updated[4..6].copy_from_slice(&max_rows.max(blank_row_index + 1).to_le_bytes());
    updated[6..8].copy_from_slice(&(group_count + 1).to_le_bytes());
    for (group_index, &start) in group_starts.iter().enumerate().skip(insertion_group) {
        updated[start..start + 4].copy_from_slice(
            &u32::try_from(group_index + 1)
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                .to_le_bytes(),
        );
    }
    updated.splice(insertion_offset..insertion_offset, inserted);

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung creation did not preserve row framing",
        })?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung creation did not preserve record framing",
        })?;
    let created = verified_rows
        .iter()
        .find(|row| row.group_index == insertion_group && row.row_index == blank_row_index)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let created_kinds = verified_records
        .iter()
        .filter(|record| {
            record.group_index == insertion_group && record.row_index == blank_row_index
        })
        .map(|record| record.kind)
        .collect::<Vec<_>>();
    let verified_graph = verified
        .iec_circuit_graph()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung creation did not preserve the circuit graph",
        })?;
    if verified_rows.len() != rows.len() + 1
        || verified_records.len() != records.len() + 3
        || created.record_count != 3
        || created_kinds
            != [
                IecRecordKind::Contact(contact_code),
                IecRecordKind::LongWire,
                IecRecordKind::Coil(coil_code),
            ]
        || !verified_graph.power_components.iter().any(|component| {
            component.group_index == insertion_group
                && component.touches_left_rail
                && component.touches_right_rail
        })
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn delete_iec_ld_linear_rung_bytes(
    program: &LadderProgramData,
    row_index: u16,
    expected_contact_code: u8,
    expected_contact_variable: &str,
    expected_coil_code: u8,
    expected_coil_variable: &str,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let graph = program
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let row =
        rows.iter()
            .find(|row| row.row_index == row_index)
            .ok_or(XgwxError::InvalidLadderEdit {
                reason: "IEC simple-rung deletion requires a decoded stored row",
            })?;
    let group_rows = rows
        .iter()
        .filter(|candidate| candidate.group_index == row.group_index)
        .collect::<Vec<_>>();
    let row_records = records
        .iter()
        .filter(|record| record.group_index == row.group_index && record.row_index == row.row_index)
        .collect::<Vec<_>>();
    if group_rows != [row]
        || row.record_count != 3
        || row_records.len() != 3
        || row_records[0].kind != IecRecordKind::Contact(expected_contact_code)
        || row_records[1].kind != IecRecordKind::LongWire
        || row_records[2].kind != IecRecordKind::Coil(expected_coil_code)
        || !graph.power_components.iter().any(|component| {
            component.group_index == row.group_index
                && component.touches_left_rail
                && component.touches_right_rail
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC simple-rung deletion requires the captured contact-wire-coil group",
        });
    }

    let group_count = u16::from_le_bytes(
        program
            .data
            .get(6..8)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    let group_start = row
        .start
        .checked_sub(10)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group_flags = program.data.get(group_start + 4..group_start + 10);
    let native_layout = group_flags == Some([1, 0, 0, 0, 1, 0].as_slice());
    if group_count <= 1
        || usize::from(group_count) <= row.group_index
        || program.data.get(group_start..group_start + 4)
            != Some(
                u32::try_from(row.group_index)
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes()
                    .as_slice(),
            )
        || !(native_layout || group_flags == Some([0, 0, 0, 0, 1, 0].as_slice()))
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }

    let y = row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?
        .to_le_bytes();
    let mut expected_row = Vec::from(u32::from(row_index).to_le_bytes());
    expected_row.extend_from_slice(&[
        0xff, 0x43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x27, 0, 0, 0, 94, y[0], y[1], 0, 94, y[0],
        y[1], 0, 1, y[0], y[1], 0, 3, 0,
    ]);
    if program.data.get(row.start..row.records_start) != Some(expected_row.as_slice()) {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC simple-rung row header has changed",
        });
    }

    let expected_element =
        |code: u8, raw_x: u8, flags: [u8; 6], value: &str| -> Result<Vec<u8>, XgwxError> {
            let utf16 = value.encode_utf16().collect::<Vec<_>>();
            if utf16.is_empty()
                || utf16.len() > u8::MAX as usize
                || value.chars().any(char::is_control)
            {
                return Err(XgwxError::InvalidLadderEdit {
                    reason: "IEC rung variables must be 1 to 255 UTF-16 units without controls",
                });
            }
            let mut bytes = vec![
                0xff,
                code,
                0,
                0,
                0,
                raw_x,
                y[0],
                y[1],
                0,
                flags[0],
                flags[1],
                flags[2],
                flags[3],
                flags[4],
                flags[5],
                0xff,
                0xfe,
                0xff,
                utf16.len() as u8,
            ];
            for unit in utf16 {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            Ok(bytes)
        };
    let expected_contact = expected_element(
        expected_contact_code,
        1,
        [1, 0, if native_layout { 4 } else { 0 }, 0, 0, 0],
        expected_contact_variable,
    )?;
    let expected_wire = [
        0xff,
        0x02,
        0,
        0,
        0,
        4,
        y[0],
        y[1],
        0,
        0,
        0,
        if native_layout { 4 } else { 0 },
        0,
        0,
        0,
        91,
        y[0],
        y[1],
        0,
    ];
    let expected_coil = expected_element(
        expected_coil_code,
        94,
        [1, 0, if native_layout { 0x24 } else { 0x20 }, 0, 0, 0],
        expected_coil_variable,
    )?;
    if program.data.get(row_records[0].offset..row_records[0].end)
        != Some(expected_contact.as_slice())
        || program.data.get(row_records[1].offset..row_records[1].end)
            != Some(expected_wire.as_slice())
        || program.data.get(row_records[2].offset..row_records[2].end)
            != Some(expected_coil.as_slice())
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC simple-rung operands or records have changed",
        });
    }

    let group_starts = (0..usize::from(group_count))
        .map(|group_index| {
            rows.iter()
                .find(|row| row.group_index == group_index)
                .and_then(|row| row.start.checked_sub(10))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut updated = program.data.clone();
    updated[6..8].copy_from_slice(&(group_count - 1).to_le_bytes());
    for (group_index, &start) in group_starts.iter().enumerate().skip(row.group_index + 1) {
        updated[start..start + 4].copy_from_slice(
            &u32::try_from(group_index - 1)
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                .to_le_bytes(),
        );
    }
    updated.drain(group_start..row.end);

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung deletion did not preserve row framing",
        })?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung deletion did not preserve record framing",
        })?;
    let expected_rows = rows
        .iter()
        .filter(|candidate| candidate.group_index != row.group_index)
        .collect::<Vec<_>>();
    let row_shapes_match = expected_rows
        .iter()
        .zip(&verified_rows)
        .all(|(before, after)| {
            after.group_index
                == before.group_index - usize::from(before.group_index > row.group_index)
                && after.row_index == before.row_index
                && after.record_count == before.record_count
        });
    let verified_graph = verified
        .iec_circuit_graph()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC rung deletion did not preserve the circuit graph",
        })?;
    if verified_rows.len() + 1 != rows.len()
        || verified_records.len() + 3 != records.len()
        || verified_graph.edges.len() + 3 != graph.edges.len()
        || !row_shapes_match
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn delete_iec_ld_ff_branch_output_row_bytes(
    program: &LadderProgramData,
    program_index: usize,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2)
        || program.version.as_deref() != Some("LD VER 1.1")
        || (program_index, group_index, row_index) != (0, 9, 15)
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group = rows
        .iter()
        .filter(|row| row.group_index == group_index)
        .collect::<Vec<_>>();
    let [top, lower] = group.as_slice() else {
        return Err(XgwxError::UnsupportedLadderLayout);
    };
    let group_start = top
        .start
        .checked_sub(10)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    // The native capture covers this precise FF/output branch. Keep other
    // two-row function branches read-only until their Delete Line behavior is
    // captured too.
    if top.row_index != 14
        || lower.row_index != 15
        || lower.end - group_start != 375
        || xg_crc64(&program.data[group_start..lower.end]) != 0x46c7_e912_a64c_fa8d
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let upper = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == top.row_index)
        .collect::<Vec<_>>();
    let lower_records = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == lower.row_index)
        .collect::<Vec<_>>();
    if upper.len() != 6
        || lower_records.len() != 4
        || upper[0].kind != IecRecordKind::Contact(6)
        || upper[1].kind != IecRecordKind::FunctionBlock
        || upper[2].kind != IecRecordKind::LongWire
        || upper[3].kind != IecRecordKind::BranchStart
        || upper[4].kind != IecRecordKind::LongWire
        || upper[5].kind != IecRecordKind::Coil(14)
        || lower_records[0].kind != IecRecordKind::LinkReference(105)
        || lower_records[1].kind != IecRecordKind::BranchEnd
        || lower_records[2].kind != IecRecordKind::LongWire
        || lower_records[3].kind != IecRecordKind::Coil(14)
        || !program
            .iec_function_references()
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .iter()
            .any(|reference| {
                reference.record_offset == lower_records[0].offset
                    && reference.target_record_offset == upper[1].offset
            })
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }

    let mut without_row = program.data.clone();
    without_row[group_start + 8..group_start + 10].copy_from_slice(&1_u16.to_le_bytes());
    without_row[top.start + 17..top.start + 19].copy_from_slice(&0x32_u16.to_le_bytes());
    without_row[top.start + 29] = 4;
    without_row[top.start + 33..top.start + 35].copy_from_slice(&4_u16.to_le_bytes());
    // Drain from the end so record offsets in the original frame stay valid.
    without_row.drain(lower.start..lower.end);
    without_row.drain(upper[3].offset..upper[3].end);
    without_row.drain(upper[1].offset..upper[1].end);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    if verified.iec_row_frames().map(|items| items.len()) != Some(rows.len() - 1)
        || verified.iec_record_frames().map(|items| items.len()) != Some(records.len() - 6)
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
        || verified.iec_circuit_graph().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn delete_iec_ld_branch_top_row_bytes(
    program: &LadderProgramData,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let geometry = program
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let graph = program
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group_rows = rows
        .iter()
        .filter(|row| row.group_index == group_index)
        .collect::<Vec<_>>();
    if group_rows.len() != 2
        || group_rows[0].row_index != row_index
        || group_rows[1].row_index != row_index.saturating_add(1)
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC upper-line deletion requires a captured two-row branch group",
        });
    }
    let top = group_rows[0];
    let lower = group_rows[1];
    let top_records = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == row_index)
        .collect::<Vec<_>>();
    let lower_records = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == lower.row_index)
        .collect::<Vec<_>>();
    let starts = top_records
        .iter()
        .filter(|record| record.kind == IecRecordKind::BranchStart)
        .collect::<Vec<_>>();
    let ends = lower_records
        .iter()
        .filter(|record| record.kind == IecRecordKind::BranchEnd)
        .collect::<Vec<_>>();
    if starts.len() != 1
        || ends.len() != 1
        || top_records
            .last()
            .is_none_or(|record| !matches!(record.kind, IecRecordKind::Coil(_)))
        || !top_records.iter().all(|record| {
            matches!(
                record.kind,
                IecRecordKind::Contact(_)
                    | IecRecordKind::LongWire
                    | IecRecordKind::ShortWire
                    | IecRecordKind::BranchStart
                    | IecRecordKind::Coil(_)
            )
        })
        || lower_records.last().map(|record| record.offset) != Some(ends[0].offset)
        || !lower_records[..lower_records.len() - 1]
            .iter()
            .all(|record| matches!(record.kind, IecRecordKind::Contact(_)))
        || !geometry.vertical.iter().any(|connection| {
            connection.group_index == group_index
                && connection.start_offset == starts[0].offset
                && connection.end_offset == ends[0].offset
        })
        || geometry
            .vertical
            .iter()
            .filter(|connection| connection.group_index == group_index)
            .count()
            != 1
        || !graph.power_components.iter().any(|component| {
            component.group_index == group_index
                && component.touches_left_rail
                && component.touches_right_rail
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC upper-line deletion requires one terminal coil and a contact-only lower branch",
        });
    }
    let group_start = top
        .start
        .checked_sub(10)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if program.data.get(group_start + 8..group_start + 10) != Some(2_u16.to_le_bytes().as_slice())
        || program.data.get(lower.start + 33..lower.start + 35)
            != Some(
                u16::try_from(lower_records.len())
                    .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                    .to_le_bytes()
                    .as_slice(),
            )
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let mut without_top = program.data.clone();
    without_top[group_start + 8..group_start + 10].copy_from_slice(&1_u16.to_le_bytes());
    without_top[lower.start + 33..lower.start + 35].copy_from_slice(
        &u16::try_from(lower_records.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_top.drain(ends[0].offset..ends[0].end);
    without_top.drain(top.start..top.end);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_top.len();
    intermediate.data = without_top;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    verified
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified_rows.len() != rows.len() - 1
        || verified_records.len() != records.len() - top_records.len() - 1
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn delete_iec_ld_blank_row_bytes(
    program: &LadderProgramData,
    blank_row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let max_rows = u16::from_le_bytes(
        program
            .data
            .get(4..6)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    if blank_row_index == 0
        || blank_row_index >= max_rows
        || rows.iter().any(|row| row.row_index == blank_row_index)
        || !rows.iter().any(|row| row.row_index > blank_row_index)
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC blank-row deletion requires an empty row before a decoded later row",
        });
    }

    let mut updated = program.data.clone();
    updated[4..6].copy_from_slice(&(max_rows - 1).to_le_bytes());
    for row in rows.iter().filter(|row| row.row_index > blank_row_index) {
        let changed = row
            .row_index
            .checked_sub(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        updated[row.start..row.start + 4].copy_from_slice(&u32::from(changed).to_le_bytes());
        shift_iec_y_after(&mut updated, row.start + 22, blank_row_index, -1)?;
        shift_iec_y_after(&mut updated, row.start + 30, blank_row_index, -1)?;
    }
    for record in &records {
        match record.kind {
            IecRecordKind::LongWire => {
                shift_iec_y_after(&mut updated, record.offset + 6, blank_row_index, -1)?;
                shift_iec_y_after(&mut updated, record.offset + 16, blank_row_index, -1)?;
            }
            IecRecordKind::BranchStart => {
                shift_iec_y_after(&mut updated, record.offset + 8, blank_row_index, -1)?;
                shift_iec_y_after(&mut updated, record.offset + 18, blank_row_index, -1)?;
            }
            IecRecordKind::BranchEnd | IecRecordKind::LinkReference(_) => {
                shift_iec_y_after(&mut updated, record.offset + 6, blank_row_index, -1)?;
            }
            IecRecordKind::FunctionBlock => {
                shift_iec_function_block_rows(
                    &mut updated,
                    record.offset,
                    record.end,
                    record.row_index,
                    blank_row_index,
                    -1,
                )?;
            }
            IecRecordKind::ShortWire
            | IecRecordKind::Contact(_)
            | IecRecordKind::Coil(_)
            | IecRecordKind::Comment
            | IecRecordKind::FunctionOperand => {
                shift_iec_y_after(&mut updated, record.offset + 6, blank_row_index, -1)?;
            }
        }
    }

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC blank-row deletion did not preserve row framing",
        })?;
    if verified_rows.len() != rows.len()
        || rows.iter().zip(&verified_rows).any(|(before, after)| {
            after.group_index != before.group_index
                || after.start != before.start
                || after.end != before.end
                || after.record_count != before.record_count
                || after.row_index
                    != before.row_index - u16::from(before.row_index > blank_row_index)
        })
        || verified.iec_record_frames().is_none()
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

#[allow(clippy::too_many_arguments)]
fn edit_iec_ld_branch_segment_bytes(
    program: &LadderProgramData,
    group_index: usize,
    start_row_index: u16,
    end_row_index: u16,
    x: u8,
    expected: bool,
    present: bool,
) -> Result<Vec<u8>, XgwxError> {
    if program.project_type != Some(2)
        || program.version.as_deref() != Some("LD VER 1.1")
        || end_row_index != start_row_index.saturating_add(1)
        || !(3..=93).contains(&x)
        || !x.is_multiple_of(3)
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC branch needs adjacent rows and an x boundary from 3 through 93",
        });
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let geometry = program
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let start_row = rows
        .iter()
        .find(|row| row.group_index == group_index && row.row_index == start_row_index)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC branch start row is not in the selected group",
        })?;
    let end_row = rows
        .iter()
        .find(|row| row.group_index == group_index && row.row_index == end_row_index)
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "IEC branch end row is not in the selected group",
        })?;
    let connection = geometry.vertical.iter().find(|connection| {
        connection.group_index == group_index
            && connection.start_row_index == start_row_index
            && connection.end_row_index == end_row_index
            && connection.x == x
    });
    if connection.is_some() != expected {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC branch changed since selection",
        });
    }
    if expected == present {
        return Ok(program.data.clone());
    }

    let start_records = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == start_row_index)
        .collect::<Vec<_>>();
    let end_records = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == end_row_index)
        .collect::<Vec<_>>();
    let simple_record = |record: &&crate::IecRecordFrame| {
        matches!(
            record.kind,
            IecRecordKind::LongWire
                | IecRecordKind::ShortWire
                | IecRecordKind::Contact(_)
                | IecRecordKind::Coil(_)
                | IecRecordKind::BranchStart
                | IecRecordKind::BranchEnd
        )
    };
    if present
        && (!start_records.iter().all(simple_record) || !end_records.iter().all(simple_record))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC branch insertion is limited to decoded contact, coil, and wire rows",
        });
    }

    let mut updated = program.data.clone();
    let change_count = |bytes: &mut [u8], row: &crate::IecRowFrame, delta: i32| {
        let offset = row.start + 33;
        let count = u16::from_le_bytes(
            bytes
                .get(offset..offset + 2)
                .and_then(|value| value.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        let changed = if delta > 0 {
            count.checked_add(delta as u16)
        } else {
            count.checked_sub(delta.unsigned_abs() as u16)
        }
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
        bytes[offset..offset + 2].copy_from_slice(&changed.to_le_bytes());
        Ok::<(), XgwxError>(())
    };

    if present {
        change_count(&mut updated, start_row, 1)?;
        change_count(&mut updated, end_row, 1)?;
        let coordinate = |record: &&crate::IecRecordFrame| match record.kind {
            IecRecordKind::BranchStart => program.data.get(record.offset + 7).copied(),
            IecRecordKind::BranchEnd
            | IecRecordKind::LongWire
            | IecRecordKind::ShortWire
            | IecRecordKind::Contact(_)
            | IecRecordKind::Coil(_) => program.data.get(record.offset + 5).copied(),
            _ => None,
        };
        let start_insertion = start_records
            .iter()
            .take_while(|record| coordinate(record).is_some_and(|record_x| record_x <= x))
            .last()
            .map_or(start_row.records_start, |record| record.end);
        let end_insertion = end_records
            .iter()
            .find(|record| coordinate(record).is_some_and(|record_x| record_x >= x))
            .map_or(end_row.end, |record| record.offset);
        let source_y = start_row_index
            .checked_mul(4)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let target_y = end_row_index
            .checked_mul(4)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let mut branch_start = vec![0, 0, 0, 0, 0, 2, 0, x];
        branch_start.extend_from_slice(&source_y.to_le_bytes());
        branch_start.extend_from_slice(&[0; 7]);
        branch_start.push(x - 1);
        branch_start.extend_from_slice(&target_y.to_le_bytes());
        branch_start.extend_from_slice(&[0; 7]);
        debug_assert_eq!(branch_start.len(), 27);
        let mut branch_end = vec![1, 0, 0, 0, 0, x];
        branch_end.extend_from_slice(&source_y.to_le_bytes());
        branch_end.push(0);
        debug_assert_eq!(branch_end.len(), 9);
        for (offset, bytes) in [(end_insertion, branch_end), (start_insertion, branch_start)] {
            updated.splice(offset..offset, bytes);
        }
    } else {
        let connection = connection.expect("expected state checked above");
        let parallel_count = geometry
            .vertical
            .iter()
            .filter(|candidate| {
                candidate.group_index == group_index
                    && candidate.start_row_index == start_row_index
                    && candidate.end_row_index == end_row_index
            })
            .count();
        if parallel_count < 2 {
            return remove_iec_ld_final_branch_row(program, &rows, &records, &geometry, connection);
        }
        change_count(&mut updated, start_row, -1)?;
        change_count(&mut updated, end_row, -1)?;
        updated.drain(connection.end_offset..connection.end_offset + 9);
        updated.drain(connection.start_offset..connection.start_offset + 27);
    }

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let exists = verified_geometry.vertical.iter().any(|connection| {
        connection.group_index == group_index
            && connection.start_row_index == start_row_index
            && connection.end_row_index == end_row_index
            && connection.x == x
    });
    let expected_count = if present {
        geometry.vertical.len() + 1
    } else {
        geometry.vertical.len() - 1
    };
    if exists != present
        || verified_geometry.vertical.len() != expected_count
        || verified_records.len()
            != if present {
                records.len() + 2
            } else {
                records.len() - 2
            }
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn remove_iec_ld_final_branch_row(
    program: &LadderProgramData,
    rows: &[crate::IecRowFrame],
    records: &[crate::IecRecordFrame],
    geometry: &crate::IecGeometry,
    connection: &crate::IecVerticalConnection,
) -> Result<Vec<u8>, XgwxError> {
    let group_rows = rows
        .iter()
        .filter(|row| row.group_index == connection.group_index)
        .collect::<Vec<_>>();
    let start_row = group_rows
        .iter()
        .find(|row| row.row_index == connection.start_row_index)
        .copied()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let end_row = group_rows
        .iter()
        .find(|row| row.row_index == connection.end_row_index)
        .copied()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if group_rows.len() > 2 {
        return if end_row.row_index == group_rows.last().unwrap().row_index {
            remove_iec_ld_terminal_contact_branch_row(
                program,
                rows,
                records,
                geometry,
                connection,
                &group_rows,
            )
        } else {
            remove_iec_ld_middle_contact_branch_row(
                program,
                rows,
                records,
                geometry,
                connection,
                &group_rows,
            )
        };
    }
    if group_rows.len() != 2
        || group_rows[0].row_index != start_row.row_index
        || group_rows[1].row_index != end_row.row_index
        || geometry.vertical.iter().any(|candidate| {
            candidate.group_index == connection.group_index
                && candidate.start_offset != connection.start_offset
        })
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "removing this final IEC branch requires an unsupported row-group rebuild",
        });
    }

    let end_records = records
        .iter()
        .filter(|record| {
            record.group_index == connection.group_index && record.row_index == end_row.row_index
        })
        .collect::<Vec<_>>();
    let short_wire_branch_row = connection.x == 6
        && end_records.len() == 3
        && matches!(end_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        && end_records[1].kind == IecRecordKind::ShortWire
        && end_records[2].kind == IecRecordKind::BranchEnd
        && program.data[end_records[0].offset + 5] == 1
        && program.data[end_records[1].offset + 5] == 4
        && program.data[end_records[2].offset + 5] == 6;
    let output_branch_row = connection.x == 24
        && end_records.len() == 3
        && end_records[0].kind == IecRecordKind::BranchEnd
        && end_records[0].offset == connection.end_offset
        && end_records[1].kind == IecRecordKind::LongWire
        && matches!(end_records[2].kind, IecRecordKind::Coil(0x0e..=0x13))
        && [24, 25, 94]
            .iter()
            .zip(&end_records)
            .all(|(x, record)| program.data[record.offset + 5] == *x)
        && program.data[end_records[1].offset + 15] == 91;
    if end_records.len() < 2
        || (!output_branch_row
            && (end_records.last().map(|record| record.offset) != Some(connection.end_offset)
                || end_records.last().map(|record| record.kind) != Some(IecRecordKind::BranchEnd)
                || (!end_records[..end_records.len() - 1]
                    .iter()
                    .all(|record| matches!(record.kind, IecRecordKind::Contact(_)))
                    && !short_wire_branch_row)))
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "final IEC branch removal is limited to a captured contact-only branch row",
        });
    }

    let start_records = records
        .iter()
        .filter(|record| {
            record.group_index == connection.group_index && record.row_index == start_row.row_index
        })
        .collect::<Vec<_>>();
    let serial_continuation = (4..=6).contains(&start_records.len())
        && start_records.len().is_multiple_of(2)
        && start_records[2..start_records.len() - 1]
            .iter()
            .enumerate()
            .all(|(index, record)| {
                if index.is_multiple_of(2) {
                    record.kind == IecRecordKind::LongWire
                } else {
                    matches!(record.kind, IecRecordKind::Contact(0x06..=0x0b))
                }
            })
        && {
            let mut next_x = 4u8;
            start_records[2..start_records.len() - 1]
                .iter()
                .all(|record| match record.kind {
                    IecRecordKind::LongWire => {
                        let end_x = program.data[record.offset + 15];
                        let valid = program.data[record.offset + 5] == next_x && end_x >= next_x;
                        next_x = end_x.saturating_add(3);
                        valid
                    }
                    IecRecordKind::Contact(_) => {
                        let valid = program.data[record.offset + 5] == next_x;
                        next_x = next_x.saturating_add(3);
                        valid
                    }
                    _ => false,
                })
                && program.data[start_records.last().unwrap().offset + 5] == next_x
        };
    let adjacent_contact_continuation = start_records.len() == 5
        && matches!(start_records[2].kind, IecRecordKind::Contact(0x06..=0x0b))
        && start_records[3].kind == IecRecordKind::LongWire
        && program.data[start_records[2].offset + 5] == 4
        && program.data[start_records[3].offset + 5] == 7
        && program.data[start_records[3].offset + 15] == 91
        && program.data[start_records[4].offset + 5] == 94;
    let two_leading_contacts_continuation = connection.x == 6
        && start_records.len() == 8
        && end_records.len() == 3
        && matches!(start_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        && matches!(start_records[1].kind, IecRecordKind::Contact(0x06..=0x0b))
        && start_records[2].kind == IecRecordKind::BranchStart
        && start_records[3..6]
            .iter()
            .all(|record| matches!(record.kind, IecRecordKind::Contact(0x06..=0x0b)))
        && start_records[6].kind == IecRecordKind::LongWire
        && matches!(start_records[7].kind, IecRecordKind::Coil(0x0e..=0x13))
        && end_records[..2]
            .iter()
            .all(|record| matches!(record.kind, IecRecordKind::Contact(0x06..=0x0b)))
        && end_records[2].kind == IecRecordKind::BranchEnd
        && [1, 4, 7, 10, 13, 16, 94]
            .iter()
            .zip(
                start_records
                    .iter()
                    .filter(|record| record.kind != IecRecordKind::BranchStart),
            )
            .all(|(x, record)| program.data[record.offset + 5] == *x)
        && [1, 4]
            .iter()
            .zip(end_records.iter().take(2))
            .all(|(x, record)| program.data[record.offset + 5] == *x)
        && program.data[start_records[2].offset + 7] == 6
        && program.data[end_records[2].offset + 5] == 6
        && program.data[start_records[6].offset + 15] == 91;
    let short_wire_continuation = short_wire_branch_row
        && matches!(
            start_records.first().map(|record| record.kind),
            Some(IecRecordKind::Contact(0x06..=0x0b))
        )
        && start_records.get(1).map(|record| record.kind) == Some(IecRecordKind::ShortWire)
        && start_records.get(2).map(|record| record.kind) == Some(IecRecordKind::BranchStart)
        && start_records.get(3).map(|record| record.kind) == Some(IecRecordKind::ShortWire)
        && [1, 4, 7]
            .iter()
            .zip([0, 1, 3])
            .all(|(x, index)| program.data[start_records[index].offset + 5] == *x)
        && program.data[start_records[2].offset + 7] == 6
        && match start_records.as_slice() {
            [_, _, _, _, first, second, wire, coil] => {
                matches!(first.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && matches!(second.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && wire.kind == IecRecordKind::LongWire
                    && matches!(coil.kind, IecRecordKind::Coil(0x0e..=0x13))
                    && [10, 13, 16, 94]
                        .iter()
                        .zip([first, second, wire, coil])
                        .all(|(x, record)| program.data[record.offset + 5] == *x)
                    && program.data[wire.offset + 15] == 91
            }
            [_, _, _, _, gap, first, second, wire, coil] => {
                gap.kind == IecRecordKind::LongWire
                    && matches!(first.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && matches!(second.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && wire.kind == IecRecordKind::LongWire
                    && matches!(coil.kind, IecRecordKind::Coil(0x0e..=0x13))
                    && [10, 13, 16, 19, 94]
                        .iter()
                        .zip([gap, first, second, wire, coil])
                        .all(|(x, record)| program.data[record.offset + 5] == *x)
                    && program.data[gap.offset + 15] == 10
                    && program.data[wire.offset + 15] == 91
            }
            _ => false,
        };
    let output_branch_continuation = output_branch_row
        && start_records.len() == 11
        && [
            IecRecordKind::Contact(6),
            IecRecordKind::LongWire,
            IecRecordKind::ShortWire,
            IecRecordKind::LongWire,
            IecRecordKind::Contact(7),
            IecRecordKind::Contact(7),
            IecRecordKind::Contact(7),
            IecRecordKind::LongWire,
            IecRecordKind::BranchStart,
            IecRecordKind::LongWire,
            IecRecordKind::Coil(14),
        ]
        .iter()
        .zip(&start_records)
        .all(|(kind, record)| record.kind == *kind)
        && [1, 4, 7, 10, 13, 16, 19, 22, 25, 94]
            .iter()
            .zip(
                start_records
                    .iter()
                    .filter(|record| record.kind != IecRecordKind::BranchStart),
            )
            .all(|(x, record)| program.data[record.offset + 5] == *x)
        && program.data[start_records[8].offset + 7] == 24
        && start_records[8].offset == connection.start_offset
        && program.data[start_records[9].offset + 15] == 91
        && program.data[start_row.start + 29] == 22;
    if (connection.x == 3
        && (serial_continuation || adjacent_contact_continuation)
        && matches!(start_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        && start_records[1].kind == IecRecordKind::BranchStart
        && matches!(
            start_records.last().map(|record| record.kind),
            Some(IecRecordKind::Coil(0x0e..=0x13))
        )
        && end_records.len() == 2
        && matches!(end_records[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        && end_records[1].kind == IecRecordKind::BranchEnd
        && program.data[start_records[0].offset + 5] == 1
        && program.data[end_records[0].offset + 5] == 1)
        || two_leading_contacts_continuation
        || short_wire_continuation
        || output_branch_continuation
    {
        let mut without_branch = program.data.clone();
        without_branch[start_row.start - 2..start_row.start].copy_from_slice(&1u16.to_le_bytes());
        if output_branch_continuation {
            without_branch[start_row.start + 29] = 19;
        }
        without_branch[start_row.start + 33..start_row.start + 35].copy_from_slice(
            &u16::try_from(start_records.len() - 1)
                .map_err(|_| XgwxError::UnsupportedLadderLayout)?
                .to_le_bytes(),
        );
        without_branch.drain(end_row.start..end_row.end);
        without_branch.drain(connection.start_offset..connection.start_offset + 27);
        let mut intermediate = program.clone();
        intermediate.decoded_len = without_branch.len();
        intermediate.data = without_branch;
        intermediate
            .iec_circuit_graph()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let updated = delete_iec_ld_blank_row_bytes(&intermediate, end_row.row_index)?;
        let mut verified = program.clone();
        verified.decoded_len = updated.len();
        verified.data = updated.clone();
        verified
            .iec_circuit_graph()
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        return Ok(updated);
    }
    let wire = start_records
        .iter()
        .filter(|record| record.kind == IecRecordKind::LongWire)
        .find(|record| {
            program.data.get(record.offset + 5).copied() == Some(connection.x + 1)
                && program
                    .data
                    .get(record.offset + 15)
                    .is_some_and(|end_x| *end_x >= connection.x + 4)
        })
        .copied()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "final IEC branch removal needs the captured outgoing wire shape",
        })?;

    let mut updated = program.data.clone();
    let max_rows = u16::from_le_bytes(
        updated[4..6]
            .try_into()
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?,
    );
    updated[4..6].copy_from_slice(
        &max_rows
            .checked_sub(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    let group_count_offset = start_row
        .start
        .checked_sub(2)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    updated[group_count_offset..group_count_offset + 2].copy_from_slice(&1_u16.to_le_bytes());

    let deleted_row = end_row.row_index;
    let shift_y = |bytes: &mut [u8], offset: usize| -> Result<(), XgwxError> {
        let y = u16::from_le_bytes(
            bytes
                .get(offset..offset + 2)
                .and_then(|value| value.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        if y % 4 != 0 {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let row = y / 4;
        if row == deleted_row {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "another IEC record still references the branch row",
            });
        }
        if row > deleted_row {
            bytes[offset..offset + 2].copy_from_slice(&(y - 4).to_le_bytes());
        }
        Ok(())
    };

    for row in rows.iter().filter(|row| row.row_index > deleted_row) {
        let changed = row
            .row_index
            .checked_sub(1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        updated[row.start..row.start + 4].copy_from_slice(&u32::from(changed).to_le_bytes());
        shift_y(&mut updated, row.start + 22)?;
        shift_y(&mut updated, row.start + 30)?;
    }
    for record in records
        .iter()
        .filter(|record| record.row_index > deleted_row)
    {
        match record.kind {
            IecRecordKind::LongWire => {
                shift_y(&mut updated, record.offset + 6)?;
                shift_y(&mut updated, record.offset + 16)?;
            }
            IecRecordKind::BranchStart => {
                shift_y(&mut updated, record.offset + 8)?;
                shift_y(&mut updated, record.offset + 18)?;
            }
            IecRecordKind::BranchEnd | IecRecordKind::LinkReference(_) => {
                shift_y(&mut updated, record.offset + 6)?;
            }
            IecRecordKind::FunctionBlock => {
                shift_iec_function_block_rows(
                    &mut updated,
                    record.offset,
                    record.end,
                    record.row_index,
                    deleted_row,
                    -1,
                )?;
            }
            IecRecordKind::ShortWire
            | IecRecordKind::Contact(_)
            | IecRecordKind::Coil(_)
            | IecRecordKind::Comment
            | IecRecordKind::FunctionOperand => {
                shift_y(&mut updated, record.offset + 6)?;
            }
        }
    }

    updated[wire.offset + 5] = connection.x + 4;
    updated[wire.offset + 9..wire.offset + 15].fill(0);
    let source_y = connection
        .start_row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut short_wire = vec![0xff, 0x01, 0, 0, 0, connection.x + 1];
    short_wire.extend_from_slice(&source_y.to_le_bytes());
    short_wire.extend_from_slice(&[0, 0, 0, 4, 0, 0, 0]);
    debug_assert_eq!(short_wire.len(), 15);

    updated.drain(end_row.start..end_row.end);
    updated.splice(wire.offset..wire.offset, short_wire);
    updated.drain(connection.start_offset..connection.start_offset + 27);

    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "final IEC branch edit did not preserve row framing",
        })?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "final IEC branch edit did not preserve record framing",
        })?;
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::InvalidLadderEdit {
            reason: "final IEC branch edit did not preserve ladder geometry",
        })?;
    if verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
        || verified_rows.len() + 1 != rows.len()
        || verified_records.len() + 3 != records.len()
        || verified_geometry.vertical.len() + 1 != geometry.vertical.len()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn remove_iec_ld_terminal_contact_branch_row(
    program: &LadderProgramData,
    rows: &[crate::IecRowFrame],
    records: &[crate::IecRecordFrame],
    geometry: &crate::IecGeometry,
    connection: &crate::IecVerticalConnection,
    group_rows: &[&crate::IecRowFrame],
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = XgwxError::InvalidLadderEdit {
        reason: "removing this final IEC branch requires an unsupported row-group rebuild",
    };
    let [start_row, end_row] = group_rows[group_rows.len() - 2..] else {
        return Err(unsupported);
    };
    if !matches!(connection.x, 3 | 6)
        || connection.start_row_index != start_row.row_index
        || connection.end_row_index != end_row.row_index
        || group_rows.iter().any(|row| {
            records.iter().any(|record| {
                record.group_index == connection.group_index
                    && record.row_index == row.row_index
                    && matches!(
                        record.kind,
                        IecRecordKind::FunctionBlock
                            | IecRecordKind::LinkReference(_)
                            | IecRecordKind::FunctionOperand
                    )
            })
        })
        || geometry
            .vertical
            .iter()
            .filter(|branch| branch.group_index == connection.group_index)
            .count()
            != group_rows.len() - 1
        || group_rows.windows(2).any(|pair| {
            pair[1].row_index != pair[0].row_index + 1
                || !geometry.vertical.iter().any(|branch| {
                    branch.group_index == connection.group_index
                        && branch.start_row_index == pair[0].row_index
                        && branch.end_row_index == pair[1].row_index
                        && branch.x == connection.x
                })
        })
    {
        return Err(unsupported);
    }
    let upper = records
        .iter()
        .filter(|record| {
            record.group_index == connection.group_index && record.row_index == start_row.row_index
        })
        .collect::<Vec<_>>();
    let lower = records
        .iter()
        .filter(|record| {
            record.group_index == connection.group_index && record.row_index == end_row.row_index
        })
        .collect::<Vec<_>>();
    let single_contact = connection.x == 3;
    if upper.len() != if single_contact { 3 } else { 4 }
        || lower.len() != if single_contact { 2 } else { 3 }
        || !matches!(upper[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || (!single_contact
            && !matches!(
                upper[1].kind,
                IecRecordKind::Contact(0x06..=0x0b) | IecRecordKind::ShortWire
            ))
        || upper[upper.len() - 2].kind != IecRecordKind::BranchEnd
        || upper[upper.len() - 1].kind != IecRecordKind::BranchStart
        || !matches!(lower[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || (!single_contact
            && !matches!(
                lower[1].kind,
                IecRecordKind::Contact(0x06..=0x0b) | IecRecordKind::ShortWire
            ))
        || lower[lower.len() - 1].kind != IecRecordKind::BranchEnd
        || upper[upper.len() - 1].offset != connection.start_offset
        || lower[lower.len() - 1].offset != connection.end_offset
        || (if single_contact {
            [upper[0], upper[1], lower[0], lower[1]]
                .iter()
                .zip([1, 3, 1, 3])
                .any(|(record, x)| program.data[record.offset + 5] != x)
        } else {
            [upper[0], upper[1], upper[2], lower[0], lower[1], lower[2]]
                .iter()
                .zip([1, 4, 6, 1, 4, 6])
                .any(|(record, x)| program.data[record.offset + 5] != x)
        })
        || program.data[upper[upper.len() - 1].offset + 7] != connection.x
        || program.data[start_row.start + 29] != connection.x
    {
        return Err(unsupported);
    }
    let mut without_row = program.data.clone();
    without_row[group_rows[0].start - 2..group_rows[0].start].copy_from_slice(
        &u16::try_from(group_rows.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row[start_row.start + 29] = connection.x - 1;
    without_row[start_row.start + 33..start_row.start + 35].copy_from_slice(
        &u16::try_from(upper.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row.drain(end_row.start..end_row.end);
    without_row.drain(connection.start_offset..connection.start_offset + 27);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    intermediate
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, end_row.row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified_rows.len() + 1 != rows.len()
        || verified_records.len() + if single_contact { 3 } else { 4 } != records.len()
        || verified_geometry.vertical.len() + 1 != geometry.vertical.len()
        || verified.iec_circuit_graph().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn remove_iec_ld_middle_contact_branch_row(
    program: &LadderProgramData,
    rows: &[crate::IecRowFrame],
    records: &[crate::IecRecordFrame],
    geometry: &crate::IecGeometry,
    connection: &crate::IecVerticalConnection,
    group_rows: &[&crate::IecRowFrame],
) -> Result<Vec<u8>, XgwxError> {
    if connection.x == 3 {
        return remove_iec_ld_middle_single_contact_branch_row(
            program, rows, records, geometry, connection, group_rows,
        );
    }
    let unsupported = XgwxError::InvalidLadderEdit {
        reason: "removing this middle IEC branch requires an unsupported row-group rebuild",
    };
    let middle_index = group_rows
        .iter()
        .position(|row| row.row_index == connection.end_row_index)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if middle_index == 0 || middle_index + 1 >= group_rows.len() || connection.x != 6 {
        return Err(unsupported);
    }
    let upper_row = group_rows[middle_index - 1];
    let middle_row = group_rows[middle_index];
    let lower_row = group_rows[middle_index + 1];
    let upper_y = upper_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let middle_y = middle_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let lower_y = lower_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if connection.start_row_index != upper_row.row_index
        || group_rows.iter().any(|row| {
            records.iter().any(|record| {
                record.group_index == connection.group_index
                    && record.row_index == row.row_index
                    && matches!(
                        record.kind,
                        IecRecordKind::FunctionBlock
                            | IecRecordKind::LinkReference(_)
                            | IecRecordKind::FunctionOperand
                    )
            })
        })
        || geometry
            .vertical
            .iter()
            .filter(|branch| branch.group_index == connection.group_index)
            .count()
            != group_rows.len() - 1
        || group_rows.windows(2).any(|pair| {
            pair[1].row_index != pair[0].row_index + 1
                || !geometry.vertical.iter().any(|branch| {
                    branch.group_index == connection.group_index
                        && branch.start_row_index == pair[0].row_index
                        && branch.end_row_index == pair[1].row_index
                        && branch.x == 6
                })
        })
    {
        return Err(unsupported);
    }
    let row_records = |row_index| {
        records
            .iter()
            .filter(|record| {
                record.group_index == connection.group_index && record.row_index == row_index
            })
            .collect::<Vec<_>>()
    };
    let upper = row_records(upper_row.row_index);
    let middle = row_records(middle_row.row_index);
    let lower = row_records(lower_row.row_index);
    let upper_branch_index = if middle_index == 1 { 2 } else { 3 };
    let upper_shape = if middle_index == 1 {
        match upper.as_slice() {
            [first, second, branch, wire, coil] => {
                matches!(first.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && second.kind == IecRecordKind::ShortWire
                    && branch.kind == IecRecordKind::BranchStart
                    && wire.kind == IecRecordKind::LongWire
                    && coil.kind == IecRecordKind::Coil(14)
                    && [1, 4, 7, 94]
                        .iter()
                        .zip([first, second, wire, coil])
                        .all(|(x, record)| program.data[record.offset + 5] == *x)
            }
            [
                first,
                second,
                branch,
                short_wire,
                wire,
                contact,
                output_wire,
                coil,
            ] => {
                matches!(first.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && matches!(second.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && branch.kind == IecRecordKind::BranchStart
                    && short_wire.kind == IecRecordKind::ShortWire
                    && wire.kind == IecRecordKind::LongWire
                    && matches!(contact.kind, IecRecordKind::Contact(0x06..=0x0b))
                    && output_wire.kind == IecRecordKind::LongWire
                    && coil.kind == IecRecordKind::Coil(14)
                    && [1, 4, 7, 10, 19, 22, 94]
                        .iter()
                        .zip([first, second, short_wire, wire, contact, output_wire, coil])
                        .all(|(x, record)| program.data[record.offset + 5] == *x)
            }
            _ => false,
        }
    } else {
        upper.len() == 4
            && matches!(upper[0].kind, IecRecordKind::Contact(0x06..=0x0b))
            && matches!(
                upper[1].kind,
                IecRecordKind::Contact(0x06..=0x0b) | IecRecordKind::ShortWire
            )
            && upper[2].kind == IecRecordKind::BranchEnd
            && upper[3].kind == IecRecordKind::BranchStart
            && [1, 4, 6]
                .iter()
                .zip([upper[0], upper[1], upper[2]])
                .all(|(x, record)| program.data[record.offset + 5] == *x)
    };
    if !upper_shape || middle.len() != 4 || !(lower.len() == 3 || lower.len() == 4) {
        return Err(unsupported);
    }
    let upper_branch = upper[upper_branch_index];
    if program.data[upper_branch.offset + 7] != 6
        || !matches!(middle[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || !matches!(
            middle[1].kind,
            IecRecordKind::Contact(0x06..=0x0b) | IecRecordKind::ShortWire
        )
        || middle[2].kind != IecRecordKind::BranchEnd
        || middle[3].kind != IecRecordKind::BranchStart
        || !matches!(lower[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || !matches!(
            lower[1].kind,
            IecRecordKind::Contact(0x06..=0x0b) | IecRecordKind::ShortWire
        )
        || lower[2].kind != IecRecordKind::BranchEnd
        || (lower.len() == 4 && lower[3].kind != IecRecordKind::BranchStart)
        || upper_branch.offset != connection.start_offset
        || middle[2].offset != connection.end_offset
        || !geometry.vertical.iter().any(|branch| {
            branch.group_index == connection.group_index
                && branch.start_row_index == middle_row.row_index
                && branch.end_row_index == lower_row.row_index
                && branch.x == 6
                && branch.start_offset == middle[3].offset
                && branch.end_offset == lower[2].offset
        })
        || [1, 4, 6, 1, 4, 6]
            .iter()
            .zip([
                middle[0], middle[1], middle[2], lower[0], lower[1], lower[2],
            ])
            .any(|(x, record)| program.data[record.offset + 5] != *x)
        || program.data[middle_row.start + 29] != 6
        || program.data[upper_branch.offset + 18..upper_branch.offset + 20]
            != middle_y.to_le_bytes()
        || program.data[lower[2].offset + 6..lower[2].offset + 8] != middle_y.to_le_bytes()
    {
        return Err(unsupported);
    }
    let mut without_row = program.data.clone();
    without_row[group_rows[0].start - 2..group_rows[0].start].copy_from_slice(
        &u16::try_from(group_rows.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row[upper_branch.offset + 18..upper_branch.offset + 20]
        .copy_from_slice(&lower_y.to_le_bytes());
    without_row[lower[2].offset + 6..lower[2].offset + 8].copy_from_slice(&upper_y.to_le_bytes());
    without_row.drain(middle_row.start..middle_row.end);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, middle_row.row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_rows = verified
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_records = verified
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified_rows.len() + 1 != rows.len()
        || verified_records.len() + 4 != records.len()
        || verified_geometry.vertical.len() + 1 != geometry.vertical.len()
        || verified.iec_circuit_graph().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn remove_iec_ld_middle_nested_contact_branch_row(
    program: &LadderProgramData,
    rows: &[crate::IecRowFrame],
    records: &[crate::IecRecordFrame],
    geometry: &crate::IecGeometry,
    connection: &crate::IecVerticalConnection,
    group_rows: &[&crate::IecRowFrame],
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = XgwxError::InvalidLadderEdit {
        reason: "removing this middle IEC branch requires an unsupported row-group rebuild",
    };
    let middle_index = group_rows
        .iter()
        .position(|row| row.row_index == connection.end_row_index)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if middle_index == 0 || middle_index + 1 >= group_rows.len() {
        return Err(unsupported);
    }
    let upper_row = group_rows[middle_index - 1];
    let middle_row = group_rows[middle_index];
    let lower_row = group_rows[middle_index + 1];
    if connection.start_row_index != upper_row.row_index
        || upper_row.row_index.checked_add(1) != Some(middle_row.row_index)
        || middle_row.row_index.checked_add(1) != Some(lower_row.row_index)
        || records.iter().any(|record| {
            record.group_index == connection.group_index
                && matches!(
                    record.kind,
                    IecRecordKind::FunctionBlock
                        | IecRecordKind::LinkReference(_)
                        | IecRecordKind::FunctionOperand
                )
        })
    {
        return Err(unsupported);
    }
    let row_records = |row_index| {
        records
            .iter()
            .filter(|record| {
                record.group_index == connection.group_index && record.row_index == row_index
            })
            .collect::<Vec<_>>()
    };
    let upper = row_records(upper_row.row_index);
    let middle = row_records(middle_row.row_index);
    let lower = row_records(lower_row.row_index);
    let [end3, start3, end6, contact, end9] = middle.as_slice() else {
        return Err(unsupported);
    };
    if end3.kind != IecRecordKind::BranchEnd
        || start3.kind != IecRecordKind::BranchStart
        || end6.kind != IecRecordKind::BranchEnd
        || !matches!(contact.kind, IecRecordKind::Contact(0x06..=0x0b))
        || end9.kind != IecRecordKind::BranchEnd
        || lower.first().map(|record| record.kind) != Some(IecRecordKind::BranchEnd)
        || [end3, end6, contact, end9]
            .iter()
            .zip([3, 6, 7, 9])
            .any(|(record, x)| program.data[record.offset + 5] != x)
        || program.data[start3.offset + 7] != 3
        || program.data[lower[0].offset + 5] != 3
        || program.data[middle_row.start + 29] != 8
        || end3.offset != connection.end_offset
    {
        return Err(unsupported);
    }
    let upper_branches = [3, 6, 9].map(|x| {
        geometry.vertical.iter().find(|branch| {
            branch.group_index == connection.group_index
                && branch.start_row_index == upper_row.row_index
                && branch.end_row_index == middle_row.row_index
                && branch.x == x
        })
    });
    let [Some(upper3), Some(upper6), Some(upper9)] = upper_branches else {
        return Err(unsupported);
    };
    let lower_branch = geometry.vertical.iter().find(|branch| {
        branch.group_index == connection.group_index
            && branch.start_row_index == middle_row.row_index
            && branch.end_row_index == lower_row.row_index
            && branch.x == 3
    });
    let Some(lower_branch) = lower_branch else {
        return Err(unsupported);
    };
    if upper3.start_offset != connection.start_offset
        || upper3.end_offset != end3.offset
        || upper6.end_offset != end6.offset
        || upper9.end_offset != end9.offset
        || lower_branch.start_offset != start3.offset
        || lower_branch.end_offset != lower[0].offset
        || geometry.vertical.iter().any(|branch| {
            branch.group_index == connection.group_index
                && ((branch.start_row_index == upper_row.row_index
                    && branch.end_row_index == middle_row.row_index
                    && ![3, 6, 9].contains(&branch.x))
                    || (branch.start_row_index == middle_row.row_index
                        && branch.end_row_index == lower_row.row_index
                        && branch.x != 3))
        })
        || ![upper3, upper6, upper9].iter().all(|branch| {
            upper.iter().any(|record| {
                record.offset == branch.start_offset && record.kind == IecRecordKind::BranchStart
            })
        })
    {
        return Err(unsupported);
    }
    let upper_y = upper_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let lower_y = lower_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut without_row = program.data.clone();
    without_row[group_rows[0].start - 2..group_rows[0].start].copy_from_slice(
        &u16::try_from(group_rows.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row[upper_row.start + 33..upper_row.start + 35].copy_from_slice(
        &u16::try_from(upper.len() - 2)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row[upper3.start_offset + 18..upper3.start_offset + 20]
        .copy_from_slice(&lower_y.to_le_bytes());
    without_row[lower[0].offset + 6..lower[0].offset + 8].copy_from_slice(&upper_y.to_le_bytes());
    without_row.drain(middle_row.start..middle_row.end);
    for offset in [upper9.start_offset, upper6.start_offset] {
        without_row.drain(offset..offset + 27);
    }
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    intermediate
        .iec_circuit_graph()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, middle_row.row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified.iec_row_frames().is_none()
        || verified.iec_record_frames().map(|frames| frames.len() + 7) != Some(records.len())
        || verified_geometry.vertical.len() + 3 != geometry.vertical.len()
        || verified.iec_circuit_graph().is_none()
        || rows.len() != verified.iec_row_frames().unwrap().len() + 1
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn delete_iec_ld_chained_contact_branch_row_bytes(
    program: &LadderProgramData,
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    delete_iec_ld_chained_branch_row_bytes(program, group_index, row_index, true)
}

fn delete_iec_ld_chained_branch_row_bytes(
    program: &LadderProgramData,
    group_index: usize,
    row_index: u16,
    contact_only: bool,
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = XgwxError::InvalidLadderEdit {
        reason: "this IEC row is not a simple chained branch line",
    };
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return Err(unsupported);
    }
    let rows = program
        .iec_row_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let records = program
        .iec_record_frames()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let geometry = program
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let group_rows = rows
        .iter()
        .filter(|row| row.group_index == group_index)
        .collect::<Vec<_>>();
    let middle_index = group_rows
        .iter()
        .position(|row| row.row_index == row_index)
        .ok_or_else(|| XgwxError::InvalidLadderEdit {
            reason: "this IEC row is not a simple chained branch line",
        })?;
    if middle_index == 0 || middle_index + 1 == group_rows.len() {
        return Err(unsupported);
    }
    let (upper_row, middle_row, lower_row) = (
        group_rows[middle_index - 1],
        group_rows[middle_index],
        group_rows[middle_index + 1],
    );
    if upper_row.row_index.checked_add(1) != Some(row_index)
        || row_index.checked_add(1) != Some(lower_row.row_index)
    {
        return Err(unsupported);
    }
    let middle = records
        .iter()
        .filter(|record| record.group_index == group_index && record.row_index == row_index)
        .collect::<Vec<_>>();
    let (end, start) = match (contact_only, middle.as_slice()) {
        (true, [contact, end, start])
            if matches!(contact.kind, IecRecordKind::Contact(0x06..=0x0b))
                && program.data[contact.offset + 5] == 1 =>
        {
            (*end, *start)
        }
        (false, [end, start]) => (*end, *start),
        _ => return Err(unsupported),
    };
    let x = program.data[end.offset + 5];
    if end.kind != IecRecordKind::BranchEnd
        || start.kind != IecRecordKind::BranchStart
        || x < 3
        || x > 93
        || !x.is_multiple_of(3)
        || (contact_only && x != 3)
        || program.data[start.offset + 5] != 2
        || program.data[start.offset + 7] != x
        || program.data[middle_row.start + 29] != x
    {
        return Err(unsupported);
    }
    let incoming = geometry
        .vertical
        .iter()
        .find(|branch| {
            branch.group_index == group_index
                && branch.end_row_index == row_index
                && branch.x == x
                && branch.end_offset == end.offset
        })
        .ok_or_else(|| XgwxError::InvalidLadderEdit {
            reason: "this IEC row is not a simple chained branch line",
        })?;
    let outgoing = geometry
        .vertical
        .iter()
        .find(|branch| {
            branch.group_index == group_index
                && branch.start_row_index == row_index
                && branch.x == x
                && branch.start_offset == start.offset
        })
        .ok_or_else(|| XgwxError::InvalidLadderEdit {
            reason: "this IEC row is not a simple chained branch line",
        })?;
    if incoming.start_row_index != upper_row.row_index
        || outgoing.end_row_index != lower_row.row_index
        || geometry
            .vertical
            .iter()
            .filter(|branch| {
                branch.group_index == group_index
                    && (branch.start_row_index == row_index || branch.end_row_index == row_index)
            })
            .count()
            != 2
        || geometry.vertical.iter().any(|branch| {
            branch.group_index == group_index
                && (branch.start_row_index == row_index || branch.end_row_index == row_index)
                && branch.start_offset != start.offset
                && branch.end_offset != end.offset
        })
    {
        return Err(unsupported);
    }
    let upper_y = upper_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let lower_y = lower_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut without_row = program.data.clone();
    without_row[group_rows[0].start - 2..group_rows[0].start].copy_from_slice(
        &u16::try_from(group_rows.len() - 1)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    without_row[incoming.start_offset + 18..incoming.start_offset + 20]
        .copy_from_slice(&lower_y.to_le_bytes());
    without_row[outgoing.end_offset + 6..outgoing.end_offset + 8]
        .copy_from_slice(&upper_y.to_le_bytes());
    without_row.drain(middle_row.start..middle_row.end);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified.iec_row_frames().map(|frames| frames.len() + 1) != Some(rows.len())
        || verified
            .iec_record_frames()
            .map(|frames| frames.len() + middle.len())
            != Some(records.len())
        || verified_geometry.vertical.len() + 1 != geometry.vertical.len()
        || verified.iec_circuit_graph().is_none()
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn remove_iec_ld_middle_single_contact_branch_row(
    program: &LadderProgramData,
    rows: &[crate::IecRowFrame],
    records: &[crate::IecRecordFrame],
    geometry: &crate::IecGeometry,
    connection: &crate::IecVerticalConnection,
    group_rows: &[&crate::IecRowFrame],
) -> Result<Vec<u8>, XgwxError> {
    let unsupported = XgwxError::InvalidLadderEdit {
        reason: "removing this middle IEC branch requires an unsupported row-group rebuild",
    };
    let [upper_row, middle_row, lower_row] = group_rows else {
        return Err(unsupported);
    };
    let upper_y = upper_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let middle_y = middle_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let lower_y = lower_row
        .row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if connection.start_row_index != upper_row.row_index
        || connection.end_row_index != middle_row.row_index
        || middle_row.row_index.checked_add(1) != Some(lower_row.row_index)
        || geometry
            .vertical
            .iter()
            .filter(|branch| branch.group_index == connection.group_index)
            .count()
            != 2
        || !geometry.vertical.iter().any(|branch| {
            branch.group_index == connection.group_index
                && branch.start_row_index == middle_row.row_index
                && branch.end_row_index == lower_row.row_index
                && branch.x == 3
        })
    {
        return Err(unsupported);
    }
    let row_records = |row_index| {
        records
            .iter()
            .filter(|record| {
                record.group_index == connection.group_index && record.row_index == row_index
            })
            .collect::<Vec<_>>()
    };
    let upper = row_records(upper_row.row_index);
    let middle = row_records(middle_row.row_index);
    let lower = row_records(lower_row.row_index);
    if upper.len() != 6
        || middle.len() != 3
        || lower.len() != 2
        || !matches!(upper[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || upper[1].kind != IecRecordKind::BranchStart
        || !matches!(upper[2].kind, IecRecordKind::Contact(0x06..=0x0b))
        || !matches!(upper[3].kind, IecRecordKind::Contact(0x06..=0x0b))
        || upper[4].kind != IecRecordKind::LongWire
        || !matches!(upper[5].kind, IecRecordKind::Coil(0x0e..=0x13))
        || !matches!(middle[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || middle[1].kind != IecRecordKind::BranchEnd
        || middle[2].kind != IecRecordKind::BranchStart
        || !matches!(lower[0].kind, IecRecordKind::Contact(0x06..=0x0b))
        || lower[1].kind != IecRecordKind::BranchEnd
        || upper[1].offset != connection.start_offset
        || middle[1].offset != connection.end_offset
        || [
            upper[0], upper[2], upper[3], upper[4], upper[5], middle[0], middle[1], lower[0],
            lower[1],
        ]
        .iter()
        .zip([1, 4, 7, 10, 94, 1, 3, 1, 3])
        .any(|(record, x)| program.data[record.offset + 5] != x)
        || program.data[upper[1].offset + 7] != 3
        || program.data[middle[2].offset + 7] != 3
        || program.data[upper[4].offset + 15] != 91
        || program.data[middle_row.start + 29] != 3
        || program.data[upper[1].offset + 18..upper[1].offset + 20] != middle_y.to_le_bytes()
        || program.data[middle[1].offset + 6..middle[1].offset + 8] != upper_y.to_le_bytes()
        || program.data[middle[2].offset + 18..middle[2].offset + 20] != lower_y.to_le_bytes()
        || program.data[lower[1].offset + 6..lower[1].offset + 8] != middle_y.to_le_bytes()
    {
        return Err(unsupported);
    }
    let mut without_row = program.data.clone();
    without_row[upper_row.start - 2..upper_row.start].copy_from_slice(&2u16.to_le_bytes());
    without_row[upper[1].offset + 18..upper[1].offset + 20].copy_from_slice(&lower_y.to_le_bytes());
    without_row[lower[1].offset + 6..lower[1].offset + 8].copy_from_slice(&upper_y.to_le_bytes());
    without_row.drain(middle_row.start..middle_row.end);
    let mut intermediate = program.clone();
    intermediate.decoded_len = without_row.len();
    intermediate.data = without_row;
    let updated = delete_iec_ld_blank_row_bytes(&intermediate, middle_row.row_index)?;
    let mut verified = program.clone();
    verified.decoded_len = updated.len();
    verified.data = updated.clone();
    let verified_geometry = verified
        .iec_geometry()
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if verified.iec_row_frames().is_none()
        || verified.iec_record_frames().is_none()
        || verified.iec_record_frames().unwrap().len() + 3 != records.len()
        || verified_geometry.vertical.len() + 1 != geometry.vertical.len()
        || verified.iec_circuit_graph().is_none()
        || verified.iec_function_blocks().is_none()
        || verified.iec_function_references().is_none()
        || verified.iec_function_operand_links().is_none()
        || rows.len() != verified.iec_row_frames().unwrap().len() + 1
    {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    Ok(updated)
}

fn shift_iec_y_after(
    bytes: &mut [u8],
    offset: usize,
    boundary_row: u16,
    delta_rows: i16,
) -> Result<(), XgwxError> {
    let y = u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    if !y.is_multiple_of(4) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    if delta_rows < 0 && y / 4 == boundary_row {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC blank row is still referenced by a decoded coordinate",
        });
    }
    if y / 4 <= boundary_row {
        return Ok(());
    }
    let magnitude = delta_rows
        .unsigned_abs()
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let changed = if delta_rows >= 0 {
        y.checked_add(magnitude)
    } else {
        y.checked_sub(magnitude)
    }
    .ok_or(XgwxError::UnsupportedLadderLayout)?;
    bytes[offset..offset + 2].copy_from_slice(&changed.to_le_bytes());
    Ok(())
}

fn translate_iec_y(bytes: &mut [u8], offset: usize, delta_rows: i32) -> Result<(), XgwxError> {
    let y = u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .and_then(|value| value.try_into().ok())
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    if !y.is_multiple_of(4) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let changed = i32::from(y) + delta_rows * 4;
    bytes[offset..offset + 2].copy_from_slice(
        &u16::try_from(changed)
            .map_err(|_| XgwxError::UnsupportedLadderLayout)?
            .to_le_bytes(),
    );
    Ok(())
}

fn shift_iec_function_block_rows(
    bytes: &mut [u8],
    start: usize,
    end: usize,
    row_index: u16,
    boundary_row: u16,
    delta_rows: i16,
) -> Result<(), XgwxError> {
    shift_iec_function_block_coordinates(bytes, start, end, row_index, |bytes, offset| {
        shift_iec_y_after(bytes, offset, boundary_row, delta_rows)
    })
}

fn translate_iec_function_block_rows(
    bytes: &mut [u8],
    start: usize,
    end: usize,
    row_index: u16,
    delta_rows: i32,
) -> Result<(), XgwxError> {
    shift_iec_function_block_coordinates(bytes, start, end, row_index, |bytes, offset| {
        translate_iec_y(bytes, offset, delta_rows)
    })
}

fn shift_iec_function_block_coordinates(
    bytes: &mut [u8],
    start: usize,
    end: usize,
    row_index: u16,
    mut shift: impl FnMut(&mut [u8], usize) -> Result<(), XgwxError>,
) -> Result<(), XgwxError> {
    let block = bytes
        .get(start..end)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let pin_count = usize::from(*block.get(20).ok_or(XgwxError::UnsupportedLadderLayout)?);
    let row_y = row_index
        .checked_mul(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    if block.get(6..8) != Some(&row_y.to_le_bytes())
        || block.get(27..29) != Some(&row_y.to_le_bytes())
    {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC row shift found an unsupported function block coordinate layout",
        });
    }

    let mut cursor = 38;
    cursor = skip_iec_function_pin_descriptor(block, cursor)?;
    cursor = skip_iec_function_pin_descriptor(block, cursor)?;
    cursor = skip_iec_function_marker(block, cursor)?;
    cursor = skip_iec_function_marker(block, cursor)?;
    let mut pin_y_offsets = Vec::with_capacity(pin_count);
    for pin in 0..pin_count {
        let expected_y = row_y
            .checked_add(
                4 + u16::try_from(pin).map_err(|_| XgwxError::UnsupportedLadderLayout)? * 4,
            )
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        let header = block
            .get(cursor..cursor + 12)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        if header[0] != block[5]
            || header[1..3] != expected_y.to_le_bytes()
            || header[3..6] != [0; 3]
            || header[8..10] != [0; 2]
            || u16::from_le_bytes([header[10], header[11]]) != u16::from(pin + 1 < pin_count) * 2
        {
            return Err(XgwxError::InvalidLadderEdit {
                reason: "IEC row shift found an unsupported function block pin header",
            });
        }
        pin_y_offsets.push(cursor + 1);
        cursor += 12;
        if pin + 1 < pin_count {
            cursor = skip_iec_function_pin_descriptor(block, cursor)?;
            cursor = skip_iec_function_pin_descriptor(block, cursor)?;
        }
    }
    if cursor != block.len() {
        return Err(XgwxError::UnsupportedLadderLayout);
    }

    shift(bytes, start + 6)?;
    shift(bytes, start + 27)?;
    for offset in pin_y_offsets {
        shift(bytes, start + offset)?;
    }
    Ok(())
}

fn skip_iec_function_marker(block: &[u8], cursor: usize) -> Result<usize, XgwxError> {
    if block.get(cursor..cursor + 3) != Some(&[0xff, 0xfe, 0xff]) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let units = usize::from(
        *block
            .get(cursor + 3)
            .ok_or(XgwxError::UnsupportedLadderLayout)?,
    );
    let end = cursor
        .checked_add(4 + units * 2)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    block
        .get(cursor..end)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    Ok(end)
}

fn skip_iec_function_pin_descriptor(block: &[u8], cursor: usize) -> Result<usize, XgwxError> {
    let type_end = skip_iec_function_marker(block, cursor)?;
    let name_start = type_end
        .checked_add(4)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let name_end = skip_iec_function_marker(block, name_start)?;
    let end = name_end
        .checked_add(5)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    block
        .get(name_end..end)
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    Ok(end)
}

/// Position the captured L1 group using the row and group fields observed in
/// native XG5000 L1 and L26 insertions. Offsets refer to the unedited 390-byte
/// template, so text substitution must happen afterward.
fn position_iec_standalone_word_to_udint_group(
    captured: &[u8],
    group_index: usize,
    row_index: u16,
) -> Result<Vec<u8>, XgwxError> {
    if captured.len() != 390 || captured.get(..10) != Some(&[1, 0, 0, 0, 0, 0, 0, 0, 3, 0]) {
        return Err(XgwxError::UnsupportedLadderLayout);
    }
    let group = u32::try_from(group_index).map_err(|_| XgwxError::UnsupportedLadderLayout)?;
    let row_delta = row_index
        .checked_sub(1)
        .and_then(|value| value.checked_mul(4))
        .ok_or(XgwxError::UnsupportedLadderLayout)?;
    let mut bytes = captured.to_vec();
    bytes[..4].copy_from_slice(&group.to_le_bytes());
    for (offset, original) in [(10, 1u16), (248, 2), (346, 3)] {
        if bytes.get(offset..offset + 4) != Some(&(u32::from(original)).to_le_bytes()) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let positioned = row_index
            .checked_add(original - 1)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        bytes[offset..offset + 4].copy_from_slice(&u32::from(positioned).to_le_bytes());
    }
    for offset in [
        0x20, 0x24, 0x28, 0x33, 0x3d, 0x46, 0x5b, 0xb5, 0xed, 0x10e, 0x112, 0x116, 0x121, 0x140,
        0x149, 0x170, 0x174, 0x178, 0x183,
    ] {
        let original = u16::from_le_bytes(
            bytes
                .get(offset..offset + 2)
                .and_then(|slice| slice.try_into().ok())
                .ok_or(XgwxError::UnsupportedLadderLayout)?,
        );
        if !matches!(original, 4 | 8 | 12) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        let positioned = original
            .checked_add(row_delta)
            .ok_or(XgwxError::UnsupportedLadderLayout)?;
        bytes[offset..offset + 2].copy_from_slice(&positioned.to_le_bytes());
    }
    Ok(bytes)
}

fn replace_iec_ld_text_bytes(
    payload: &[u8],
    program_index: usize,
    offset: usize,
    expected: &str,
    replacement: &str,
) -> Result<Vec<u8>, XgwxError> {
    let replacement_units = replacement.encode_utf16().collect::<Vec<_>>();
    if replacement_units.len() > u8::MAX as usize {
        return Err(XgwxError::InvalidLadderEdit {
            reason: "IEC text exceeds 255 UTF-16 units",
        });
    }
    let length_offset = offset + UTF16_MARKER.len();
    let text_start = length_offset + 1;
    let text_end = text_start + expected.encode_utf16().count() * 2;
    if payload.get(offset..length_offset) != Some(UTF16_MARKER)
        || payload.get(length_offset).copied() != Some(expected.encode_utf16().count() as u8)
        || payload
            .get(text_start..text_end)
            .and_then(decode_utf16_bytes)
            .as_deref()
            != Some(expected)
    {
        return Err(XgwxError::LadderCellChanged {
            program_index,
            offset,
        });
    }
    let mut updated = Vec::with_capacity(payload.len() + replacement_units.len() * 2);
    updated.extend_from_slice(&payload[..length_offset]);
    updated.push(replacement_units.len() as u8);
    for unit in &replacement_units {
        updated.extend_from_slice(&unit.to_le_bytes());
    }
    updated.extend_from_slice(&payload[text_end..]);
    Ok(updated)
}

fn encode_payload_text(
    original_text: &str,
    compressed: bool,
    payload: &[u8],
) -> Result<String, XgwxError> {
    let raw = if compressed {
        let mut encoder = BzEncoder::new(Vec::new(), Bzip2Compression::best());
        encoder.write_all(payload).map_err(XgwxError::Bzip2)?;
        encoder.finish().map_err(XgwxError::Bzip2)?
    } else {
        payload.to_vec()
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
    Ok(format_base64_like(original_text, &encoded))
}

fn format_base64_like(original: &str, encoded: &str) -> String {
    let prefix_len = original
        .bytes()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(original.len());
    let suffix_start = original
        .bytes()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(original.len(), |index| index + 1);
    let prefix = &original[..prefix_len];
    let suffix = &original[suffix_start..];

    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let continuation_indent = original
        .split_inclusive(newline)
        .nth(1)
        .map(|line| line.trim_end_matches(['\r', '\n']))
        .map(|line| {
            line.chars()
                .take_while(|character| character.is_whitespace())
                .collect::<String>()
        })
        .unwrap_or_default();

    let mut formatted = String::with_capacity(prefix.len() + encoded.len() + suffix.len());
    formatted.push_str(prefix);
    for (index, chunk) in encoded.as_bytes().chunks(76).enumerate() {
        if index > 0 {
            formatted.push_str(newline);
            formatted.push_str(&continuation_indent);
        }
        formatted.push_str(std::str::from_utf8(chunk).expect("base64 is ASCII"));
    }
    formatted.push_str(suffix);
    formatted
}

fn assemble_workspace(header: &[u8], main_gzip: &[u8], trailer: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(header.len() + main_gzip.len() + trailer.len());
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(main_gzip);
    bytes.extend_from_slice(trailer);
    bytes
}
