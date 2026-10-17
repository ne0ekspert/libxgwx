# Project and program passwords

Password support is based on controlled XG5000 4.82.1.0 saves in an isolated
Windows 10 VM and sections 3.6–3.7 of the Korean XG5000 manual (PDF pages
136–145, project password instructions beginning on page 137).

## Rust API

```rust
use xgwx::XgwxDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse()/from_path() request a password for protected projects.
    let mut doc = XgwxDocument::from_path_with_password("project.xgwx", "project1")?;
    // Individual program blocks authenticate separately.
    doc.unlock_program(0, "program1")?;
    let _ladder = doc.program().get(0)?.ladder()?;

    // Requires write. None removes a password; Some("") is invalid.
    doc.project_mut().set_password(Some("project1"), Some("newpass1"))?;
    doc.program_mut().get(0)?.set_password(Some("program1"), None)?;
    doc.write_to("updated.xgwx")?;
    Ok(())
}
```

`ProjectView::password_protected()` reports project protection.
`ProgramRef::password_protected()` reports stored program protection, while
`ProgramRef::is_locked()` reports whether the current session still needs its
password. `Program::unlock()` is the selected-program equivalent of
`XgwxDocument::unlock_program()`.

Passwords are case sensitive and preserve spaces. The captured Korean edition
uses CP949 (Windows Korean), with a maximum of eight characters and sixteen
encoded bytes. Unrepresentable characters, NUL, excessive length, and values
that would produce an embedded native string terminator are rejected without
changing the document. Password errors never include supplied credentials, and
successful authentication retains only program identities in the session.

## Stored layout and preservation

The supported project layouts have 138- or 142-byte headers with signature
`58 47 00 04 02 00` (XGK/generated) or `58 47 00 04 03 00` (native XGI). Each 66-byte password field occupies bytes 68–133. The
stored value uses the native RC4 stream over CP949 bytes with zero padding.
Changing this field also updates the additive header checksum at bytes 64–67.
The native header sum uses signed password bytes; the gzip sum uses unsigned bytes.
The protected-program variant inserts `FFFFFFFF` at bytes 134–137, moves the
compressed-size field to bytes 138–141, and complements the checksum. Program
password writes maintain this marker; removing the final protected program
returns to the 138-byte layout.
The main XML gzip and trailing security records remain intact when otherwise
unchanged.

A protected program stores the password bytes as uppercase hexadecimal in the
native misspelled `Encrytption` attribute. In the captured current format,
XGK LD uses `Body` for a compatibility warning and `Body2` for the actual source.
Authentication swaps their roles in the working document so existing editors
operate on the real source. Serialization restores native ordering. An unchanged
unlocked program serializes byte-identically; source edits update the real body
while retaining the compatibility body and password.

XGI LD, ST, IL, and SFC use a single `Body`, confirmed by separate native
password saves for all four languages. Authentication leaves that body in place.
Setting, changing, and removing protection edits only the password attribute and
workspace header; it does not add a compatibility body. Locked typed readers
and editors still require authentication, including text-source/declaration lists.

Typed ladder/source/declaration readers reject or omit locked content rather
than presenting the compatibility warning as the program. Raw container/XML
inspection remains a format-inspection interface; this library does not add an
independent cryptographic access-control boundary around native file contents.

## Current scope

Project opening, setting, changing, and removal support the captured workspace
header. Program opening, setting, changing, and removal support XGK LD with
the captured `Body`/`Body2` envelope and XGI LD/ST/IL/SFC with their native
single-body layout. Other initial layouts fail with
`PasswordError::UnsupportedLayout`; unknown single-body layouts also cannot be
unlocked. Native IEC acceptance uses XGI-CPUE.

The C ABI exposes password-aware parse, program unlock, status, and separate
project/program setters; see [the C API](c-api.md). WASM exports
`unlock_xgwx_password`, `set_xgwx_password`, and
`clear_xgwx_password_session`. Existing WASM readers/writers share bounded
in-memory authenticated history; returned bytes always retain stored protection.
Switching files clears the webview session. `XgwxDocument::parse_in_session`
allows bindings to reparse external/edited bytes while inheriting only matching
project credentials and matching program credentials/unique identities.
No session retains plaintext passwords. User-function passwords, PLC device
passwords, and the XGK automatic-allocation read-only source-protection option
remain follow-up work.

