//! Exact native LD record boundaries shared by rendering and structural edits.
use crate::XgwxError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
pub enum LadderEditKind {
    NormallyOpen,
    NormallyClosed,
    AddressedRisingPulse,
    AddressedFallingPulse,
    AddressedRisingPulseNot,
    AddressedFallingPulseNot,
    Inverse,
    RisingPulse,
    FallingPulse,
    Output,
    InverseOutput,
    Set,
    Reset,
    RisingPulseOutput,
    FallingPulseOutput,
}

impl LadderEditKind {
    #[cfg(feature = "write")]
    pub(crate) fn marker(self) -> u8 {
        match self {
            Self::NormallyOpen => 6,
            Self::NormallyClosed => 7,
            Self::AddressedRisingPulse => 8,
            Self::AddressedFallingPulse => 9,
            Self::AddressedRisingPulseNot => 10,
            Self::AddressedFallingPulseNot => 11,
            Self::Inverse => 0x3e,
            Self::RisingPulse => 0x48,
            Self::FallingPulse => 0x49,
            Self::Output => 14,
            Self::InverseOutput => 15,
            Self::Set => 16,
            Self::Reset => 17,
            Self::RisingPulseOutput => 18,
            Self::FallingPulseOutput => 19,
        }
    }
    fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            6 => Some(Self::NormallyOpen),
            7 => Some(Self::NormallyClosed),
            8 => Some(Self::AddressedRisingPulse),
            9 => Some(Self::AddressedFallingPulse),
            10 => Some(Self::AddressedRisingPulseNot),
            11 => Some(Self::AddressedFallingPulseNot),
            14 => Some(Self::Output),
            15 => Some(Self::InverseOutput),
            16 => Some(Self::Set),
            17 => Some(Self::Reset),
            18 => Some(Self::RisingPulseOutput),
            19 => Some(Self::FallingPulseOutput),
            0x3e => Some(Self::Inverse),
            0x48 => Some(Self::RisingPulse),
            0x49 => Some(Self::FallingPulse),
            _ => None,
        }
    }
    #[cfg(feature = "write")]
    pub(crate) fn is_coil(self) -> bool {
        matches!(
            self,
            Self::Output
                | Self::InverseOutput
                | Self::Set
                | Self::Reset
                | Self::RisingPulseOutput
                | Self::FallingPulseOutput
        )
    }
    #[cfg(feature = "write")]
    pub(crate) fn has_operand(self) -> bool {
        !matches!(self, Self::Inverse | Self::RisingPulse | Self::FallingPulse)
    }
}

/// One supported contact, coil or operandless logic operation. Operands are
/// device addresses, not arbitrary instruction strings or variable symbols;
/// operandless operations use an empty string.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct LadderEditElement {
    pub kind: LadderEditKind,
    pub operand: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub(crate) bytes: Vec<u8>,
    pub(crate) x: u8,
    pub(crate) wire_end: Option<u8>,
    pub(crate) element: Option<LadderEditElement>,
}
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) prefix: Vec<u8>,
    pub(crate) y: u32,
    pub(crate) records: Vec<Record>,
}
#[derive(Debug)]
pub(crate) struct EditableProgram {
    pub(crate) header: Vec<u8>,
    pub(crate) rows: Vec<Row>,
    #[cfg(feature = "write")]
    pub(crate) group_headers: Vec<(usize, Vec<u8>)>,
}

pub(crate) fn unsupported() -> XgwxError {
    XgwxError::UnsupportedLadderLayout
}
pub(crate) fn u16_at(bytes: &[u8], start: usize) -> Result<usize, XgwxError> {
    let b = bytes.get(start..start + 2).ok_or_else(unsupported)?;
    Ok(usize::from(u16::from_le_bytes([b[0], b[1]])))
}
/// Supported physical rows in the XGK ladder editor (manual section 5.1).
pub(crate) const XGK_MAX_ROWS: usize = 65535;

