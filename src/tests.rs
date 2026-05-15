use super::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::env;
use std::io::Write;

#[test]
fn parses_synthetic_workspace() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Project FileVer="1.2.3.4" GUID="synthetic-guid" Version="513" Attribute="7" WksNodeCount="1">Synthetic
  <Configuration GUID="config-guid">PLC</Configuration>
  <Network Name="Network" Type="Ethernet" NetworkType="FEnet">
    <NetworkModule ConfigName="PLC" Base="0" Slot="1" ID="42" />
  </Network>
  <Program Task="scan" ObjectID="1" Version="2" Kind="0">Main</Program>
</Project>"#;
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(xml)).expect("synthetic parses");

    assert!(doc.header.gzip_offset > 0);
    assert_eq!(doc.header.label.as_deref(), Some("XG5000 WORKSPACE FILE"));
    assert_eq!(doc.header.label_following_u32, Some(1));
    assert_eq!(doc.root.name, "Project");
    assert_eq!(doc.project_name(), Some("Synthetic"));
    assert_eq!(doc.project_info().file_version.as_deref(), Some("1.2.3.4"));
    assert_eq!(doc.configurations().len(), 1);
    assert_eq!(doc.networks()[0].modules.len(), 1);
    assert_eq!(doc.programs()[0].name.as_deref(), Some("Main"));
}

#[test]
fn decodes_synthetic_ladder_records() {
    let data = synthetic_ladder_data();
    let strings = extract_ladder_strings(&data);
    let elements = extract_ladder_elements(&data, &strings);
    let structure = extract_ladder_structure(&data, &elements);

    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00001"
            && element.contact == Some(LadderContact::NormallyOpen)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00002"
            && element.contact == Some(LadderContact::NormallyClosed)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00005"
            && element.contact == Some(LadderContact::Inverse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00006"
            && element.contact == Some(LadderContact::RisingPulse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00007"
            && element.contact == Some(LadderContact::FallingPulse)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00003"
            && element.coil == Some(LadderCoil::Output)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::DeviceRef
            && element.value == "M00004"
            && element.coil == Some(LadderCoil::Reset)
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::InstructionCall
            && element.value == "MOV"
            && element.operands == ["D000001", "D000002"]
    }));
    assert!(elements.iter().any(|element| {
        element.kind == LadderElementKind::InternalRef
            && element.value == "F0092"
            && element.contact == Some(LadderContact::NormallyOpen)
    }));
    assert!(structure.rungs.iter().any(|rung| {
        rung.cells
            .iter()
            .any(|cell| cell.value == "F0092" && cell.contact == Some(LadderContact::NormallyOpen))
    }));
    assert!(structure.vertical_lines.iter().any(|line| (
        line.raw_x,
        line.raw_y_start,
        line.raw_y_end
    ) == (0x03, 0x00, 0x04)));
    assert_eq!(
        structure
            .vertical_lines
            .iter()
            .filter(|line| (line.raw_x, line.raw_y_start, line.raw_y_end) == (0x06, 0x04, 0x0c))
            .count(),
        1
    );
    assert!(structure.unknown_records.iter().any(|record| {
        record.marker == [0xff, 0x55]
            && (record.raw_x, record.raw_y) == (0x20, 0x04)
            && record.bytes.starts_with(&[0xff, 0x55])
    }));
    assert!(structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x00, 0x01, 0x58)));
    assert!(structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x04, 0x08, 0x5e)));
    assert!(
        !structure
            .unknown_records
            .iter()
            .any(|record| record.marker == [0xff, 0x02])
    );
}

