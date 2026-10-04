//! Exact row-local record boundaries for captured IEC LD program payloads.
//!
//! Function records include decoded control ports, data-port rows, IEC type
//! masks, and native reference ordinals. Their end is accepted only when the
//! entire row has exactly one valid segmentation matching its record count.
use crate::{LadderProgramData, LadderString};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IecRecordKind {
    LongWire,
    ShortWire,
    Contact(u8),
    Coil(u8),
    Comment,
    FunctionOperand,
    BranchStart,
    BranchEnd,
    LinkReference(u8),
    FunctionBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecRecordFrame {
    pub group_index: usize,
    pub row_index: u16,
    pub offset: usize,
    pub end: usize,
    pub kind: IecRecordKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecFunctionBlock {
    pub group_index: usize,
    pub row_index: u16,
    pub record_offset: usize,
    pub record_end: usize,
    pub raw_x: u8,
    pub opcode_family: u8,
    pub opcode: u16,
    pub pin_count: u8,
    pub name: LadderString,
    pub instance: Option<LadderString>,
    pub control_input: IecFunctionPin,
    pub control_output: IecFunctionPin,
    /// Data pins in native visual order, from the top row down and left to right.
    pub pins: Vec<IecFunctionPin>,
    /// Marker strings after the function name, retained for binary diagnostics.
    pub field_strings: Vec<LadderString>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IecFunctionPinDirection {
    Input,
    Output,
}

/// One decoded control or data pin inside an IEC function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecFunctionPin {
    pub name: LadderString,
    pub type_expression: Option<LadderString>,
    pub raw_type_flags: u32,
    pub data_type_mask: u32,
    pub data_type: Option<&'static str>,
    pub is_array: bool,
    pub direction: IecFunctionPinDirection,
    pub is_control: bool,
    /// Ordinal used by the corresponding `0x68`/`0x69` reference record.
    pub reference_ordinal: Option<u8>,
    pub raw_x: u8,
    pub row_index: u16,
}

/// A captured link record that points to a function block by group and
/// stored x/y coordinate. Its ordinal identifies a pin in the block header;
/// the link code distinguishes input (`0x68`) from output (`0x69`) in this
/// captured IEC LD variant. The resolved pin coordinates and type mask are
/// retained for circuit and expression validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecFunctionReference {
    pub group_index: usize,
    pub row_index: u16,
    pub record_offset: usize,
    pub target_record_offset: usize,
    pub target_row_index: u16,
    pub raw_x: u8,
    pub code: u8,
    pub ordinal: u8,
    pub is_output: bool,
    pub pin_row_index: u16,
    pub pin_raw_x: u8,
    pub data_type_mask: u32,
    pub is_array: bool,
}

/// The function pin represented by one `FF 46` expression record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecFunctionOperandLink {
    pub group_index: usize,
    pub row_index: u16,
    pub record_offset: usize,
    pub target_record_offset: usize,
    pub ordinal: u8,
    pub is_output: bool,
    pub pin_row_index: u16,
    pub pin_raw_x: u8,
    pub data_type_mask: u32,
    pub is_array: bool,
}

/// A terminal function block whose native Delete operation removes the
/// preceding wire, the block, and its otherwise empty pin rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecTerminalFunctionDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub block_offset: usize,
    pub wire_offset: usize,
    pub raw_x: u8,
    pub pin_count: u8,
}

/// A retained contact followed by the captured two-row terminal MOVE gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecTerminalFunctionInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub contact_offset: usize,
    pub insertion_offset: usize,
    pub raw_x: u8,
}

/// A standalone function group whose native Delete operation removes the
/// complete group and leaves its stored rows as an implicit blank gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecStandaloneFunctionDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub block_offset: usize,
    pub wire_offset: usize,
    pub raw_x: u8,
    pub pin_count: u8,
}

/// The three-row blank gap left by deleting the captured standalone
/// WORD_TO_UDINT group at L1 in the smart-home project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecStandaloneFunctionInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub insertion_offset: usize,
    pub raw_x: u8,
}

/// A function cell whose native Delete operation removes the block and its
/// pin-link records while retaining unrelated records in the same rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecFunctionCellDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub block_offset: usize,
    pub raw_x: u8,
    pub pin_count: u8,
}

/// Captured ADD/SUB cell sharing a five-row group with R_TRIG and MOVE. Native
/// Delete removes its incoming wire and pin records, then splits the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecConnectedArithmeticDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub block_offset: usize,
    pub wire_offset: usize,
    pub raw_x: u8,
}

/// A one-cell gap that can accept the captured connected single-output FF
/// function shape while retaining the other records in both stored rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecFunctionCellInsertionSite {
    pub function_name: &'static str,
    pub group_index: usize,
    pub row_index: u16,
    pub insertion_offset: usize,
    pub reference_offset: usize,
    pub raw_x: u8,
}

/// Empty comparison header whose BOOL output connects to a retained MOVE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecWiredComparisonInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub raw_x: u8,
}

/// A long wire in a linear contact-wire-...-contact-wire-coil row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecNoContactInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub wire_offset: usize,
    pub start_x: u8,
    pub end_x: u8,
}

/// A one-cell IEC wire that can be replaced by an addressed contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecShortWireContactInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub wire_offset: usize,
    pub raw_x: u8,
}

/// Empty x1 contact cell on the upper line of a captured two-row branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecLeadingContactInsertionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub insertion_offset: usize,
}

/// A contact between two captured wire fragments that XG5000 can delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecNoContactDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub contact_offset: usize,
    pub raw_x: u8,
    pub contact_code: u8,
}

/// A one-cell gap between two long-wire fragments left by deleting a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecHorizontalWireRepairSite {
    pub group_index: usize,
    pub row_index: u16,
    pub insertion_offset: usize,
    pub raw_x: u8,
}

/// A one-cell short wire between long-wire fragments that can be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IecHorizontalWireDeletionSite {
    pub group_index: usize,
    pub row_index: u16,
    pub wire_offset: usize,
    pub raw_x: u8,
}

impl LadderProgramData {
    /// Identify every IEC LD record in this program, or return `None` if the
    /// captured grammar cannot segment a row unambiguously.
    pub fn iec_record_frames(&self) -> Option<Vec<IecRecordFrame>> {
        let rows = self.iec_row_frames()?;
        let mut frames = Vec::new();
        for row in rows {
            if row.record_count > 128 || row.end - row.records_start > 8192 {
                return None;
            }
            let mut memo = HashMap::new();
            let Choice::One(records) = solve(
                &self.data,
                row.records_start,
                row.end,
                row.row_index,
                row.record_count,
                &mut memo,
            ) else {
                return None;
            };
            frames.extend(
                records
                    .into_iter()
                    .map(|(offset, end, kind)| IecRecordFrame {
                        group_index: row.group_index,
                        row_index: row.row_index,
                        offset,
                        end,
                        kind,
                    }),
            );
        }
        Some(frames)
    }

    /// Decode function block identities and positions from framed IEC rows.
    pub fn iec_function_blocks(&self) -> Option<Vec<IecFunctionBlock>> {
        let records = self.iec_record_frames()?;
        let mut blocks = Vec::new();
        for record in records {
            if record.kind != IecRecordKind::FunctionBlock {
                continue;
            }
            blocks.push(parse_function_block(&self.data, record)?);
        }
        Some(blocks)
    }

    /// Resolve captured `0x68`/`0x69` link records to exactly one function
    /// block each. Return `None` if a link has an unsupported code or target.
    pub fn iec_function_references(&self) -> Option<Vec<IecFunctionReference>> {
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let mut targets = HashMap::new();
        for block in &blocks {
            if targets
                .insert((block.group_index, block.row_index, block.raw_x), block)
                .is_some()
            {
                return None;
            }
        }
        let mut references = Vec::new();
        let mut seen_ordinals = HashMap::<usize, u32>::new();
        for record in records {
            let IecRecordKind::LinkReference(code) = record.kind else {
                continue;
            };
            if !matches!(code, 0x68 | 0x69) {
                return None;
            }
            let bytes = self.data.get(record.offset..record.end)?;
            let target_row_index = u16::from_le_bytes([bytes[6], bytes[7]]) / 4;
            if target_row_index >= record.row_index {
                return None;
            }
            let block = *targets.get(&(record.group_index, target_row_index, bytes[5]))?;
            let ordinal = bytes[0];
            let pin = block_pin_for_ordinal(block, ordinal)?;
            if (code == 0x69) != (pin.direction == IecFunctionPinDirection::Output) {
                return None;
            }
            let bit = 1_u32 << (ordinal - 1);
            let seen = seen_ordinals.entry(block.record_offset).or_default();
            if *seen & bit != 0 {
                return None;
            }
            *seen |= bit;
            references.push(IecFunctionReference {
                group_index: record.group_index,
                row_index: record.row_index,
                record_offset: record.offset,
                target_record_offset: block.record_offset,
                target_row_index,
                raw_x: bytes[5],
                code,
                ordinal,
                is_output: code == 0x69,
                pin_row_index: pin.row_index,
                pin_raw_x: pin.raw_x,
                data_type_mask: pin.data_type_mask,
                is_array: pin.is_array,
            });
        }
        for block in targets.values() {
            let expected = if block.pin_count == 32 {
                u32::MAX
            } else {
                (1_u32 << block.pin_count) - 1
            };
            if seen_ordinals.get(&block.record_offset).copied() != Some(expected) {
                return None;
            }
        }
        Some(references)
    }

