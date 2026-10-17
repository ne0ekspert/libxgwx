# User-defined functions and function blocks

XGI projects support creating and editing native **ST** user-defined functions
and function blocks. These definitions live in `UserFunctions`, independently
of scan programs. Adding a definition does not change scan-program indices.

Use `document.user_pou().list()` to inspect definitions, and
`document.user_pou_mut().create(&NewUserPou { ... })` to create one. A creation
request supplies its name, kind, two unused GUIDs, ST body, comment, parameters
and locals. Functions also require a scalar `return_type`; their named return
variable is generated automatically. Function blocks have no return type and
are instantiated through an ordinary program declaration of the new FB type.

Select an existing definition using
`document.user_pou_mut().get(index, &snapshot.object_id)`. Its `set_source`,
`set_variables` and `delete` methods take the previously read `UserPou` snapshot
and reject stale edits atomically. Interface edits and deletion reject existing
calls or instances, and require inspectable ST callers. Source edits preserve interfaces and invalidate the native
compiler's generated `@Temp` cache. Run XG5000 Check Program after editing.

Parameters support input, output and in/out classes; both kinds support private
locals. Supported scalar types are BOOL, BYTE/WORD/DWORD/LWORD,
SINT/INT/DINT/LINT, USINT/UINT/UDINT/ULINT, REAL/LREAL, TIME, DATE,
TIME_OF_DAY and DATE_AND_TIME. EN/ENO are enabled. Names are ASCII IEC
identifiers, at most 32 characters. Source contains the ST body without
FUNCTION or VAR wrappers. At most 64 input and 64 output parameters and
128 declarations are written.

Initial scope excludes LD/IL/SFC definition bodies, other PLC families, nested FB
members, arrays, structures, strings, retained/initialized members and changing
the function return declaration after creation. Unknown layouts remain visible
and read only. Rust, WASM, C ABI, MCP and the VS Code UI expose the supported operations.
See [C API](c-api.md#user-defined-functions-and-function-blocks) for C transaction
shapes. MCP exposes inspection and guarded creation/deletion tools, with source
and interface edits through `get_ir(section:"user_pous")` → `apply_patch`.

The complete generator demonstrates definitions, a custom FB instance, local
scan variables and both kinds of calls:

```sh
cargo run --manifest-path dev-tools/Cargo.toml --bin user_pou_acceptance -- /tmp/user-pous.xgwx
```

The calling ST body is:

```iecst
Result := UserDouble(Value := 4);
Counter(Reset := FALSE, Increment := Result);
Observed := Counter.Total;
```

`Counter` is declared as `UserAccumulator`. Each FB instance owns its state.
This is native source generation, not an ST interpreter or PLC download API.

## Native acceptance, 2026-10-17

XG5000 **4.82.1.0** in an offline Windows 10 VM opened the complete generated
project, checked all programs with strict type checking, reported **0 errors
and 0 warnings**, and saved it through **Save As**. Re-parsing the saved file
retained identities, definitions, interfaces, source, calls and the custom FB
instance. The checked-in `fixtures/user-pous/generated.xgwx` and
`native-roundtrip.xgwx` are synthetic, self-contained reproductions.

A second native project tested every supported scalar input, output, in/out and result type
and a mixed-type FB with a private local. All 22 definitions passed Check
Program with **0 errors and 0 warnings**. Its native Save As is
`scalars-native-roundtrip.xgwx`. `scalars-generated.xgwx` uses the resulting
verified widths and allocation rules. Tests compare every definition's UF50/UB50
symbol bytes and Size against that native output. Allocation starts after the
reserved enable byte, aligns scalars to their bit width, and rounds definition
size to 64 bits. DATE occupies 16 bits and DATE_AND_TIME occupies 64 bits.

A third native check edited both compiled ST bodies while retaining their existing
caller and FB instance. Check Program again reported **0 errors and 0 warnings**;
`edited-generated.xgwx` and `edited-native-roundtrip.xgwx` retain that case. Tests
verify source changes invalidate the compiler cache and native recompilation
regenerates it while preserving the user declarations.

The extracted `native-definitions.xml` contains only controlled native
function/FB definitions, not the application project used to capture them.
The creation templates contain native blank ST definition records.

These checks establish native open, compile and save compatibility. They do
not establish PLC execution, download behavior or every combination of parameters.

Browser verification used the real rebuilt WASM bundle in a mocked VS Code host.
Playwright covered creation, interface/source application, custom FB type
selection, source undo/redo by document reload, and desktop/narrow layouts with
no console errors. The Browser plugin was not available. This does not replace
a native VS Code extension-host keyboard/save acceptance run.