#[test]
fn recognizes_additional_ladder_mnemonics() {
    let categorized_samples = [
        ("LOAD NOT", LadderMnemonicCategory::BasicInstructions),
        ("BRST", LadderMnemonicCategory::BasicInstructions),
        ("TON", LadderMnemonicCategory::TimerCounter),
        ("CTUD", LadderMnemonicCategory::TimerCounter),
        ("$MOVP", LadderMnemonicCategory::DataTransfer),
        ("GBMOVP", LadderMnemonicCategory::DataTransfer),
        ("WTODWP", LadderMnemonicCategory::BcdBinConversion),
        ("L2UDP", LadderMnemonicCategory::DataTypeConversion),
        ("CMP8P", LadderMnemonicCategory::Comparison),
        ("LOAD X", LadderMnemonicCategory::Comparison),
        ("OR4X", LadderMnemonicCategory::Comparison),
        ("DINCUP", LadderMnemonicCategory::IncrementDecrement),
        ("RCR8P", LadderMnemonicCategory::Rotation),
        ("DBSFRP", LadderMnemonicCategory::Shift),
        ("GSWAP2P", LadderMnemonicCategory::Exchange),
        ("$ADDP", LadderMnemonicCategory::BinaryArithmetic),
        ("GADDP", LadderMnemonicCategory::BinaryArithmetic),
        ("ADDCP", LadderMnemonicCategory::BcdArithmetic),
        ("ABXNRP", LadderMnemonicCategory::LogicalOperations),
        ("SEGP", LadderMnemonicCategory::Display),
        ("DETECTP", LadderMnemonicCategory::DataProcessing),
        ("FIINSP", LadderMnemonicCategory::DataTableProcessing),
        ("DDABCDP", LadderMnemonicCategory::StringProcessing),
        ("EXPTP", LadderMnemonicCategory::SpecialFunctions),
        ("PIDRUN", LadderMnemonicCategory::DataControl),
        ("ADDCAL", LadderMnemonicCategory::Time),
        ("JMP", LadderMnemonicCategory::Branching),
        ("BREAK", LadderMnemonicCategory::Loop),
        ("STC", LadderMnemonicCategory::Flag),
        ("TFLK", LadderMnemonicCategory::System),
        ("EIN", LadderMnemonicCategory::Interrupt),
        ("LNEGP", LadderMnemonicCategory::SignInversion),
        ("RSETP", LadderMnemonicCategory::File),
        ("FWRITE", LadderMnemonicCategory::FAreaControl),
        ("BRESET", LadderMnemonicCategory::WordBitControl),
        ("GETEP", LadderMnemonicCategory::SpecialCommunication),
        ("GETIP", LadderMnemonicCategory::Communication),
        ("PWM", LadderMnemonicCategory::Positioning),
        ("XCCCONEX", LadderMnemonicCategory::Positioning),
        ("XGETP", LadderMnemonicCategory::MotionControl),
    ];

    for (mnemonic, category) in categorized_samples {
        let info = ladder_mnemonic_info(mnemonic).expect("mnemonic metadata exists");
        assert_eq!(info.category, category, "{mnemonic} category");
        assert!(!info.description.is_empty());
        assert!(is_ladder_operation(mnemonic), "{mnemonic} is recognized");
    }

    let known = known_ladder_mnemonics();
    assert!(known.len() > 500, "manual mnemonic coverage is broad");
    for (index, info) in known.iter().enumerate() {
        assert!(!info.mnemonic.is_empty());
        assert!(!info.description.is_empty());
        assert_eq!(
            known
                .iter()
                .position(|other| other.mnemonic == info.mnemonic),
            Some(index),
            "{} appears only once",
            info.mnemonic
        );
    }

    for invalid_group_arithmetic in ["GADDU", "GADDUP", "GMUL", "GMULP", "GDIV", "GDIVP"] {
        assert_eq!(ladder_mnemonic_info(invalid_group_arithmetic), None);
    }
    for invalid_string_arithmetic in ["$SUB", "$SUBP", "$MUL", "$MULP", "$DIV", "$DIVP"] {
        assert_eq!(ladder_mnemonic_info(invalid_string_arithmetic), None);
    }

    assert_eq!(ladder_operation_kind("BRSTP"), LadderElementKind::Operation);
    assert_eq!(
        ladder_operation_kind("LOAD NOT"),
        LadderElementKind::Operation
    );
    assert_eq!(
        ladder_operation_kind("MCSCLR"),
        LadderElementKind::Operation
    );

    for comparison in [
        "=3", "<>3", ">3", "<3", ">=3", "<=3", "4=", "4<>", "4>", "4<", "4>=", "4<=", "8=", "8<>",
        "8>", "8<", "8>=", "8<=",
    ] {
        assert!(
            is_ladder_comparison_mnemonic(comparison),
            "{comparison} is recognized as comparison mnemonic"
        );
        assert_eq!(
            ladder_operation_kind(comparison),
            LadderElementKind::Comparison,
            "{comparison} is a comparison"
        );
    }

    for comparison in ["4=3", "4>=3", "8<>3", "8<=3"] {
        assert!(
            !is_ladder_comparison_mnemonic(comparison),
            "{comparison} is invalid because prefix and suffix are both present"
        );
    }

    for timer in ["TFLK", "TMON", "TRTG", "CTR", "CTUD"] {
        assert_eq!(
            ladder_operation_kind(timer),
            LadderElementKind::Timer,
            "{timer} is timer-like"
        );
    }

    for (source, expected_value, expected_kind, expected_operands) in [
        (
            "INC",
            "INC",
            LadderElementKind::InstructionCall,
            Vec::<&str>::new(),
        ),
        ("TFLK", "TFLK", LadderElementKind::Timer, Vec::new()),
        ("<=3", "<=3", LadderElementKind::Comparison, Vec::new()),
        ("CTUD", "CTUD", LadderElementKind::Timer, Vec::new()),
        (
            "INC D00001",
            "INC",
            LadderElementKind::InstructionCall,
            vec!["D00001"],
        ),
        (
            "TFLK T0 D0",
            "TFLK",
            LadderElementKind::Timer,
            vec!["T0", "D0"],
        ),
        (
            "<=3 D0 D1",
            "<=3",
            LadderElementKind::Comparison,
            vec!["D0", "D1"],
        ),
        (
            "CTUD C0 D0",
            "CTUD",
            LadderElementKind::Timer,
            vec!["C0", "D0"],
        ),
    ] {
        let element = parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: source.len(),
                value: source.to_owned(),
            },
        )
        .unwrap_or_else(|| panic!("{source} parses"));

        assert_eq!(element.value, expected_value);
        assert_eq!(element.kind, expected_kind, "{source} kind");
        assert_eq!(element.operands, expected_operands, "{source} operands");
    }

    assert_eq!(
        parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: 8,
                value: "Auto Run".to_owned(),
            },
        ),
        None
    );

    for instruction in ["INC,D00001", "INCP,D00001", "DEC,D00001", "DECP,D00001"] {
        let element = parse_ladder_element(
            &[],
            &LadderString {
                offset: 0,
                end_offset: instruction.len(),
                value: instruction.to_owned(),
            },
        )
        .expect("instruction parses");

        assert_eq!(element.kind, LadderElementKind::InstructionCall);
        assert_eq!(element.operands, ["D00001"]);
    }

    let comparison = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 12,
            value: ">=3,D0,D1".to_owned(),
        },
    )
    .expect("comparison parses");

    assert_eq!(comparison.kind, LadderElementKind::Comparison);
    assert_eq!(comparison.value, ">=3");
    assert_eq!(comparison.operands, ["D0", "D1"]);

    let prefixed_comparison = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 13,
            value: "4>=,D0,D1".to_owned(),
        },
    )
    .expect("prefixed comparison parses");

    assert_eq!(prefixed_comparison.kind, LadderElementKind::Comparison);
    assert_eq!(prefixed_comparison.value, "4>=");
    assert_eq!(prefixed_comparison.operands, ["D0", "D1"]);

    let bcd_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 19,
            value: "DADDBP,D0,D2,D4".to_owned(),
        },
    )
    .expect("BCD arithmetic instruction parses");

    assert_eq!(bcd_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(bcd_instruction.value, "DADDBP");
    assert_eq!(bcd_instruction.operands, ["D0", "D2", "D4"]);

    let conversion_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "GBCDP,D0,D2".to_owned(),
        },
    )
    .expect("BCD/BIN conversion instruction parses");

    assert_eq!(
        conversion_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(conversion_instruction.value, "GBCDP");
    assert_eq!(conversion_instruction.operands, ["D0", "D2"]);

    let string_add_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 17,
        value: "$ADDP,S0,S1,S2".to_owned(),
    })
    .expect("string add instruction parses");

    assert_eq!(string_add_instruction.mnemonic, "$ADDP");
    assert_eq!(string_add_instruction.operands, ["S0", "S1", "S2"]);

    let whitespace_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 14,
        value: "CTUD C0 D0".to_owned(),
    })
    .expect("whitespace instruction parses");

    assert_eq!(whitespace_instruction.mnemonic, "CTUD");
    assert_eq!(whitespace_instruction.operands, ["C0", "D0"]);

    let binary_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "DADDUP,D0,D2,D4".to_owned(),
        },
    )
    .expect("binary arithmetic instruction parses");

    assert_eq!(binary_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(binary_instruction.value, "DADDUP");
    assert_eq!(binary_instruction.operands, ["D0", "D2", "D4"]);

    let exchange_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "SWAP2P,D0,D2".to_owned(),
        },
    )
    .expect("exchange instruction parses");

    assert_eq!(
        exchange_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(exchange_instruction.value, "SWAP2P");
    assert_eq!(exchange_instruction.operands, ["D0", "D2"]);

    let logical_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "DWXORP,D0,D2,D4".to_owned(),
        },
    )
    .expect("logical instruction parses");

    assert_eq!(logical_instruction.kind, LadderElementKind::InstructionCall);
    assert_eq!(logical_instruction.value, "DWXORP");
    assert_eq!(logical_instruction.operands, ["D0", "D2", "D4"]);

    let string_move_instruction = parse_ladder_instruction(&LadderString {
        offset: 0,
        end_offset: 15,
        value: "$MOVP,S0,S1".to_owned(),
    })
    .expect("string move instruction parses");

    assert_eq!(string_move_instruction.mnemonic, "$MOVP");
    assert_eq!(string_move_instruction.operands, ["S0", "S1"]);

    let transfer_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 17,
            value: "GBMOVP,M0,M10".to_owned(),
        },
    )
    .expect("data transfer instruction parses");

    assert_eq!(
        transfer_instruction.kind,
        LadderElementKind::InstructionCall
    );
    assert_eq!(transfer_instruction.value, "GBMOVP");
    assert_eq!(transfer_instruction.operands, ["M0", "M10"]);

    let spaced_comparison_instruction = parse_ladder_element(
        &[],
        &LadderString {
            offset: 0,
            end_offset: 15,
            value: "LOAD X,D0,D1".to_owned(),
        },
    )
    .expect("spaced comparison instruction parses");

    assert_eq!(
        spaced_comparison_instruction.kind,
        LadderElementKind::Comparison
    );
    assert_eq!(spaced_comparison_instruction.value, "LOAD X");
    assert_eq!(spaced_comparison_instruction.operands, ["D0", "D1"]);
}

