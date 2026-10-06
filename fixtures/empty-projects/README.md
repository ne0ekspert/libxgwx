# Blank XG5000 projects

Created and saved with native XG5000 on 2026-10-05:

- `new-xgk.xgwx`: XGK-CPUSN, XGK Ladder Diagram.
- `new-xgi.xgwx`: XGI-CPUE, IEC Ladder Diagram.

Both contain project `NewProject`, PLC `LSPLC`, and empty program `NewProgram`.
They contain no user application or smart-home project data. Untouched ladder
ProgramData decodes to an eight-byte zero header. XG5000 reports an empty-program
error until instructions are added; this is expected for a new blank project.

The New File command copies these native templates without changing their
internal project identities. CPU and project metadata can be edited afterward.
