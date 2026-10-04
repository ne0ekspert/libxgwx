# Native branch fixtures

Captured in offline XG5000 4.82.1.0 on Windows 10, 2026-09-09,
using the parent directory's XGK-CPUUN linear fixture (R17).
These are complete decoded ProgramData payloads.

| File | Native operation |
| --- | --- |
| R40.bin | R17: F6 adds vertical connection after column 1 to the END row |
| R41.bin | R40: Ctrl+L before the END row stretches the connection |
| R42.bin | R17: Ctrl+L before END creates an implicit blank row |
| R43.bin | R42: F6 connects the first row to the blank row after column 1 |
| R44.bin | R43: insert NO M00002 in the lower row, column 0 |

R44 passes native Check Program with zero errors and warnings.
See [acceptance details](../../../docs/ladder-editing.md).

## SHA-256

- R40.bin: 9112ddf0011b3958a0e0d95a98b057e864dff6889b2e60e26f5d96f1c3504061
- R41.bin: 83de0fb358cb70daa86c79310d95b70c44a3eef214c798809ee383d11f7b43dc
- R42.bin: 2524069237f8bea95aff940d74df8be9984dc4043c6c0ce1bf34ff73e15d116d
- R43.bin: 43192ec6556ddfb5e6108c5142315712abea69c2e9e546f0aa0a6004f1c21216
- R44.bin: 08785540ec654ac1614c2d03c2e7ce890c1272e2a16c790b68f7e575f55633b7

## Outputs on lower branch rows

`branch_outputs.bin` is a complete 2,531-byte ProgramData payload from native
Save As of a WASM-generated private copy of `fixtures/elements.xgwx`, using
XGK-CPUSN and XG5000 4.82.1.0 on 2026-10-04. OUT M00030 was added to raw row
12 and MOV 1 D100 to raw row 16. Both trailing wires start at coordinate 7,
immediately after the retained boundary at coordinate 6.

Native all-program logical, syntax and duplicate-coil checks completed with
0 errors, 0 warnings and 13 messages. All 2,531 program bytes and the one local
symbol table match the generated source exactly, without normalization. The
writer regression reconstructs the entire capture from `elements.xgwx`.

SHA-256: `8b6261c838949e8b226206f52acc5877fc2d50764ed8bf904979036dc28828db`.
