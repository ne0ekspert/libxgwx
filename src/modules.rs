#![deny(missing_docs)]
//! Owned, read-only module views built from the existing XML summaries.
use crate::{
    Base, CnetConfigInfoSummary, FenetConfigInfoSummary, ModuleSummary, NetworkModuleSummary,
    XgpdConfigInfoSummary, XgwxDocument, XmlElement,
};

/// Common metadata for an installed hardware module, distinct from a PLC rack/base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareModule {
    /// Owning PLC configuration in `XgwxDocument::configurations()` order.
    /// `None` means the hardware record is outside a Configuration element.
    pub configuration_index: Option<usize>,
    /// Installed hardware summary with its native attributes and current location.
    pub hardware: ModuleSummary,
}

/// Verified I/O categories. Unknown module identities are never guessed from names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOKind {
    /// Digital input points.
    DigitalInput,
    /// Digital output points.
    DigitalOutput,
    /// Combined digital input and output points.
    DigitalInputOutput,
    /// Analog input channels.
    AnalogInput,
    /// Analog output channels.
    AnalogOutput,
    /// Combined analog input and output channels.
    AnalogInputOutput,
    /// Temperature measurement input channels.
    TemperatureInput,
}

/// An installed module whose numeric identity has a known I/O classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IOModule {
    /// Common installed hardware metadata and owning configuration scope.
    pub base: HardwareModule,
    /// I/O category established by the known numeric module identity.
    pub kind: IOKind,
}

/// A network membership record, its installed hardware, and protocol settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkModule {
    /// Installed hardware, when uniquely associated within the configuration.
    /// Built-in communication interfaces may have no separate hardware record.
    pub base: Option<HardwareModule>,
    /// Candidates retained when hardware or configuration scope is ambiguous.
    pub hardware_candidates: Vec<HardwareModule>,
    /// Native network membership record, including protocol identity and current location.
    pub metadata: NetworkModuleSummary,
    /// Uniquely resolved owning configuration, when available.
    pub configuration_index: Option<usize>,
    /// Resolved, missing, or ambiguous association with protocol settings.
    pub configuration: ConfigurationMatch,
}

impl NetworkModule {
    /// Borrow uniquely resolved Cnet settings. Missing, ambiguous, and other
    /// protocol configurations return `None`; inspect `configuration` for details.
    pub fn cnet_config_info(&self) -> Option<&CnetConfigInfoSummary> {
        match &self.configuration {
            ConfigurationMatch::Matched(NetworkConfiguration::Cnet(config)) => Some(config),
            _ => None,
        }
    }

    /// Borrow uniquely resolved FEnet settings. Missing, ambiguous, and other
    /// protocol configurations return `None`; inspect `configuration` for details.
    pub fn fenet_config_info(&self) -> Option<&FenetConfigInfoSummary> {
        match &self.configuration {
            ConfigurationMatch::Matched(NetworkConfiguration::Fenet(config)) => Some(config),
            _ => None,
        }
    }
}

/// Result of associating a network record with its protocol settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationMatch {
    /// No matching protocol settings were found.
    Missing,
    /// Exactly one protocol configuration matched this network membership.
    Matched(NetworkConfiguration),
    /// Multiple settings or multiple possible owning configurations.
    /// The list can be empty if scope is ambiguous but no settings match.
    Ambiguous(Vec<NetworkConfiguration>),
}

/// Decoded protocol settings, including their preserved raw XML attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkConfiguration {
    /// Decoded FEnet Ethernet settings and their raw native attributes.
    Fenet(Box<FenetConfigInfoSummary>),
    /// Decoded Cnet serial-port settings and their raw native attributes.
    Cnet(CnetConfigInfoSummary),
    /// Preserved generic XGPD settings for another protocol.
    Other(XgpdConfigInfoSummary),
}

/// A unified view retaining both recognized and unclassified installed hardware,
/// plus network records that have no unique installed hardware association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Module {
    /// Installed hardware with a verified I/O category.
    Io(IOModule),
    /// Network membership with its hardware and configuration association.
    Network(Box<NetworkModule>),
    /// Installed hardware whose identity is not classified as I/O or matched to a network.
    Other(HardwareModule),
}

