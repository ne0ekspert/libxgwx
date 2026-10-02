//! Validated coordinate graph for captured `ProjectType=2` IEC LD programs.
//!
//! XG5000 stores visual elements at cell anchors `1, 4, 7, ...`, while branch
//! junctions use the intervening boundaries `0, 3, 6, ...`. This module maps
//! the decoded records onto those boundaries and keeps typed function-pin
//! bindings alongside the electrical geometry.

use crate::{IecFunctionPin, IecFunctionPinDirection, IecRecordKind, LadderProgramData};
use std::collections::{BTreeSet, HashMap, VecDeque};

const IEC_LAST_CELL_ANCHOR: u8 = 94;
const IEC_RIGHT_RAIL_X: u8 = 96;

/// A connection boundary in the IEC LD grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IecCircuitPoint {
    pub group_index: usize,
    pub row_index: u16,
    /// Boundary coordinate. Cell anchor `1` spans boundaries `0..3`.
    pub x: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IecCircuitEdgeKind {
    HorizontalWire,
    Contact(u8),
    Coil(u8),
    FunctionEnable,
    VerticalBranch,
}

/// One electrical edge. Horizontal element records span one or more cells;
/// paired branch records produce one vertical edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecCircuitEdge {
    pub kind: IecCircuitEdgeKind,
    pub start: IecCircuitPoint,
    pub end: IecCircuitPoint,
    pub record_offset: usize,
    pub paired_record_offset: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IecCircuitAreaKind {
    HorizontalWire,
    Contact(u8),
    Coil(u8),
    FunctionBlock,
    FunctionOperand,
}

/// Grid cells occupied by one visible record. `start_x` and `end_x` are cell
/// anchors rather than boundary coordinates. Function blocks also occupy their
/// decoded body rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecCircuitArea {
    pub kind: IecCircuitAreaKind,
    pub group_index: usize,
    pub start_row_index: u16,
    pub end_row_index: u16,
    pub start_x: u8,
    pub end_x: u8,
    pub record_offset: usize,
}

/// A native function-pin reference and its optional visible expression cell.
/// Some captured pins are linked directly and therefore have no `FF 46`
/// expression record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecFunctionBinding {
    pub group_index: usize,
    pub block_record_offset: usize,
    pub reference_record_offset: usize,
    pub expression_record_offset: Option<usize>,
    pub ordinal: u8,
    pub direction: IecFunctionPinDirection,
    pub function_name: String,
    pub pin_name: String,
    pub pin_point: IecCircuitPoint,
    pub expression_cell_x: Option<u8>,
    pub data_type_mask: u32,
    pub is_array: bool,
}

/// One connected component of the stored electrical geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecPowerComponent {
    pub group_index: usize,
    pub edge_indices: Vec<usize>,
    pub points: Vec<IecCircuitPoint>,
    pub touches_left_rail: bool,
    pub touches_right_rail: bool,
}

/// Circuit-level view used to validate and plan structural edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecCircuitGraph {
    pub edges: Vec<IecCircuitEdge>,
    pub occupied_areas: Vec<IecCircuitArea>,
    pub function_bindings: Vec<IecFunctionBinding>,
    pub power_components: Vec<IecPowerComponent>,
    /// Vertical endpoints with no second incident edge. Native incomplete
    /// circuits may contain these; the strict graph API rejects them.
    pub open_branch_endpoints: Vec<IecCircuitPoint>,
}

impl LadderProgramData {
    /// Combine decoded IEC elements, wires, branches, and typed function pins
    /// into one validated coordinate graph. Returns `None` if records overlap,
    /// a branch endpoint is isolated, or a pin/expression coordinate disagrees
    /// with the decoded function body.
    pub fn iec_circuit_graph(&self) -> Option<IecCircuitGraph> {
        let graph = self.iec_circuit_layout()?;
        graph.open_branch_endpoints.is_empty().then_some(graph)
    }

