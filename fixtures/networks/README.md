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

## Cnet serial settings

The `cnetbase-native`, `cnetascii-native`, `cnetrtu-native`, and
`cnetrep-native` XGWX fixtures were saved by XG5000 4.82.1.0 on 2026-10-06.
They extend the blank project with XGL-CH2A/B at Base 0 Slot 2; there is no
user application data. The first two FEnet records and empty ladder remain
present for preservation checks.

- Base: RS232C/RS422, 9600 baud, XGT server, station 0 on both ports.
- ASCII: port 1 uses Modbus ASCII, station 255, 300 baud, 7/2/Odd and permits
  parity errors; port 2 uses P2P, station 31, RS485, 64000 baud, response count
  37, delay count 17, inter-character count 23 and enabled termination.
- RTU: port 1 uses Modbus RTU, 600 baud and 8 data bits; port 2 uses Smart
  server and 115200 baud, preserving the other captured values.
- Repeater: both ports have Repeater 1 and Bps 14 (115200 baud). XG5000
  synchronizes port 1's baud selector to port 2 when enabling repeater mode.

Native selectors are driver 0=P2P, 2=XGT, 3=Modbus ASCII, 4=Modbus RTU,
7=Smart. Bps is an index into the native dropdown, not a rate multiplier:
0=300, 1=600, 8=9600, 12=64000, 14=115200. `CharTimeOut` stores the
inter-character count; `RequestDelayTime` stores the delay. The separate
`InterCharTimeOut` attribute remains untouched.

Ranges and physical interface descriptions were checked against the official
[LS Cnet manual](https://sol.ls-electric.com/uploads/document/17001955800930/XGL-C22B_T6_Manual_V3.3_202305_EN.pdf), chapter 4.
Native captures and writer/WASM tests are offline configuration checks; they do
not constitute live PLC communication tests.

`cnet-round-native.xgwx` is Native Save As of the Rust-generated ASCII/P2P
configuration plus new C22 and C42 modules at Slots 3 and 4. All three native
dialogs displayed the generated electrical modes and values. Native Save As
preserved every Cnet module/port attribute and the full ladder payload.
C22's native dialog does not offer repeater mode; its writer/UI guard rejects
that option. CH2 and C42 expose it. Empty-program compilation is outside this
serial-configuration acceptance check.