impl XgwxDocument {
    /// List installed hardware in document order with its configuration scope.
    pub fn hardware_modules(&self) -> Vec<HardwareModule> {
        Inventory::new(&self.root).hardware
    }

    /// List confirmed I/O modules. Use `modules` to retain unknown hardware.
    /// Classification uses known numeric `(Id, SubType)` pairs, not display names.
    pub fn io_modules(&self) -> Vec<IOModule> {
        self.hardware_modules()
            .into_iter()
            .filter_map(as_io)
            .collect()
    }

    /// List network memberships with configuration-scoped settings associations.
    ///
    /// XML ancestry takes precedence; otherwise `ConfigName` selects a PLC
    /// configuration by name. A missing name uses the only configuration, if
    /// there is one. Duplicate names or multiple unnamed choices stay ambiguous.
    /// Settings match the current type/id, base, slot and, when both are present,
    /// subtype. Location values are file-local and are not persistent identities.
    /// Unknown protocol layouts remain visible through `network_modules`/`root`.
    pub fn resolved_network_modules(&self) -> Vec<NetworkModule> {
        Inventory::new(&self.root).resolved_networks()
    }

    /// List installed hardware in document order, then unmatched network records.
    /// Raw hardware records are available through `module_summaries`. Duplicate network memberships are
    /// retained, rather than discarding additional references to the same hardware.
    ///
    /// ```no_run
    /// use xgwx::{Module, XgwxDocument};
    /// let doc = XgwxDocument::from_path("project.xgwx")?;
    /// for module in doc.modules() {
    ///     match module {
    ///         Module::Io(io) => println!("{:?}: {:?}", io.kind, io.base.hardware.name),
    ///         Module::Network(network) => println!("{:#?}", network.configuration),
    ///         Module::Other(hardware) => println!("{:?}", hardware.hardware.name),
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn modules(&self) -> Vec<Module> {
        Inventory::new(&self.root)
            .scoped_modules()
            .into_iter()
            .map(|(_, module)| module)
            .collect()
    }
}

pub(crate) fn base_handles(doc: &XgwxDocument) -> Vec<Base<'_>> {
    Inventory::new(&doc.root)
        .bases
        .into_iter()
        .enumerate()
        .map(|(index, (configuration_index, element))| {
            Base::new(doc, index, configuration_index, element)
        })
        .collect()
}

pub(crate) fn base_modules(doc: &XgwxDocument, index: usize) -> Vec<Module> {
    Inventory::new(&doc.root)
        .scoped_modules()
        .into_iter()
        .filter_map(|(owner, module)| (owner == Some(index)).then_some(module))
        .collect()
}

