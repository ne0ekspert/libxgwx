//! Preflight cross-reference expansion without decompressing any ST payload.
use super::{number, st_code, st_source_kind};
use crate::{XgwxError, XmlElement};
use std::collections::BTreeMap;

const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_WORK: usize = 4_000_000;

fn charge(
    total: &mut usize,
    amount: usize,
    limit: usize,
    resource: &'static str,
) -> Result<(), XgwxError> {
    *total = total.saturating_add(amount);
    if *total > limit {
        Err(XgwxError::ResourceLimitExceeded { resource, limit })
    } else {
        Ok(())
    }
}

// Index the first list selected by every Program, including nested Programs.
// Pointer keys identify XML nodes only; no pointer is dereferenced.
fn index_lists<'a>(
    node: &'a XmlElement,
    lists: &mut BTreeMap<usize, (&'a XmlElement, usize)>,
    work: &mut usize,
) -> Result<Option<&'a XmlElement>, XgwxError> {
    charge(work, 1, MAX_WORK, "SFC source inspection work")?;
    let mut first = (node.name == "SFC_ProgramList").then_some(node);
    for child in &node.children {
        let found = index_lists(child, lists, work)?;
        if first.is_none() {
            first = found;
        }
    }
    if node.name == "Program" {
        if let Some(list) = first {
            lists
                .entry(list as *const XmlElement as usize)
                .or_insert((list, 0))
                .1 += 1;
        }
    }
    Ok(first)
}

fn nodes<'a>(root: &'a XmlElement, work: &mut usize) -> Result<Vec<&'a XmlElement>, XgwxError> {
    let mut pending = vec![root];
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        charge(work, 1, MAX_WORK, "SFC source inspection work")?;
        result.push(node);
        pending.extend(node.children.iter().rev());
    }
    Ok(result)
}

fn property<'a>(node: &'a XmlElement, key: &str) -> Option<&'a str> {
    // SfcEntity's BTreeMap keeps the last local attribute name, including
    // colliding names from distinct XML namespaces.
    node.attributes
        .iter()
        .rev()
        .find(|a| a.name == key)
        .map(|a| a.value.as_str())
}

pub(crate) fn validate_source_expansion(root: &XmlElement) -> Result<(), XgwxError> {
    if root.descendants_named("SFC_ProgramList").next().is_none() {
        return Ok(());
    }
    let mut work = 0;
    let mut lists = BTreeMap::new();
    index_lists(root, &mut lists, &mut work)?;
    let mut total = 0;
    for (list, uses) in lists.into_values() {
        let list_nodes = nodes(list, &mut work)?;
        charge(
            &mut work,
            list_nodes.len().saturating_mul(uses.saturating_sub(1)),
            MAX_WORK,
            "SFC source inspection work",
        )?;
        let blocks: Vec<_> = list_nodes
            .into_iter()
            .filter(|node| node.name == "SFC_ProgramProperty")
            .collect();
        let mut sources = BTreeMap::new();
        let mut main_count = 0;
        let mut supported = true;
        for block in &blocks {
            if block.attribute("MainBlock") == Some("1") {
                main_count += 1;
                continue;
            }
            let Some(kind) = st_source_kind(block) else {
                supported = false;
                break;
            };
            let Some(name) = block.attribute("PragramName") else {
                supported = false;
                break;
            };
            let Some((_, count)) = st_code(block) else {
                supported = false;
                break;
            };
            // A valid UTF-16 unit expands to at most three UTF-8 bytes. Include
            // every decoded definition, even if no row refers to it.
            let bytes = count.saturating_mul(3);
            charge(
                &mut total,
                bytes.saturating_mul(uses),
                MAX_SOURCE_BYTES,
                "SFC expanded source bytes",
            )?;
            if sources.insert(name, (kind, bytes)).is_some() {
                supported = false;
                break;
            }
        }
        if !supported || main_count != 1 {
            continue;
        }
        for block in blocks
            .into_iter()
            .filter(|block| block.attribute("MainBlock") == Some("1"))
        {
            if number(block, "LanguageType") != Some(3) || number(block, "Language") != Some(2) {
                continue;
            }
            // Both linear charts and branch projections obtain their row
            // sources from EntityStep properties in the first entity grid.
            let block_nodes = nodes(block, &mut work)?;
            let Some(grid) = block_nodes
                .into_iter()
                .find(|node| node.name == "EntityGrid")
            else {
                continue;
            };
            for entity in nodes(grid, &mut work)?
                .into_iter()
                .filter(|node| node.name == "EntityProperty")
            {
                let kind = match number(entity, "Type") {
                    Some(1) => 2,
                    Some(2) => 1,
                    _ => continue,
                };
                // The decoder's property map retains the last duplicate name.
                let Some(step) = entity
                    .children
                    .iter()
                    .rev()
                    .find(|node| node.name == "EntityStep")
                else {
                    continue;
                };
                if property(step, "PropertyProgram") != Some("1") {
                    continue;
                }
                if let Some((source_kind, bytes)) =
                    property(step, "Title").and_then(|title| sources.get(title))
                {
                    if *source_kind == kind {
                        charge(
                            &mut total,
                            bytes.saturating_mul(uses),
                            MAX_SOURCE_BYTES,
                            "SFC expanded source bytes",
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