#[test]
fn decodes_elements_fixture_pulse_contacts_and_coils() {
    let doc = XgwxDocument::from_path("fixtures/elements.xgwx").expect("fixture parses");
    let program = doc
        .ladder_programs()
        .into_iter()
        .next()
        .expect("fixture has a ladder program")
        .expect("ladder program decodes");

    assert_ladder_contact(&program, "M00002", LadderContact::AddressedRisingPulse);
    assert_ladder_contact(&program, "M00003", LadderContact::AddressedRisingPulseNot);
    assert_ladder_contact(&program, "M00004", LadderContact::AddressedFallingPulse);
    assert_ladder_contact(&program, "M00005", LadderContact::AddressedFallingPulseNot);
    assert_ladder_coil(&program, "P00021", LadderCoil::Inverse);
    assert_ladder_coil(&program, "M00100", LadderCoil::RisingPulse);
    assert_ladder_coil(&program, "M00101", LadderCoil::FallingPulse);
    assert_ladder_instruction(
        &program,
        "XDST",
        &["1", "1", "7000", "1000", "100", "0", "0"],
    );
    assert_eq!(
        program.structure.rung_comments,
        vec![LadderRungComment {
            offset: 0x0035,
            raw_x: 0x01,
            raw_y: 0x00,
            text: "렁 설명문 1".to_owned(),
        }]
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x03,
                raw_y_start: 0x08,
                raw_y_end: 0x0c,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x06,
                raw_y_start: 0x08,
                raw_y_end: 0x10,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x18,
                raw_y_start: 0x14,
                raw_y_end: 0x1c,
            })
    );
    assert!(
        program
            .structure
            .branch_groups
            .contains(&LadderBranchGroup {
                raw_x: 0x18,
                raw_y_start: 0x20,
                raw_y_end: 0x28,
            })
    );
    assert_marker_only_ladder_contact(&program, 0x04, 0x08, LadderContact::Inverse);
    assert_marker_only_ladder_contact(&program, 0x13, 0x04, LadderContact::RisingPulse);
    assert_marker_only_ladder_contact(&program, 0x16, 0x04, LadderContact::FallingPulse);
    assert!(!program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x08, 0x01, 0x03)));
    assert!(program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x08, 0x01, 0x06)));
    assert!(program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x0c, 0x01, 0x03)));
    assert!(!program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x0c, 0x01, 0x06)));
    assert!(program.structure.horizontal_lines.iter().any(|line| (
        line.raw_y,
        line.raw_x_start,
        line.raw_x_end
    ) == (0x10, 0x01, 0x06)));
    assert_eq!(
        program.structure.output_comments,
        vec![LadderOutputComment {
            offset: 0x0195,
            raw_x: 0x61,
            raw_y: 0x04,
            text: "출력 설명문 1".to_owned(),
        }]
    );
    assert!(!program.structure.unknown_records.iter().any(|record| {
        matches!(
            record.marker,
            [0xff, 0x01]
                | [0xff, 0x06]
                | [0xff, 0x3e]
                | [0xff, 0x3f]
                | [0xff, 0x40]
                | [0xff, 0x48]
                | [0xff, 0x49]
        )
    }));
}

