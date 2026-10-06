# Native FEnet edit acceptance

`fenet-edited-native.xgwx` was opened and saved with XG5000 on 2026-10-06.
It comes from the blank XGK-CPUSN project template, with two XGL-EFMT(B) modules
at Base 0 Slots 0 and 1. Slot 0 was edited with the Rust writer:

- IP: 10.20.30.40
- Subnet: 255.255.0.0
- Gateway: 10.20.0.1
- DNS: 1.1.1.1
- DHCP: enabled
- Alias: Main Ethernet

XG5000's FEnet dialog displayed those values. Save As retained all six edits,
Slot 1's defaults, and the blank NewProgram payload. This does not claim an
online PLC test or successful compilation of the intentionally empty program.
Secondary interface fields have local writer/WASM validation; this native
capture exercises primary interface fields.

## Common fields and timeout units

`fenet-modbus-native.xgwx` and `fenet-smart-native.xgwx` were edited and
saved in XG5000 on 2026-10-06, starting from the same blank project capture.
Both use StationNo 63 and GlofaSocketCnt 1 at Base 0 Slot 0.

- Modbus server: DriverType 5, ClientWaitTime 2, RcvWaitTime 255, both units 0 (seconds).
- Smart server: DriverType 7, ClientWaitTime 20, RcvWaitTime 30, both units 1 (10 ms).
- XGT server uses DriverType 2 in the original native fixture.

Native validation rejected station 64 with RAPIEnet disabled and timeout
counts outside 2–255. The dialog specifies dedicated connection count 1–16,
and reports that firmware V6.0 and later ignores this count and uses 64.
These are offline configuration captures, not online PLC communication tests.