    /// Resolve every captured `FF 46` expression to one function pin.
    pub fn iec_function_operand_links(&self) -> Option<Vec<IecFunctionOperandLink>> {
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let reference_keys = references
            .iter()
            .map(|reference| {
                (
                    (reference.target_record_offset, reference.ordinal),
                    reference,
                )
            })
            .collect::<HashMap<_, _>>();
        let mut positions = HashMap::new();
        for block in &blocks {
            for pin in block
                .pins
                .iter()
                .chain([&block.control_input, &block.control_output])
            {
                let Some(ordinal) = pin.reference_ordinal else {
                    continue;
                };
                let expression_x = match pin.direction {
                    IecFunctionPinDirection::Input => pin.raw_x.checked_sub(3)?,
                    IecFunctionPinDirection::Output => pin.raw_x,
                };
                if positions
                    .insert(
                        (block.group_index, pin.row_index, expression_x),
                        (block.record_offset, ordinal),
                    )
                    .is_some()
                {
                    return None;
                }
            }
        }
        let mut links = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for record in records {
            if record.kind != IecRecordKind::FunctionOperand {
                continue;
            }
            let x = *self.data.get(record.offset + 5)?;
            let key = (record.group_index, record.row_index, x);
            let &(target_record_offset, ordinal) = positions.get(&key)?;
            let reference = *reference_keys.get(&(target_record_offset, ordinal))?;
            if !seen.insert((target_record_offset, ordinal)) {
                return None;
            }
            links.push(IecFunctionOperandLink {
                group_index: record.group_index,
                row_index: record.row_index,
                record_offset: record.offset,
                target_record_offset,
                ordinal,
                is_output: reference.is_output,
                pin_row_index: reference.pin_row_index,
                pin_raw_x: reference.pin_raw_x,
                data_type_mask: reference.data_type_mask,
                is_array: reference.is_array,
            });
        }
        Some(links)
    }

