# Global variable text fixture

`global_text.bin` is the complete 11,160-byte decoded global Symbols table
from native XG5000 4.82.1.0 Save As of edits to the public `elements.xgwx`
fixture (XGK-CPUSN). It contains all 82 variable records, including longer
and shorter names, Korean text, a supplementary Unicode character in a
comment, an empty comment and a 255-UTF-16-unit comment.

Generated `VARGEN.xgwx` passed all-program logical, syntax and duplicate-coil
checks with 0 errors, 0 warnings and 13 messages. Native Save As to
`VARTEXTROUND.xgwx` preserved the entire Symbols table and the complete
ProgramData payload exactly, without normalization. Other variable records
and opaque trailing fields are included in the comparison.

Reproduction:

```sh
cargo run --features write --example variable_text_acceptance -- fixtures/elements.xgwx VARGEN.xgwx
# Open, Check Program, and Save As in XG5000.
cargo run --features write --example variable_text_acceptance -- --compare VARGEN.xgwx VARTEXTROUND.xgwx
cargo run --features write --example variable_text_acceptance -- --symbols VARTEXTROUND.xgwx global_text.bin
```

SHA-256: `e557fc9f0f6093d5e02a886c338c868b9c2be2283b7b5e2687509137a2468897`.

Local capture evidence is under
`VMs/xg5000-win10/captures/iec-full-edit-audit-20261004/variable-*`.
The native test used names up to 27 units; the binary writer's 255-unit name
limit is covered locally, not by a separate native name-length boundary test.