    /// Decode structurally valid geometry, including native incomplete circuits.
    /// Overlaps, unmatched branch records and invalid pin bindings still fail.
    /// Open endpoints are reported separately from structural failures.
    pub fn iec_circuit_layout(&self) -> Option<IecCircuitGraph> {
        circuit_graph(self)
    }
}

fn circuit_graph(program: &LadderProgramData) -> Option<IecCircuitGraph> {
    let records = program.iec_record_frames()?;
    let geometry = program.iec_geometry()?;
    let blocks = program.iec_function_blocks()?;
    let references = program.iec_function_references()?;
    let operand_links = program.iec_function_operand_links()?;

    let horizontal = geometry
        .horizontal
        .iter()
        .map(|segment| (segment.offset, segment))
        .collect::<HashMap<_, _>>();
    let blocks_by_offset = blocks
        .iter()
        .map(|block| (block.record_offset, block))
        .collect::<HashMap<_, _>>();
    let operand_by_pin = operand_links
        .iter()
        .map(|link| ((link.target_record_offset, link.ordinal), link))
        .collect::<HashMap<_, _>>();

    let mut edges = Vec::new();
    let mut occupied_areas = Vec::new();
    for record in &records {
        let bytes = program.data.get(record.offset..record.end)?;
        match record.kind {
            IecRecordKind::LongWire => {
                let segment = *horizontal.get(&record.offset)?;
                let start = cell_left_boundary(segment.start_x)?;
                let end = cell_right_boundary(segment.end_x)?;
                edges.push(horizontal_edge(
                    record.group_index,
                    record.row_index,
                    start,
                    end,
                    IecCircuitEdgeKind::HorizontalWire,
                    record.offset,
                ));
                occupied_areas.push(single_row_area(
                    IecCircuitAreaKind::HorizontalWire,
                    record.group_index,
                    record.row_index,
                    segment.start_x,
                    segment.end_x,
                    record.offset,
                )?);
            }
            IecRecordKind::ShortWire => {
                let segment = *horizontal.get(&record.offset)?;
                if segment.end_x != segment.start_x.checked_add(3)? {
                    return None;
                }
                edges.push(horizontal_edge(
                    record.group_index,
                    record.row_index,
                    cell_left_boundary(segment.start_x)?,
                    cell_right_boundary(segment.start_x)?,
                    IecCircuitEdgeKind::HorizontalWire,
                    record.offset,
                ));
                occupied_areas.push(single_row_area(
                    IecCircuitAreaKind::HorizontalWire,
                    record.group_index,
                    record.row_index,
                    segment.start_x,
                    segment.start_x,
                    record.offset,
                )?);
            }
            IecRecordKind::Contact(code) | IecRecordKind::Coil(code) => {
                let raw_x = *bytes.get(5)?;
                let edge_kind = if matches!(record.kind, IecRecordKind::Contact(_)) {
                    IecCircuitEdgeKind::Contact(code)
                } else {
                    IecCircuitEdgeKind::Coil(code)
                };
                let area_kind = if matches!(record.kind, IecRecordKind::Contact(_)) {
                    IecCircuitAreaKind::Contact(code)
                } else {
                    IecCircuitAreaKind::Coil(code)
                };
                edges.push(horizontal_edge(
                    record.group_index,
                    record.row_index,
                    cell_left_boundary(raw_x)?,
                    cell_right_boundary(raw_x)?,
                    edge_kind,
                    record.offset,
                ));
                occupied_areas.push(single_row_area(
                    area_kind,
                    record.group_index,
                    record.row_index,
                    raw_x,
                    raw_x,
                    record.offset,
                )?);
            }
            IecRecordKind::FunctionBlock => {
                let block = *blocks_by_offset.get(&record.offset)?;
                edges.push(horizontal_edge(
                    block.group_index,
                    block.row_index,
                    cell_left_boundary(block.raw_x)?,
                    cell_right_boundary(block.raw_x)?,
                    IecCircuitEdgeKind::FunctionEnable,
                    block.record_offset,
                ));
                occupied_areas.push(IecCircuitArea {
                    kind: IecCircuitAreaKind::FunctionBlock,
                    group_index: block.group_index,
                    start_row_index: block.row_index,
                    end_row_index: block.row_index.checked_add(u16::from(block.pin_count))?,
                    start_x: block.raw_x,
                    end_x: block.raw_x,
                    record_offset: block.record_offset,
                });
            }
            IecRecordKind::FunctionOperand => {
                let raw_x = *bytes.get(5)?;
                occupied_areas.push(single_row_area(
                    IecCircuitAreaKind::FunctionOperand,
                    record.group_index,
                    record.row_index,
                    raw_x,
                    raw_x,
                    record.offset,
                )?);
            }
            IecRecordKind::Comment
            | IecRecordKind::BranchStart
            | IecRecordKind::BranchEnd
            | IecRecordKind::LinkReference(_) => {}
        }
    }
    for connection in &geometry.vertical {
        edges.push(IecCircuitEdge {
            kind: IecCircuitEdgeKind::VerticalBranch,
            start: IecCircuitPoint {
                group_index: connection.group_index,
                row_index: connection.start_row_index,
                x: connection.x,
            },
            end: IecCircuitPoint {
                group_index: connection.group_index,
                row_index: connection.end_row_index,
                x: connection.x,
            },
            record_offset: connection.start_offset,
            paired_record_offset: Some(connection.end_offset),
        });
    }

    validate_occupied_areas(&occupied_areas)?;
    let open_branch_endpoints = open_branch_endpoints(&edges);

    let mut function_bindings = Vec::with_capacity(references.len());
    for reference in &references {
        let block = *blocks_by_offset.get(&reference.target_record_offset)?;
        let pin = pin_for_ordinal(block, reference.ordinal)?;
        let link = operand_by_pin
            .get(&(reference.target_record_offset, reference.ordinal))
            .copied();
        let expression_cell_x = if let Some(link) = link {
            let record = records
                .iter()
                .find(|record| record.offset == link.record_offset)?;
            let raw_x = *program.data.get(record.offset + 5)?;
            let expected = match pin.direction {
                IecFunctionPinDirection::Input => pin.raw_x.checked_sub(3)?,
                IecFunctionPinDirection::Output => pin.raw_x,
            };
            if record.group_index != block.group_index
                || record.row_index != pin.row_index
                || raw_x != expected
            {
                return None;
            }
            Some(raw_x)
        } else {
            None
        };
        function_bindings.push(IecFunctionBinding {
            group_index: block.group_index,
            block_record_offset: block.record_offset,
            reference_record_offset: reference.record_offset,
            expression_record_offset: link.map(|link| link.record_offset),
            ordinal: reference.ordinal,
            direction: pin.direction,
            function_name: block.name.value.clone(),
            pin_name: pin.name.value.clone(),
            pin_point: IecCircuitPoint {
                group_index: block.group_index,
                row_index: pin.row_index,
                x: cell_left_boundary(pin.raw_x)?,
            },
            expression_cell_x,
            data_type_mask: pin.data_type_mask,
            is_array: pin.is_array,
        });
    }
    function_bindings.sort_by_key(|binding| binding.reference_record_offset);

    let power_components = power_components(&edges);
    Some(IecCircuitGraph {
        edges,
        occupied_areas,
        function_bindings,
        power_components,
        open_branch_endpoints,
    })
}