    /// Find the compact terminal-function groups covered by the captured
    /// XG5000 Delete operation. The top row must be exactly contact, wire,
    /// function block; every following group row must belong only to that
    /// block's operands and pin references.
    pub fn iec_terminal_function_deletion_sites(
        &self,
    ) -> Option<Vec<IecTerminalFunctionDeletionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let operand_links = self.iec_function_operand_links()?;
        let mut sites = Vec::new();
        for block in blocks {
            let group_rows = rows
                .iter()
                .filter(|row| row.group_index == block.group_index)
                .collect::<Vec<_>>();
            if group_rows.len() != usize::from(block.pin_count) + 1
                || group_rows.first().map(|row| row.row_index) != Some(block.row_index)
                || group_rows
                    .iter()
                    .enumerate()
                    .any(|(index, row)| row.row_index != block.row_index + index as u16)
            {
                continue;
            }
            let top_records = records
                .iter()
                .filter(|record| {
                    record.group_index == block.group_index && record.row_index == block.row_index
                })
                .collect::<Vec<_>>();
            if top_records.len() != 3
                || !matches!(top_records[0].kind, IecRecordKind::Contact(6..=11))
                || !matches!(
                    top_records[1].kind,
                    IecRecordKind::LongWire | IecRecordKind::ShortWire
                )
                || top_records[2].kind != IecRecordKind::FunctionBlock
                || top_records[2].offset != block.record_offset
                || top_records[1].end != block.record_offset
            {
                continue;
            }
            let wire = self.data.get(top_records[1].offset..top_records[1].end)?;
            let wire_end = if top_records[1].kind == IecRecordKind::ShortWire {
                wire[5]
            } else {
                wire[15]
            };
            if wire_end.checked_add(3) != Some(block.raw_x) {
                continue;
            }
            let child_records = records
                .iter()
                .filter(|record| {
                    record.group_index == block.group_index && record.row_index > block.row_index
                })
                .collect::<Vec<_>>();
            if child_records.is_empty()
                || child_records.iter().any(|record| {
                    !matches!(
                        record.kind,
                        IecRecordKind::FunctionOperand | IecRecordKind::LinkReference(_)
                    )
                })
                || child_records.iter().any(|record| match record.kind {
                    IecRecordKind::FunctionOperand => !operand_links.iter().any(|link| {
                        link.record_offset == record.offset
                            && link.target_record_offset == block.record_offset
                    }),
                    IecRecordKind::LinkReference(_) => !references.iter().any(|reference| {
                        reference.record_offset == record.offset
                            && reference.target_record_offset == block.record_offset
                    }),
                    _ => true,
                })
            {
                continue;
            }
            sites.push(IecTerminalFunctionDeletionSite {
                group_index: block.group_index,
                row_index: block.row_index,
                block_offset: block.record_offset,
                wire_offset: top_records[1].offset,
                raw_x: block.raw_x,
                pin_count: block.pin_count,
            });
        }
        Some(sites)
    }

    /// Find the retained-contact shape produced by deleting one of the five
    /// elevator MOVEs or the captured lighting MOVE. Its two pin rows are
    /// implicit until insertion.
    /// Native-compatible deletions from horizontal scalar chains with shared pin rows.
    #[cfg(feature = "write")]
    pub fn iec_scalar_chain_deletion_sites(&self) -> Option<Vec<IecFunctionCellDeletionSite>> {
        Some(
            self.iec_function_blocks()?
                .into_iter()
                .filter_map(|block| {
                    crate::iec_function_write::remove_chain(
                        self,
                        block.record_offset,
                        &block.name.value,
                    )
                    .ok()?;
                    Some(IecFunctionCellDeletionSite {
                        group_index: block.group_index,
                        row_index: block.row_index,
                        block_offset: block.record_offset,
                        raw_x: block.raw_x,
                        pin_count: block.pin_count,
                    })
                })
                .collect(),
        )
    }

    /// Captured comparison-result scaffolds accepting two input operands.
    #[cfg(feature = "write")]
    pub fn iec_wired_comparison_insertion_sites(
        &self,
    ) -> Option<Vec<IecWiredComparisonInsertionSite>> {
        let mut sites = self
            .iec_function_blocks()?
            .iter()
            .filter_map(|block| {
                let row_index = block.row_index.checked_sub(1)?;
                let group_index =
                    crate::iec_function_write::wired_comparison_scaffold(self, row_index, 10)?;
                Some(IecWiredComparisonInsertionSite {
                    group_index,
                    row_index,
                    raw_x: 10,
                })
            })
            .collect::<Vec<_>>();
        sites.extend(crate::iec_paired_comparison_write::sites(self)?);
        sites.extend(crate::iec_coil_comparison_write::sites(self)?);
        Some(sites)
    }

    /// A retained normal contact and two vacant rows for a terminal TON.
    pub fn iec_terminal_timer_insertion_sites(
        &self,
    ) -> Option<Vec<IecTerminalFunctionInsertionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let mut sites = Vec::new();
        #[cfg(feature = "write")]
        for contact in records
            .iter()
            .filter(|r| r.kind == IecRecordKind::Contact(6))
        {
            if let Some(site) = crate::iec_connected_timer_write::scaffold(self, contact.offset) {
                sites.push(site);
            }
            if let Some(site) =
                crate::iec_conversion_pair_write::timer_scaffold(self, contact.offset)
            {
                sites.push(site);
            }
            if let Some(site) = crate::iec_long_feed_timer_write::scaffold(self, contact.offset) {
                sites.push(site);
            }
        }
        for row in &rows {
            let start = row.start.checked_sub(10)?;
            let same_group = rows
                .iter()
                .filter(|r| r.group_index == row.group_index)
                .count();
            let Some(contact) = records
                .iter()
                .find(|r| r.group_index == row.group_index && r.row_index == row.row_index)
            else {
                continue;
            };
            let next = rows.iter().find(|r| r.group_index == row.group_index + 1);
            if row.row_index > u16::MAX / 4 - 2
                || same_group != 1
                || row.record_count != 1
                || contact.kind != IecRecordKind::Contact(6)
                || contact.end != row.end
                || self.data.get(contact.offset + 5) != Some(&1)
                || self.data.get(start + 4..start + 8) != Some(&[0, 0, 0, 0])
                || self.data.get(start + 8..start + 10) != Some(&[1, 0])
                || self.data.get(row.start + 29) != Some(&1)
                || next.is_none_or(|next| {
                    next.row_index != row.row_index + 3 || next.start != row.end + 10
                })
            {
                continue;
            }
            sites.push(IecTerminalFunctionInsertionSite {
                group_index: row.group_index,
                row_index: row.row_index,
                contact_offset: contact.offset,
                insertion_offset: row.end,
                raw_x: 7,
            });
        }
        Some(sites)
    }

    pub fn iec_terminal_function_insertion_sites(
        &self,
    ) -> Option<Vec<IecTerminalFunctionInsertionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let mut sites = Vec::new();
        for row in &rows {
            let group_index = row.group_index;
            let elevator =
                (1..=5).contains(&group_index) && row.row_index == (3 * group_index - 2) as u16;
            let lighting = group_index == 32 && row.row_index == 63;
            if (!elevator && !lighting) || row.record_count != 1 {
                continue;
            }
            let Some(next_group) = rows
                .iter()
                .find(|other| other.group_index == group_index + 1)
            else {
                continue;
            };
            let Some(contact) = records.iter().find(|record| {
                record.group_index == group_index && record.row_index == row.row_index
            }) else {
                continue;
            };
            let Some(group_start) = row.start.checked_sub(10) else {
                continue;
            };
            let mut expected_header = [0u8; 10];
            expected_header[..4].copy_from_slice(&(group_index as u32).to_le_bytes());
            expected_header[8..10].copy_from_slice(&1u16.to_le_bytes());
            if rows
                .iter()
                .filter(|other| other.group_index == group_index)
                .count()
                != 1
                || next_group.row_index != row.row_index + if lighting { 4 } else { 3 }
                || row.end.checked_add(10) != Some(next_group.start)
                || contact.kind
                    != if lighting {
                        IecRecordKind::Contact(6)
                    } else {
                        IecRecordKind::Contact(8)
                    }
                || contact.end != row.end
                || self.data.get(group_start..row.start) != Some(expected_header.as_slice())
                || self.data.get(row.start + 17..row.start + 21)
                    != Some(
                        (if lighting { 33u32 } else { 39u32 })
                            .to_le_bytes()
                            .as_slice(),
                    )
                || self.data.get(row.start + 29).copied() != Some(1)
                || self.data.get(contact.offset + 5).copied() != Some(1)
            {
                continue;
            }
            sites.push(IecTerminalFunctionInsertionSite {
                group_index,
                row_index: row.row_index,
                contact_offset: contact.offset,
                insertion_offset: row.end,
                raw_x: if lighting { 16 } else { 7 },
            });
        }
        Some(sites)
    }

    /// Find the standalone function group covered by the captured XG5000
    /// Delete operation. Its top row is exactly a one-cell long or short wire
    /// followed by a function block, and all subordinate rows belong only to
    /// that block. The short feed also occurs after deleting a chain's tail.
    pub fn iec_standalone_function_deletion_sites(
        &self,
    ) -> Option<Vec<IecStandaloneFunctionDeletionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let operand_links = self.iec_function_operand_links()?;
        let mut sites = Vec::new();
        for block in blocks {
            let group_rows = rows
                .iter()
                .filter(|row| row.group_index == block.group_index)
                .collect::<Vec<_>>();
            if group_rows.len() != usize::from(block.pin_count) + 1
                || group_rows.first().map(|row| row.row_index) != Some(block.row_index)
                || group_rows
                    .iter()
                    .enumerate()
                    .any(|(index, row)| row.row_index != block.row_index + index as u16)
            {
                continue;
            }
            let top_records = records
                .iter()
                .filter(|record| {
                    record.group_index == block.group_index && record.row_index == block.row_index
                })
                .collect::<Vec<_>>();
            if top_records.len() != 2
                || !matches!(
                    top_records[0].kind,
                    IecRecordKind::LongWire | IecRecordKind::ShortWire
                )
                || top_records[1].kind != IecRecordKind::FunctionBlock
                || top_records[1].offset != block.record_offset
                || top_records[0].end != block.record_offset
            {
                continue;
            }
            let wire = self.data.get(top_records[0].offset..top_records[0].end)?;
            let one_cell_feed = match top_records[0].kind {
                IecRecordKind::LongWire => wire.get(15).copied() == Some(1),
                IecRecordKind::ShortWire => wire.get(9..15) == Some([0; 6].as_slice()),
                _ => false,
            };
            if wire[5] != 1 || block.raw_x != 4 || !one_cell_feed {
                continue;
            }
            let child_records = records
                .iter()
                .filter(|record| {
                    record.group_index == block.group_index && record.row_index > block.row_index
                })
                .collect::<Vec<_>>();
            if child_records.is_empty()
                || child_records.iter().any(|record| {
                    !matches!(
                        record.kind,
                        IecRecordKind::FunctionOperand | IecRecordKind::LinkReference(_)
                    )
                })
                || child_records.iter().any(|record| match record.kind {
                    IecRecordKind::FunctionOperand => !operand_links.iter().any(|link| {
                        link.record_offset == record.offset
                            && link.target_record_offset == block.record_offset
                    }),
                    IecRecordKind::LinkReference(_) => !references.iter().any(|reference| {
                        reference.record_offset == record.offset
                            && reference.target_record_offset == block.record_offset
                    }),
                    _ => true,
                })
            {
                continue;
            }
            sites.push(IecStandaloneFunctionDeletionSite {
                group_index: block.group_index,
                row_index: block.row_index,
                block_offset: block.record_offset,
                wire_offset: top_records[0].offset,
                raw_x: block.raw_x,
                pin_count: block.pin_count,
            });
        }
        Some(sites)
    }

    /// Find a three-row gap at a native group boundary for a standalone block.
    pub fn iec_standalone_function_insertion_sites(
        &self,
    ) -> Option<Vec<IecStandaloneFunctionInsertionSite>> {
        let rows = self.iec_row_frames()?;
        self.iec_record_frames()?;
        let mut sites = Vec::new();
        for pair in rows.windows(2) {
            let [before, after] = pair else { continue };
            let Some(row_index) = before.row_index.checked_add(1) else {
                continue;
            };
            let Some(group_index) = before.group_index.checked_add(1) else {
                continue;
            };
            let Some(next_row) = row_index.checked_add(3) else {
                continue;
            };
            let Some(next_group_start) = before.end.checked_add(10) else {
                continue;
            };
            let Some(group_number) = u32::try_from(group_index).ok() else {
                continue;
            };
            let mut expected_header = [0u8; 10];
            expected_header[..4].copy_from_slice(&group_number.to_le_bytes());
            expected_header[8..].copy_from_slice(&1u16.to_le_bytes());
            if after.row_index == next_row
                && after.group_index == group_index
                && next_group_start == after.start
                && self.data.get(before.end..after.start) == Some(expected_header.as_slice())
                && row_index.checked_mul(4).is_some()
                && row_index
                    .checked_add(2)
                    .and_then(|row| row.checked_mul(4))
                    .is_some()
            {
                sites.push(IecStandaloneFunctionInsertionSite {
                    group_index,
                    row_index,
                    insertion_offset: before.end,
                    raw_x: 4,
                });
            }
        }
        Some(sites)
    }

    /// Find the two captured five-row R_TRIG/arithmetic/MOVE layouts. Every
    /// removed pin record must resolve to the ADD or SUB block.
    pub fn iec_connected_arithmetic_deletion_sites(
        &self,
    ) -> Option<Vec<IecConnectedArithmeticDeletionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let operands = self.iec_function_operand_links()?;
        let mut sites = Vec::new();
        for add in blocks
            .iter()
            .filter(|block| matches!(block.name.value.as_str(), "ADD" | "SUB"))
        {
            let group_rows = rows
                .iter()
                .filter(|row| row.group_index == add.group_index)
                .collect::<Vec<_>>();
            if group_rows.len() != 5
                || group_rows[0].row_index != add.row_index
                || !group_rows
                    .iter()
                    .enumerate()
                    .all(|(i, row)| row.row_index == add.row_index + i as u16)
                || add.pin_count != 3
                || add.raw_x != 16
            {
                continue;
            }
            let group_records = group_rows
                .iter()
                .map(|row| {
                    records
                        .iter()
                        .filter(|record| {
                            record.group_index == row.group_index
                                && record.row_index == row.row_index
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if group_records.iter().map(Vec::len).collect::<Vec<_>>() != [5, 4, 4, 4, 1]
                || !matches!(group_records[0][0].kind, IecRecordKind::Contact(6..=11))
                || !matches!(
                    group_records[0][1].kind,
                    IecRecordKind::Contact(6..=11) | IecRecordKind::ShortWire
                )
                || group_records[0][2].kind != IecRecordKind::FunctionBlock
                || group_records[0][3].kind != IecRecordKind::LongWire
                || group_records[0][4].offset != add.record_offset
                || group_records[0][3].end != add.record_offset
                || group_records[1][0].kind != IecRecordKind::LinkReference(0x69)
                || group_records[1][1].kind != IecRecordKind::FunctionOperand
                || group_records[1][2].kind != IecRecordKind::LinkReference(0x68)
                || group_records[1][3].kind != IecRecordKind::FunctionOperand
                || group_records[2][0].kind != IecRecordKind::LongWire
                || group_records[2][1].kind != IecRecordKind::FunctionBlock
                || group_records[2][2].kind != IecRecordKind::FunctionOperand
                || group_records[2][3].kind != IecRecordKind::LinkReference(0x68)
                || group_records[3][0].kind != IecRecordKind::FunctionOperand
                || group_records[3][1].kind != IecRecordKind::LinkReference(0x68)
                || group_records[3][2].kind != IecRecordKind::FunctionOperand
                || group_records[3][3].kind != IecRecordKind::LinkReference(0x69)
                || group_records[4][0].kind != IecRecordKind::LinkReference(0x69)
                || !blocks.iter().any(|block| {
                    block.record_offset == group_records[0][2].offset
                        && block.name.value == "R_TRIG"
                })
                || !blocks.iter().any(|block| {
                    block.record_offset == group_records[2][1].offset && block.name.value == "MOVE"
                })
                || self.data.get(group_records[0][3].offset + 5) != Some(&10)
                || self.data.get(group_records[0][3].offset + 15) != Some(&13)
            {
                continue;
            }
            let add_refs = references
                .iter()
                .filter(|reference| reference.target_record_offset == add.record_offset)
                .collect::<Vec<_>>();
            let add_operands = operands
                .iter()
                .filter(|operand| operand.target_record_offset == add.record_offset)
                .collect::<Vec<_>>();
            let removed_offsets = [
                group_records[1][1].offset,
                group_records[1][2].offset,
                group_records[1][3].offset,
                group_records[2][2].offset,
                group_records[2][3].offset,
                group_records[3][3].offset,
            ];
            if add_refs.len() != 3 || add_operands.len() != 3 {
                continue;
            }
            let linked_offsets = add_refs
                .iter()
                .map(|item| item.record_offset)
                .chain(add_operands.iter().map(|item| item.record_offset))
                .collect::<Vec<_>>();
            if linked_offsets.len() != removed_offsets.len()
                || !removed_offsets
                    .iter()
                    .all(|offset| linked_offsets.contains(offset))
            {
                continue;
            }
            sites.push(IecConnectedArithmeticDeletionSite {
                group_index: add.group_index,
                row_index: add.row_index,
                block_offset: add.record_offset,
                wire_offset: group_records[0][3].offset,
                raw_x: add.raw_x,
            });
        }
        Some(sites)
    }

    /// Find the connected single-output function-cell shape covered by the
    /// captured XG5000 Delete operation. The block follows one addressed
    /// contact and its link row also contains unrelated retained rung records.
    pub fn iec_function_cell_deletion_sites(&self) -> Option<Vec<IecFunctionCellDeletionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let operand_links = self.iec_function_operand_links()?;
        let mut sites = Vec::new();
        for block in &blocks {
            let group_rows = rows
                .iter()
                .filter(|row| row.group_index == block.group_index)
                .collect::<Vec<_>>();
            // Native Delete on a connected R_TRIG removes only its body and
            // output reference. The other functions and stored rows stay put;
            // the resulting horizontal gap can be repaired independently.
            if block.name.value == "R_TRIG"
                && block.opcode_family == 0x21
                && block.opcode == 0x1f
                && block.pin_count == 1
            {
                let own_refs = references
                    .iter()
                    .filter(|reference| reference.target_record_offset == block.record_offset)
                    .collect::<Vec<_>>();
                let body_index = records
                    .iter()
                    .position(|r| r.offset == block.record_offset)?;
                let preceding = body_index.checked_sub(1).and_then(|i| records.get(i));
                let following = records.get(body_index + 1);
                let retained_rows = group_rows.iter().all(|row| {
                    records.iter().any(|record| {
                        record.group_index == row.group_index
                            && record.row_index == row.row_index
                            && record.offset != block.record_offset
                            && !own_refs.iter().any(|r| r.record_offset == record.offset)
                    })
                });
                if own_refs.len() == 1
                    && own_refs[0].is_output
                    && own_refs[0].group_index == block.group_index
                    && block.row_index.checked_add(1) == Some(own_refs[0].row_index)
                    && !operand_links
                        .iter()
                        .any(|link| link.target_record_offset == block.record_offset)
                    && preceding.is_some_and(|r| {
                        r.group_index == block.group_index
                            && r.row_index == block.row_index
                            && match r.kind {
                                IecRecordKind::Contact(6..=11) | IecRecordKind::ShortWire => {
                                    self.data[r.offset + 5].checked_add(3) == Some(block.raw_x)
                                }
                                IecRecordKind::LongWire => {
                                    self.data[r.offset + 15].checked_add(3) == Some(block.raw_x)
                                }
                                _ => false,
                            }
                    })
                    && following.is_some_and(|r| {
                        r.group_index == block.group_index
                            && r.row_index == block.row_index
                            && r.kind == IecRecordKind::LongWire
                            && block.raw_x.checked_add(3) == Some(self.data[r.offset + 5])
                    })
                    && retained_rows
                {
                    sites.push(IecFunctionCellDeletionSite {
                        group_index: block.group_index,
                        row_index: block.row_index,
                        block_offset: block.record_offset,
                        raw_x: block.raw_x,
                        pin_count: block.pin_count,
                    });
                }
                continue;
            }
            if block.pin_count != 1
                || group_rows.len() != 2
                || group_rows.first().map(|row| row.row_index) != Some(block.row_index)
                || group_rows.get(1).map(|row| row.row_index) != Some(block.row_index + 1)
                || blocks
                    .iter()
                    .filter(|candidate| candidate.group_index == block.group_index)
                    .count()
                    != 1
            {
                continue;
            }
            let top_records = records
                .iter()
                .filter(|record| {
                    record.group_index == block.group_index && record.row_index == block.row_index
                })
                .collect::<Vec<_>>();
            if top_records.len() < 3
                || !matches!(top_records[0].kind, IecRecordKind::Contact(6..=11))
                || top_records[1].kind != IecRecordKind::FunctionBlock
                || top_records[1].offset != block.record_offset
                || top_records[0].end != block.record_offset
                || self.data.get(top_records[0].offset + 5).copied() != Some(1)
                || block.raw_x != 4
            {
                continue;
            }
            let block_references = references
                .iter()
                .filter(|reference| reference.target_record_offset == block.record_offset)
                .collect::<Vec<_>>();
            let block_operands = operand_links
                .iter()
                .filter(|link| link.target_record_offset == block.record_offset)
                .collect::<Vec<_>>();
            if block_references.len() != 1
                || !block_references[0].is_output
                || block_references[0].group_index != block.group_index
                || block_references[0].row_index != block.row_index + 1
                || !block_operands.is_empty()
            {
                continue;
            }
            let removed_offsets = [block.record_offset, block_references[0].record_offset];
            if group_rows.iter().any(|row| {
                records
                    .iter()
                    .filter(|record| {
                        record.group_index == row.group_index
                            && record.row_index == row.row_index
                            && !removed_offsets.contains(&record.offset)
                    })
                    .count()
                    == 0
            }) {
                continue;
            }
            sites.push(IecFunctionCellDeletionSite {
                group_index: block.group_index,
                row_index: block.row_index,
                block_offset: block.record_offset,
                raw_x: block.raw_x,
                pin_count: block.pin_count,
            });
        }
        Some(sites)
    }

    /// Find the captured two-row gap produced by deleting a connected FF
    /// function cell. The top row retains the leading contact and following
    /// rung records; the second row retains the branch path and output coil.
    pub fn iec_function_cell_insertion_sites(&self) -> Option<Vec<IecFunctionCellInsertionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let blocks = self.iec_function_blocks()?;
        let references = self.iec_function_references()?;
        let mut sites = Vec::new();
        for first_row in &rows {
            let group_rows = rows
                .iter()
                .filter(|row| row.group_index == first_row.group_index)
                .collect::<Vec<_>>();
            if group_rows.len() != 2
                || group_rows[0].row_index != first_row.row_index
                || group_rows[1].row_index != first_row.row_index + 1
                || blocks
                    .iter()
                    .any(|block| block.group_index == first_row.group_index)
                || references
                    .iter()
                    .any(|reference| reference.group_index == first_row.group_index)
            {
                continue;
            }
            let top = records
                .iter()
                .filter(|record| {
                    record.group_index == first_row.group_index
                        && record.row_index == first_row.row_index
                })
                .collect::<Vec<_>>();
            let bottom = records
                .iter()
                .filter(|record| {
                    record.group_index == first_row.group_index
                        && record.row_index == first_row.row_index + 1
                })
                .collect::<Vec<_>>();
            if top.len() < 2
                || bottom.is_empty()
                || !matches!(top[0].kind, IecRecordKind::Contact(6..=11))
                || top[1].kind != IecRecordKind::LongWire
                || bottom[0].kind != IecRecordKind::BranchEnd
                || top[0].end != top[1].offset
                || self.data.get(top[0].offset + 5).copied() != Some(1)
                || self.data.get(top[1].offset + 5).copied() != Some(7)
                || self.data.get(bottom[0].offset + 5).copied() != Some(24)
            {
                continue;
            }
            sites.push(IecFunctionCellInsertionSite {
                function_name: "FF",
                group_index: first_row.group_index,
                row_index: first_row.row_index,
                insertion_offset: top[1].offset,
                reference_offset: bottom[0].offset,
                raw_x: 4,
            });
        }
        for gap in self.iec_horizontal_wire_repair_sites()? {
            let wire_index = records
                .iter()
                .position(|r| r.offset == gap.insertion_offset)?;
            let wire = &records[wire_index];
            let Some(next) = records.get(wire_index + 1) else {
                continue;
            };
            let Some(bottom) = rows
                .iter()
                .find(|r| r.group_index == gap.group_index && r.row_index == gap.row_index + 1)
            else {
                continue;
            };
            if wire.kind != IecRecordKind::LongWire
                || next.kind != IecRecordKind::FunctionBlock
                || next.group_index != gap.group_index
                || next.row_index != gap.row_index
                || records
                    .iter()
                    .filter(|r| r.group_index == gap.group_index && r.row_index == bottom.row_index)
                    .any(|r| self.data[r.offset + 5] <= gap.raw_x)
            {
                continue;
            }
            sites.push(IecFunctionCellInsertionSite {
                function_name: "R_TRIG",
                group_index: gap.group_index,
                row_index: gap.row_index,
                insertion_offset: gap.insertion_offset,
                reference_offset: bottom.records_start,
                raw_x: gap.raw_x,
            });
        }
        Some(sites)
    }

    /// Find captured two-row branch upper lines with an empty x1 contact cell.
    pub fn iec_leading_contact_insertion_sites(
        &self,
    ) -> Option<Vec<IecLeadingContactInsertionSite>> {
        let rows = self.iec_row_frames()?;
        let records = self.iec_record_frames()?;
        let mut sites = Vec::new();
        for window in rows.windows(2) {
            let [top, lower] = window else { continue };
            if top.group_index != lower.group_index
                || lower.row_index != top.row_index.saturating_add(1)
                || rows
                    .iter()
                    .filter(|row| row.group_index == top.group_index)
                    .count()
                    != 2
            {
                continue;
            }
            let upper = records
                .iter()
                .filter(|record| {
                    record.group_index == top.group_index && record.row_index == top.row_index
                })
                .collect::<Vec<_>>();
            let lower_records = records
                .iter()
                .filter(|record| {
                    record.group_index == lower.group_index && record.row_index == lower.row_index
                })
                .collect::<Vec<_>>();
            if upper.len() != top.record_count as usize
                || upper.len() < 3
                || !matches!(upper[0].kind, IecRecordKind::Contact(6..=11))
                || self.data.get(upper[0].offset + 5) != Some(&4)
                || upper
                    .iter()
                    .filter(|record| record.kind == IecRecordKind::BranchStart)
                    .count()
                    != 1
                || !upper
                    .last()
                    .is_some_and(|record| matches!(record.kind, IecRecordKind::Coil(_)))
                || lower_records
                    .last()
                    .is_none_or(|record| record.kind != IecRecordKind::BranchEnd)
            {
                continue;
            }
            sites.push(IecLeadingContactInsertionSite {
                group_index: top.group_index,
                row_index: top.row_index,
                insertion_offset: top.records_start,
            });
        }
        #[cfg(feature = "write")]
        sites.extend(
            crate::iec_contact_mesh_move_write::leading_contact_sites(self, false)?
                .into_iter()
                .map(|(group_index, row_index, insertion_offset, _)| {
                    IecLeadingContactInsertionSite {
                        group_index,
                        row_index,
                        insertion_offset,
                    }
                }),
        );
        Some(sites)
    }

    /// Find framed long wires with room for a contact on the IEC grid.
    /// The writer validates the resulting circuit graph before accepting an edit.
    pub fn iec_no_contact_insertion_sites(&self) -> Option<Vec<IecNoContactInsertionSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in rows {
            let row_records = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            if row_records.len() != row.record_count as usize {
                continue;
            }
            for wire in row_records
                .iter()
                .filter(|record| record.kind == IecRecordKind::LongWire)
            {
                let bytes = self.data.get(wire.offset..wire.end)?;
                if !matches!(&bytes[9..15], [0, 0, 4, 0, 0, 0] | [0, 0, 0, 0, 0, 0])
                    || bytes[5].saturating_add(3) > bytes[15]
                {
                    continue;
                }
                sites.push(IecNoContactInsertionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    wire_offset: wire.offset,
                    start_x: bytes[5],
                    end_x: bytes[15],
                });
            }
        }
        Some(sites)
    }

    /// Find framed one-cell wires with the captured contact-grid geometry.
    /// The writer validates the resulting circuit graph for a chosen operand.
    pub fn iec_short_wire_contact_insertion_sites(
        &self,
    ) -> Option<Vec<IecShortWireContactInsertionSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in rows {
            let row_records = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            if row_records.len() != row.record_count as usize {
                continue;
            }
            for wire in row_records
                .into_iter()
                .filter(|record| record.kind == IecRecordKind::ShortWire)
            {
                let bytes = self.data.get(wire.offset..wire.end)?;
                let y = row.row_index.checked_mul(4)?.to_le_bytes();
                if bytes.len() != 15
                    || bytes[5] == 0
                    || bytes[5] > 91
                    || !(bytes[5] - 1).is_multiple_of(3)
                    || bytes[6..9] != [y[0], y[1], 0]
                    || bytes[9..15] != [0, 0, 4, 0, 0, 0]
                {
                    continue;
                }
                sites.push(IecShortWireContactInsertionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    wire_offset: wire.offset,
                    raw_x: bytes[5],
                });
            }
        }
        Some(sites)
    }

    /// Find decoded contacts supported by native Delete. These are contacts
    /// between matching long wires and captured leading contacts on a
    /// two-row branch whose next element is a contact at x4.
    pub fn iec_no_contact_deletion_sites(&self) -> Option<Vec<IecNoContactDeletionSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in &rows {
            let parts = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            if parts.len() != row.record_count as usize {
                continue;
            }
            if let [contact, endpoint] = parts.as_slice()
                && let IecRecordKind::Contact(contact_code @ 6..=11) = contact.kind
                && endpoint.kind == IecRecordKind::BranchEnd
                && self.data.get(contact.offset + 5) == Some(&1)
                && self.data.get(endpoint.offset + 5) == Some(&3)
            {
                sites.push(IecNoContactDeletionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    contact_offset: contact.offset,
                    raw_x: 1,
                    contact_code,
                });
            }
            if row.record_count < 3 {
                continue;
            }
            let group_rows = rows
                .iter()
                .filter(|candidate| candidate.group_index == row.group_index)
                .collect::<Vec<_>>();
            if group_rows.len() == 2
                && group_rows[0].row_index == row.row_index
                && group_rows[1].row_index == row.row_index.saturating_add(1)
                && parts.len() >= 4
                && matches!(parts[0].kind, IecRecordKind::Contact(6..=11))
                && matches!(parts[1].kind, IecRecordKind::Contact(6..=11))
                && parts
                    .iter()
                    .filter(|record| record.kind == IecRecordKind::BranchStart)
                    .count()
                    == 1
                && parts
                    .last()
                    .is_some_and(|record| matches!(record.kind, IecRecordKind::Coil(_)))
                && records.iter().any(|record| {
                    record.group_index == row.group_index
                        && record.row_index == group_rows[1].row_index
                        && record.kind == IecRecordKind::BranchEnd
                })
                && self.data.get(parts[0].offset + 5) == Some(&1)
                && self.data.get(parts[1].offset + 5) == Some(&4)
            {
                let IecRecordKind::Contact(contact_code) = parts[0].kind else {
                    unreachable!()
                };
                sites.push(IecNoContactDeletionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    contact_offset: parts[0].offset,
                    raw_x: 1,
                    contact_code,
                });
            }
            for window in parts.windows(3) {
                let [left_frame, contact_frame, right_frame] = window else {
                    continue;
                };
                let IecRecordKind::Contact(contact_code @ 6..=11) = contact_frame.kind else {
                    continue;
                };
                if left_frame.kind != IecRecordKind::LongWire
                    || right_frame.kind != IecRecordKind::LongWire
                {
                    continue;
                }
                let left = self.data.get(left_frame.offset..left_frame.end)?;
                let contact = self.data.get(contact_frame.offset..contact_frame.end)?;
                let right = self.data.get(right_frame.offset..right_frame.end)?;
                if left.len() != 19
                    || right.len() != 19
                    || contact.len() < 21
                    || !matches!(&left[9..15], [0, 0, 4, 0, 0, 0] | [0, 0, 0, 0, 0, 0])
                    || right[9..15] != left[9..15]
                    || left[15].checked_add(3) != Some(contact[5])
                    || contact[5].checked_add(3) != Some(right[5])
                    || left[6..9] != contact[6..9]
                    || contact[6..9] != right[6..9]
                    || left[16..19] != right[16..19]
                {
                    continue;
                }
                sites.push(IecNoContactDeletionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    contact_offset: contact_frame.offset,
                    raw_x: contact[5],
                    contact_code,
                });
            }
        }
        #[cfg(feature = "write")]
        sites.extend(
            crate::iec_contact_mesh_move_write::leading_contact_sites(self, true)?
                .into_iter()
                .map(|(group_index, row_index, contact_offset, contact_code)| {
                    IecNoContactDeletionSite {
                        group_index,
                        row_index,
                        contact_offset,
                        raw_x: 1,
                        contact_code,
                    }
                }),
        );
        #[cfg(feature = "write")]
        for site in crate::iec_contact_write::deletion_sites(self)? {
            if !sites
                .iter()
                .any(|s| s.contact_offset == site.contact_offset)
            {
                sites.push(site);
            }
        }
        Some(sites)
    }

    /// Find contacts whose captured row shape supports XG5000 Cell Delete.
    /// A leading branch contact moves the next contact into x1 and puts a
    /// short wire at x4; contacts between long wires shift later cells left.
    pub fn iec_no_contact_cell_deletion_sites(&self) -> Option<Vec<IecNoContactDeletionSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in rows {
            if row.record_count < 5 {
                continue;
            }
            let parts = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            if parts.len() != row.record_count as usize
                || !parts
                    .last()
                    .is_some_and(|record| matches!(record.kind, IecRecordKind::Coil(_)))
            {
                continue;
            }
            for index in 1..parts.len() - 2 {
                if !matches!(parts[index].kind, IecRecordKind::Contact(6..=11))
                    || parts[index - 1].kind != IecRecordKind::LongWire
                    || parts[index + 1].kind != IecRecordKind::LongWire
                    || !parts[index + 1..].iter().all(|record| {
                        matches!(
                            record.kind,
                            IecRecordKind::Contact(_)
                                | IecRecordKind::LongWire
                                | IecRecordKind::Coil(_)
                        )
                    })
                {
                    continue;
                }
                let left = self
                    .data
                    .get(parts[index - 1].offset..parts[index - 1].end)?;
                let contact = self.data.get(parts[index].offset..parts[index].end)?;
                let right = self
                    .data
                    .get(parts[index + 1].offset..parts[index + 1].end)?;
                if left.len() != 19
                    || right.len() != 19
                    || contact.len() < 21
                    || !matches!(&left[9..15], [0, 0, 4, 0, 0, 0] | [0, 0, 0, 0, 0, 0])
                    || right[9..15] != left[9..15]
                    || left[15].checked_add(3) != Some(contact[5])
                    || contact[5].checked_add(3) != Some(right[5])
                    || left[6..9] != contact[6..9]
                    || contact[6..9] != right[6..9]
                    || left[16..19] != right[16..19]
                {
                    continue;
                }
                sites.push(IecNoContactDeletionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    contact_offset: parts[index].offset,
                    raw_x: contact[5],
                    contact_code: match parts[index].kind {
                        IecRecordKind::Contact(code) => code,
                        _ => unreachable!(),
                    },
                });
            }
        }
        for leading in self
            .iec_no_contact_deletion_sites()?
            .into_iter()
            .filter(|site| site.raw_x == 1)
        {
            let parts = records
                .iter()
                .filter(|record| {
                    record.group_index == leading.group_index
                        && record.row_index == leading.row_index
                })
                .collect::<Vec<_>>();
            if parts.len() >= 3
                && parts[0].offset == leading.contact_offset
                && matches!(parts[1].kind, IecRecordKind::Contact(6..=11))
                && parts[2].kind == IecRecordKind::BranchStart
                && self.data.get(parts[1].offset + 5) == Some(&4)
            {
                sites.push(leading);
            }
        }
        Some(sites)
    }

    /// Find one-cell gaps between matching long wires. XG5000 reconnects
    /// these gaps by inserting an `FF 01` short wire.
    pub fn iec_horizontal_wire_repair_sites(&self) -> Option<Vec<IecHorizontalWireRepairSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in rows {
            if row.record_count < 3 {
                continue;
            }
            let parts = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            if parts.len() != row.record_count as usize {
                continue;
            }
            for (index, pair) in parts.windows(2).enumerate() {
                let [left_frame, right_frame] = pair else {
                    continue;
                };
                if matches!(
                    left_frame.kind,
                    IecRecordKind::Contact(6..=11) | IecRecordKind::ShortWire
                ) && right_frame.kind == IecRecordKind::LongWire
                    && left_frame.end == right_frame.offset
                {
                    let left = self.data.get(left_frame.offset..left_frame.end)?;
                    let right = self.data.get(right_frame.offset..right_frame.end)?;
                    if right.len() == 19
                        && left[5].checked_add(6) == Some(right[5])
                        && left[6..9] == right[6..9]
                        && right[6..9] == right[16..19]
                        && right[5] <= right[15]
                        && right[9..15] == [0; 6]
                        && (left_frame.kind != IecRecordKind::ShortWire || left[9..15] == [0; 6])
                    {
                        sites.push(IecHorizontalWireRepairSite {
                            group_index: row.group_index,
                            row_index: row.row_index,
                            insertion_offset: right_frame.offset,
                            raw_x: left[5] + 3,
                        });
                    }
                    continue;
                }
                if left_frame.kind != IecRecordKind::LongWire
                    || right_frame.kind != IecRecordKind::LongWire
                    || left_frame.end != right_frame.offset
                    || !parts[..index]
                        .iter()
                        .any(|record| matches!(record.kind, IecRecordKind::Contact(_)))
                {
                    continue;
                }
                let left = self.data.get(left_frame.offset..left_frame.end)?;
                let right = self.data.get(right_frame.offset..right_frame.end)?;
                if left.len() != 19 || right.len() != 19 {
                    continue;
                }
                let gap_x = left[15].checked_add(3)?;
                if left[5] > left[15]
                    || right[5] >= right[15]
                    || left[15].checked_add(6) != Some(right[5])
                    || (self.data.get(row.start + 29) != Some(&gap_x)
                        && self.data.get(row.start + 29).copied()
                            != parts
                                .iter()
                                .filter(|r| {
                                    matches!(
                                        r.kind,
                                        IecRecordKind::Contact(_) | IecRecordKind::FunctionBlock
                                    )
                                })
                                .map(|r| self.data[r.offset + 5])
                                .max())
                    || !matches!(&left[9..15], [0, 0, 4, 0, 0, 0] | [0, 0, 0, 0, 0, 0])
                    || right[9..15] != left[9..15]
                    || left[6..9] != right[6..9]
                    || left[16..19] != right[16..19]
                {
                    continue;
                }
                sites.push(IecHorizontalWireRepairSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    insertion_offset: right_frame.offset,
                    raw_x: gap_x,
                });
            }
        }
        Some(sites)
    }

    pub fn iec_horizontal_wire_deletion_sites(&self) -> Option<Vec<IecHorizontalWireDeletionSite>> {
        let records = self.iec_record_frames()?;
        let rows = self.iec_row_frames()?;
        let mut sites = Vec::new();
        for row in rows {
            let parts = records
                .iter()
                .filter(|record| {
                    record.group_index == row.group_index && record.row_index == row.row_index
                })
                .collect::<Vec<_>>();
            for (index, triple) in parts.windows(3).enumerate() {
                let [left, wire, right] = triple else {
                    continue;
                };
                if left.kind != IecRecordKind::LongWire
                    || wire.kind != IecRecordKind::ShortWire
                    || right.kind != IecRecordKind::LongWire
                    || left.end != wire.offset
                    || wire.end != right.offset
                    || !parts[..index]
                        .iter()
                        .any(|record| matches!(record.kind, IecRecordKind::Contact(_)))
                {
                    continue;
                }
                let left_bytes = self.data.get(left.offset..left.end)?;
                let wire_bytes = self.data.get(wire.offset..wire.end)?;
                let right_bytes = self.data.get(right.offset..right.end)?;
                if left_bytes.len() != 19
                    || wire_bytes.len() != 15
                    || right_bytes.len() != 19
                    || left_bytes[15].checked_add(3) != Some(wire_bytes[5])
                    || wire_bytes[5].checked_add(3) != Some(right_bytes[5])
                    || left_bytes[6..9] != wire_bytes[6..9]
                    || right_bytes[6..9] != wire_bytes[6..9]
                    || left_bytes[9..15] != wire_bytes[9..15]
                    || right_bytes[9..15] != wire_bytes[9..15]
                {
                    continue;
                }
                let mut updated = Vec::with_capacity(self.data.len() - wire_bytes.len());
                updated.extend_from_slice(&self.data[..wire.offset]);
                updated.extend_from_slice(&self.data[wire.end..]);
                updated[row.start + 29] = wire_bytes[5];
                updated[row.start + 33..row.start + 35]
                    .copy_from_slice(&(row.record_count - 1).to_le_bytes());
                let mut preview = self.clone();
                preview.decoded_len = updated.len();
                preview.data = updated;
                if preview.iec_circuit_graph().is_none()
                    || !preview
                        .iec_horizontal_wire_repair_sites()?
                        .iter()
                        .any(|site| {
                            site.insertion_offset == wire.offset && site.raw_x == wire_bytes[5]
                        })
                {
                    continue;
                }
                sites.push(IecHorizontalWireDeletionSite {
                    group_index: row.group_index,
                    row_index: row.row_index,
                    wire_offset: wire.offset,
                    raw_x: wire_bytes[5],
                });
            }
        }
        Some(sites)
    }
}

