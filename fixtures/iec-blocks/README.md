# Native IEC LD block fixtures

Captured in XG5000 4.82.1.0 on 2026-10-17 using an offline snapshot VM.
All projects are synthetic and contain generic declarations only.

- `native-unassigned.xgwx`: native fresh insertions of R_TRIG, F_TRIG, TON,
  TOF, TP, RS, SR, CTU_DINT, CTD_DINT, CTUD_DINT, NE, MOD, AND, OR, XOR, NOT
  and WORD_TO_UDINT. The earlier synthetic FF networks are retained. Inputs
  deliberately remain empty; this is a shape capture, not a compiled suite.
- `native-ctud-wires.xgwx`: native F5 cells from CTUD_DINT.QU and QD.
- `native-logic-wire.xgwx`: adds an F5 cell from AND.OUT. These open-ended wire
  captures verify binary encoding, not complete executable circuits.
- `empty-with-instances.xgwx`: empty program with the native instance table,
  used for portable Rust, WASM, C ABI and MCP insertion tests.
- `generated.xgwx`: all 17 commands generated through `insert_function`, at
  alternating x4/x16 positions, with a single leading contact on alternating
  networks. Data inputs are populated, numeric outputs have destinations,
  and stateful data outputs are intentionally left unassigned.
- `native-roundtrip.xgwx`: native Open, strict all-program Check Program and
  Save As of `generated.xgwx`. Checking reports **0 errors, 0 warnings and
  16 messages**. The complete 7427-byte ProgramData and every decoded instance
  declaration, including allocation offsets, are unchanged.

`src/iec_block_templates/*.bin` contain only isolated normalized native network
prototypes at L0/x4, retaining hidden pin references and row heights. The
`iec_block_prototypes` dev tool extracts them; its source argument must be a
synthetic shape capture with one block per normal network. A unit test rebuilds
all 17 networks byte-for-byte. `iec_blocks_acceptance` regenerates the empty and
populated suites from the native declarations and an empty-program seed.

Native CTUD has direct BOOL QU/QD outputs. QD has no variable-reference ordinal;
CD/R/LD/PV and CV keep their own reference ordinals. RS/SR reserve their final
hidden reference for Q. The decoder distinguishes these from ordinary data
pins. Numeric CV and ET may remain unassigned; they cannot drive BOOL wires.

These checks cover editing, native compilation and Save As. No PLC download or
execution was performed.