fn cell_left_boundary(raw_x: u8) -> Option<u8> {
    ((1..=IEC_LAST_CELL_ANCHOR).contains(&raw_x) && raw_x % 3 == 1).then(|| raw_x - 1)
}

fn cell_right_boundary(raw_x: u8) -> Option<u8> {
    cell_left_boundary(raw_x)?.checked_add(3)
}

fn horizontal_edge(
    group_index: usize,
    row_index: u16,
    start_x: u8,
    end_x: u8,
    kind: IecCircuitEdgeKind,
    record_offset: usize,
) -> IecCircuitEdge {
    IecCircuitEdge {
        kind,
        start: IecCircuitPoint {
            group_index,
            row_index,
            x: start_x,
        },
        end: IecCircuitPoint {
            group_index,
            row_index,
            x: end_x,
        },
        record_offset,
        paired_record_offset: None,
    }
}

fn single_row_area(
    kind: IecCircuitAreaKind,
    group_index: usize,
    row_index: u16,
    start_x: u8,
    end_x: u8,
    record_offset: usize,
) -> Option<IecCircuitArea> {
    if cell_left_boundary(start_x).is_none()
        || cell_right_boundary(end_x).is_none()
        || start_x > end_x
        || !(end_x - start_x).is_multiple_of(3)
    {
        return None;
    }
    Some(IecCircuitArea {
        kind,
        group_index,
        start_row_index: row_index,
        end_row_index: row_index,
        start_x,
        end_x,
        record_offset,
    })
}