#[test]
fn decodes_synthetic_hsc_parameter_counter_modes() {
    let mut payload = vec!['0'; 448];
    payload[1] = '1';
    payload[6] = '1';
    payload[7] = '2';
    payload[8] = '3';
    payload[9] = '4';
    payload[16] = '1';
    payload[17] = '8';
    payload[18] = '2';
    payload[19] = 'A';
    payload[40] = 'C';
    payload[41] = '8';
    payload[48] = '1';
    payload[49] = '4';
    payload[56] = '6';
    payload[57] = '4';
    payload[88] = '8';
    payload[89] = '8';
    payload[90] = '1';
    payload[91] = '3';
    payload[92] = '0';
    payload[93] = '4';
    let payload = payload.into_iter().collect::<String>();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Project>
  <Parameters>
    <Parameter Type="HSC PARAMETER" PAYLOAD_ASC_LENGTH="448" PAYLOAD="{payload}" />
  </Parameters>
</Project>"#
    );
    let doc = XgwxDocument::parse(&synthetic_xgwx_bytes(&xml)).expect("synthetic parses");
    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");

    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(hsc[0].payload_bytes.len(), 224);
    assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.counter_mode_raw)
            .collect::<Vec<_>>(),
        [Some(1), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.pulse_input_mode_raw)
            .collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(3)]
    );
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[1].counter_mode,
        Some(HscCounterMode::LinearCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::OnePhaseOneInputOneX)
    );
    assert_eq!(
        hsc[0].channels[1].pulse_input_mode,
        Some(HscPulseInputMode::OnePhaseTwoInputOneX)
    );
    assert_eq!(
        hsc[0].channels[2].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(
        hsc[0].channels[3].pulse_input_mode,
        Some(HscPulseInputMode::TwoPhaseFourX)
    );
    assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
    assert_eq!(hsc[0].channels[0].external_preset, Some(0x2a));
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
    assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
    assert!(
        hsc[0]
            .channels
            .iter()
            .all(|channel| channel.raw.len() == 56)
    );
}

