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

    /// Replace one decoded ladder cell string while preserving binary layout.
    ///
    /// The replacement must contain exactly as many UTF-16 code units as the
    /// original string. This keeps every proprietary record offset and all
    /// undecoded topology bytes stable; only the string bytes and compression
    /// envelopes are regenerated.
    pub fn update_ladder_cell_text(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        let expected_units = expected.encode_utf16().count();
        let replacement_units = replacement.encode_utf16().count();
        if replacement_units != expected_units {
            return Err(XgwxError::LadderCellLengthChanged {
                expected_utf16_units: expected_units,
                actual_utf16_units: replacement_units,
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

        for (chunk, unit) in payload[text_start..text_end]
            .chunks_exact_mut(2)
            .zip(replacement.encode_utf16())
        {
            chunk.copy_from_slice(&unit.to_le_bytes());
        }

        let replacement_text = encode_payload_text(original_text, compressed, &payload)?;
        self.apply_xml_replacements(vec![(text_node.range(), replacement_text)])
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
            || data.attribute("ProjectType") != Some("1")
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
    let Some(offset) = find_gzip_member(trailer, 0) else {
        return Ok(());
    };
    let member = parse_gzip_member(trailer, offset)?;
    if member.data.starts_with(XG_FRAME_MAGIC) && !validate_xg_frame(&member.data) {
        return Err(XgwxError::AuthenticatedRewriteUnsupported);
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
