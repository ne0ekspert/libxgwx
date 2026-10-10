# Native text-program CPU captures

Captured in offline XG5000 4.82.1. Native blank projects preserve each CPU's hardware/network/default parameter profile. `xgk-auto` is XGK-CPUA in Auto-allocation mode. Compact profiles: XEC-E/H/S/U, XEM-H2/HP, GIPAM and KL; redundant profile: XGR-CPUH (type 101).

Generate candidates with `cargo run --features write,il --example text-cpu-expansion-acceptance`. Each source candidate declares local `Count : INT`, edits the original ST program, creates an additional ST program and (for IEC CPUs) an IL program. ST increments Count; IL loads, adds and stores Count. Native Check Program enables syntax checking, strict ST types and all programs. Native Save As pairs are compared by source, identity, language and declaration semantics, not compressed bytes or compiled caches.

XGK Auto ST passes 0 errors / 0 warnings / 17 messages. IEC CPU candidates pass 0 errors / 0 warnings / 21 messages. XGK vendor IL passes 0 errors / 0 warnings / 10 messages with series/negated/edge contacts, OUT/OUTP/SET/RST and MOV. Screenshots are in `evidence/`. Rendered editor screenshots use a local mock VS Code host; they do not represent PLC execution.

`scalars/*.bin` contains native local PB50 declaration captures for the XGK scalar type menu. Native BIT is presented as BOOL; XGK INT=9 and REAL=7, unlike the IEC mapping. These are declaration captures, not compilable source acceptance programs. Native Check Program assigns D allocation storage, retained in saved files and normalized only for shared symbol decoding. Unsupported NIBBLE, STRING, timer/counter, array and instance shapes remain guarded by the XGK declaration writer.

XGK classic IL writes native ladder records (`Kind=0`, `ProjectType=1`), never IEC CodeList. Branch/comment programs and Auto-allocation ladder IL remain read only in the textual projection. CPUS/P is guarded because it was absent from the native XGI CPU chooser; no synthetic file is treated as its acceptance evidence.