struct Inventory<'a> {
    names: Vec<Option<String>>,
    bases: Vec<(Option<usize>, &'a XmlElement)>,
    hardware: Vec<HardwareModule>,
    hardware_bases: Vec<Option<usize>>,
    networks: Vec<(Option<usize>, &'a XmlElement)>,
    network_bases: Vec<Option<usize>>,
    settings: Vec<(Option<usize>, NetworkConfiguration)>,
}

impl<'a> Inventory<'a> {
    fn scoped_modules(&self) -> Vec<(Option<usize>, Module)> {
        let networks = self.resolved_networks();
        let mut used = vec![false; networks.len()];
        let mut modules = Vec::new();
        for (hardware_index, hardware) in self.hardware.iter().enumerate() {
            let owner = self.base_owner(
                self.hardware_bases[hardware_index],
                hardware.configuration_index,
                hardware.hardware.base,
            );
            let network = networks
                .iter()
                .enumerate()
                .find(|(index, network)| !used[*index] && network.base.as_ref() == Some(hardware));
            if let Some((index, network)) = network {
                used[index] = true;
                modules.push((owner, Module::Network(Box::new(network.clone()))));
            } else if let Some(io) = as_io(hardware.clone()) {
                modules.push((owner, Module::Io(io)));
            } else {
                modules.push((owner, Module::Other(hardware.clone())));
            }
        }
        modules.extend(networks.into_iter().zip(used).enumerate().filter_map(
            |(index, (network, used))| {
                if used {
                    return None;
                }
                let hardware_owner = network.base.as_ref().and_then(|hardware| {
                    self.hardware
                        .iter()
                        .position(|candidate| candidate == hardware)
                        .and_then(|i| {
                            self.base_owner(
                                self.hardware_bases[i],
                                hardware.configuration_index,
                                hardware.hardware.base,
                            )
                        })
                });
                let owner = hardware_owner.or_else(|| {
                    // `None` may mean ambiguous PLC scope. Resolve location
                    // only when the same scope rule yields one owning PLC.
                    let scopes = self.scopes(self.networks[index].0, &network.metadata);
                    if scopes.len() != 1 {
                        return self.network_bases[index];
                    }
                    self.base_owner(self.network_bases[index], scopes[0], network.metadata.base)
                });
                Some((owner, Module::Network(Box::new(network))))
            },
        ));
        modules
    }
    fn base_owner(
        &self,
        explicit: Option<usize>,
        scope: Option<usize>,
        number: Option<u32>,
    ) -> Option<usize> {
        if explicit.is_some() {
            return explicit;
        }
        let number = number?;
        let mut candidates = self
            .bases
            .iter()
            .enumerate()
            .filter(|(_, (owner, element))| {
                *owner == scope && crate::attr_u32(element, "Base") == Some(number)
            })
            .map(|(index, _)| index);
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }

    fn new(root: &'a XmlElement) -> Self {
        let mut inventory = Self {
            names: Vec::new(),
            bases: Vec::new(),
            hardware: Vec::new(),
            hardware_bases: Vec::new(),
            networks: Vec::new(),
            network_bases: Vec::new(),
            settings: Vec::new(),
        };
        inventory.visit(root, None, None);
        inventory
    }

    fn visit(
        &mut self,
        element: &'a XmlElement,
        mut scope: Option<usize>,
        mut base_scope: Option<usize>,
    ) {
        if element.name == "Configuration" {
            scope = Some(self.names.len());
            self.names.push(crate::text_value(element));
            base_scope = None;
        }
        if element.name == "Base" {
            base_scope = Some(self.bases.len());
            self.bases.push((scope, element));
        }
        match element.name.as_str() {
            "Module" => {
                self.hardware_bases.push(base_scope);
                self.hardware.push(HardwareModule {
                    configuration_index: scope,
                    hardware: ModuleSummary::from_element(element),
                });
            }
            "NetworkModule" => {
                self.network_bases.push(base_scope);
                self.networks.push((scope, element));
            }
            "XGPD_CONFIG_INFO_FENET" => self.settings.push((
                scope,
                NetworkConfiguration::Fenet(Box::new(FenetConfigInfoSummary::from_element(
                    element,
                ))),
            )),
            "XGPD_CONFIG_INFO_CNET" => self.settings.push((
                scope,
                NetworkConfiguration::Cnet(CnetConfigInfoSummary::from_element(element)),
            )),
            "XGPD_CONFIG_INFO_DNET" | "XGPD_CONFIG_INFO_FDENET" | "XGPD_CONFIG_INFO_RNET" => {
                self.settings.push((
                    scope,
                    NetworkConfiguration::Other(XgpdConfigInfoSummary::from_element(element)),
                ));
            }
            _ => {}
        }
        for child in &element.children {
            self.visit(child, scope, base_scope);
        }
    }

    fn scopes(&self, owner: Option<usize>, module: &NetworkModuleSummary) -> Vec<Option<usize>> {
        if let Some(owner) = owner {
            return vec![Some(owner)];
        }
        if let Some(name) = module
            .config_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
        {
            return self
                .names
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.as_deref() == Some(name))
                .map(|(index, _)| Some(index))
                .collect();
        }
        if self.names.is_empty() {
            vec![None]
        } else {
            (0..self.names.len()).map(Some).collect()
        }
    }

    fn resolved_networks(&self) -> Vec<NetworkModule> {
        self.networks
            .iter()
            .map(|(owner, element)| {
                let metadata = NetworkModuleSummary::from_element(element);
                let scopes = self.scopes(*owner, &metadata);
                let hardware: Vec<_> = self
                    .hardware
                    .iter()
                    .filter(|hardware| {
                        scopes.contains(&hardware.configuration_index)
                            && identity_matches(
                                &metadata,
                                hardware.hardware.id,
                                hardware.hardware.base,
                                hardware.hardware.slot,
                                hardware.hardware.sub_type,
                            )
                    })
                    .cloned()
                    .collect();
                let settings: Vec<_> = self
                    .settings
                    .iter()
                    .filter(|(scope, config)| {
                        let (id, base, slot, subtype) = match config {
                            NetworkConfiguration::Fenet(c) => {
                                (c.type_code, c.base, c.slot, c.sub_type)
                            }
                            NetworkConfiguration::Cnet(c) => {
                                (c.type_code, c.base, c.slot, c.sub_type)
                            }
                            NetworkConfiguration::Other(c) => {
                                (c.type_code, c.base, c.slot, c.sub_type)
                            }
                        };
                        scopes.contains(scope)
                            && identity_matches(&metadata, id, base, slot, subtype)
                    })
                    .map(|(_, config)| config.clone())
                    .collect();

                let configuration = if scopes.len() > 1 || settings.len() > 1 {
                    ConfigurationMatch::Ambiguous(settings)
                } else if let Some(setting) = settings.into_iter().next() {
                    ConfigurationMatch::Matched(setting)
                } else {
                    ConfigurationMatch::Missing
                };
                let base = (scopes.len() == 1 && hardware.len() == 1).then(|| hardware[0].clone());
                let hardware_candidates = if base.is_none() { hardware } else { Vec::new() };
                NetworkModule {
                    base,
                    hardware_candidates,
                    metadata,
                    configuration_index: if scopes.len() == 1 { scopes[0] } else { None },
                    configuration,
                }
            })
            .collect()
    }
}

