//! Indexed collision lookup with a shared budget for owned preview output.
use crate::{VariableSummary, XgwxError};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const MAX_ASSOCIATIONS: usize = 65_536;
const MAX_COPIED_NAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Budget {
    associations: usize,
    name_bytes: usize,
}

impl Budget {
    fn consume(&mut self, variable: &VariableSummary) -> Result<(), XgwxError> {
        self.associations = self.associations.saturating_add(1);
        if self.associations > MAX_ASSOCIATIONS {
            return Err(XgwxError::ResourceLimitExceeded {
                resource: "I/O collision associations",
                limit: MAX_ASSOCIATIONS,
            });
        }
        self.name_bytes = self
            .name_bytes
            .saturating_add(variable.name.as_ref().map_or(0, String::len));
        if self.name_bytes > MAX_COPIED_NAME_BYTES {
            return Err(XgwxError::ResourceLimitExceeded {
                resource: "I/O collision name bytes",
                limit: MAX_COPIED_NAME_BYTES,
            });
        }
        Ok(())
    }
}

pub(super) struct Index<'a> {
    variables: &'a [VariableSummary],
    names: HashMap<String, Vec<usize>>,
    sources: HashMap<&'a str, Vec<usize>>,
    // Key by width as well as start: a range query then returns only genuine
    // overlaps, without storing up to 64 copies of every variable index.
    addresses: BTreeMap<(u32, u32), Vec<usize>>,
}

impl<'a> Index<'a> {
    pub(super) fn new(variables: &'a [VariableSummary]) -> Self {
        let mut result = Self {
            variables,
            names: HashMap::new(),
            sources: HashMap::new(),
            addresses: BTreeMap::new(),
        };
        for (index, variable) in variables.iter().enumerate() {
            if let Some(name) = &variable.name {
                result
                    .names
                    .entry(name.to_ascii_lowercase())
                    .or_default()
                    .push(index);
            }
            if let Some(source) = variable.source_ref.as_deref() {
                result.sources.entry(source).or_default().push(index);
            }
            if let Some((width, start)) = address_range(variable) {
                result
                    .addresses
                    .entry((width, start))
                    .or_default()
                    .push(index);
            }
        }
        result
    }

    pub(super) fn matching(
        &self,
        name: &str,
        source: &str,
        bit: u32,
        budget: &mut Budget,
    ) -> Result<Vec<usize>, XgwxError> {
        let mut selected = BTreeSet::new();
        let mut add = |indices: &[usize]| -> Result<(), XgwxError> {
            for &index in indices {
                if selected.insert(index) {
                    budget.consume(&self.variables[index])?;
                }
            }
            Ok(())
        };
        if let Some(indices) = self.names.get(&name.to_ascii_lowercase()) {
            add(indices)?;
        }
        if let Some(indices) = self.sources.get(source) {
            add(indices)?;
        }
        // Half-open saturating ranges never contain u32::MAX itself.
        if bit != u32::MAX {
            for width in [1, 8, 16, 32, 64] {
                for indices in self
                    .addresses
                    .range((width, bit.saturating_sub(width - 1))..=(width, bit))
                    .map(|(_, indices)| indices)
                {
                    add(indices)?;
                }
            }
        }
        Ok(selected.into_iter().collect())
    }
}

fn address_range(variable: &VariableSummary) -> Option<(u32, u32)> {
    #[cfg(test)]
    super::COVER_CHECKS.with(|count| count.set(count.get() + 1));
    if variable.address_area.as_deref() != Some("P") {
        return None;
    }
    let number = variable.address_number?;
    let width = match variable.data_type.as_deref()? {
        "BIT" => 1,
        "BYTE" => 8,
        "WORD" => 16,
        "DWORD" => 32,
        "LWORD" => 64,
        _ => return None,
    };
    let start = if width == 1 {
        number
    } else {
        number.saturating_mul(16)
    };
    (start < start.saturating_add(width)).then_some((width, start))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(
        name: Option<&str>,
        source: Option<&str>,
        area: &str,
        number: u32,
        kind: &str,
    ) -> VariableSummary {
        VariableSummary {
            format_version: None,
            name: name.map(str::to_owned),
            type_name: None,
            scope: None,
            address_area: Some(area.into()),
            address_number: Some(number),
            data_type: Some(kind.into()),
            description: None,
            comment: None,
            address: None,
            source_ref: source.map(str::to_owned),
            range: None,
        }
    }

    #[test]
    fn indexed_matches_preserve_legacy_collision_semantics() {
        let variables = vec![
            variable(Some("Target"), Some("IO:X"), "P", 0, "BIT"),
            variable(Some("target"), Some("io:x"), "P", 0, "BYTE"),
            variable(Some("Näme"), None, "P", 1, "WORD"),
            variable(None, Some("IO:X"), "P", 2, "DWORD"),
            variable(Some("Other"), None, "P", 3, "LWORD"),
            variable(Some("Other"), None, "p", 0, "BIT"),
            variable(None, None, "P", u32::MAX, "BIT"),
            variable(None, None, "P", u32::MAX / 16, "LWORD"),
            variable(None, None, "P", 0, "bit"),
        ];
        let index = Index::new(&variables);
        let mut bits: Vec<u32> = (0..130).collect();
        bits.extend((u32::MAX - 65)..=u32::MAX);
        for name in ["target", "NÄME", "Näme", "absent"] {
            for source in ["IO:X", "io:x", "absent"] {
                for &bit in &bits {
                    let expected: Vec<_> = variables
                        .iter()
                        .enumerate()
                        .filter_map(|(i, v)| {
                            (v.name
                                .as_deref()
                                .is_some_and(|n| n.eq_ignore_ascii_case(name))
                                || v.source_ref.as_deref() == Some(source)
                                || super::super::variable_covers(v, bit))
                            .then_some(i)
                        })
                        .collect();
                    let actual = index
                        .matching(name, source, bit, &mut Budget::default())
                        .unwrap();
                    assert_eq!(actual, expected, "{name:?}, {source:?}, {bit}");
                }
            }
        }
    }

    #[test]
    fn association_budget_is_global_and_deduplicates_each_row() {
        let variables: Vec<_> = (0..1024)
            .map(|i| variable((i == 0).then_some("Shared"), None, "P", 0, "LWORD"))
            .collect();
        let index = Index::new(&variables);
        let mut budget = Budget::default();
        for bit in 0..64 {
            assert_eq!(
                index
                    .matching("Shared", "absent", bit, &mut budget)
                    .unwrap()
                    .len(),
                1024
            );
        }
        assert!(matches!(
            index.matching("Shared", "absent", 64, &mut budget),
            Err(XgwxError::ResourceLimitExceeded {
                resource: "I/O collision associations",
                ..
            })
        ));
    }
}
