# Creating scan programs

Generated fixtures append `AddedProgram` to native blank XGK LD or native XGI
SFC projects. Reproduce using `cargo run --features write --manifest-path dev-tools/Cargo.toml --bin program_create_acceptance`. Program and local-symbol GUIDs are fresh; the native
scan task is retained. Templates in `src/program_templates` come from the saved
blank projects in `fixtures/empty-projects` and `fixtures/sfc/new-xgi-sfc.xgwx`.

XG5000 4.82.1 opened and saved all three generated cases offline:

| Case | CPU | Native Save As | Check Program result |
| --- | --- | --- | --- |
| `xgk-ld` | XGK-CPUSN | `GKCRES` | Two E0000 diagnostics for the two empty LD programs; 0 warnings |
| `xgi-ld` | XGI-CPUE | `GICRES` | One E0000 diagnostic for the newly created empty LD program; 0 warnings |
| `xgi-sfc` | XGI-CPUE | `SFCCRES` | E0000 (empty) and E4001 (no initial step) for the new empty SFC main block; 0 warnings |

All-program checking enabled logic, syntax and duplicate-coil checks; XGI also
used strict type checking. Empty programs intentionally require instructions or
steps before checks pass. No PLC execution was tested.

Save As preserved both program names, kinds, task bindings, program identities,
editable SFC rows/ST sources, and declarations. Native workspace node counts are
15 for the XGK case (14 plus one new program node), and 21 for the XGI cases
(18 plus three new nodes). The writer uses those native increments; the final
XGK generated fixture carries the corrected native count. Roundtrip fixtures
contain the native saved output. Native Check Program/Save As can refresh compiled
caches and formatting, so binary file identity is not expected.

The native XGI Add Program dialog offers LD, SFC, ST and IL(IEC). It has no separate
FBD choice. IEC function blocks are used within LD; this creation API currently
supports LD and the captured SFC models.

## Deleting programs

The `*-delete-generated` fixtures delete `AddedProgram` from the corresponding
native creation roundtrips. Generate them with `cargo run --features write
--manifest-path dev-tools/Cargo.toml --bin program_delete_acceptance`. XG5000 4.82.1 resaved these as `DGKRES`,
`DGIRES` and `DSFCRES`, preserving the remaining program identities, code, SFC
rows and declarations. All-program logic/syntax checks (strict types for XGI)
reported 0 errors, 0 warnings and 19 messages for both XGI cases. XGK retained
only E0000 for the original empty LD program (1 error, 0 warnings, 9 messages).
Deleting the active/inactive or last program, undo reload, save/reload and creating
a replacement, and retaining unapplied ST drafts in surviving programs were checked in the rendered webview with a mocked VS Code
host. No PLC execution was tested.

## Reordering programs

`xgi-sfc-order-generated` moves `AddedProgram` ahead of `NewProgram` in the native
SFC creation roundtrip using `move_program(1, 0, ...)`. XG5000 4.82.1 displayed
that scan order and saved it as `OSFCRES` (`xgi-sfc-order-roundtrip`). Save As
preserved program identities, task bindings, declarations and editable SFC/ST
sources in the new order. All-program checking retained only the expected
E0000 and E4001 diagnostics for the empty added SFC program (2 errors, 0 warnings).
The rendered mocked VS Code host checked real mouse dragging before/after rows,
up/down/end/no-op moves, selection retention, save/reload, Undo reload and ST draft
retention through reorder and Undo across XGK LD, XGI LD and SFC. Rust and WASM
checks cover exact record preservation and stale source/destination rejection.
No PLC execution was tested; native reorder acceptance was run for SFC.
