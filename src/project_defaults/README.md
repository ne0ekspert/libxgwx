# Native project defaults

These XML and security records were extracted from the native blank fixtures
listed in `manifest.json`. They contain no user application. They are compiled
into libxgwx; creation performs no file I/O and embeds no `.xgwx` container.

`create_project(cpu_model, language)` assembles the native fixed header, gzip XML,
four-byte alignment, compressed-size field, additive checksum and security tail
using the writer's shared serializer. It applies guarded CPU selection and uses
the existing program writer to generate ST/IL variants. Native identities and
opaque parameter/security defaults are retained; derived variant program/symbol
identities are stable library defaults. Names remain NewProject/LSPLC/NewProgram.

This is library-owned generation from captured defaults, not a claim that all
native parameter/security formats have been reverse-engineered. Original native
workspace fixtures stay in fixtures/ for regression and XG5000 comparison. Local
checks must be distinguished from XG5000 Open/Check Program/Save As acceptance.