#[test]
fn decodes_xgb_enet01_hsc_parameter_counter_modes() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");

    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(hsc[0].payload_bytes.len(), 224);
    assert_eq!(hsc[0].initial_unknown_nibble, Some(0));
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.counter_mode_raw)
            .collect::<Vec<_>>(),
        [Some(1), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0]
            .channels
            .iter()
            .map(|channel| channel.pulse_input_mode_raw)
            .collect::<Vec<_>>(),
        [Some(2), Some(0), Some(0), Some(0)]
    );
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(hsc[0].channels[0].compare_output_mode_raw, Some(4));
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].internal_preset, Some(0x18));
    assert_eq!(hsc[0].channels[0].external_preset, Some(0));
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].compare_output_min, Some(20));
    assert_eq!(hsc[0].channels[0].compare_output_max, Some(100));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));
    assert!(
        hsc[0].channels[1..]
            .iter()
            .all(|channel| channel.counter_mode == Some(HscCounterMode::LinearCounter))
    );
    assert!(hsc[0].channels[1..].iter().all(|channel| {
        channel.pulse_input_mode == Some(HscPulseInputMode::OnePhaseOneInputOneX)
    }));
}

#[test]
fn decodes_xgb_enet01_position_parameter_axes() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let position = doc.position_parameters();

    assert_eq!(position.len(), 1);
    assert_eq!(position[0].axis_count, Some(2));
    assert_eq!(position[0].axes.len(), 2);
    assert_eq!(position[0].axes[0].axis_name, "X");
    assert_eq!(position[0].axes[1].axis_name, "Y");

    for axis in &position[0].axes {
        assert_eq!(axis.step_count, Some(30));
        assert_eq!(axis.steps.len(), 30);
        assert_eq!(axis.steps[0].target_position, Some(0));
        assert_eq!(axis.steps[0].operation_velocity, Some(0));

        let parameter = axis.parameter.as_ref().expect("axis parameter");
        assert_eq!(parameter.bias_velocity, Some(1));
        assert_eq!(parameter.velocity_limit, Some(100000));
        assert_eq!(
            parameter.accel_times,
            [Some(500), Some(1000), Some(1500), Some(2000)]
        );
        assert_eq!(
            parameter.decel_times,
            [Some(500), Some(1000), Some(1500), Some(2000)]
        );
        assert_eq!(parameter.soft_upper_limit, Some(i32::MAX));
        assert_eq!(parameter.soft_lower_limit, Some(i32::MIN));
        assert_eq!(parameter.s_curve_ratio, Some(50));
        assert_eq!(parameter.use_limit, Some(1));
        assert_eq!(parameter.return_velocity_high, Some(5000));
        assert_eq!(parameter.return_velocity_low, Some(500));
        assert_eq!(parameter.jog_velocity_high, Some(5000));
        assert_eq!(parameter.jog_velocity_low, Some(1000));
    }
}

#[test]
fn decodes_xgb_enet01_pid_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let cal = doc.pid_cal_parameters();
    let tune = doc.pid_tune_parameters();

    assert_eq!(cal.len(), 1);
    assert_eq!(cal[0].header, [Some(17736), Some(17473)]);
    assert_eq!(cal[0].set_pid_out, Some(0));
    assert_eq!(cal[0].prevent_anti_windup, Some(0));
    assert_eq!(cal[0].differential_control_method, Some(65535));
    assert_eq!(cal[0].loops.len(), 16);
    assert_eq!(cal[0].loops[0].target_value, Some(0));
    assert_eq!(cal[0].loops[0].scan_time, Some(100));
    assert_eq!(cal[0].loops[0].proportional_gain_left, Some(1));
    assert_eq!(cal[0].loops[0].mv_max, Some(4000));
    assert_eq!(cal[0].loops[0].mv_min, Some(0));
    assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
    assert_eq!(cal[0].loops[0].pwm_out_period, Some(100));
    assert_eq!(cal[0].loops[0].pv_max, Some(4000));

    assert_eq!(tune.len(), 1);
    assert_eq!(tune[0].set_direction, Some(0));
    assert_eq!(tune[0].permit_pwm, Some(0));
    assert_eq!(tune[0].checksum, Some(0));
    assert_eq!(tune[0].footer, [Some(12358), Some(21552)]);
    assert_eq!(tune[0].loops.len(), 16);
    assert_eq!(tune[0].loops[0].target_value, Some(0));
    assert_eq!(tune[0].loops[0].scan_time, Some(100));
    assert_eq!(tune[0].loops[0].mv_max, Some(4000));
    assert_eq!(tune[0].loops[0].mv_min, Some(0));
    assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
    assert_eq!(tune[0].loops[0].out_period, Some(100));
    assert_eq!(tune[0].loops[0].hysteresis, Some(10));
}

