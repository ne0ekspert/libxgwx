use super::*;

pub(crate) fn extract_ladder_strings(data: &[u8]) -> Vec<LadderString> {
    extract_utf16_marker_strings(data, true, false)
}

pub(crate) fn extract_utf16_marker_strings(
    data: &[u8],
    ascii_graphic_only: bool,
    include_empty: bool,
) -> Vec<LadderString> {
    let mut strings = Vec::new();
    let mut offset = 0;

    while offset + UTF16_MARKER.len() < data.len() {
        if data[offset..].starts_with(UTF16_MARKER) {
            let len_offset = offset + UTF16_MARKER.len();
            let char_len = data[len_offset] as usize;
            let text_start = len_offset + 1;
            let text_end = text_start + char_len.saturating_mul(2);

            if let Some(encoded) = data.get(text_start..text_end) {
                if let Some(value) = decode_utf16_bytes(encoded)
                    && (include_empty || !value.is_empty())
                    && (!ascii_graphic_only
                        || value.chars().all(|ch| ch.is_ascii_graphic() || ch == ' '))
                {
                    strings.push(LadderString {
                        offset,
                        end_offset: text_end,
                        value,
                    });
                }

                offset = text_end;
                continue;
            }
        }

        offset += 1;
    }

    strings
}

pub(crate) fn extract_ladder_instructions(strings: &[LadderString]) -> Vec<LadderInstruction> {
    strings
        .iter()
        .filter_map(parse_ladder_instruction)
        .collect()
}

pub(crate) fn extract_ladder_elements(data: &[u8], strings: &[LadderString]) -> Vec<LadderElement> {
    let mut elements = Vec::new();
    let mut index = 0;

    while index < strings.len() {
        let Some(element) = parse_ladder_element(data, &strings[index]) else {
            index += 1;
            continue;
        };

        let skip = duplicate_decomposed_string_count(strings, index, &element);
        elements.push(element);
        index += skip + 1;
    }

    elements
}

pub(crate) fn extract_ladder_structure(data: &[u8], elements: &[LadderElement]) -> LadderStructure {
    let vertical_lines = extract_ladder_vertical_lines(data);
    let branch_groups = extract_ladder_branch_groups(&vertical_lines);
    let mut cells = elements
        .iter()
        .filter_map(|element| {
            let (raw_x, raw_y) = ladder_element_coordinate(data, element)?;
            Some(LadderCell {
                offset: element.offset,
                raw_x,
                raw_y,
                kind: element.kind,
                value: element.value.clone(),
                operands: element.operands.clone(),
                contact: element.contact,
                coil: element.coil,
            })
        })
        .collect::<Vec<_>>();

    push_marker_only_ladder_cells(data, &mut cells);
    cells.sort_by_key(|cell| (cell.raw_y, cell.raw_x, cell.offset));
    let horizontal_lines = extract_ladder_horizontal_lines(data, &cells);
    let rung_comments = extract_ladder_rung_comments(data);
    let output_comments = extract_ladder_output_comments(data);
    let unknown_records = extract_ladder_unknown_records(data, &cells);

    let mut rungs = Vec::new();
    for cell in cells {
        if rungs
            .last()
            .is_none_or(|rung: &LadderRung| rung.raw_y != cell.raw_y)
        {
            rungs.push(LadderRung {
                raw_y: cell.raw_y,
                cells: Vec::new(),
            });
        }

        if let Some(rung) = rungs.last_mut() {
            rung.cells.push(cell);
        }
    }

    LadderStructure {
        rungs,
        vertical_lines,
        branch_groups,
        horizontal_lines,
        rung_comments,
        output_comments,
        unknown_records,
    }
}

pub(crate) fn extract_ladder_vertical_lines(data: &[u8]) -> Vec<LadderVerticalLine> {
    let mut lines = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x43])
    }) {
        let marker = offset + relative;
        if let (Some(target), Some(source)) = (
            read_ladder_coordinate(data, marker + 17),
            read_ladder_coordinate(data, marker + 36),
        ) {
            push_ladder_vertical_line_from_ff43(&mut lines, target, source);
        }

        offset = marker + 2;
    }

    push_ladder_marker_vertical_lines(data, &mut lines);

    lines.sort_by_key(|line| (line.raw_y_start, line.raw_y_end, line.raw_x));
    lines.dedup();
    merge_ladder_vertical_lines(&mut lines);
    lines
}