const IEC_PIN_INPUT_FLAG: u32 = 0x0020_0000;
const IEC_PIN_ARRAY_FLAG: u32 = 0x0200_0000;
const IEC_PIN_TYPE_MASK: u32 = 0x000f_ffff;
const IEC_PIN_KNOWN_FLAGS: u32 = IEC_PIN_INPUT_FLAG | IEC_PIN_ARRAY_FLAG | IEC_PIN_TYPE_MASK;

fn parse_function_block(data: &[u8], record: IecRecordFrame) -> Option<IecFunctionBlock> {
    let bytes = data.get(record.offset..record.end)?;
    if bytes.len() < 94
        || bytes.get(1) != Some(&0x67)
        || bytes.get(5).copied()? == 0
        || u16::from_le_bytes([bytes[6], bytes[7]]) != record.row_index.checked_mul(4)?
    {
        return None;
    }
    let raw_x = bytes[5];
    let output_x = raw_x.checked_add(3)?;
    let opcode_family = bytes[15];
    let opcode = u16::from_le_bytes([bytes[16], bytes[17]]);
    let pin_count = bytes[20];
    if !(1..=32).contains(&pin_count) {
        return None;
    }

    let (mut control_input, cursor) =
        parse_function_pin_descriptor(bytes, record.offset, 38, raw_x, record.row_index, true)?;
    let control_input = control_input.take()?;
    if control_input.direction != IecFunctionPinDirection::Input
        || control_input.data_type_mask != 1
        || control_input.is_array
        || !matches!(control_input.name.value.as_str(), "EN" | "IN" | "CLK")
    {
        return None;
    }
    let (mut control_output, cursor) = parse_function_pin_descriptor(
        bytes,
        record.offset,
        cursor,
        output_x,
        record.row_index,
        true,
    )?;
    let mut control_output = control_output.take()?;
    if control_output.direction != IecFunctionPinDirection::Output
        || control_output.data_type_mask != 1
        || control_output.is_array
        || !matches!(control_output.name.value.as_str(), "ENO" | "Q")
    {
        return None;
    }

    let (name, cursor) = parse_marker_string(bytes, record.offset, cursor)?;
    let (instance_field, mut cursor) = parse_marker_string(bytes, record.offset, cursor)?;
    if name.value.is_empty() || (opcode_family == 0x21) != !instance_field.value.is_empty() {
        return None;
    }
    let instance = (!instance_field.value.is_empty()).then(|| instance_field.clone());
    let mut pins = Vec::new();
    let mut body_flag = None;
    for ordinal in 1..=pin_count {
        let header = bytes.get(cursor..cursor + 12)?;
        let expected_row = record.row_index.checked_add(u16::from(ordinal))?;
        let expected_y = expected_row.checked_mul(4)?;
        let current_body_flag = u16::from_le_bytes([header[6], header[7]]);
        if !(matches!(current_body_flag, 0 | 2 | 4) || current_body_flag == u16::from(raw_x))
            || body_flag.is_some_and(|expected| expected != current_body_flag)
        {
            return None;
        }
        body_flag.get_or_insert(current_body_flag);
        let continuation_flag = u16::from(ordinal < pin_count) * 2;
        if header[0] != raw_x
            || u16::from_le_bytes([header[1], header[2]]) != expected_y
            || header[3..6] != [0; 3]
            || header[8..10] != [0; 2]
            || u16::from_le_bytes([header[10], header[11]]) != continuation_flag
        {
            return None;
        }
        cursor += 12;
        if ordinal == pin_count {
            continue;
        }
        let (left, next) = parse_function_pin_descriptor(
            bytes,
            record.offset,
            cursor,
            raw_x,
            expected_row,
            false,
        )?;
        cursor = next;
        let (right, next) = parse_function_pin_descriptor(
            bytes,
            record.offset,
            cursor,
            output_x,
            expected_row,
            false,
        )?;
        cursor = next;
        if left
            .as_ref()
            .is_some_and(|pin| pin.direction != IecFunctionPinDirection::Input)
            || right
                .as_ref()
                .is_some_and(|pin| pin.direction != IecFunctionPinDirection::Output)
        {
            return None;
        }
        pins.extend(left);
        pins.extend(right);
    }
    if cursor != bytes.len() {
        return None;
    }

    let mut ordered = pins
        .iter()
        .enumerate()
        .filter_map(|(index, pin)| {
            (pin.direction == IecFunctionPinDirection::Input).then_some(index)
        })
        .chain(pins.iter().enumerate().filter_map(|(index, pin)| {
            (pin.direction == IecFunctionPinDirection::Output).then_some(index)
        }))
        .collect::<Vec<_>>();
    if ordered.len() == usize::from(pin_count) {
        for (ordinal, index) in ordered.drain(..).enumerate() {
            pins[index].reference_ordinal = Some(u8::try_from(ordinal + 1).ok()?);
        }
    } else if ordered.is_empty() && pin_count == 1 {
        control_output.reference_ordinal = Some(1);
    } else {
        return None;
    }

    let strings = crate::internal::extract_utf16_marker_strings(bytes, false, true)
        .into_iter()
        .map(|mut string| {
            string.offset += record.offset;
            string.end_offset += record.offset;
            string
        })
        .collect::<Vec<_>>();
    if strings.len() < 6
        || strings[0]
            != control_input
                .type_expression
                .clone()
                .unwrap_or_else(|| strings[0].clone())
        || strings[1] != control_input.name
        || strings[2]
            != control_output
                .type_expression
                .clone()
                .unwrap_or_else(|| strings[2].clone())
        || strings[3] != control_output.name
        || strings[4] != name
        || strings[5] != instance_field
    {
        return None;
    }
    let field_strings = strings
        .into_iter()
        .skip(if instance.is_some() { 6 } else { 5 })
        .collect();

    Some(IecFunctionBlock {
        group_index: record.group_index,
        row_index: record.row_index,
        record_offset: record.offset,
        record_end: record.end,
        raw_x,
        opcode_family,
        opcode,
        pin_count,
        name,
        instance,
        control_input,
        control_output,
        pins,
        field_strings,
    })
}

