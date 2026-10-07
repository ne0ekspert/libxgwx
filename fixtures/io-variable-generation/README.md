# Digital I/O variable generation acceptance

These synthetic XGK-CPUSN projects use variable point allocation and public
module templates. Base 0 has DI8 at slot 0, DO32 at slot 2, and DI16 at slot 10;
base 1 has DI64 at slot 0. The preview and generated table contain 120 BIT variables.

- `base.xgwx`: configured modules, no global symbols.
- `generated.xgwx`: library-generated symbols with native binary-data metadata.
- `resaved-native.xgwx`: opened and saved as IOVARCAP in XG5000 on 2026-10-07.

The native variable table displayed all 120 names, P addresses, BIT types, and
comments. Check Program reported one E0000 error for the template's empty
NewProgram, zero warnings, and nine messages. The decoded Symbols payload
(14964 bytes) and ProgramData payload (8 bytes) remained byte-identical after
native Save As. This validates serialization; it does not constitute a running
PLC hardware test.