pub(crate) fn push_ladder_marker_vertical_lines(data: &[u8], lines: &mut Vec<LadderVerticalLine>) {
    for span in ladder_marker_vertical_spans(data) {
        lines.push(LadderVerticalLine {
            raw_x: span.raw_x,
            raw_y_start: span.raw_y_start,
            raw_y_end: span.raw_y_end,
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LadderMarkerRecord {
    offset: usize,
    raw_x: u8,
    raw_y: u8,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LadderMarkerVerticalSpan {
    raw_x: u8,
    raw_y_start: u8,
    raw_y_end: u8,
}

pub(crate) fn ladder_marker_vertical_spans(data: &[u8]) -> Vec<LadderMarkerVerticalSpan> {
    let starts = ladder_marker_records(data, [0xff, 0x3e]);
    let ends = ladder_marker_records(data, [0xff, 0x01]);
    let mut spans = Vec::new();

    for start in starts {
        if has_ladder_inline_string_after_marker(data, start.offset) {
            continue;
        }

        if let Some(end) = ends
            .iter()
            .copied()
            .filter(|end| {
                end.raw_x == start.raw_x
                    && end.raw_y > start.raw_y
                    && (end.raw_y - start.raw_y) % 4 == 0
            })
            .min_by_key(|end| end.raw_y)
            && let Some(raw_x) = marker_vertical_line_x(data, start, end)
        {
            if let Some(raw_y_end) = marker_intermediate_branch_y(data, start, end)
                && let Some(raw_x) = start.raw_x.checked_sub(1).filter(|raw_x| *raw_x > 1)
            {
                spans.push(LadderMarkerVerticalSpan {
                    raw_x,
                    raw_y_start: start.raw_y,
                    raw_y_end,
                });
            }

            spans.push(LadderMarkerVerticalSpan {
                raw_x,
                raw_y_start: start.raw_y,
                raw_y_end: end.raw_y,
            });
        }
    }

    spans
}

pub(crate) fn ladder_marker_records(data: &[u8], marker: [u8; 2]) -> Vec<LadderMarkerRecord> {
    let mut records = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data
        .get(offset..)
        .and_then(|remaining| remaining.windows(2).position(|window| window == marker))
    {
        let marker_offset = offset + relative;
        if !is_ladder_utf16_length_marker(data, marker_offset)
            && let Some((raw_x, raw_y)) = ladder_record_coordinate(data, marker_offset)
            && raw_y > 0
        {
            records.push(LadderMarkerRecord {
                offset: marker_offset,
                raw_x,
                raw_y,
            });
        }

        offset = marker_offset + 2;
    }

    records
}

pub(crate) fn marker_vertical_line_x(
    data: &[u8],
    start: LadderMarkerRecord,
    end: LadderMarkerRecord,
) -> Option<u8> {
    marker_embedded_branch_coordinates(data, start.offset, start.raw_y, end.raw_y)
        .into_iter()
        .chain(marker_embedded_branch_coordinates(
            data,
            end.offset,
            start.raw_y,
            end.raw_y,
        ))
        .filter(|(raw_x, _)| *raw_x > start.raw_x)
        .map(|(raw_x, _)| raw_x)
        .max()
}

pub(crate) fn marker_intermediate_branch_y(
    data: &[u8],
    start: LadderMarkerRecord,
    end: LadderMarkerRecord,
) -> Option<u8> {
    marker_embedded_branch_coordinates(data, start.offset, start.raw_y, end.raw_y)
        .into_iter()
        .chain(marker_embedded_branch_coordinates(
            data,
            end.offset,
            start.raw_y,
            end.raw_y,
        ))
        .map(|(_, raw_y)| raw_y)
        .filter(|raw_y| *raw_y > start.raw_y && *raw_y < end.raw_y)
        .min()
}

pub(crate) fn marker_embedded_branch_coordinates(
    data: &[u8],
    offset: usize,
    raw_y_start: u8,
    raw_y_end: u8,
) -> Vec<(u8, u8)> {
    let end_relative = data
        .get(offset + 2..(offset + 48).min(data.len()))
        .and_then(|bytes| bytes.iter().position(|byte| *byte == 0xff))
        .map(|relative| relative + 2)
        .unwrap_or(48);

    (7..end_relative)
        .filter_map(|relative| read_ladder_coordinate(data, offset + relative))
        .filter(|(_, raw_y)| *raw_y >= raw_y_start && *raw_y <= raw_y_end)
        .collect()
}

pub(crate) fn push_ladder_vertical_line_from_ff43(
    lines: &mut Vec<LadderVerticalLine>,
    target: (u8, u8),
    source: (u8, u8),
) {
    let (target_x, target_y) = target;
    let (source_x, source_y) = source;
    if target_y == source_y {
        return;
    }

    let raw_y_start = target_y.min(source_y);
    let raw_y_end = target_y.max(source_y);
    if (raw_y_end - raw_y_start) % 4 != 0 {
        return;
    }

    let raw_x = if source_x > 1 {
        Some(source_x)
    } else if target_x > 1 {
        Some(target_x)
    } else {
        None
    };

    if let Some(raw_x) = raw_x.filter(|raw_x| *raw_x > 1) {
        lines.push(LadderVerticalLine {
            raw_x,
            raw_y_start,
            raw_y_end,
        });
    }
}

pub(crate) fn merge_ladder_vertical_lines(lines: &mut Vec<LadderVerticalLine>) {
    lines.sort_by_key(|line| (line.raw_x, line.raw_y_start, line.raw_y_end));
    let mut merged: Vec<LadderVerticalLine> = Vec::new();

    for line in lines.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.raw_x == line.raw_x
            && line.raw_y_start <= last.raw_y_end
        {
            last.raw_y_end = last.raw_y_end.max(line.raw_y_end);
            continue;
        }

        merged.push(line);
    }

    merged.sort_by_key(|line| (line.raw_y_start, line.raw_y_end, line.raw_x));
    *lines = merged;
}

pub(crate) fn extract_ladder_branch_groups(lines: &[LadderVerticalLine]) -> Vec<LadderBranchGroup> {
    lines
        .iter()
        .map(|line| LadderBranchGroup {
            raw_x: line.raw_x,
            raw_y_start: line.raw_y_start,
            raw_y_end: line.raw_y_end,
        })
        .collect()
}

pub(crate) fn extract_ladder_horizontal_lines(
    data: &[u8],
    cells: &[LadderCell],
) -> Vec<LadderHorizontalLine> {
    let mut lines = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x43])
    }) {
        let marker = offset + relative;
        match data.get(marker + 13).copied() {
            Some(0x24 | 0x32) => {
                if let (Some((target_x, raw_y)), Some((source_x, _))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) {
                    push_ladder_horizontal_line(&mut lines, raw_y, source_x, target_x);
                }
            }
            Some(0x27) => {
                if let (Some((target_x, raw_y)), Some((source_x, _))) = (
                    read_ladder_coordinate(data, marker + 17),
                    read_ladder_coordinate(data, marker + 36),
                ) {
                    let raw_x_start = if source_x == 0x03 {
                        target_x.saturating_sub(3)
                    } else {
                        source_x
                    };
                    push_ladder_horizontal_line(&mut lines, raw_y, raw_x_start, target_x);
                }
            }
            _ => {}
        }

        offset = marker + 2;
    }

    push_ladder_ff02_horizontal_lines(&mut lines, data, cells);
    push_ladder_marker_branch_horizontal_lines(&mut lines, data, cells);

    lines.sort_by_key(|line| (line.raw_y, line.raw_x_start, line.raw_x_end));
    lines.dedup();
    lines
}