fn parse_function_pin_descriptor(
    bytes: &[u8],
    record_offset: usize,
    cursor: usize,
    raw_x: u8,
    row_index: u16,
    is_control: bool,
) -> Option<(Option<IecFunctionPin>, usize)> {
    let (type_field, cursor) = parse_marker_string(bytes, record_offset, cursor)?;
    let raw_type_flags = u32::from_le_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?);
    let (name, cursor) = parse_marker_string(bytes, record_offset, cursor + 4)?;
    let role = bytes.get(cursor..cursor + 5)?;
    let cursor = cursor + 5;
    if name.value.is_empty() {
        return (type_field.value.is_empty() && raw_type_flags == 0 && role == [0; 5])
            .then_some((None, cursor));
    }
    let direction = match role {
        [1, 0, 0, 0, 0] => IecFunctionPinDirection::Input,
        [3, 0, 0, 0, 0] => IecFunctionPinDirection::Output,
        _ => return None,
    };
    if raw_type_flags & !IEC_PIN_KNOWN_FLAGS != 0
        || (raw_type_flags & IEC_PIN_INPUT_FLAG != 0)
            != (direction == IecFunctionPinDirection::Input)
    {
        return None;
    }
    let data_type_mask = raw_type_flags & IEC_PIN_TYPE_MASK;
    let is_array = raw_type_flags & IEC_PIN_ARRAY_FLAG != 0;
    if data_type_mask == 0 || is_array != !type_field.value.is_empty() {
        return None;
    }
    Some((
        Some(IecFunctionPin {
            name,
            type_expression: (!type_field.value.is_empty()).then_some(type_field),
            raw_type_flags,
            data_type_mask,
            data_type: function_pin_type_name(data_type_mask),
            is_array,
            direction,
            is_control,
            reference_ordinal: None,
            raw_x,
            row_index,
        }),
        cursor,
    ))
}

