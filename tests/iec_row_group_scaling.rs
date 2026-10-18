#![cfg(feature = "write")]

use xgwx::XgwxDocument;

fn grouped_rows(group_count: u16, rows_per_group: u16) -> Vec<u8> {
    let count = group_count * rows_per_group;
    let mut data = vec![0; 8];
    data[4..6].copy_from_slice(&count.to_le_bytes());
    data[6..8].copy_from_slice(&group_count.to_le_bytes());
    for group in 0..u32::from(group_count) {
        data.extend_from_slice(&group.to_le_bytes());
        data.extend_from_slice(&[0; 4]);
        data.extend_from_slice(&rows_per_group.to_le_bytes());

        for local_row in 0..u32::from(rows_per_group) {
            let index = group * u32::from(rows_per_group) + local_row;
            let mut row = [0u8; 35];
            row[..4].copy_from_slice(&index.to_le_bytes());
            row[4..6].copy_from_slice(&[0xff, 0x43]);
            let y = (index as u16 * 4).to_le_bytes();
            for offset in [21, 29] {
                row[offset] = 94;
                row[offset + 1..offset + 3].copy_from_slice(&y);
            }
            data.extend_from_slice(&row);
        }
    }
    data
}

#[test]
fn many_one_row_iec_groups_keep_exact_frame_boundaries() {
    let doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/iec-row300-native.xgwx"
    ))
    .unwrap();
    let mut program = doc.program().ladder_programs().remove(0).unwrap();
    let count = 8_192;
    program.data = grouped_rows(count, 1);
    program.decoded_len = program.data.len();
    let frames = program.iec_row_frames().unwrap();
    assert_eq!(frames.len(), count as usize);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.group_index, index);
        assert_eq!(frame.row_index, index as u16);
        assert_eq!(frame.start, 18 + index * 45);
        assert_eq!(frame.records_start, frame.start + 35);
        assert_eq!(frame.end, frame.records_start);
        assert_eq!(frame.record_count, 0);
    }
    assert!(
        program
            .iec_terminal_timer_insertion_sites()
            .unwrap()
            .is_empty()
    );
    assert!(
        program
            .iec_function_cell_insertion_sites()
            .unwrap()
            .is_empty()
    );

    // A group claiming two rows must not consume a row from the next group.
    program.data[16..18].copy_from_slice(&2u16.to_le_bytes());
    assert!(program.iec_row_frames().is_none());
}

#[test]
fn many_two_and_six_row_groups_keep_browser_edit_inspection_bounded() {
    let doc = XgwxDocument::parse(include_bytes!(
        "../fixtures/canvas-rows/iec-row300-native.xgwx"
    ))
    .unwrap();
    let mut program = doc.program().ladder_programs().remove(0).unwrap();
    for (group_count, rows_per_group) in [(4_096, 2), (1_365, 6)] {
        program.data = grouped_rows(group_count, rows_per_group);
        program.decoded_len = program.data.len();
        assert_eq!(
            program.iec_row_frames().unwrap().len(),
            usize::from(group_count * rows_per_group)
        );
        assert!(
            program
                .iec_leading_contact_insertion_sites()
                .unwrap()
                .is_empty()
        );
    }
}