## Fixtures and verification

`fixtures/passwords/` contains disposable projects created in the native VM:

| Fixture | Project password | Program 0 password |
| --- | --- | --- |
| `xgk-plain.xgwx` | none | none |
| `xgk-project-ascii.xgwx` | `Ab1! x` | none |
| `xgk-project-eight.xgwx` | `12345678` | none |
| `xgk-program-ascii.xgwx` | none | `Ab1! x` |
| `xgk-program-korean.xgwx` | none | `가나다라마바사아` |
| `xgi-{ld,st,il,sfc}-native.xgwx` | none | `Ab1! x` |
| `xgi-set-resaved.xgwx` | none | all four: `Ab1! x` |
| `xgi-change-resaved.xgwx` | none | all four: `12345678` |
| `xgi-remove-resaved.xgwx` | none | all four: none |
| `generated-both-resaved.xgwx` | `Ab1! x` | `12345678` |
| `generated-edit-resaved.xgwx` | `Ab1! x` | `12345678` |
| `generated-removed-resaved.xgwx` | none | none |

The Korean fixture was entered through native Windows control messages because
the password controls rejected clipboard input; XG5000 performed its normal
confirmation and save validation. No vendor binaries are included in the repo.

`cargo test --features write --test passwords` verifies these native samples,
wrong/missing passwords, exact case/space handling, preservation, and atomic
rejection. `dev-tools/password_acceptance.rs` generates combined protection,
protected source edits, and password removal for native compatibility checks.
Its `iec-plain` mode generates language-specific inputs for native capture;
`iec` generates `SETIEC`, `CHGIEC`, and `REMIEC`, each containing all four
IEC languages. `SETIEC` uses `Ab1! x`, `CHGIEC` uses `12345678`, and `REMIEC`
has no program passwords. Pass the mode after the output directory.

## Native acceptance (2026-10-17)

The generated files were copied to writable storage in the isolated Windows 10
snapshot and opened in XG5000 4.82.1.0. A wrong project password was rejected,
the correct `Ab1! x` password opened the project, and program opening separately
required `12345678`. The protected original displayed contact `M00000` and coil
`M00001`; the protected edit displayed `M00002` and `M00001`. The removal case
opened without either prompt and displayed the edited source.

Native Save As outputs for all three cases are retained above. Host-side
inspection confirmed both password fields were retained for the protected cases,
removed for the plain case, and the real ladder payload was byte-identical to
the corresponding generated payload in every case. These are native
open/display/save checks; no connected PLC operation was attempted.

Validation on this revision: the full `il,wasm,write,ffi` host test suite passed
415 tests (135 existing ignores), including 21 password regressions and a C ABI password test. The
read-only password tests passed all seven cases, and the combined host feature
check passed. The extension bundle was rebuilt for `wasm32-unknown-unknown` using the
installed rustup toolchain and optimized with `wasm-opt -Oz`.

The final writer's 142-byte protected output was also reopened in a fresh VM
snapshot and saved by XG5000. Both credentials and the actual ladder bytes were
identical after this final native round trip.


### IEC program password continuation

Four language-specific native saves confirmed that XGI-CPUE LD, ST, IL, and SFC
retain a single body under protection. Library-generated projects containing all
four programs were then opened separately for initial passwords (`Ab1! x`),
changed passwords (`12345678`), and removal. XG5000 rejected a wrong initial
password and rejected the old password after the change. Correct passwords
opened the expected ladder, ST, IL, and SFC editors; after removal all four
opened without prompts.

The three native Save As outputs are `xgi-set-resaved.xgwx`,
`xgi-change-resaved.xgwx`, and `xgi-remove-resaved.xgwx`. All twelve saved
programs retained their expected password attribute, body contents, and local
variable table. Compressed binary/source payloads were compared after decoding;
XML formatting differences were ignored. Both protected projects retained the
142-byte header, and removal retained the 138-byte plain header. The SFC test
uses the captured empty chart; no connected PLC operation was attempted.
