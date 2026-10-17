# XG5000 task definitions

`doc.task()` inspects native task records and their PLC-scoped program membership.
`doc.task_mut()` creates and edits tasks with the `write` feature. The existing
`doc.program().tasks()` reader remains available.

```rust
use xgwx::{create_project, TaskSettings, TaskTrigger};
let mut doc = create_project("XGK-CPUSN", "LD")?;
doc.task_mut().create(0, &TaskSettings {
    name: "ControlLoop".into(),
    priority: 2,
    task_number: 0,
    trigger: TaskTrigger::Periodic { interval_ms: 100 },
})?;
let expected = doc.task().get(1)?.snapshot()?;
doc.task_mut().get_if(1, &expected)?.rename("FastLoop")?;
let bytes = doc.to_verified_bytes()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The document index selects a task record; `task_number` is the native PLC task
number and has a different meaning. Lower priority numbers execute first.
Task-list order does not change scheduling. Programs assigned to a task retain
program-list order; use the existing program move operation to change that order.

## Supported scheduling

The scheduling writer supports the standard XGK and XGI CPU types listed by
`doc.task().can_create(configuration_index)`. Native capture and acceptance
use XGK-CPUSN and XGI-CPUUN. Other models in those standard families share the
layout but have not each undergone native acceptance in this run.

| Trigger | Native Kind | Task number | Settings |
| --- | --- | --- | --- |
| Built-in scan | 0 | Native default | Read and assign programs; cannot create, edit, rename or delete |
| Initialization | 1 | Stored but inactive, 0..95 | At most one per PLC |
| Periodic | 2 | 0..31 | Interval 1..4294967295 milliseconds |
| Internal bit device | 3, DeviceType 0 | 64..95 | Rising, falling, changed, on, off |
| Internal word device | 3, DeviceType 1 | 64..95 | Less, less-or-equal, equal, greater-or-equal, greater; value 0..65535 |

User priorities are 2..7. User-task names are limited by the writer to 32 UTF-16
units of letters, digits and underscores and must be unique within a PLC,
ignoring case. Native task numbers must be unused in their trigger range.

XGK task records use native device expressions such as `M00000` and `D000000`.
XGI task records use canonical direct addresses such as `%MX0` and `%MW0`.
Use an X address for a bit trigger and W for a word trigger. The writer checks
address syntax, not every CPU's memory limits or physical-device availability.
Native Check Program and device-specific verification remain necessary.

External-contact, high-speed-counter, positioning, compact XGB, redundant XGR,
and unknown scheduling layouts are not enabled for creation or scheduling edits.
Their raw task attributes remain readable; an unknown task Kind cannot be renamed
or deleted through the guarded editor. These need additional native captures.

## Membership and transactions

`task.programs()` returns selected programs only from the task's own PLC.
Duplicate task names within a PLC are ambiguous and reject membership-dependent
edits. The same task name in different PLCs is supported.

- `task.update(&settings)` replaces supported scheduling fields and renames all
  attached program references atomically. Unknown attributes and program bodies
  are preserved.
- `task.rename(name)` changes only the name and attached `Program Task` references.
- `task.assign_program(index, expected_object_id, expected_task)` checks the
  program identity, previous assignment and PLC ownership.
- `task.create_program(&NewProgram)` creates a native empty program directly in
  that task. It retains the existing program creator's single-PLC limitation.
- `task.delete()` rejects the scan task and populated tasks. Reassign or explicitly
  delete programs first. It never implicitly deletes application code.

Creation and deletion maintain `WksNodeCount`. A captured `TaskSnapshot` includes
the owning configuration GUID and every native task attribute, so stale changes
and tasks from another PLC cannot silently match by name. Missing or duplicate
configuration GUIDs reject selected editing. Read-only summaries remain available.

`ProgramPatch.task` now requires an existing unambiguous task in the same PLC.
It no longer accepts an arbitrary dangling name. Assignment can temporarily leave
a PLC without a scan program while constructing a larger edit; XG5000 Check
Program requires at least one scan program in the completed project.

## C and WASM

C inspection section `tasks` returns paginated definitions, `expected` snapshots,
decoded settings, configuration indices and program indices. Pass that exact
`expected` object to `update_task`, `rename_task`, `delete_task`,
`assign_task_program`, or `create_task_program` in the normal atomic JSON edit
transaction. `create_task` takes `configuration_index` and `settings`.

Settings use strict snake_case in both transports:

```json
{"name":"ControlLoop","priority":2,"task_number":0,
 "trigger":{"kind":"periodic","interval_ms":100}}
```

Bit settings use `{"kind":"internal_bit","device":"%MX0","condition":"rising"}`;
word settings use `{"kind":"internal_word","device":"%MW0","condition":"greater_or_equal","value":100}`.
Initialization uses `{"kind":"initialization"}`. Missing required fields,
unknown fields, invalid enums, ranges and stale snapshots reject the edit batch.

WASM `parse_xgwx` exposes a `tasks` array with camelCase wrapper fields and the
same native definition, `expected` snapshot and snake_case settings. Independent
exports are `create_xgwx_task`, `update_xgwx_task`, `rename_xgwx_task`,
`delete_xgwx_task`, `assign_xgwx_task_program`, and `create_xgwx_task_program`.
They preserve the existing WASM password session and verified serialization path.

Run the complete Rust example from the repository:

```sh
cargo run --features write --example tasks -- /tmp/new-task-examples
```

It creates new XGK/XGI workspaces, retains a scan program, assigns and renames a
periodic program, fills programs with memory-only rungs, creates every supported internal-device condition, and deletes
an empty temporary task. It refuses to overwrite an existing output directory.
See [native fixtures and evidence](../fixtures/tasks/README.md).

## Native acceptance, 2026-10-17

XG5000 **4.82.1.0**, build **2026-07-20**, was used in an isolated offline
Windows 10 VM with temporary copy-on-write disks. Both final examples contain
real memory-only ladder rungs and a retained scan program.

| CPU | Tasks / programs | Check Program | Save As |
| --- | --- | --- | --- |
| XGK-CPUSN | 13 / 12 | 0 errors, 0 warnings | All task fields, program identities, assignments, bodies and task-local order retained; WksNodeCount 37 |
| XGI-CPUUN | 13 / 12 | 0 errors, 0 warnings | All task fields, program identities, assignments, bodies and task-local order retained; WksNodeCount 64 |

Both examples cover initialization, periodic scheduling at priority 7 and 250 ms,
all five bit conditions, all five word comparisons at value 65535, task rename,
program assignment, creation directly in a task and empty-task deletion.
XG5000 regroups the global Program list by task on Save As; comparisons therefore
use ObjectId and task-local execution order instead of global document indices.
Reinspect after opening a native resave.

The checked-in `*-generated.xgwx` and `*-roundtrip.xgwx` pairs are checked by
`native_check_and_save_as_keep_all_task_fields_and_program_membership`.
Screenshots, About version and run logs are archived locally under the ignored
`target/task-acceptance-20261017/` directory. Fixture hashes are in
`fixtures/tasks/SHA256SUMS`.

Earlier example iterations exposed native project requirements: no scan program
produced O0002, and empty bodies produced E0000. The final fixtures correct both;
those intermediate failures are not attributed to task serialization.
This acceptance covers native editing, checking and persistence; PLC execution
and physical interrupt behavior were not tested.

The WASM runtime regression can be run against a fresh wasm-bindgen Node build:

```sh
node tests/wasm_tasks.cjs /absolute/path/to/pkg/libxgwx.js
```
