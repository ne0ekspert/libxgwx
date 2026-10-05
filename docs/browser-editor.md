# Local browser workspace editor

Open editor.html and choose a workspace. The PLC editor opens directly.
Apply changes and review them in the ladder and properties panels. Download edited copy emits a separate
`*-edited.xgwx`; the uploaded source is never written or transmitted.

Supported edits:

- Existing base slot count (4/6/8/10/12) on XGK, with occupied module span checks.
- Existing module Comment XML attribute on supported writable containers.
- XGK catalog-matched input filter and module/channel options, using the existing verified writer mappings.
- Existing program name and Comment attribute (description).
- Existing IEC LD VER 1.1 addressed contact/coil kind (six kinds each).
- Existing IEC contact/coil operand, selected from declared primitive local
  BOOL symbols; coil destinations exclude input-only declarations.
- Existing linked scalar function operand, selected from declared primitive
  locals matching the decoded pin mask; arrays and input-only output targets
  are excluded. The writer also checks compatible peer operands.
- Existing decoded global variable description, retaining its exact UTF-16
  code-unit length. Binary offsets remain stable.

Missing fields are disabled. Tasks, IDs, XGK ladder edits, new IEC placements,
function/instance changes, free expressions, variable names/types/addresses,
CPU, module model/ID/location, module insertion/deletion, networks, parameter settings and local variable declarations
are available as read-only project tree details. Their edits need validated
controls and are not offered. Generalized
conversion placement is not offered. No raw offset or payload editor is exposed.
Unmapped hardware options and structural changes on XGI/XGB remain disabled. These need additional format-specific editing controls and validation. A file
may be readable but not writable: the existing Rust writer rejects unsupported
header/alignment/security layouts; the UI displays that reason before editing.

Apply is transactional: the Rust writer serializes and reparses the full XML,
checks the parsed tree, untouched header bytes and trailing security data;
checksum, compressed length and alignment padding may change. The bounded hardware API applies the existing writer to a clone, validates the expected old value, and checks that only one selected XML attribute value changed. The bounded IEC
API additionally checks the exact permitted payload delta: either one kind byte
or one length-prefixed UTF-16 operand string. It compares the complete XML
outside the chosen ProgramData text byte for byte. Existing writer topology
and typed pin-binding guards still run before serialization. The browser also
compares the entire summary to the intended patch, including compatibility
aliases, and verifies the exact bytes again before every download. No native
XG5000 open/save or PLC execution claim is made by these format checks.

Unapplied text blocks download and is retained when another form is applied.
Undo/Redo restore exact snapshots (toolbar or Ctrl/Cmd+Z and Shift+Z).
Reopen the original file to discard all changes after confirmation.
Replacing a dirty workspace and leaving the page ask before discarding changes.
Invalid uploads preserve the current workspace. Concurrent reads cannot replace
new edits without confirmation. Reloading ends this memory-only session.

The project workspace is inspired by XG5000's dock layout. There are no top
view tabs or raw analysis panels. Desktop puts project/program navigation on the left,
a wide ladder canvas in the center and record/workspace properties on the right.
The IEC canvas uses decoded source coordinates and circuit wire edges; it shows
supported contact/coil records. Function operands have a separate decoded pin
panel, not a native function-block wiring simulation. Other decoded programs
retain the existing read-only ladder renderer. Clicking a canvas record or
pressing Enter opens its properties. Program switching retains metadata drafts.
Mobile initially collapses Project, allows horizontal scrolling inside the
ladder canvas and places Properties below it. Editable metadata is in the Program / variable properties disclosure. Raw
bytes, offsets, XML, container metrics and change logs are not shown. The underlying edit API and format guards are
unchanged by this layout. Arrow keys
navigate records; Enter opens the control; Escape cancels a draft. Mobile uses
the same visible controls. Each inspector captures workspace revision, kind,
value and offset; stale file/record selections are rejected. Changing file
while an asynchronous read is in progress cannot silently discard a new draft.

This UI is independently implemented from public libxgwx APIs and format data;
no vscode-xgwx GPL source is copied or translated.

Build as GitHub Pages does:

```sh
wasm-pack build --target web --out-dir web/dist/pkg --out-name libxgwx --features wasm,write --no-default-features
```

The static assets include editing.js, ladder-edit.js, project-settings.js hardware-edit.js, network-edit.js and global-variables.js. Keep all `.xgwx` fixtures out of web/dist.
Development fixtures are used only by local tests and are not demo content.

The project tree is derived only from decoded file summary entries: CPU,
explicit bases/modules, parameter types/sections, networks and communication
module configurations, decoded HSC/position/PID/safety settings, global variables
and program-local declarations. Empty categories and the static system-variable
catalog are not represented as declarations in the file. Unknown or binary-only
parameter sections show their presence and decoding limitation without raw data.
Settings navigation retains pending forms. Global table description actions link to the
same verified editor; type/address and local declaration changes stay read-only.
Program selection synchronizes program properties when there is no metadata draft.
The vscode-xgwx tree was consulted for available categories only; its GPL code
was not copied or translated.

Each Hardware Base is a single project-tree item. Selecting it replaces the central ladder canvas with physical slots and module settings; selecting a program restores its ladder. Empty positions are selectable but insertion is not offered. Unknown module spans are marked as unverified occupancy. Pending hardware edits survive program navigation; changing base, slot or field asks before discarding them. Hardware forms capture the workspace revision and reject stale applications. Undo/Redo, upload replacement and download use the same verified snapshot flow as ladder edits. Hardware edits have not been opened or checked in native XG5000 or executed on a PLC.

The hardware rack is one horizontal row on screens wider than 700px and one vertical column on narrower screens. Base size controls stay at the top of the central view. The complete declared slot order includes empty and reserved positions. Supported XGK sizes are 4, 6, 8, 10 and 12; shrinking across occupied module spans is rejected by the existing writer. Unsupported CPU or missing size fields show the write rejection reason.

Network / Communication opens a central file-settings view. Existing Network Name and NetworkModule ConfigName/Alias/Description are editable through existing writer APIs. Type/NetworkType, IP/port/station/address/mode, Cnet/FEnet/XGPD protocol settings and unknown services are read-only because their validated writers or allowed values are not available. Linked decoded configuration records are matched by NetworkModule Id to configuration Type, as documented by the parser; base/slot is not treated as stable linkage. No socket, device or server connection is made. The bounded browser API checks the old attribute value, unique module identity and parent network, and exact preservation of XML outside the one selected attribute. Unapplied network drafts survive program/Base navigation, while switching network selections asks before discarding. Shared snapshot Undo/Redo and download verification apply. Native XG5000 and PLC execution remain untested.

Global Variables is one selectable project-tree item, including when no globals are decoded. It opens a semantic central table of actual names, types, addresses and descriptions. Initial values are not exposed by the current WASM parser and are explicitly marked as undecoded. The table has bounded vertical scrolling for large lists and internal horizontal scrolling on mobile. Description cells are clickable and keyboard-focusable in writable workspaces; Enter or Space opens their editor. There is no action/button column. Read-only cells remain plain table values. Description actions retain the table and open existing variable properties. UTF-16 length validation and the existing variable writer remain unchanged. Arrow Up/Down and Home/End navigate available editable description cells; Enter opens the description. Undo/Redo redraw the table, and pending variable drafts survive Base/Network/program navigation.
