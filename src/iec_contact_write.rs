//! Remove addressed contacts while retaining their stored rows and circuit records.
use crate::{IecNoContactDeletionSite, IecRecordKind as K, LadderProgramData, XgwxError};

pub(crate) fn deletion_sites(program: &LadderProgramData) -> Option<Vec<IecNoContactDeletionSite>> {
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
                let K::Contact(code @ 6..=11) = record.kind else {
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
                    || !matches!(&bytes[9..15], [1, 0, 0, 0, 0, 0] | [1, 0, 4, 0, 0, 0])
                    || bytes[6..9] != [y[0], y[1], 0]
                {
                    return None;
                }
                Some(IecNoContactDeletionSite {
                    group_index: record.group_index,
                    row_index: record.row_index,
                    contact_offset: record.offset,
                    raw_x: x,
                    contact_code: code,
                })
            })
            .collect(),
    )
}

pub(crate) fn remove(program: &LadderProgramData, offset: usize) -> Result<Vec<u8>, XgwxError> {
    let unsupported = || XgwxError::UnsupportedLadderLayout;
    let site = deletion_sites(program)
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|s| s.contact_offset == offset)
        .ok_or_else(unsupported)?;
    let rows = program.iec_row_frames().ok_or_else(unsupported)?;
    let row = rows
        .iter()
        .find(|r| r.group_index == site.group_index && r.row_index == site.row_index)
        .ok_or_else(unsupported)?;
    let record = program
        .iec_record_frames()
        .ok_or_else(unsupported)?
        .into_iter()
        .find(|r| r.offset == offset)
        .ok_or_else(unsupported)?;
    let mut out = program.data.clone();
    out[row.start + 33..row.start + 35].copy_from_slice(&(row.record_count - 1).to_le_bytes());
    out.drain(record.offset..record.end);
    let mut verified = program.clone();
    verified.decoded_len = out.len();
    verified.data = out;
    verified.iec_circuit_layout().ok_or_else(unsupported)?;
    Ok(verified.data)
}
