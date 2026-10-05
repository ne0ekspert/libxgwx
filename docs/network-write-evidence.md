# Network protocol write evidence and capture requirements

Reviewed 2026-10-05. This is an evidence inventory, not native acceptance of new writes.
No device/server connection, OS network change, deployment, or new protocol writer was performed.

## Current conclusion

The parser establishes where several protocol values are stored and how they are read. It does not establish each module/CPU's valid values, coupled updates or native acceptance of changing those values. Existing communication profiles support creating captured defaults; they do not validate arbitrary edits to an existing configuration. The current browser therefore edits existing network metadata only. Network names/descriptions are not a complete communication-settings editor.

| Scope | Confirmed storage / read interpretation | Missing evidence before enabling writes |
| --- | --- | --- |
| FEnet IPv4 | `XGPD_CONFIG_INFO_FENET`; decimal XML octets `IpAddr_0..3`, `Subnet_0..3`, `Gateway_0..3`, `Dns_0..3`; second interface uses `IpAddr2_0..3` etc. `attr_u8` constrains decoded octets to 0..255. No binary offset is needed for these XML attributes. | One-field native before/after captures, module-specific IP/subnet/gateway/DNS validity, DHCP/manual coupling, second-interface availability and any companion-field updates. |
| FEnet modes/timeouts | Decimal XML `Dhcp`, `DriverType`, `RcvWaitTime`, `ClientWaitTime`, `GlofaSocketCnt`. Reader accepts u32; observed driver values do not constitute an enum. | Native allowed modes, numeric min/max/units, model support and dependencies. |
| FEnet TCP/UDP ports / services | The XGL-EFMT(B) default profile contains `ServerPortEnable`, `ServerPortIndividualType_n`, `ServerPortIndividualStartPortNo_n`, `ServerPortIndividualPortCount_n` for n=0..7. These fields are not exposed as a typed editable service table by the current FEnet WASM summary. | Non-default capture for each enable/type/start/count; protocol enum, active-row/count rules, legal port/range/conflict rules, and native Save As persistence. Default zero rows cannot prove their editing semantics. |
| Cnet serial | `XGPD_CONFIG_INFO_CNET` / child `XGPD_CONFIG_INFO_CNET_PORT`. Decimal XML `Mode`: 0 RS232C, 1 RS422, 2 RS485; `DataBit`: 0=7, 1=8; `StopBit`: 0=1, 1=2; `Parity`: 0 none, 1 even, 2 odd. `Bps` is read as raw value times 1200 (fixture raw8=9600). | Native per-port supported electrical modes and their effects on other ports; full permitted baud-rate list; valid data/stop/parity combinations. Reader enums alone are not per-model write acceptance. |
| Cnet station / I/O / timeouts | Decimal XML `StationNo`, `StationNoB`, `RxTimeOut`, `CharTimeOut`, `InterCharTimeOut`, `DriverType`; DI/DO/AI/AO device type uses numeric ASCII code, with separate DataType/Size/Addr attributes. | Station/address/size bounds, device-area/type compatibility, driver modes, timeout units/limits and coupled changes. |
| FDEnet / Dnet / Rnet / other communication models | Generic XGPD XML attributes and captured creation defaults exist. Some communication catalog rows are visible defaults only, with no verified writable option encoding. | Separate native captures for the exact module, subtype, CPU family and protocol. Do not reuse FEnet/Cnet layouts or infer offsets in opaque Details payloads. |

## Existing evidence and limits

- `src/internal/parameters.rs::ipv4_attrs` reads the four XML octets. `src/model.rs::FenetConfigInfoSummary` and `CnetPortConfigSummary` define the read mappings; `src/wasm/network.rs` exports the decoded fields.
- `src/tests.rs::decodes_xgb_enet01_fenet_ipv4_parameters`, `decodes_xgb_enet02_fenet_ipv4_parameters`, and the matching Cnet tests inspect developer-only supplied samples. FEnet Type23041/SubType32771 and built-in Cnet Type23104/SubType32773 are observed. These tests assert decoded values; they neither mutate those fields nor demonstrate XG5000 acceptance of mutations. Enet01/02 are not documented as a controlled one-field before/after pair.
- `src/writer.rs::network_profile` has captured defaults for XGL-EFMT(B), XGL-EDMT/EDMF, XGL-DMEA/B and XGL-RMEA/B. Existing network update APIs change metadata, not protocol settings.
- `docs/cpu-hardware-validation.md` records offline XGB-XBMS / XBM-DR16S with built-in Cnet and XBL-EMTA expansion Ethernet. Native evidence covers unchanged baseline, I/O comment and XGK CPU change. It explicitly requires more captures for compact built-in settings writers. It does not validate IP/port/serial edits.
- `vscode-xgwx/media/main.js::appendNetworkConfigurationFields` displays FEnet/Cnet parameters read-only; the editable network forms call metadata APIs. No additional protocol-write proof was found there. Source was read for capability assessment, not copied or translated.
- NetworkModule Id and configuration Type provide the documented linkage. Base/slot may be renumbered by XG5000. A future writer must also reject ambiguous same-type records and use a captured, stable per-record identity.

## Minimal offline capture plan

Keep supplied Good People samples private. Public tests should use independently constructed data.

1. Record XG5000 version, CPU type, module model/Id/SubType, port/interface and NetworkModule/configuration identity. Save an unchanged control copy first to identify native normalization.
2. For XGB-XBMS + XBL-EMTA (Type23041/SubType32771), capture separate native saved copies changing only IP, subnet, gateway and DNS. Also capture DHCP/manual transitions and each available second-interface setting. For XGK + XGL-EFMT(B), repeat independently; the matching type code does not prove identical CPU semantics.
3. For built-in Cnet Type23104/SubType32773, capture each serial port separately: each native allowed Mode, each available baud rate, DataBit, StopBit and Parity. Capture station, driver, timeout and I/O mappings individually with boundary values and invalid-value rejection. Repeat for each expansion Cnet model before enabling that model.
4. For FEnet server-port/service rows, capture one non-default active row, then each protocol type, start port and count, a second active row and removal/disable. Record native range/conflict errors. Capture any additional payload or companion attributes changed by Save As.
5. Diff complete decompressed XML and every binary payload against both original and unchanged native control. Establish the precise permitted delta and explicitly account for native normalization; unknown additional changes block support.
6. Construct the bounded writer only after that mapping is established. Verify negative cases (missing/duplicate target, unknown model/subtype, stale expected value, invalid bounds/enum and unsupported field), atomic failure, exact non-target XML/payload/header/trailer preservation, and library reparse.
7. Open the writer-produced copy in native XG5000, inspect the configuration dialog, run offline validation where available, and Save As. Confirm intended values persist and compare non-target data with the control. Check Program alone does not prove communication parameter validity. No PLC connection is needed or authorized.
