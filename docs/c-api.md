# C API

Build the native interface with `ffi`. Add `write` for creation, guarded edits
and verified serialization, and `il` for ladder-to-IL conversion:

```sh
cargo build --release --lib --features ffi,write,il
```

The build produces a shared library (`libxgwx.so`, `libxgwx.dylib`, or
`xgwx.dll`) and a static library. Distribute the matching
[`include/xgwx.h`](../include/xgwx.h) with the library. The header is included
in the crates.io package; examples and fixtures remain repository-only.
The C API targets native platforms; use the existing `wasm` API in browsers.

## C example

The [C inspector](../examples/c/inspect.c) prints a JSON summary and optionally
renames a project into a new output file. On Linux:

```sh
cc -std=c11 -Wall -Wextra -Werror -Iinclude examples/c/inspect.c \
  -Ltarget/release -lxgwx -Wl,-rpath,"$PWD/target/release" -o /tmp/xgwx-inspect-c
/tmp/xgwx-inspect-c path/to/project.xgwx
/tmp/xgwx-inspect-c input.xgwx new-output.xgwx "Current name" "New name"
```

The output path must not already exist. The ABI itself performs no filesystem
I/O: callers supply bytes and choose how to save the returned container.

## ABI and ownership

- Check `xgwx_abi_version()` against `XGWX_ABI_VERSION`. `xgwx_features()` reports
  the writer and IL capability bits. All symbols remain available when a feature
  is disabled; unsupported calls return `XGWX_FEATURE_UNAVAILABLE`.
- Documents are opaque pointers. Initialize output slots to NULL. Inputs are
  copied or consumed during the call and may be released afterward. Free each
  document once with `xgwx_document_free`, then discard its pointer.
- Initialize `xgwx_buffer` to `{0}`. Returned bytes are **not NUL-terminated**.
  Release with `xgwx_buffer_free`, which resets the struct. Never use another
  allocator's free/realloc, modify the pointer or length, or copy its ownership.
- Text is UTF-8 with explicit byte lengths. NULL input pointers are accepted
  only for zero length. Foreign pointers must be properly aligned and valid for
  their declared access; arbitrary invalid addresses cannot be checked by Rust.
- Fallible calls return `xgwx_status`. Copy `xgwx_last_error()` immediately after
  failure. This borrowed NUL-terminated UTF-8 message is local to the thread and
  changes on its next fallible call or thread exit. Free/version/features calls
  leave it unchanged.
- Failed operations preserve documents and output slots. Unwinding panics
  return `XGWX_PANIC`; aborts and allocation failures cannot be caught. Build
  with the default unwind panic strategy for this behavior.
- Serialize all access to each handle, including reads and release. Separate
  handles can be used on separate threads. Output buffers are independent of
  their source handle and remain valid after that handle is freed.

For ctypes, P/Invoke or another FFI, map `size_t` to a pointer-sized unsigned
integer, status to signed 32-bit, documents to opaque pointers, and buffers to
pointer-plus-size structs. Declare exact signatures from the header and copy
text using its byte length before releasing it through the library.

## Operations

| Function | Behavior | Features |
| --- | --- | --- |
| `xgwx_document_parse` | Parse workspace bytes | `ffi` |
| `xgwx_document_inspect_json` | Inspect project, hardware, programs, networks and globals | `ffi` |
| `xgwx_document_xml` | Copy complete decompressed XML | `ffi` |
| `xgwx_document_list_json` | Load modules, networks, programs, globals, network modules or standalone sources | `ffi` |
| `xgwx_module_catalog_json` | Load selectable XGK models and verified options | `ffi,write` |
| `xgwx_document_module_options_json` | Read one installed module's option values | `ffi,write` |
| `xgwx_document_apply_edits_json` | Apply guarded list/metadata/source edits atomically | `ffi,write` |
| `xgwx_project_create` | Generate a supported blank project | `ffi,write` |
| `xgwx_document_rename_project` | Rename with an expected-current-name guard | `ffi,write` |
| `xgwx_document_select_cpu` | Perform guarded CPU selection | `ffi,write` |
| `xgwx_document_serialize` | Return a verified workspace container | `ffi,write` |
| `xgwx_document_program_il` | Convert a selected supported ladder program to IL | `ffi,il` |

JSON uses `schema_version: 1`, snake_case fields and the sections listed in the
header. A global-variable decoding error sets `variables` to null and
`variables_error` to the reason; other sections remain available. Program indices
follow inspection's program order. Use the XML export for information outside
this summary. The ABI currently exposes the operations above, rather than every
Rust writer or editable ladder IR. Existing CPU and layout guards still apply.