pub(crate) fn push_ladder_marker_branch_horizontal_lines(
    lines: &mut Vec<LadderHorizontalLine>,
    data: &[u8],
    cells: &[LadderCell],
) {
    let spans = ladder_marker_vertical_spans(data);
    for span in &spans {
        let mut raw_y = span.raw_y_start;
        while raw_y <= span.raw_y_end {
            if !is_shadowed_marker_branch_tap(&spans, span, raw_y)
                && let Some(raw_x_start) = cells
                    .iter()
                    .filter(|cell| cell.raw_y == raw_y && cell.raw_x < span.raw_x)
                    .map(|cell| cell.raw_x)
                    .min()
            {
                push_ladder_horizontal_line(lines, raw_y, raw_x_start, span.raw_x);
            }

            let Some(next_y) = raw_y.checked_add(4) else {
                break;
            };
            raw_y = next_y;
        }
    }
}

pub(crate) fn is_shadowed_marker_branch_tap(
    spans: &[LadderMarkerVerticalSpan],
    span: &LadderMarkerVerticalSpan,
    raw_y: u8,
) -> bool {
    spans.iter().any(|other| {
        other.raw_y_start == raw_y && other.raw_y_end > span.raw_y_end && other.raw_x > span.raw_x
    }) || (raw_y != span.raw_y_start
        && raw_y != span.raw_y_end
        && spans.iter().any(|other| {
            other.raw_y_start == span.raw_y_start
                && other.raw_y_end == raw_y
                && other.raw_x < span.raw_x
        }))
}