fn parse_marker_string(
    bytes: &[u8],
    record_offset: usize,
    cursor: usize,
) -> Option<(LadderString, usize)> {
    if bytes.get(cursor..cursor + 3)? != [0xff, 0xfe, 0xff] {
        return None;
    }
    let char_len = usize::from(*bytes.get(cursor + 3)?);
    let text_start = cursor + 4;
    let end = text_start.checked_add(char_len.checked_mul(2)?)?;
    let value = String::from_utf16(
        &bytes
            .get(text_start..end)?
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    )
    .ok()?;
    Some((
        LadderString {
            offset: record_offset + cursor,
            end_offset: record_offset + end,
            value,
        },
        end,
    ))
}

fn function_pin_type_name(mask: u32) -> Option<&'static str> {
    match mask {
        0x000f_ffff => Some("ANY"),
        0x0000_7fe0 => Some("ANY_NUM"),
        mask if mask.is_power_of_two() => {
            crate::iec_symbols::iec_primitive_type_name(mask.trailing_zeros() + 1)
        }
        _ => None,
    }
}

fn block_pin_for_ordinal(block: &IecFunctionBlock, ordinal: u8) -> Option<&IecFunctionPin> {
    block
        .pins
        .iter()
        .chain([&block.control_input, &block.control_output])
        .find(|pin| pin.reference_ordinal == Some(ordinal))
}

