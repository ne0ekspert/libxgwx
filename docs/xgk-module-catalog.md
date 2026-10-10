# XGK Module Catalog

The library contains one native Rust catalog representing the latest stable
XG5000 module definitions. [`src/catalog_data.rs`](../src/catalog_data.rs) stores
the 90 selectable XGK modules, including their model, category, `Id`,
`SubType`, full display name, default `Details` payload, and verified option
encodings. Definitions also record physical slot span; for example, XGF-TC4UD
occupies two consecutive base slots. It also retains every captured visible option, including
nested per-file and per-file-data controls, so consumers can show unmapped fields as read only.
The library does not load module JSON files at build time or
runtime, and catalog entries do not carry an XG5000 version identifier.

Placement-specific base and slot values are intentionally omitted. Catalog
updates replace the Rust definitions as the supported latest-stable snapshot
rather than adding a parallel versioned catalog.

With the `write` feature, `xgk_module_catalog()` exposes this snapshot and
`XgwxDocument::select_module(base, slot, model)` atomically selects a catalog
model. Selection preserves `Base`, `Slot`, and `Comment`, while replacing
`Id`, `SubType`, `Name`, and `Details` with the captured XG5000 defaults:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-RD8A")?;
document.write_to("project-with-rd8a.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```

Selection validates physical placement. Multi-slot modules are rejected if
they extend past the base or overlap a module in a following slot.
`XgwxDocument::delete_module(base, slot)` removes one uniquely identified
module while retaining the surrounding base and unrelated workspace data.
`XgwxDocument::insert_module(base, slot, model)` adds a catalog-default module
to an empty physical slot and applies the same base-capacity and overlap checks.
Insertion, replacement, and deletion keep the companion network entries in
sync for captured XGK communication modules, including Cnet, FEnet,
EtherNet/IP, BACnet, FDEnet, Dnet, and Rnet. Captured XGPD defaults are also
written for `XGL-EDMT`, `XGL-EDMF`, `XGL-DMEA/B`, and `XGL-RMEA/B`.

Catalog entries expose every captured dialog row through `visible_options`.
The writable `options` subset contains only fields whose `Details` byte mapping
and numeric choices have been verified. Use
`module_option_values` to read the current selections and `set_module_option`
to update one module-wide, channel, or group value. Unknown keys, indices, and
values fail without mutating the document.

The verified high-speed-counter subset includes the dropdown settings for
`XGF-HD2A`, `XGF-HO2A`, and `XGF-HO8A`; their numeric counter, comparison, and
frequency fields remain read only until their range and encoding rules are
mapped separately.

For example:

```rust,no_run
use xgwx::XgwxDocument;

let mut document = XgwxDocument::from_path("project.xgwx")?;
document.select_module(0, 2, "XGF-AD8A")?;
document.set_module_option(0, 2, "inputRange", 5, 6)?;
document.write_to("project-with-options.xgwx")?;
# Ok::<(), xgwx::XgwxError>(())
```