pub(crate) fn push_ladder_ff02_horizontal_lines(
    lines: &mut Vec<LadderHorizontalLine>,
    data: &[u8],
    cells: &[LadderCell],
) {
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x02])
    }) {
        let marker = offset + relative;
        if let Some((raw_x, raw_y)) = ladder_record_coordinate(data, marker)
            && let Some(raw_x_end) = next_ladder_horizontal_stop_x(cells, raw_x, raw_y)
        {
            push_ladder_horizontal_line(lines, raw_y, raw_x, raw_x_end);
        }

        offset = marker + 2;
    }
}

pub(crate) fn next_ladder_horizontal_stop_x(
    cells: &[LadderCell],
    raw_x: u8,
    raw_y: u8,
) -> Option<u8> {
    cells
        .iter()
        .filter(|cell| {
            cell.raw_y == raw_y && cell.raw_x > raw_x && is_ladder_horizontal_stop_cell(cell)
        })
        .map(|cell| cell.raw_x)
        .min()
}

pub(crate) fn is_ladder_horizontal_stop_cell(cell: &LadderCell) -> bool {
    cell.coil.is_some()
        || matches!(
            cell.kind,
            LadderElementKind::InstructionCall
                | LadderElementKind::Operation
                | LadderElementKind::Comparison
                | LadderElementKind::Timer
                | LadderElementKind::Logic
        )
}

pub(crate) fn push_marker_only_ladder_cells(data: &[u8], cells: &mut Vec<LadderCell>) {
    let mut offset = 0;

    while offset + 2 <= data.len() {
        let marker = [data[offset], data[offset + 1]];
        if let Some(contact) = marker_only_ladder_contact(marker)
            && let Some((raw_x, raw_y)) = ladder_record_coordinate(data, offset)
            && !is_known_ladder_cell_record(cells, offset, marker, raw_x, raw_y)
        {
            cells.push(LadderCell {
                offset,
                raw_x,
                raw_y,
                kind: LadderElementKind::DeviceRef,
                value: String::new(),
                operands: Vec::new(),
                contact: Some(contact),
                coil: None,
            });
        }

        offset += 1;
    }
}

pub(crate) fn marker_only_ladder_contact(marker: [u8; 2]) -> Option<LadderContact> {
    match marker {
        [0xff, 0x3e] => Some(LadderContact::Inverse),
        [0xff, 0x48] => Some(LadderContact::RisingPulse),
        [0xff, 0x49] => Some(LadderContact::FallingPulse),
        _ => None,
    }
}

pub(crate) fn push_ladder_horizontal_line(
    lines: &mut Vec<LadderHorizontalLine>,
    raw_y: u8,
    raw_x_start: u8,
    raw_x_end: u8,
) {
    let (raw_x_start, raw_x_end) = (raw_x_start.min(raw_x_end), raw_x_start.max(raw_x_end));
    if raw_x_start < raw_x_end {
        lines.push(LadderHorizontalLine {
            raw_y,
            raw_x_start,
            raw_x_end,
        });
    }
}

pub(crate) fn extract_ladder_rung_comments(data: &[u8]) -> Vec<LadderRungComment> {
    let mut comments = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x3f])
    }) {
        let marker = offset + relative;
        if let Some((raw_x, raw_y)) = ladder_rung_comment_coordinate(data, marker)
            && let Some(text) = read_ladder_inline_utf16_string(data, marker + 2, marker + 96)
        {
            comments.push(LadderRungComment {
                offset: marker,
                raw_x,
                raw_y,
                text,
            });
        }

        offset = marker + 2;
    }

    comments.sort_by_key(|comment| (comment.raw_y, comment.raw_x, comment.offset));
    comments.dedup_by_key(|comment| (comment.offset, comment.raw_x, comment.raw_y));
    comments
}

