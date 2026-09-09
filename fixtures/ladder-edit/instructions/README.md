# Native instruction replacement captures

Offline XG5000 4.82.1.0 on Windows 10, 2026-09-09.
Start from `fixtures/elements.xgwx` unchanged, then use native instruction editing:

- R70.bin: replace `MOV,0,D000000` with `ADD,1,2,D000000` and Save As.
- R71.bin: from R70, replace ADD with `TON,T0000,100` and Save As.

These files contain decompressed ProgramData only. Native editing recalculates
all row display heights on the first edit; tests normalize only those unrelated
height bytes for the source-to-R70 comparison. R70-to-R71 is byte exact.
Generated replacement files were separately opened, checked (0 errors/warnings),
and resaved in XG5000 without ProgramData changes; see docs/ladder-editing.md.