#[test]
fn decodes_xgb_enet01_fenet_ipv4_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let fenet = doc.fenet_config_infos();

    assert_eq!(fenet.len(), 1);
    assert_eq!(fenet[0].type_code, Some(23041));
    assert_eq!(fenet[0].base, Some(0));
    assert_eq!(fenet[0].slot, Some(1));
    assert_eq!(fenet[0].sub_type, Some(32771));
    assert_eq!(
        fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.100")
    );
    assert_eq!(
        fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
        Some("255.255.255.0")
    );
    assert_eq!(
        fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.1")
    );
    assert_eq!(
        fenet[0].dns.as_ref().map(|ip| ip.address.as_str()),
        Some("0.0.0.0")
    );
    assert_eq!(
        fenet[0].ip_address2.as_ref().map(|ip| ip.address.as_str()),
        Some("0.0.0.0")
    );
}

#[test]
fn decodes_xgb_enet01_cnet_port_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet01.xgwx").expect("fixture parses");
    let cnet = doc.cnet_config_infos();

    assert_eq!(cnet.len(), 1);
    assert_eq!(cnet[0].type_code, Some(23104));
    assert_eq!(cnet[0].base, Some(0));
    assert_eq!(cnet[0].slot, Some(0));
    assert_eq!(cnet[0].sub_type, Some(32773));
    assert_eq!(cnet[0].ports.len(), 2);

    assert_eq!(cnet[0].ports[0].station_no, Some(5));
    assert_eq!(cnet[0].ports[0].mode, Some(0));
    assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
    assert_eq!(cnet[0].ports[0].bps, Some(8));
    assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
    assert_eq!(cnet[0].ports[0].data_bit, Some(1));
    assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
    assert_eq!(cnet[0].ports[0].stop_bit, Some(0));
    assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
    assert_eq!(cnet[0].ports[0].parity, Some(0));
    assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
    assert_eq!(cnet[0].ports[0].driver_type, Some(2));
    assert_eq!(cnet[0].ports[0].do_addr, Some(20));
    assert_eq!(cnet[0].ports[0].do_device, Some('P'));
    assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
    assert_eq!(cnet[0].ports[0].ai_addr, Some(20));
    assert_eq!(cnet[0].ports[0].ai_device, Some('P'));
    assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
    assert_eq!(cnet[0].ports[0].ao_addr, Some(30));
    assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));

    assert_eq!(cnet[0].ports[1].station_no, Some(15));
    assert_eq!(cnet[0].ports[1].mode, Some(2));
    assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
    assert_eq!(cnet[0].ports[1].do_addr, Some(400));
    assert_eq!(cnet[0].ports[1].do_device, Some('M'));
    assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
}

#[test]
fn decodes_xgb_enet02_hsc_position_and_pid_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");

    let hsc = doc
        .hsc_parameters()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("hsc payload decodes");
    assert_eq!(hsc.len(), 1);
    assert_eq!(hsc[0].payload_asc_length, Some(448));
    assert_eq!(
        hsc[0].channels[0].counter_mode,
        Some(HscCounterMode::RingCounter)
    );
    assert_eq!(
        hsc[0].channels[0].pulse_input_mode,
        Some(HscPulseInputMode::CwCcw)
    );
    assert_eq!(
        hsc[0].channels[0].compare_output_mode,
        Some(HscCompareOutputMode::GreaterThan)
    );
    assert_eq!(hsc[0].channels[0].ring_counter_max, Some(200));
    assert_eq!(hsc[0].channels[0].unit_time_ms, Some(5000));
    assert_eq!(hsc[0].channels[0].pulses_per_revolution, Some(4));

    let position = doc.position_parameters();
    assert_eq!(position.len(), 1);
    assert_eq!(position[0].axis_count, Some(2));
    assert_eq!(position[0].axes.len(), 2);
    assert_eq!(position[0].axes[0].axis_name, "X");
    assert_eq!(position[0].axes[1].axis_name, "Y");

    let cal = doc.pid_cal_parameters();
    let tune = doc.pid_tune_parameters();
    assert_eq!(cal.len(), 1);
    assert_eq!(tune.len(), 1);
    assert_eq!(cal[0].loops.len(), 16);
    assert_eq!(tune[0].loops.len(), 16);
    assert_eq!(cal[0].loops[0].forward_pwm, Some(32));
    assert_eq!(tune[0].loops[0].set_pwm_at_point, Some(32));
}

#[test]
fn decodes_xgb_enet02_fenet_ipv4_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
    let fenet = doc.fenet_config_infos();

    assert_eq!(fenet.len(), 1);
    assert_eq!(fenet[0].station_no, Some(1));
    assert_eq!(fenet[0].type_code, Some(23041));
    assert_eq!(fenet[0].base, Some(0));
    assert_eq!(fenet[0].slot, Some(1));
    assert_eq!(fenet[0].sub_type, Some(32771));
    assert_eq!(fenet[0].driver_type, Some(5));
    assert_eq!(fenet[0].rcv_wait_time, Some(20));
    assert_eq!(
        fenet[0].ip_address.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.100")
    );
    assert_eq!(
        fenet[0].subnet.as_ref().map(|ip| ip.address.as_str()),
        Some("255.255.255.0")
    );
    assert_eq!(
        fenet[0].gateway.as_ref().map(|ip| ip.address.as_str()),
        Some("192.168.0.1")
    );
}

