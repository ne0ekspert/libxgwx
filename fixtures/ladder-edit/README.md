# Native linear LD fixtures

Created in offline XG5000 4.82.1.0 on Windows 10 on 2026-09-09.
These small test programs were authored for libxgwx using an XGK-CPUUN project.

| File | Native state |
| --- | --- |
| R10.bin / empty.xgwx | Empty program |
| R11.bin | First NO contact M00000 |
| R12.bin | NO plus output M00010 and trailing wire |
| R14.bin | NO deleted from R12, leaving a gap |
| R15.bin | NC M00001 inserted at column 1 |
| R16.bin | NO restored at column 0 |
| R17.bin / linear.xgwx | R16 plus a separate END row |
| R18.bin | NC deleted from R17; high-water coordinate retained |

The .bin files are complete decoded ProgramData payloads from native Save As
outputs (base64, and bzip2 when Compressed=1). Native R13 was an unsuccessful
editing attempt identical to R12 and is intentionally not a fixture.

See [acceptance details](../../docs/ladder-editing.md). These captures establish
file-format behavior; they do not certify PLC execution.

## SHA-256

- R10.bin: af5570f5a1810b7af78caf4bc70a660f0df51e42baf91d4de5b2328de0e83dfc
- R11.bin: a09ab86af90cd91a96964709c0105ca493c9ac4dbbf1e53444478c9ed42e8fb3
- R12.bin: d4b9d01b8ec8b399d02e10b1c94943c011eeb46a5ac22583cd8453238a794352
- R14.bin: ba175b5159485d304dfa49eb4fa242d4880249942f17c272196d5379ca1d0903
- R15.bin: 4d90549e39ccfe9fac43f12211353df3150b504793a47ea0de872673cd575222
- R16.bin: f7bd45d14dffc3a3937eb84171272da5baa46692854374e52a90eab65131359e
- R17.bin: 8cdb1dcf425bc6e3bc81b76746a2259845edfa3dcdc44e1244a676522674fe4d
- R18.bin: 64225e948269792df62ac79281e25e8d7888c0db1b6bee3eb5dbaed95b4856cd
- empty.xgwx: 5ad2407554181f716c333e1fb4b7531947161a66940b58317f2e3295ced981d7
- linear.xgwx: 1471fb970a7e6a85616fa8dac0537258636ac073f56d8aa12690ecc42d8dcaae