pub(crate) fn extract_ladder_output_comments(data: &[u8]) -> Vec<LadderOutputComment> {
    let mut comments = Vec::new();
    let mut offset = 0;

    while let Some(relative) = data.get(offset..).and_then(|remaining| {
        remaining
            .windows(2)
            .position(|window| window == [0xff, 0x40])
    }) {
        let marker = offset + relative;
        if let Some((raw_x, raw_y)) = ladder_record_coordinate(data, marker)
            && let Some(text) = read_ladder_inline_utf16_string(data, marker + 2, marker + 96)
        {
            comments.push(LadderOutputComment {
                offset: marker,
                raw_x,
                raw_y,
                text,
            });
        }

        offset = marker + 2;
    }

    comments.sort_by_key(|comment| (comment.raw_y, comment.raw_x, comment.offset));
    comments.dedup_by_key(|comment| (comment.offset, comment.raw_x, comment.raw_y));
    comments
}

pub(crate) fn extract_ladder_unknown_records(
    data: &[u8],
    cells: &[LadderCell],
) -> Vec<LadderUnknownRecord> {
    let mut records = Vec::new();
    let mut offset = 0;

    while offset + 2 <= data.len() {
        if data.get(offset) != Some(&0xff) {
            offset += 1;
            continue;
        }

        let marker = [data[offset], data[offset + 1]];
        if is_ladder_utf16_length_marker(data, offset) {
            offset += 1;
            continue;
        }
        if is_ladder_marker_vertical_endpoint(data, offset, marker) {
            offset += 1;
            continue;
        }
        if marker == [0xff, 0xfe]
            || marker == [0xff, 0x02]
            || marker == [0xff, 0x3f]
            || marker == [0xff, 0x40]
            || marker == [0xff, 0x43]
        {
            offset += 1;
            continue;
        }

        if let Some((raw_x, raw_y)) = ladder_record_coordinate(data, offset)
            && !is_known_ladder_cell_record(cells, offset, marker, raw_x, raw_y)
        {
            let end = (offset + 24).min(data.len());
            records.push(LadderUnknownRecord {
                offset,
                marker,
                raw_x,
                raw_y,
                bytes: data[offset..end].to_vec(),
            });
        }

        offset += 1;
    }

    records.sort_by_key(|record| (record.raw_y, record.raw_x, record.offset));
    records.dedup_by_key(|record| (record.offset, record.raw_x, record.raw_y));
    records
}

pub(crate) fn read_ladder_inline_utf16_string(
    data: &[u8],
    start: usize,
    end: usize,
) -> Option<String> {
    let end = end.min(data.len());
    let mut offset = start;

    while offset + 4 <= end {
        if data.get(offset..offset + 3) == Some(UTF16_MARKER) {
            let unit_count = usize::from(*data.get(offset + 3)?);
            let text_start = offset + 4;
            let text_end = text_start + unit_count * 2;
            if text_end <= data.len() && text_end <= end {
                if let Some(text) = decode_utf16_bytes(&data[text_start..text_end]) {
                    return Some(text);
                }
            }
        }

        offset += 1;
    }

    None
}

fn decode_utf16_bytes(bytes: &[u8]) -> Option<String> {
    char::decode_utf16(
        bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]])),
    )
    .collect::<Result<String, _>>()
    .ok()
}

pub(crate) fn ladder_record_coordinate(data: &[u8], offset: usize) -> Option<(u8, u8)> {
    ladder_record_coordinate_with_min_x(data, offset, 2)
}

pub(crate) fn ladder_rung_comment_coordinate(data: &[u8], offset: usize) -> Option<(u8, u8)> {
    ladder_record_coordinate_with_min_x(data, offset, 1)
}

pub(crate) fn ladder_record_coordinate_with_min_x(
    data: &[u8],
    offset: usize,
    min_raw_x: u8,
) -> Option<(u8, u8)> {
    [5, 10, 17, 36]
        .iter()
        .filter_map(|relative| read_ladder_coordinate(data, offset + relative))
        .find(|(raw_x, _)| *raw_x >= min_raw_x)
}

pub(crate) fn is_ladder_utf16_length_marker(data: &[u8], offset: usize) -> bool {
    offset >= 2 && data.get(offset - 2..offset + 2) == Some(&[0xff, 0xfe, 0xff, data[offset + 1]])
}