#[derive(Clone)]
enum Choice {
    None,
    One(Vec<(usize, usize, IecRecordKind)>),
    Ambiguous,
}

fn combine(accumulated: &mut Choice, first: (usize, usize, IecRecordKind), rest: Choice) {
    match rest {
        Choice::None => {}
        Choice::Ambiguous => *accumulated = Choice::Ambiguous,
        Choice::One(mut suffix) => {
            if matches!(accumulated, Choice::One(_) | Choice::Ambiguous) {
                *accumulated = Choice::Ambiguous;
            } else {
                suffix.insert(0, first);
                *accumulated = Choice::One(suffix);
            }
        }
    }
}

fn solve(
    data: &[u8],
    offset: usize,
    end: usize,
    row_index: u16,
    remaining: u16,
    memo: &mut HashMap<(usize, u16), Choice>,
) -> Choice {
    if remaining == 0 {
        return if offset == end {
            Choice::One(Vec::new())
        } else {
            Choice::None
        };
    }
    if offset >= end {
        return Choice::None;
    }
    if let Some(found) = memo.get(&(offset, remaining)) {
        return found.clone();
    }
    let mut result = Choice::None;
    if let Some((kind, next)) = simple_record(data, offset, end, row_index) {
        let rest = solve(data, next, end, row_index, remaining - 1, memo);
        combine(&mut result, (offset, next, kind), rest);
    }
    if function_start(data, offset, end, row_index) {
        for next in offset + 40..=end {
            if next != end
                && simple_record(data, next, end, row_index).is_none()
                && !function_start(data, next, end, row_index)
            {
                continue;
            }
            let rest = solve(data, next, end, row_index, remaining - 1, memo);
            combine(
                &mut result,
                (offset, next, IecRecordKind::FunctionBlock),
                rest,
            );
            if matches!(result, Choice::Ambiguous) {
                break;
            }
        }
    }
    memo.insert((offset, remaining), result.clone());
    result
}

