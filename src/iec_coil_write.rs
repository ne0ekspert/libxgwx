//! Remove addressed coils while retaining their stored rows and circuit records.
use crate::{IecRecordKind as K, LadderProgramData, XgwxError};

pub(crate) fn deletion_sites(program: &LadderProgramData) -> Option<Vec<usize>> {
    if program.project_type != Some(2) || program.version.as_deref() != Some("LD VER 1.1") {
        return None;
    }
    let rows = program.iec_row_frames()?;
    let records = program.iec_record_frames()?;
    program.iec_circuit_layout()?;
    Some(
        records
            .iter()
            .filter_map(|record| {
                let K::Coil(14..=19) = record.kind else {
                    return None;
                };
                let row = rows.iter().find(|row| {
                    row.group_index == record.group_index && row.row_index == record.row_index
                })?;
                // Keep a stored frame with surviving records. Sole-element row removal
                // requires its own native row/group semantics.
                if row.record_count < 2 {
                    return None;
                }
                let first = rows
                    .iter()
                    .find(|row| row.group_index == record.group_index)?;
                let mode = &program.data[first.start - 6..first.start - 2];
                if !matches!(mode, [0, 0, 0, 0] | [1, 0, 0, 0]) {
                    return None;
                }
                let bytes = &program.data[record.offset..record.end];
                let x = bytes[5];
                let y = record.row_index.checked_mul(4)?.to_le_bytes();
                if !(1..=94).contains(&x)
                    || !(x - 1).is_multiple_of(3)
                    || !matches!(&bytes[9..15], [1, 0, 0x20, 0, 0, 0] | [1, 0, 0x24, 0, 0, 0])
                    || bytes[6..9] != [y[0], y[1], 0]
                {
                    return None;
                }
                Some(record.offset)
            })
            .collect(),
    )
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let _site = deletion_sites(program)
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|s| *s == offset)
        .ok_or_else(unsupported)?;
    let record = program
        .iec_record_frames()
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|r| r.offset == offset)
        .ok_or_else(unsupported)?;
    let row = program
        .iec_row_frames()
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|r| r.group_index == record.group_index && r.row_index == record.row_index)
        .ok_or_else(unsupported)?;
    let records = program.iec_record_frames().ok_or_else(unsupported)?;
    let feed = records.iter().find(|r| {
        r.group_index == record.group_index
            && r.row_index == record.row_index
            && r.end == record.offset
            && r.kind == K::LongWire
            && program.data[record.offset + 5] == 94
            && program.data[r.offset + 15] == 91
    });
    let removed_count = if feed.is_some() { 2 } else { 1 };
    if row.record_count <= removed_count {
        return Err(unsupported());
    }
    let mut out = program.data.clone();
    out[row.start + 33..row.start + 35]
        .copy_from_slice(&(row.record_count - removed_count).to_le_bytes());
    out.drain(feed.map_or(record.offset, |r| r.offset)..record.end);
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    verified.iec_circuit_layout().ok_or_else(unsupported)?;
    Ok(verified.data)
}
