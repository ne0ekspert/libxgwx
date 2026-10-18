//! Bound owned module expansion before any summaries or candidate lists are built.
use super::{identity_matches, network_scopes};
use crate::{NetworkModuleSummary, XgwxError, XmlElement};

const MAX_RECORDS: usize = 200_000;
const MAX_ASSOCIATIONS: usize = 4096;
const MAX_WORK: usize = 4_000_000;
const MAX_STORAGE: usize = 256 * 1024 * 1024;

fn check(value: usize, limit: usize, resource: &'static str) -> Result<(), XgwxError> {
    if value > limit {
        Err(XgwxError::ResourceLimitExceeded { resource, limit })
    } else {
        Ok(())
    }
}

// Include struct/vector overhead, duplicated fields, and escaped JSON strings.
// These are conservative allocation estimates, not the XML's compressed size.
fn summary_weight(element: &XmlElement) -> usize {
    let mut weight = 1024usize;
    for attribute in &element.attributes {
        weight = weight.saturating_add(128).saturating_add(
            attribute
                .name
                .len()
                .saturating_add(attribute.value.len())
                .saturating_mul(8),
        );
    }
    if element.name == "XGPD_CONFIG_INFO_CNET" {
        for port in element
            .children
            .iter()
            .filter(|c| c.name == "XGPD_CONFIG_INFO_CNET_PORT")
        {
            weight = weight
                .saturating_add(2048)
                .saturating_add(summary_weight(port));
        }
    }
    weight
}

#[derive(Default)]
struct Inventory<'a> {
    names: Vec<Option<String>>,
    candidates: Vec<Candidate>,
    networks: Vec<(Option<usize>, &'a XmlElement)>,
    bases: usize,
    hardware: usize,
    nodes: usize,
    storage: usize,
}

struct Candidate {
    owner: Option<usize>,
    id: Option<u32>,
    base: Option<u32>,
    slot: Option<u32>,
    subtype: Option<u32>,
    weight: usize,
}

impl<'a> Inventory<'a> {
    fn visit(
        &mut self,
        element: &'a XmlElement,
        mut owner: Option<usize>,
    ) -> Result<(), XgwxError> {
        self.nodes = self.nodes.saturating_add(1);
        match element.name.as_str() {
            "Configuration" => {
                owner = Some(self.names.len());
                let name = crate::text_value(element);
                self.storage = self
                    .storage
                    .saturating_add(1024)
                    .saturating_add(name.as_ref().map_or(0, |name| name.len()).saturating_mul(8));
                self.names.push(name);
            }
            "Base" => self.bases += 1,
            "NetworkModule" => {
                self.networks.push((owner, element));
                self.storage = self.storage.saturating_add(summary_weight(element));
            }
            "Module"
            | "XGPD_CONFIG_INFO_FENET"
            | "XGPD_CONFIG_INFO_CNET"
            | "XGPD_CONFIG_INFO_DNET"
            | "XGPD_CONFIG_INFO_FDENET"
            | "XGPD_CONFIG_INFO_RNET" => {
                self.hardware += usize::from(element.name == "Module");
                let weight = summary_weight(element);
                self.storage = self.storage.saturating_add(weight);
                self.candidates.push(Candidate {
                    owner,
                    id: crate::attr_u32(
                        element,
                        if element.name == "Module" {
                            "Id"
                        } else {
                            "Type"
                        },
                    ),
                    base: crate::attr_u32(element, "Base"),
                    slot: crate::attr_u32(element, "Slot"),
                    subtype: crate::attr_u32(element, "SubType"),
                    weight,
                });
            }
            _ => {}
        }
        check(
            self.names
                .len()
                .saturating_add(self.candidates.len())
                .saturating_add(self.networks.len())
                .saturating_add(self.bases),
            MAX_RECORDS,
            "typed module records",
        )?;
        for child in &element.children {
            self.visit(child, owner)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_module_associations(root: &XmlElement) -> Result<(), XgwxError> {
    // Documents without memberships cannot expand candidates. In particular,
    // ordinary hardware-only inspection keeps its existing acceptance behavior.
    if root.descendants_named("NetworkModule").next().is_none() {
        return Ok(());
    }
    let mut inventory = Inventory::default();
    inventory.visit(root, None)?;
    let networks = inventory.networks.len();
    // WASM builds the document view and one inventory for every base. Scoped
    // module views also hold both the resolved list and cloned selected entries.
    let repeats = inventory.bases.saturating_add(1);
    let mut storage = inventory.storage.saturating_mul(4).saturating_mul(repeats);
    check(storage, MAX_STORAGE, "typed module storage")?;
    let mut work = inventory
        .nodes
        .saturating_add(
            inventory
                .hardware
                .saturating_mul(networks)
                .saturating_mul(2),
        )
        .saturating_add(
            inventory
                .hardware
                .saturating_add(networks)
                .saturating_mul(inventory.bases),
        )
        .saturating_add(
            networks
                .saturating_mul(inventory.names.len())
                .saturating_mul(2),
        )
        .saturating_mul(repeats);
    check(work, MAX_WORK, "typed module work")?;
    let mut associations = 0usize;
    let name_bytes = inventory
        .names
        .iter()
        .flatten()
        .fold(0usize, |sum, name| sum.saturating_add(name.len()));
    for (owner, element) in inventory.networks {
        let metadata = NetworkModuleSummary::from_element(element);
        if owner.is_none() {
            let comparison_bytes = metadata
                .config_name
                .as_ref()
                .map_or(0, |name| name.len())
                .saturating_mul(inventory.names.len())
                .saturating_add(name_bytes);
            work = work.saturating_add(comparison_bytes.saturating_mul(repeats));
            check(work, MAX_WORK, "typed module work")?;
        }
        let scopes = network_scopes(&inventory.names, owner, &metadata);
        work = work.saturating_add(
            inventory
                .candidates
                .len()
                .saturating_mul(scopes.len().saturating_add(1))
                .saturating_mul(repeats),
        );
        check(work, MAX_WORK, "typed module work")?;
        for candidate in &inventory.candidates {
            if scopes.contains(&candidate.owner)
                && identity_matches(
                    &metadata,
                    candidate.id,
                    candidate.base,
                    candidate.slot,
                    candidate.subtype,
                )
            {
                associations += 1;
                check(associations, MAX_ASSOCIATIONS, "typed module associations")?;
                storage = storage
                    .saturating_add(candidate.weight.saturating_mul(4).saturating_mul(repeats));
                check(storage, MAX_STORAGE, "typed module storage")?;
            }
        }
    }
    Ok(())
}
