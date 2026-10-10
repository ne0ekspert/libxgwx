# XGI CPU conversion defaults

These library-owned profiles retain only each captured Configuration's Type,
Attribute and Parameters XML. They contain no programs, symbols, project names,
workspace headers or security tails. `manifest.json` records extraction sources
and SHA-256 hashes. Original workspace fixtures remain in the repository and
are excluded from the crates.io package.

CPU conversion compares normalized parameters against these profiles and copies
the same native memory, Ethernet and motion defaults as before. Whitespace inside
Parameters is preserved so inserted Ethernet and motion records remain identical.
