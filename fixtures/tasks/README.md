# Native task captures

Created in an isolated offline Windows 10 VM with XG5000 on 2026-10-17 from
controlled blank project baselines. They contain no user application data.

- `xgk-native.xgwx`: XGK-CPUSN; scan, 100 ms periodic, initialization, rising
  `M00000`, and `D000000 > 1234` tasks. Word task priority 7, task number 95.
- `xgi-native.xgwx`: XGI-CPUUN; scan, 25 ms periodic, `%MX0` off,
  `%MW0 == 65535`, and initialization tasks.

All task records use Version 257, Type 0 and Attribute 2. Kind distinguishes
scan (0), initialization (1), periodic (2) and internal-device (3) execution.
DeviceType distinguishes bit (0) and word (1). A periodic interval is stored in
Device as decimal milliseconds. Native dialog checks established the interval
range 1..4294967295 and word comparison range 0..65535. Priority choices are
2..7; periodic numbers are 0..31 and internal-device numbers are 64..95.

These captures establish serialization layout. Generated-file native acceptance
is recorded separately in [task API documentation](../../docs/tasks.md).
Local tests also create synthetic malformed and multi-PLC variants; those are
regression tests and are not native multi-PLC acceptance evidence.

## Generated and resaved acceptance pairs

`xgk-generated.xgwx` / `xgk-roundtrip.xgwx` and
`xgi-generated.xgwx` / `xgi-roundtrip.xgwx` come from the public `tasks` example.
Each has 13 task records and 12 nonempty LD programs. Native XG5000 4.82.1.0
Check Program reports 0 errors and 0 warnings for both. Separate Save As outputs
retain every task attribute, program identity/assignment/body, task-local program
order and workspace node count. XG5000 regroups the global program list by task.
The six fixture hashes are recorded in `SHA256SUMS`.