pub(crate) fn y_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], 0])
}
#[cfg(feature = "write")]
pub(crate) fn put_y(bytes: &mut [u8], offset: usize, y: u32) {
    bytes[offset..offset + 3].copy_from_slice(&y.to_le_bytes()[..3]);
}
#[cfg(feature = "write")]
pub(crate) fn shift_y(bytes: &mut [u8], offset: usize, delta: i32) {
    let y = y_at(bytes, offset).checked_add_signed(delta).unwrap();
    put_y(bytes, offset, y);
}
#[cfg(feature = "write")]
pub(crate) fn xy(x: u8, y: u32) -> [u8; 4] {
    let b = y.to_le_bytes();
    [x, b[0], b[1], b[2]]
}
pub(crate) fn row_count(header: &[u8]) -> Result<usize, XgwxError> {
    let short = u16_at(header, 4)?;
    if short != 65535 {
        return Ok(short);
    }
    let bytes = header.get(6..10).ok_or_else(unsupported)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()) as usize)
}
pub(crate) fn header_size(header: &[u8]) -> Result<usize, XgwxError> {
    Ok(if u16_at(header, 4)? == 65535 { 12 } else { 8 })
}
#[cfg(feature = "write")]
pub(crate) fn set_row_count(header: &mut Vec<u8>, count: usize) {
    let old_size = header_size(header).unwrap();
    let groups = u16_at(header, old_size - 2).unwrap() as u16;
    header.truncate(4);
    if count >= 65535 {
        header.extend_from_slice(&65535u16.to_le_bytes());
        header.extend_from_slice(&(count as u32).to_le_bytes());
    } else {
        header.extend_from_slice(&(count as u16).to_le_bytes());
    }
    header.extend_from_slice(&groups.to_le_bytes());
}

pub(crate) fn string_at(bytes: &[u8], start: usize) -> Result<(String, usize), XgwxError> {
    if bytes.get(start..start + 3) != Some(&[255, 254, 255]) {
        return Err(unsupported());
    }
    let count = usize::from(*bytes.get(start + 3).ok_or_else(unsupported)?);
    let end = start + 4 + count * 2;
    let encoded = bytes.get(start + 4..end).ok_or_else(unsupported)?;
    let units = encoded
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    Ok((String::from_utf16(&units).map_err(|_| unsupported())?, end))
}
pub(crate) fn coordinate(bytes: &[u8], offset: usize, y: u32) -> Result<u8, XgwxError> {
    let b = bytes.get(offset..offset + 4).ok_or_else(unsupported)?;
    if y_at(b, 1) != y {
        return Err(unsupported());
    }
    Ok(b[0])
}

impl EditableProgram {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, XgwxError> {
        if bytes.len() < 8 || bytes[..4] != [0; 4] {
            return Err(unsupported());
        }
        let header_len = header_size(bytes)?;
        let row_count = row_count(bytes)?;
        let group_count = u16_at(bytes, header_len - 2)?;
        if row_count > XGK_MAX_ROWS || group_count > row_count {
            return Err(unsupported());
        }
        let mut rows = Vec::new();
        let mut group_headers = Vec::new();
        let mut pos = header_len;
        for group in 0..group_count {
            let head = bytes.get(pos..pos + 10).ok_or_else(unsupported)?.to_vec();
            let count = u16_at(&head, 8)?;
            if head[..4] != (group as u32).to_le_bytes()
                || head[4..8] != [0; 4]
                || count == 0
                || rows.len() + count > row_count
            {
                return Err(unsupported());
            }
            group_headers.push((rows.len(), head));
            pos += 10;
            for _ in 0..count {
                let prefix = bytes.get(pos..pos + 35).ok_or_else(unsupported)?.to_vec();
                let index = u32::from_le_bytes(prefix[..4].try_into().unwrap()) as usize;
                let y = (index * 4) as u32;
                if index >= row_count
                    || rows.last().is_some_and(|r: &Row| r.y >= y)
                    || prefix[4..9] != [255, 67, 0, 0, 0]
                    || prefix[9..13] != [0; 4]
                    || prefix[14..17] != [0; 3]
                    || prefix[13] > 1
                    || coordinate(&prefix, 21, y)? > 94
                    || y_at(&prefix, 26) >= XGK_MAX_ROWS as u32 * 4
                    || !y_at(&prefix, 26).is_multiple_of(4)
                {
                    return Err(unsupported());
                }
                coordinate(&prefix, 29, y)?;
                let count = u16_at(&prefix, 33)?;
                if count > 128 {
                    return Err(unsupported());
                }
                pos += 35;
                let mut records = Vec::new();
                for _ in 0..count {
                    let (record, end) = parse_record(bytes, pos, y)?;
                    records.push(record);
                    pos = end;
                }
                validate_row_records(&records)?;
                rows.push(Row { prefix, y, records });
            }
        }
        if pos != bytes.len() {
            return Err(unsupported());
        }
        // Validate reciprocal branch references inside their declared group.
        for (g, (first, head)) in group_headers.iter().enumerate() {
            let last = group_headers.get(g + 1).map_or(rows.len(), |(i, _)| *i);
            if last - first != u16_at(head, 8)? {
                return Err(unsupported());
            }
            for row in &rows[*first..last] {
                for r in &row.records {
                    if r.bytes.starts_with(&[0, 0]) {
                        let target_y = y_at(&r.bytes, 18);
                        if target_y <= row.y
                            || !rows[*first..last].iter().any(|target| {
                                target.y == target_y
                                    && target.records.iter().any(|end| {
                                        end.bytes.starts_with(&[1, 0, 0, 0, 0])
                                            && end.bytes[5..9] == r.bytes[7..11]
                                    })
                            })
                        {
                            return Err(unsupported());
                        }
                    } else if r.bytes.starts_with(&[1, 0, 0, 0, 0]) {
                        let source_y = y_at(&r.bytes, 6);
                        if source_y >= row.y
                            || !rows[*first..last].iter().any(|source| {
                                source.y == source_y
                                    && source.records.iter().any(|start| {
                                        start.bytes.starts_with(&[0, 0])
                                            && start.bytes[7..11] == r.bytes[5..9]
                                            && y_at(&start.bytes, 18) == row.y
                                    })
                            })
                        {
                            return Err(unsupported());
                        }
                    }
                }
            }
        }
        Ok(Self {
            header: bytes[..header_len].to_vec(),
            rows,
            #[cfg(feature = "write")]
            group_headers,
        })
    }