fn validate_occupied_areas(areas: &[IecCircuitArea]) -> Option<()> {
    let mut occupied = HashMap::new();
    for (index, area) in areas.iter().enumerate() {
        if area.start_row_index > area.end_row_index
            || area.start_x > area.end_x
            || cell_left_boundary(area.start_x).is_none()
            || cell_right_boundary(area.end_x).is_none()
        {
            return None;
        }
        for row_index in area.start_row_index..=area.end_row_index {
            for raw_x in (area.start_x..=area.end_x).step_by(3) {
                if occupied
                    .insert((area.group_index, row_index, raw_x), index)
                    .is_some()
                {
                    return None;
                }
            }
        }
    }
    Some(())
}

fn open_branch_endpoints(edges: &[IecCircuitEdge]) -> Vec<IecCircuitPoint> {
    let mut incident = HashMap::<IecCircuitPoint, usize>::new();
    for edge in edges {
        *incident.entry(edge.start).or_default() += 1;
        *incident.entry(edge.end).or_default() += 1;
    }
    edges
        .iter()
        .filter(|edge| edge.kind == IecCircuitEdgeKind::VerticalBranch)
        .flat_map(|edge| [edge.start, edge.end])
        .filter(|point| incident.get(point).copied().unwrap_or_default() < 2)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn pin_for_ordinal(block: &crate::IecFunctionBlock, ordinal: u8) -> Option<&IecFunctionPin> {
    block
        .pins
        .iter()
        .chain([&block.control_input, &block.control_output])
        .find(|pin| pin.reference_ordinal == Some(ordinal))
}

fn power_components(edges: &[IecCircuitEdge]) -> Vec<IecPowerComponent> {
    let mut incident = HashMap::<IecCircuitPoint, Vec<usize>>::new();
    for (index, edge) in edges.iter().enumerate() {
        incident.entry(edge.start).or_default().push(index);
        incident.entry(edge.end).or_default().push(index);
    }
    let mut seen = vec![false; edges.len()];
    let mut components = Vec::new();
    for first in 0..edges.len() {
        if seen[first] {
            continue;
        }
        let group_index = edges[first].start.group_index;
        let mut queue = VecDeque::from([first]);
        let mut edge_indices = Vec::new();
        let mut points = BTreeSet::new();
        while let Some(index) = queue.pop_front() {
            if seen[index] {
                continue;
            }
            seen[index] = true;
            let edge = edges[index];
            if edge.start.group_index != group_index || edge.end.group_index != group_index {
                continue;
            }
            edge_indices.push(index);
            points.insert(edge.start);
            points.insert(edge.end);
            for point in [edge.start, edge.end] {
                for &next in incident.get(&point).into_iter().flatten() {
                    if !seen[next] {
                        queue.push_back(next);
                    }
                }
            }
        }
        edge_indices.sort_unstable();
        let points = points.into_iter().collect::<Vec<_>>();
        components.push(IecPowerComponent {
            group_index,
            touches_left_rail: points.iter().any(|point| point.x == 0),
            touches_right_rail: points.iter().any(|point| point.x == IEC_RIGHT_RAIL_X),
            edge_indices,
            points,
        });
    }
    components.sort_by_key(|component| {
        (
            component.group_index,
            component
                .edge_indices
                .first()
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
    components
}
