use xgwx::{SfcSequencePatch, SfcVariablePatch, XgwxDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = XgwxDocument::from_path("fixtures/sfc/native-loop.xgwx")?;
    for (name, ty) in [
        ("Count", "DINT"),
        ("Delay", "TON"),
        ("TimerOff", "TOF"),
        ("Pulse", "TP"),
        ("CounterUp", "CTU_DINT"),
        ("CounterDown", "CTD_DINT"),
        ("CounterBoth", "CTUD_DINT"),
        ("Rise", "R_TRIG"),
        ("Fall", "F_TRIG"),
        ("ResetLatch", "RS"),
        ("SetLatch", "SR"),
    ] {
        doc.edit_sfc_variable(&SfcVariablePatch {
            program_index: 0,
            expected_variables: doc.sfc_variables(0)?,
            name: name.into(),
            data_type: ty.into(),
            description: "".into(),
            remove: false,
        })?;
    }
    for ty in [
        "BOOL",
        "BYTE",
        "WORD",
        "DWORD",
        "LWORD",
        "SINT",
        "INT",
        "LINT",
        "USINT",
        "UINT",
        "UDINT",
        "ULINT",
        "REAL",
        "LREAL",
        "TIME",
        "DATE",
        "TIME_OF_DAY",
        "DATE_AND_TIME",
    ] {
        doc.edit_sfc_variable(&SfcVariablePatch {
            program_index: 0,
            expected_variables: doc.sfc_variables(0)?,
            name: format!("Value_{ty}"),
            data_type: ty.into(),
            description: "".into(),
            remove: false,
        })?;
    }
    let block = doc.sfc_programs().remove(0).blocks.remove(0);
    let mut rows = block.editable_rows.clone().unwrap();
    rows[1].action = Some("UpdateValues".into());
    rows[1].action_code = Some("Count := ADD(Count, 1);\r\nValue_WORD := %MW100;\r\nValue_DWORD := DWORD#123;\r\nValue_LWORD := LWORD#123;\r\nDelay(IN := TRUE, PT := T#2s);\r\nTimerOff(IN := TRUE, PT := T#2s);\r\nPulse(IN := TRUE, PT := T#2s);\r\nCounterUp(CU := TRUE, R := FALSE, PV := 10);\r\nCounterDown(CD := TRUE, LD := FALSE, PV := 10);\r\nCounterBoth(CU := TRUE, CD := FALSE, R := FALSE, LD := FALSE, PV := 10);\r\nRise(CLK := TRUE);\r\nFall(CLK := TRUE);\r\nResetLatch(S := TRUE, R_1 := FALSE);\r\nSetLatch(S_1 := TRUE, R := FALSE);".into());
    rows[3].action = Some("Value_BOOL".into());
    rows[4].title = "Value_BOOL".into();
    rows[2].title = "Ready".into();
    rows[2].transition_code = Some("TRANS := Count >= 10 AND Delay.Q;".into());
    doc.replace_sfc_sequence(&SfcSequencePatch {
        program_index: 0,
        block_index: 0,
        expected_entities: block.entities,
        expected_rows: block.editable_rows,
        rows,
    })?;
    std::fs::write(
        std::env::args()
            .nth(1)
            .unwrap_or("/tmp/SFCSTGEN.xgwx".into()),
        doc.to_verified_bytes()?,
    )?;
    Ok(())
}