pub(crate) fn is_ladder_marker_vertical_endpoint(
    data: &[u8],
    offset: usize,
    marker: [u8; 2],
) -> bool {
    if marker != [0xff, 0x01] {
        return false;
    }

    let Some((raw_x, raw_y)) = ladder_record_coordinate(data, offset) else {
        return false;
    };

    ladder_marker_records(data, [0xff, 0x3e])
        .into_iter()
        .filter(|start| {
            start.raw_x == raw_x
                && start.raw_y < raw_y
                && (raw_y - start.raw_y) % 4 == 0
                && !has_ladder_inline_string_after_marker(data, start.offset)
        })
        .any(|start| {
            marker_vertical_line_x(
                data,
                start,
                LadderMarkerRecord {
                    offset,
                    raw_x,
                    raw_y,
                },
            )
            .is_some()
        })
}

pub(crate) fn has_ladder_inline_string_after_marker(data: &[u8], offset: usize) -> bool {
    read_ladder_inline_utf16_string(data, offset + 2, offset + 40).is_some()
}

pub(crate) fn is_known_ladder_cell_record(
    cells: &[LadderCell],
    offset: usize,
    marker: [u8; 2],
    raw_x: u8,
    raw_y: u8,
) -> bool {
    cells.iter().any(|cell| {
        cell.raw_x == raw_x
            && cell.raw_y == raw_y
            && cell.offset >= offset
            && cell.offset - offset <= 40
    }) || (is_known_ladder_cell_marker(marker)
        && cells
            .iter()
            .any(|cell| cell.offset >= offset && cell.offset - offset <= 40))
}

pub(crate) fn is_known_ladder_cell_marker(marker: [u8; 2]) -> bool {
    matches!(
        marker,
        [0xff, 0x06]
            | [0xff, 0x07]
            | [0xff, 0x08]
            | [0xff, 0x09]
            | [0xff, 0x0a]
            | [0xff, 0x0b]
            | [0xff, 0x0e]
            | [0xff, 0x10]
            | [0xff, 0x11]
            | [0xff, 0x12]
            | [0xff, 0x13]
            | [0xff, 0x3e]
            | [0xff, 0x48]
            | [0xff, 0x49]
    )
}

pub(crate) fn ladder_element_coordinate(data: &[u8], element: &LadderElement) -> Option<(u8, u8)> {
    let preferred_offset = if !element.operands.is_empty() || is_ladder_operation(&element.value) {
        element.offset.checked_sub(14)
    } else {
        element.offset.checked_sub(10)
    };

    preferred_offset
        .and_then(|offset| read_ladder_coordinate(data, offset))
        .or_else(|| {
            [14, 10, 12, 8]
                .iter()
                .filter_map(|back| element.offset.checked_sub(*back))
                .find_map(|offset| read_ladder_coordinate(data, offset))
        })
}

pub(crate) fn read_ladder_coordinate(data: &[u8], offset: usize) -> Option<(u8, u8)> {
    let bytes = data.get(offset..offset + 2)?;
    let raw_x = bytes[0];
    let raw_y = bytes[1];

    (raw_x > 0 && raw_x <= 0x80 && raw_y <= 0xf0 && raw_y % 4 == 0).then_some((raw_x, raw_y))
}

pub(crate) fn parse_ladder_element(
    data: &[u8],
    ladder_string: &LadderString,
) -> Option<LadderElement> {
    let value = ladder_string.value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some((mnemonic, operands)) = parse_ladder_operation_call(value) {
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: if is_ladder_operation(&mnemonic) {
                ladder_operation_kind(&mnemonic)
            } else {
                LadderElementKind::InstructionCall
            },
            value: mnemonic,
            operands,
            contact: None,
            coil: None,
        });
    }

    if looks_like_device_ref(value) {
        let coil = ladder_coil(data, ladder_string.offset);
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::DeviceRef,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: coil
                .is_none()
                .then(|| ladder_contact(data, ladder_string.offset))
                .flatten(),
            coil,
        });
    }

    if looks_like_internal_ref(value) {
        let coil = ladder_coil(data, ladder_string.offset);
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::InternalRef,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: coil
                .is_none()
                .then(|| ladder_contact(data, ladder_string.offset))
                .flatten(),
            coil,
        });
    }

    if looks_like_constant(value) {
        return Some(LadderElement {
            offset: ladder_string.offset,
            kind: LadderElementKind::Constant,
            value: value.to_owned(),
            operands: Vec::new(),
            contact: None,
            coil: None,
        });
    }

    None
}