## Loading and editing lists

`xgwx_document_list_json` accepts the `XGWX_LIST_*` selectors in the header and
returns `{"schema_version":1,"items":[...]}`. Modules are identified by base
and slot. Networks, programs and globals use their current list order. Program
metadata includes `object_id`; standalone source items include source, language
and editability. Source-local variable summaries are read-only in this interface.
If globals cannot be decoded, the dedicated globals query returns an error;
the larger inspection snapshot still supports partial results.

Send edits as UTF-8 JSON to `xgwx_document_apply_edits_json`:

```json
{
  "schema_version": 1,
  "edits": [
    {
      "operation": "update_program",
      "program_index": 0,
      "expected_object_id": "copy the current object_id from the program list",
      "patch": {"comment": "Updated from C"}
    }
  ]
}
```

Unknown fields, invalid types, unsupported versions and unknown operations are
rejected. Patch fields are optional; omitted or null fields retain their values.
Every operation runs on a candidate document in order. All edits and final
verified serialization must succeed before the handle is updated. Writer errors
identify the failing zero-based edit index; schema and final verification errors
have their own messages. Every failure rolls back the entire batch.
Reload lists after success, especially after insertion, deletion or reordering.

| Operation | Required fields besides `operation` |
| --- | --- |
| `update_module` | `base`, `slot`, `expected_name`, `patch` |
| `select_module` | `base`, `slot`, `expected_name`, `model` |
| `insert_module` | `base`, `slot`, `model` |
| `delete_module` | `base`, `slot`, `expected_name` |
| `set_module_option` | `base`, `slot`, `expected_name`, `key`, `index`, `expected_value`, `value` |
| `set_module_input_filter` | `base`, `slot`, `expected_name`, `expected_value`, `value` (raw bytes) |
| `update_network` | `network_index`, `expected_name`, `patch` |
| `update_network_module` | `base`, `slot`, `expected_config_name`, `patch` |
| `update_program` | `program_index`, `expected_object_id`, `patch` |
| `create_program` | `name`, `language`, `object_id`, `symbol_id` |
| `delete_program` | `program_index`, `expected_object_id` |
| `move_program` | `from`, `to`, `expected_object_id`, `expected_target_id` |
| `update_variable` | `variable_index`, `expected_name`, `patch` |
| `edit_text_program` | `program_index`, `expected_object_id`, `expected_language`, `expected_source`, `source` |

Copy expectation fields from the current list. A module's `expected_name` is its
full native `name`, including the display caption; replacement/insertion `model`
comes from the module catalog's `model` field. Option expectations come from
`xgwx_document_module_options_json`. Newly created program identities must be
unique UUID strings in the library's supported format.

| Patch | Optional fields |
| --- | --- |
| Module | `id`, `sub_type`, `name`, `comment`, `details` |
| Network | `name`, `type_name`, `network_type` |
| Network module | `config_name`, `alias`, `description` |
| Program | `name`, `task`, `version`, `kind`, `instance_name`, `comment` |
| Global variable | `name`, `address_area`, `address_number`, `data_type`, `description` |

These requests delegate to existing Rust writers, preserving their restrictions.
Use catalog model selection instead of inventing module identities. Network
protocol identities and unsupported layouts remain guarded; linked network
records are maintained by supported hardware module changes. Arbitrary network
creation/deletion, global-variable insertion/deletion, local-variable mutation
and ladder/SFC body editing are not exposed by this JSON interface.

The [Python ctypes example](../examples/python/lists.py) loads all lists and can
apply a saved request into a new output file using only the Python standard library:

```sh
python3 examples/python/lists.py target/release/libxgwx.so project.xgwx
python3 examples/python/lists.py target/release/libxgwx.so project.xgwx edits.json new-output.xgwx
```

## Verification

CI compiles and runs an actual C caller to check malformed inputs, ownership,
Unicode, rejected edits, CPU conversion and byte-preserving round trips. It also
checks feature-disabled operations. On Linux:

```sh
cargo build --lib --features ffi,write,il
cc -std=c11 -Wall -Wextra -Werror -Iinclude tests/ffi_smoke.c \
  -Ltarget/debug -lxgwx -Wl,-rpath,"$PWD/target/debug" -o /tmp/xgwx-ffi-smoke
/tmp/xgwx-ffi-smoke
```

These checks validate the language boundary and existing library behavior.
Native XG5000 acceptance remains the documented Open/Check Program/Save As process.