#[test]
fn decodes_xgb_enet02_cnet_port_parameters() {
    let doc = XgwxDocument::from_path("fixtures/XGB_Enet02.xgwx").expect("fixture parses");
    let cnet = doc.cnet_config_infos();

    assert_eq!(cnet.len(), 1);
    assert_eq!(cnet[0].type_code, Some(23104));
    assert_eq!(cnet[0].sub_type, Some(32773));
    assert_eq!(cnet[0].ports.len(), 2);
    assert_eq!(cnet[0].ports[0].station_no, Some(5));
    assert_eq!(cnet[0].ports[0].mode_kind, Some(CnetMode::Rs232C));
    assert_eq!(cnet[0].ports[0].baud_rate, Some(9600));
    assert_eq!(cnet[0].ports[0].data_bits, Some(CnetDataBits::Eight));
    assert_eq!(cnet[0].ports[0].stop_bits, Some(CnetStopBits::One));
    assert_eq!(cnet[0].ports[0].parity_mode, Some(CnetParity::None));
    assert_eq!(cnet[0].ports[0].do_address.as_deref(), Some("P00014"));
    assert_eq!(cnet[0].ports[0].ai_address.as_deref(), Some("P00020"));
    assert_eq!(cnet[0].ports[0].ao_address.as_deref(), Some("P00030"));
    assert_eq!(cnet[0].ports[1].station_no, Some(15));
    assert_eq!(cnet[0].ports[1].mode_kind, Some(CnetMode::Rs485));
    assert_eq!(cnet[0].ports[1].do_address.as_deref(), Some("M00250"));
}

#[test]
#[ignore = "set LIBXGWX_FIXTURE=/path/to/file.xgwx to run against a real workspace"]
fn parses_real_fixture_from_env() {
    let Ok(path) = env::var("LIBXGWX_FIXTURE") else {
        return;
    };
    let doc = XgwxDocument::from_path(path).expect("fixture parses");

    assert_eq!(doc.root.name, "Project");
    assert!(!doc.xml.is_empty());
    let _ = doc.project_info();
    let _ = doc.configurations();
    let _ = doc.networks();
    let _ = doc.network_modules();
    let _ = doc.bases();
    let _ = doc.modules();
    let _ = doc.tasks();
    let _ = doc.programs();
    let _ = doc.variables();
    let _ = doc.ladder_programs();
    let _ = doc.hsc_parameters();
    let _ = doc.position_parameters();
    let _ = doc.pid_cal_parameters();
    let _ = doc.pid_tune_parameters();
    let _ = doc.cnet_config_infos();
    let _ = doc.fenet_config_infos();
    let _ = doc.decoded_payloads();
}

#[test]
fn formats_xgwx_variable_addresses() {
    assert_eq!(format_ipv4_le(352364736), "192.168.0.21");
    assert_eq!(format_ipv4_le(16820416), "192.168.0.1");
    assert_eq!(format_ipv4_le(16777215), "255.255.255.0");
    assert_eq!(
        format_variable_address(Some("M"), Some(14), Some("BIT"), None).as_deref(),
        Some("M0000E")
    );
    assert_eq!(
        format_variable_address(Some("P"), Some(14), Some("BIT"), None).as_deref(),
        Some("P0000E")
    );
    assert_eq!(
        format_variable_address(Some("N"), Some(14), Some("WORD"), None).as_deref(),
        Some("N00014")
    );
    assert_eq!(
        format_variable_address(Some("X"), Some(14), Some("BIT"), None).as_deref(),
        Some("X00014")
    );
    assert_eq!(
        format_variable_address(Some("Y"), Some(14), Some("BIT"), None).as_deref(),
        Some("Y00014")
    );
}

#[test]
fn rejects_non_xgwx_data() {
    let error = XgwxDocument::parse(b"not an xgwx").expect_err("invalid magic");
    assert!(matches!(error, XgwxError::InvalidMagic));
}

fn synthetic_xgwx_bytes(xml: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"XG");
    bytes.extend_from_slice(UTF16_MARKER);
    let label = "XG5000 WORKSPACE FILE".encode_utf16().collect::<Vec<_>>();
    bytes.push(u8::try_from(label.len()).expect("label fits in u8"));
    for unit in label {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.extend_from_slice(&1u32.to_le_bytes());

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).expect("gzip write");
    bytes.extend_from_slice(&encoder.finish().expect("gzip finish"));
    bytes
}

