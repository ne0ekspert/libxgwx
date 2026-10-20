# Legacy FEnet and I/O parameter acceptance

Generated from the library's blank XGK-CPUSN LD project, with a three-step
M00000 contact / M00001 output program and one XGI-D24A/B at Base 0 Slot 0.
These fixtures contain no supplied application project data.

On 2026-10-20, XG5000 4.82.1.0 (2026-07-20 build) displayed the generated
`FENET PARAMETER` / `Safety_Comm` settings and I/O input filter:

- IP 10.20.30.40, subnet 255.255.0.0, gateway 10.20.0.1.
- Receive wait 20 seconds; retransmission 200 counts of 10 ms.
- Dedicated connections 4; XGT server (DriverType 2); host table disabled.
- XGI-D24A/B input filter 5 ms (`Details` first byte 5).

`resaved.xgwx` is native Save As of `generated.xgwx`. All eight exposed FEnet
fields, module summaries, base records, channel mappings, host table and the
complete ladder ProgramData were retained. Native container metadata and
formatting can change, so byte identity is not expected.

`modbus-native.xgwx` is a second native Save As after selecting Modbus TCP/IP
server. It confirms DriverType 5, retaining the FEnet addresses/timeouts and I/O
filter. XG5000's offline program check reported zero errors and zero warnings.
No PLC connection or live communication was performed.

The native dialog specifies receive wait 2–255 seconds, retransmission 1–600
counts of 10 ms, and dedicated connections 1–4. It offers XGT and Modbus TCP/IP
server drivers. Host-table entries and channel mappings are retained; their
editing is not implemented by this change.

Reproduce generation and compare native persistence with:

```sh
cargo run --manifest-path dev-tools/Cargo.toml --bin fenet_io_acceptance -- generate /tmp/new-fenet-io-capture
cargo run --manifest-path dev-tools/Cargo.toml --bin fenet_io_acceptance -- check fixtures/fenet-io-parameters/generated.xgwx fixtures/fenet-io-parameters/resaved.xgwx
```