fn identity_matches(
    module: &NetworkModuleSummary,
    id: Option<u32>,
    base: Option<u32>,
    slot: Option<u32>,
    subtype: Option<u32>,
) -> bool {
    let Some(key) = module.id.zip(module.base).zip(module.slot) else {
        return false;
    };
    id.zip(base).zip(slot) == Some(key)
        && module.option_type.zip(subtype).is_none_or(|(a, b)| a == b)
}

fn as_io(hardware: HardwareModule) -> Option<IOModule> {
    let identity = hardware.hardware.id.zip(hardware.hardware.sub_type)?;
    let kind = IO_IDENTITIES
        .iter()
        .find(|(id, subtype, _)| (*id, *subtype) == identity)?
        .2;
    Some(IOModule {
        base: hardware,
        kind,
    })
}

// Read-only classification extracted from the native categories in catalog_data.rs.
// Keep this small table available without enabling the writer/catalog machinery.
// A catalog consistency test checks these identities when `write` is enabled.
include!("io_module_identities.rs");

#[cfg(all(test, feature = "write"))]
mod tests {
    use super::*;

    #[test]
    fn io_classification_matches_native_catalog_categories() {
        let mut expected = Vec::new();
        for entry in crate::xgk_module_catalog() {
            let kind = match entry.category {
                "입력 모듈" => IOKind::DigitalInput,
                "출력 모듈" => IOKind::DigitalOutput,
                "입출력 모듈" => IOKind::DigitalInputOutput,
                "아날로그입력 모듈" => IOKind::AnalogInput,
                "아날로그출력 모듈" => IOKind::AnalogOutput,
                "아날로그입출력 모듈" => IOKind::AnalogInputOutput,
                "온도입력 모듈" => IOKind::TemperatureInput,
                _ => continue,
            };
            expected.push((entry.id, entry.sub_type, kind));
        }
        expected.sort_by_key(|(id, subtype, _)| (*id, *subtype));
        expected.dedup();
        assert_eq!(IO_IDENTITIES, expected);
    }
}