fn function_start(data: &[u8], offset: usize, end: usize, row_index: u16) -> bool {
    let Some(bytes) = data.get(offset..offset + 24) else {
        return false;
    };
    offset + 40 <= end
        && bytes[..5] == [0, 0x67, 0, 0, 0]
        && (1..=94).contains(&bytes[5])
        && bytes[6..8] == (row_index * 4).to_le_bytes()
        && bytes[8] == 0
        && (matches!(
            &bytes[9..15],
            [1, 0, 0, 0, 0, 0] | [1, 0, 4, 0, 0, 0] | [1, 0, 0, 2, 0, 0] | [1, 0, 4, 2, 0, 0]
        ) || bytes[9..15] == [1, 0, bytes[5], 0, 0, 0])
        && matches!(bytes[15], 0x20 | 0x21 | 0x28)
        && bytes[18..20] == [0, 0]
        && (1..=32).contains(&bytes[20])
        && bytes[21..24] == [0, 0, 0]
}

fn simple_record(
    data: &[u8],
    offset: usize,
    end: usize,
    row_index: u16,
) -> Option<(IecRecordKind, usize)> {
    let marker = data.get(offset..offset + 2)?;
    let y = row_index * 4;
    if marker == [0xff, 0x02] {
        let bytes = data.get(offset..offset + 19)?;
        if offset + 19 > end
            || bytes[..5] != [0xff, 0x02, 0, 0, 0]
            || !(1..=94).contains(&bytes[5])
            || !(bytes[5]..=94).contains(&bytes[15])
            || bytes[6..8] != y.to_le_bytes()
            || bytes[16..18] != y.to_le_bytes()
            || bytes[8] != 0
            || bytes[18] != 0
            || !matches!(&bytes[9..15], [0, 0, 0, 0, 0, 0] | [0, 0, 4, 0, 0, 0])
        {
            return None;
        }
        return Some((IecRecordKind::LongWire, offset + 19));
    }
    if marker == [0, 0] {
        let bytes = data.get(offset..offset + 27)?;
        let target_y = u16::from_le_bytes([bytes[18], bytes[19]]);
        if offset + 27 > end
            || bytes[..7] != [0, 0, 0, 0, 0, 2, 0]
            || !(1..=93).contains(&bytes[7])
            || bytes[7] % 3 != 0
            || bytes[8..10] != y.to_le_bytes()
            || !matches!(
                &bytes[10..17],
                [0, 0, 0, 0, 0, 0, 0] | [0, 0, 0, 4, 0, 0, 0]
            )
            || bytes[17] != bytes[7] - 1
            || target_y <= y
            || target_y % 4 != 0
            || !matches!(
                &bytes[20..27],
                [0, 0, 0, 0, 0, 0, 0] | [0, 0, 0, 4, 0, 0, 0]
            )
        {
            return None;
        }
        return Some((IecRecordKind::BranchStart, offset + 27));
    }
    if marker == [1, 0] {
        let bytes = data.get(offset..offset + 9)?;
        let source_y = u16::from_le_bytes([bytes[6], bytes[7]]);
        if offset + 9 > end
            || bytes[..5] != [1, 0, 0, 0, 0]
            || !(1..=93).contains(&bytes[5])
            || bytes[5] % 3 != 0
            || source_y >= y
            || source_y % 4 != 0
            || bytes[8] != 0
        {
            return None;
        }
        return Some((IecRecordKind::BranchEnd, offset + 9));
    }
    if marker[0] > 0 && marker[0] < 32 && matches!(marker[1], 0x23 | 0x24 | 0x68 | 0x69) {
        let bytes = data.get(offset..offset + 9)?;
        if offset + 9 > end
            || bytes[2..5] != [0; 3]
            || !(1..=94).contains(&bytes[5])
            || u16::from_le_bytes([bytes[6], bytes[7]]) % 4 != 0
            || bytes[8] != 0
        {
            return None;
        }
        return Some((IecRecordKind::LinkReference(marker[1]), offset + 9));
    }
    if marker[0] != 0xff {
        return None;
    }
    let bytes = data.get(offset..offset + 15)?;
    if offset + 15 > end
        || bytes[2..5] != [0; 3]
        || !(1..=94).contains(&bytes[5])
        || bytes[6..8] != y.to_le_bytes()
        || bytes[8] != 0
    {
        return None;
    }
    if marker[1] == 0x01 && matches!(&bytes[9..15], [0, 0, 0, 0, 0, 0] | [0, 0, 4, 0, 0, 0]) {
        return Some((IecRecordKind::ShortWire, offset + 15));
    }
    let kind = match marker[1] {
        0x06..=0x0b if matches!(&bytes[9..15], [1, 0, 0, 0, 0, 0] | [1, 0, 4, 0, 0, 0]) => {
            IecRecordKind::Contact(marker[1])
        }
        0x0e..=0x13 if matches!(&bytes[9..15], [1, 0, 0x20, 0, 0, 0] | [1, 0, 0x24, 0, 0, 0]) => {
            IecRecordKind::Coil(marker[1])
        }
        0x3f | 0x40 if matches!(&bytes[9..15], [0, 0, 0x20, 0, 0, 0] | [0, 0, 0x24, 0, 0, 0]) => {
            IecRecordKind::Comment
        }
        0x46 if matches!(&bytes[9..15], [0, 0, 0, 0, 0, 0] | [0, 0, 4, 0, 0, 0]) => {
            IecRecordKind::FunctionOperand
        }
        _ => return None,
    };
    let text_end = utf16_end(data, offset + 15)?;
    let next = text_end
        + if matches!(kind, IecRecordKind::Comment) {
            8
        } else {
            0
        };
    (next <= end).then_some((kind, next))
}

fn utf16_end(data: &[u8], offset: usize) -> Option<usize> {
    let header = data.get(offset..offset + 4)?;
    if header[..3] != [0xff, 0xfe, 0xff] {
        return None;
    }
    let end = offset.checked_add(4 + usize::from(header[3]) * 2)?;
    let units = data.get(offset + 4..end)?;
    char::decode_utf16(
        units
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
    )
    .collect::<Result<String, _>>()
    .ok()?;
    Some(end)
}