pub(crate) fn ladder_contact(data: &[u8], string_offset: usize) -> Option<LadderContact> {
    (8..=24).find_map(|back| {
        let marker = string_offset.checked_sub(back)?;
        match (data.get(marker).copied(), data.get(marker + 1).copied()) {
            (Some(0xff), Some(0x06)) => Some(LadderContact::NormallyOpen),
            (Some(0xff), Some(0x07)) => Some(LadderContact::NormallyClosed),
            (Some(0xff), Some(0x08)) => Some(LadderContact::AddressedRisingPulse),
            (Some(0xff), Some(0x09)) => Some(LadderContact::AddressedFallingPulse),
            (Some(0xff), Some(0x0a)) => Some(LadderContact::AddressedRisingPulseNot),
            (Some(0xff), Some(0x0b)) => Some(LadderContact::AddressedFallingPulseNot),
            (Some(0xff), Some(0x3e)) => Some(LadderContact::Inverse),
            (Some(0xff), Some(0x48)) => Some(LadderContact::RisingPulse),
            (Some(0xff), Some(0x49)) => Some(LadderContact::FallingPulse),
            _ => None,
        }
    })
}

pub(crate) fn ladder_coil(data: &[u8], string_offset: usize) -> Option<LadderCoil> {
    (8..=24).find_map(|back| {
        let marker = string_offset.checked_sub(back)?;
        match (data.get(marker).copied(), data.get(marker + 1).copied()) {
            (Some(0xff), Some(0x0e)) => Some(LadderCoil::Output),
            (Some(0xff), Some(0x0f)) => Some(LadderCoil::Inverse),
            (Some(0xff), Some(0x10)) => Some(LadderCoil::Set),
            (Some(0xff), Some(0x11)) => Some(LadderCoil::Reset),
            (Some(0xff), Some(0x12)) => Some(LadderCoil::RisingPulse),
            (Some(0xff), Some(0x13)) => Some(LadderCoil::FallingPulse),
            _ => None,
        }
    })
}

pub(crate) fn parse_ladder_instruction(ladder_string: &LadderString) -> Option<LadderInstruction> {
    let (mnemonic, operands) = parse_ladder_operation_call(ladder_string.value.trim())?;
    if operands.is_empty() {
        return None;
    }

    Some(LadderInstruction {
        offset: ladder_string.offset,
        mnemonic,
        operands,
        raw: ladder_string.value.clone(),
    })
}

pub(crate) fn parse_ladder_operation_call(value: &str) -> Option<(String, Vec<String>)> {
    let parts = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.len() > 1 && (is_ladder_operation(parts[0]) || looks_like_mnemonic(parts[0])) {
        return Some((
            parts[0].to_owned(),
            parts[1..].iter().map(|part| (*part).to_owned()).collect(),
        ));
    }

    if is_ladder_operation(value) {
        return Some((value.to_owned(), Vec::new()));
    }

    prefixed_ladder_operation_call(value)
}