fn synthetic_ladder_data() -> Vec<u8> {
    let mut data = Vec::new();
    append_ff43_record(&mut data, 0x32, (0x58, 0x00), (0x01, 0x00));
    append_ff43_record(&mut data, 0x32, (0x58, 0x04), (0x03, 0x00));
    append_ff43_record(&mut data, 0x99, (0x44, 0x08), (0x06, 0x04));
    append_ff43_record(&mut data, 0x99, (0x44, 0x0c), (0x06, 0x08));
    append_unknown_positioned_record(&mut data, 0xff55, (0x20, 0x04));
    append_device_cell(&mut data, 0xff06, (0x01, 0x00), "M00001");
    append_device_cell(&mut data, 0xff07, (0x04, 0x00), "M00002");
    append_device_cell(&mut data, 0xff3e, (0x08, 0x00), "M00005");
    append_device_cell(&mut data, 0xff48, (0x0c, 0x00), "M00006");
    append_device_cell(&mut data, 0xff49, (0x10, 0x00), "M00007");
    append_wide_marker_device_cell(&mut data, 0xff06, (0x18, 0x00), "F0092");
    append_device_cell(&mut data, 0xff0e, (0x5e, 0x00), "M00003");
    append_ff02_horizontal_marker(&mut data, (0x08, 0x04));
    append_device_cell(&mut data, 0xff11, (0x5e, 0x04), "M00004");
    append_instruction_cell(&mut data, (0x58, 0x08), "MOV,D000001,D000002");
    data
}

fn assert_ladder_contact(program: &LadderProgramData, value: &str, expected: LadderContact) {
    let element = program
        .elements
        .iter()
        .find(|element| element.value == value)
        .unwrap_or_else(|| panic!("{value} element exists"));
    assert_eq!(element.contact, Some(expected), "{value} contact");
    assert_eq!(element.coil, None, "{value} is not a coil");
}

fn assert_ladder_coil(program: &LadderProgramData, value: &str, expected: LadderCoil) {
    let element = program
        .elements
        .iter()
        .find(|element| element.value == value)
        .unwrap_or_else(|| panic!("{value} element exists"));
    assert_eq!(element.coil, Some(expected), "{value} coil");
    assert_eq!(element.contact, None, "{value} is not a contact");
}

fn assert_ladder_instruction(program: &LadderProgramData, mnemonic: &str, operands: &[&str]) {
    let element = program
        .elements
        .iter()
        .find(|element| {
            element.kind == LadderElementKind::InstructionCall && element.value == mnemonic
        })
        .unwrap_or_else(|| panic!("{mnemonic} instruction exists"));
    assert_eq!(element.operands, operands);
    assert!(!program.elements.iter().any(|element| {
        element.kind == LadderElementKind::Comment
            && element.value.starts_with(&format!("{mnemonic},"))
    }));
}

fn assert_marker_only_ladder_contact(
    program: &LadderProgramData,
    raw_x: u8,
    raw_y: u8,
    expected: LadderContact,
) {
    let cell = program
        .structure
        .rungs
        .iter()
        .flat_map(|rung| &rung.cells)
        .find(|cell| cell.raw_x == raw_x && cell.raw_y == raw_y)
        .unwrap_or_else(|| panic!("marker-only cell exists at ({raw_x}, {raw_y})"));
    assert_eq!(cell.value, "");
    assert_eq!(cell.contact, Some(expected));
    assert_eq!(cell.coil, None);
}

fn append_ff43_record(data: &mut Vec<u8>, code: u8, target: (u8, u8), branch: (u8, u8)) {
    let start = data.len();
    data.resize(start + 48, 0);
    data[start] = 0xff;
    data[start + 1] = 0x43;
    data[start + 13] = code;
    data[start + 17] = target.0;
    data[start + 18] = target.1;
    data[start + 36] = branch.0;
    data[start + 37] = branch.1;
}

fn append_device_cell(data: &mut Vec<u8>, marker: u16, coord: (u8, u8), value: &str) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0, 0, 0, coord.0, coord.1, 0, 0, 1, 0, 0, 0, 0, 0,
    ]);
    append_ladder_string(data, value);
}

fn append_wide_marker_device_cell(data: &mut Vec<u8>, marker: u16, coord: (u8, u8), value: &str) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0, 0, 0, 0, 0, 0, 0, 0, coord.0, coord.1, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    append_ladder_string(data, value);
}

fn append_unknown_positioned_record(data: &mut Vec<u8>, marker: u16, coord: (u8, u8)) {
    let [marker_hi, marker_lo] = marker.to_be_bytes();
    data.extend_from_slice(&[
        marker_hi, marker_lo, 0xaa, 0xbb, 0xcc, coord.0, coord.1, 0xdd, 0xee, 0xff, 0, 1, 2, 3,
    ]);
}

fn append_ff02_horizontal_marker(data: &mut Vec<u8>, coord: (u8, u8)) {
    data.extend_from_slice(&[
        0xff, 0x02, 0xaa, 0xbb, 0xcc, coord.0, coord.1, 0xdd, 0xee, 0xff, 0, 1, 2, 3,
    ]);
}

fn append_instruction_cell(data: &mut Vec<u8>, coord: (u8, u8), value: &str) {
    data.extend_from_slice(&[coord.0, coord.1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    append_ladder_string(data, value);
}

fn append_ladder_string(data: &mut Vec<u8>, value: &str) {
    let units = value.encode_utf16().collect::<Vec<_>>();
    data.extend_from_slice(UTF16_MARKER);
    data.push(u8::try_from(units.len()).expect("string fits in u8"));
    for unit in units {
        data.extend_from_slice(&unit.to_le_bytes());
    }
}
