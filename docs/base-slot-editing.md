# Per-base slot counts

For XGK projects, `XgwxDocument::set_base_slot_count(base, slot_count)` and the
WASM `set_xgwx_base_slot_count(bytes, base, slot_count)` update the selected
`IO PARAMETER/BaseInfo/Base@SlotCount` value. XG5000 4.82.1.0 offers 4, 6, 8, 10
and 12 slots; these choices were read directly from its Base Settings dialog.

The VS Code Hardware toolbar exposes **Slot count** and **Apply slot count** for
each selected base. Changes use the custom document's existing edit/save/undo
flow. Each base is independent, and the slot table refreshes after applying.

The writer rejects absent or duplicate base records, missing SlotCount, unsupported
CPU families, CPU limits, unsupported counts, unknown module widths, and any
installed module extending beyond the requested size. This includes the second
slot of XGF-TC4UD. It never deletes modules to make room. Compact XGB base sizes
remain protected. Only the target attribute changes in the library output.

## Validation

- Rust: all five sizes on each of four bases, save/reparse, restoration, invalid
  counts, ambiguous/missing records, XGB protection, and two-slot shrink rejection.
- WASM: independent persisted counts, unchanged modules, invalid-count and
  two-slot rejection.
- Browser: actual extension JS/WASM with a mocked VS Code bridge; Base 0 4→6,
  Base 1 12→4, matching table row counts, visible Apply control and shrink-error
  feedback. Desktop 1440×1000 and narrow 900×800. Browser plugin unavailable;
  regular Playwright used. Screenshots and scripts are in `/tmp/xgwx-base-*`.
- Native offline XG5000 4.82.1.0 on Windows 10: the bundled WASM generated a file
  with Base 0/1/2/3 counts 8/6/10/4 from the committed `elements.xgwx` fixture.
  It opened successfully; separate Save As R81 retained all Base and Module
  attributes and byte-identical decompressed ProgramData. No PLC execution.

The user's modified `fixtures/elements.xgwx` was preserved. Full regression tests
used the committed fixture in `/tmp/xgwx-base-check/libxgwx` with current source
files overlaid. Test logs distinguish this baseline from the working fixture.