    #[cfg(feature = "write")]
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut bytes = self.header.clone();
        let count = row_count(&self.header)
            .unwrap()
            .max(self.rows.last().map_or(0, |r| (r.y as usize) / 4 + 1));
        set_row_count(&mut bytes, count);
        let group_offset = bytes.len() - 2;
        bytes[group_offset..].copy_from_slice(&(self.group_headers.len() as u16).to_le_bytes());
        for (index, row) in self.rows.iter().enumerate() {
            if let Some((_, header)) = self.group_headers.iter().find(|(first, _)| *first == index)
            {
                bytes.extend(header);
            }
            let mut prefix = row.prefix.clone();
            prefix[33..35].copy_from_slice(&(row.records.len() as u16).to_le_bytes());
            bytes.extend(prefix);
            for record in &row.records {
                bytes.extend(&record.bytes);
            }
        }
        bytes
    }
}

fn validate_row_records(records: &[Record]) -> Result<(), XgwxError> {
    let mut occupied = 0;
    for (index, r) in records.iter().enumerate() {
        if r.bytes[0] == 255 && !matches!(r.bytes[1], 0x3f | 0x40) {
            if r.x <= occupied {
                return Err(unsupported());
            }
            occupied = r.wire_end.unwrap_or(r.x);
        } else if r.bytes.starts_with(&[0, 34]) {
            if r.x <= occupied {
                return Err(unsupported());
            }
            let (_, end) = string_at(&r.bytes, 19)?;
            let count = u16_at(&r.bytes, end)?;
            occupied = r.x + ((count - 1) * 3) as u8;
            for argument in 1..count {
                let reference = records.get(index + argument).ok_or_else(unsupported)?;
                let expected = vec![
                    argument as u8,
                    if argument == count - 1 { 0x24 } else { 0x23 },
                    0,
                    0,
                    0,
                    r.x,
                    r.bytes[6],
                    r.bytes[7],
                    r.bytes[8],
                ];
                if reference.bytes != expected {
                    return Err(unsupported());
                }
            }
        } else if matches!(r.bytes[1], 0x23 | 0x24) {
            let instruction = records
                .get(
                    index
                        .checked_sub(usize::from(r.bytes[0]))
                        .ok_or_else(unsupported)?,
                )
                .ok_or_else(unsupported)?;
            if !instruction.bytes.starts_with(&[0, 34]) || instruction.x != r.x {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

fn parse_record(bytes: &[u8], pos: usize, y: u32) -> Result<(Record, usize), XgwxError> {
    let marker = bytes.get(pos..pos + 2).ok_or_else(unsupported)?;
    let mut element = None;
    let mut wire_end = None;
    let x;
    let end;
    if marker == [0, 0] {
        let b = bytes.get(pos..pos + 27).ok_or_else(unsupported)?;
        x = b[7];
        if b[..7] != [0, 0, 0, 0, 0, 2, 0]
            || coordinate(b, 7, y)? != x
            || x == 0
            || x > 93
            || !x.is_multiple_of(3)
            || b[11..17] != [0; 6]
            || b[17] != x - 1
            || b[21..27] != [0; 6]
            || !y_at(b, 18).is_multiple_of(4)
        {
            return Err(unsupported());
        }
        end = pos + 27;
    } else if marker == [1, 0] {
        let b = bytes.get(pos..pos + 9).ok_or_else(unsupported)?;
        x = b[5];
        if b[..5] != [1, 0, 0, 0, 0]
            || x == 0
            || x > 93
            || !x.is_multiple_of(3)
            || !y_at(b, 6).is_multiple_of(4)
        {
            return Err(unsupported());
        }
        end = pos + 9;
    } else if matches!(marker[1], 0x23 | 0x24) && marker[0] > 0 && marker[0] < 32 {
        let b = bytes.get(pos..pos + 9).ok_or_else(unsupported)?;
        if b[2..5] != [0; 3] {
            return Err(unsupported());
        }
        x = coordinate(b, 5, y)?;
        end = pos + 9;
    } else {
        let head = bytes.get(pos..pos + 15).ok_or_else(unsupported)?;
        x = coordinate(head, 5, y)?;
        if head[2..5] != [0; 3] || x == 0 || x > 97 || x % 3 != 1 {
            return Err(unsupported());
        }
        if marker == [255, 2] {
            if head[9..15] != [0; 6] {
                return Err(unsupported());
            }
            let last = coordinate(bytes, pos + 15, y)?;
            if last < x || last > 91 || last % 3 != 1 {
                return Err(unsupported());
            }
            wire_end = Some(last);
            end = pos + 19;
        } else if marker[0] == 255 && matches!(marker[1], 6..=11 | 14..=19) {
            let coil = marker[1] >= 14;
            if head[9..11] != [1, 0]
                || head[11..15] != if coil { [32, 0, 0, 0] } else { [0; 4] }
                || (coil && x != 94)
                || (!coil && x > 25)
            {
                return Err(unsupported());
            }
            let (operand, next) = string_at(bytes, pos + 15)?;
            end = next;
            if let Some(kind) = LadderEditKind::from_marker(marker[1]) {
                element = Some(LadderEditElement { kind, operand });
            }
        } else if marker[0] == 255 && matches!(marker[1], 1 | 0x3e | 0x48 | 0x49) {
            if head[9..15]
                != if marker[1] == 1 {
                    [0; 6]
                } else {
                    [1, 0, 0, 0, 0, 0]
                }
            {
                return Err(unsupported());
            }
            end = pos + 15;
            if let Some(kind) = LadderEditKind::from_marker(marker[1]) {
                element = Some(LadderEditElement {
                    kind,
                    operand: String::new(),
                });
            }
        } else if marker[0] == 255 && matches!(marker[1], 0x3f | 0x40) {
            let (_, next) = string_at(bytes, pos + 15)?;
            bytes.get(next..next + 8).ok_or_else(unsupported)?;
            end = next + 8;
        } else if marker == [0, 34] {
            if !matches!(head[9..15], [1, 0, 0, 0, 0, 0] | [1, 0, 32, 0, 0, 0]) {
                return Err(unsupported());
            }
            let (_, next) = string_at(bytes, pos + 19)?;
            let count = u16_at(bytes, next)?;
            if count == 0 || count > 32 {
                return Err(unsupported());
            }
            let mut next = next + 2;
            for i in 0..count {
                let coordinate_x = coordinate(bytes, next, y)?;
                if coordinate_x != x.saturating_add((i * 3) as u8) || coordinate_x > 94 {
                    return Err(unsupported());
                }
                let (_, n) = string_at(bytes, next + 10)?;
                next = n;
            }
            end = next;
        } else {
            return Err(unsupported());
        }
    }
    Ok((
        Record {
            bytes: bytes[pos..end].to_vec(),
            x,
            wire_end,
            element,
        },
        end,
    ))
}

/// Exact stored wires. Contacts and coils are elements, not unconditional wires.
/// Unknown record layouts fall back to the legacy read-only decoder.
pub(crate) fn exact_geometry(
    bytes: &[u8],
) -> Option<(
    Vec<crate::LadderHorizontalLine>,
    Vec<crate::LadderVerticalLine>,
)> {
    let program = EditableProgram::parse(bytes).ok()?;
    let mut horizontal = Vec::with_capacity(program.rows.len());
    let mut vertical = Vec::new();
    for row in &program.rows {
        if row.prefix[13] != 0 {
            continue;
        }
        for r in &row.records {
            if r.bytes.starts_with(&[0, 0]) {
                vertical.push(crate::LadderVerticalLine {
                    raw_x: r.x,
                    raw_y_start: row.y,
                    raw_y_end: y_at(&r.bytes, 18),
                });
            } else if let Some(end) = r.wire_end {
                horizontal.push(crate::LadderHorizontalLine {
                    raw_y: row.y,
                    raw_x_start: r.x,
                    raw_x_end: end + 2,
                });
            } else if r.element.is_none() && r.bytes.starts_with(&[255, 1]) {
                horizontal.push(crate::LadderHorizontalLine {
                    raw_y: row.y,
                    raw_x_start: r.x,
                    raw_x_end: r.x + 2,
                });
            }
        }
    }
    Some((horizontal, vertical))
}