fn prefixed_ladder_operation_call(value: &str) -> Option<(String, Vec<String>)> {
    let mut best = None;
    for mnemonic in known_ladder_mnemonics()
        .iter()
        .map(|info| info.mnemonic)
        .chain(FALLBACK_LADDER_OPERATIONS.iter().copied())
    {
        let Some(rest) = value.strip_prefix(mnemonic) else {
            continue;
        };
        if rest.is_empty() || !rest.starts_with(|ch: char| ch == ',' || ch.is_ascii_whitespace()) {
            continue;
        }
        if best.is_none_or(|best: &str| mnemonic.len() > best.len()) {
            best = Some(mnemonic);
        }
    }

    let mnemonic = best?;
    let operands = value
        .strip_prefix(mnemonic)
        .unwrap_or_default()
        .trim_start_matches(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .map(str::trim)
        .filter(|operand| !operand.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (!operands.is_empty()).then(|| (mnemonic.to_owned(), operands))
}

pub(crate) fn duplicate_decomposed_string_count(
    strings: &[LadderString],
    index: usize,
    element: &LadderElement,
) -> usize {
    if element.operands.is_empty()
        && is_ladder_operation(&element.value)
        && strings
            .get(index + 1)
            .is_some_and(|string| string.value == element.value)
    {
        return 1;
    }

    if element.operands.is_empty() || !is_ladder_operation(&element.value) {
        return 0;
    }

    let expected = std::iter::once(element.value.as_str())
        .chain(element.operands.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let Some(following) = strings.get(index + 1..index + 1 + expected.len()) else {
        return 0;
    };

    if following
        .iter()
        .zip(expected)
        .all(|(string, expected)| string.value == expected)
    {
        following.len()
    } else {
        0
    }
}

const FALLBACK_LADDER_OPERATIONS: &[&str] = &[
    "AND", "OR", "XOR", "NOT", "SET", "RST", "RESET", "OUT", "OUTP", "FF", "TON", "TOFF", "TMR",
    "TFLK", "TMON", "TRTG", "CTR", "CTU", "CTD", "CTUD", "MOV", "MOVP", "FMOV", "FMOVP", "DMOV",
    "RMOV", "INC", "INCP", "DEC", "DECP", "I2R", "R2I", "RADD", "RSUB", "RMUL", "RDIV", "ADD",
    "SUB", "MUL", "DIV", "DADD", "DSUB", "DMUL", "DDIV", "GETM", "XDST", "FOR", "NEXT", "DNEGP",
];

pub(crate) fn is_ladder_operation(value: &str) -> bool {
    is_ladder_comparison_mnemonic(value)
        || ladder_mnemonic_info(value).is_some()
        || FALLBACK_LADDER_OPERATIONS.contains(&value)
}

pub(crate) fn ladder_operation_kind(value: &str) -> LadderElementKind {
    if is_ladder_comparison_mnemonic(value) {
        return LadderElementKind::Comparison;
    }

    if let Some(info) = ladder_mnemonic_info(value) {
        match info.category {
            LadderMnemonicCategory::Comparison => return LadderElementKind::Comparison,
            LadderMnemonicCategory::TimerCounter => return LadderElementKind::Timer,
            LadderMnemonicCategory::BasicInstructions => return LadderElementKind::Operation,
            _ => {}
        }
    }

    match value {
        "TON" | "TOFF" | "TMR" | "TFLK" | "TMON" | "TRTG" | "CTR" | "CTU" | "CTD" | "CTUD" => {
            LadderElementKind::Timer
        }
        "AND" | "OR" | "XOR" | "NOT" => LadderElementKind::Logic,
        "BRST" | "BRSTP" | "MCS" | "MCSCLR" | "SET" | "RST" | "RESET" | "OUT" | "OUTP" | "FF" => {
            LadderElementKind::Operation
        }
        _ => LadderElementKind::InstructionCall,
    }
}

pub(crate) fn is_ladder_comparison_mnemonic(value: &str) -> bool {
    if let Some(value) = value.strip_prefix('4').or_else(|| value.strip_prefix('8')) {
        return matches!(value, "=" | "<>" | ">" | "<" | ">=" | "<=");
    }

    matches!(
        value,
        "=" | "<>" | ">" | "<" | ">=" | "<=" | "=3" | "<>3" | ">3" | "<3" | ">=3" | "<=3"
    )
}

pub(crate) fn looks_like_device_ref(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };

    matches!(
        first,
        'M' | 'P' | 'T' | 'C' | 'D' | 'R' | 'U' | 'Z' | 'X' | 'Y' | 'K' | 'L' | 'N'
    ) && value
        .chars()
        .skip(1)
        .all(|ch| ch.is_ascii_hexdigit() || ch == '.' || ch == '[' || ch == ']')
        && value.chars().any(|ch| ch.is_ascii_digit())
}

pub(crate) fn looks_like_internal_ref(value: &str) -> bool {
    value.len() > 1
        && value.starts_with('F')
        && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
        && value.chars().skip(1).any(|ch| ch.is_ascii_digit())
}

pub(crate) fn looks_like_constant(value: &str) -> bool {
    if value.starts_with('h') || value.starts_with('H') {
        return value.len() > 1 && value.chars().skip(1).all(|ch| ch.is_ascii_hexdigit());
    }

    value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok()
}

pub(crate) fn looks_like_mnemonic(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' || ch == ' ')
        && value.chars().any(|ch| ch.is_ascii_alphabetic())
}
