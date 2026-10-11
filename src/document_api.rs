#![deny(missing_docs)]
//! Borrowed program and variable namespaces. They do not copy the document.
use crate::*;

/// Read-only program metadata, language capabilities and decoded bodies.
#[derive(Debug, Clone, Copy)]
pub struct ProgramView<'a> {
    pub(crate) doc: &'a XgwxDocument,
}
/// Read-only global symbols and program-local declarations.
#[derive(Debug, Clone, Copy)]
pub struct VariableView<'a> {
    pub(crate) doc: &'a XgwxDocument,
}
/// Program edits borrow the document exclusively and use the existing guards.
/// Indexed mutation bridges have been removed; select a [`crate::Program`] first.
/// ```compile_fail
/// # use xgwx::{XgwxDocument, ProgramPatch};
/// fn old_api(doc: &mut XgwxDocument) {
///     doc.program_mut().update_program(0, &ProgramPatch::default()).unwrap();
/// }
/// ```
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct ProgramEditor<'a> {
    pub(crate) doc: &'a mut XgwxDocument,
}
/// Variable edits borrow the document exclusively and use the existing guards.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct VariableEditor<'a> {
    pub(crate) doc: &'a mut XgwxDocument,
}

impl XgwxDocument {
    /// Borrow the program namespace. See [`ProgramView::list`].
    ///
    /// ```no_run
    /// let doc = xgwx::XgwxDocument::from_path("project.xgwx")?;
    /// let programs = doc.program().list();
    /// let globals = doc.variable().globals()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// The previous flat methods are no longer public:
    /// ```compile_fail
    /// fn old_api(doc: &xgwx::XgwxDocument) { let _ = doc.programs(); }
    /// ```
    pub fn program(&self) -> ProgramView<'_> {
        ProgramView { doc: self }
    }
    /// Borrow the variable namespace. See [`VariableView::globals`].
    ///
    /// ```compile_fail
    /// fn old_api(doc: &xgwx::XgwxDocument) { let _ = doc.variables(); }
    /// ```
    pub fn variable(&self) -> VariableView<'_> {
        VariableView { doc: self }
    }
    /// Borrow the program editor exclusively. Requires `write`.
    /// Collect source/identity snapshots before borrowing this editor.
    #[cfg(feature = "write")]
    pub fn program_mut(&mut self) -> ProgramEditor<'_> {
        ProgramEditor { doc: self }
    }
    /// Borrow the variable editor exclusively. Requires `write`.
    /// Collect declaration snapshots before borrowing this editor.
    #[cfg(feature = "write")]
    pub fn variable_mut(&mut self) -> VariableEditor<'_> {
        VariableEditor { doc: self }
    }
}

impl ProgramView<'_> {
    /// Return high-level summaries for all `<Task>` elements.
    pub fn tasks(&self) -> Vec<TaskSummary> {
        self.doc.tasks()
    }
    /// Return high-level summaries for all `<Program>` elements.
    pub fn programs(&self) -> Vec<ProgramSummary> {
        self.doc.programs()
    }
    /// Decode ladder `<ProgramData>` payloads for all `<Program>` elements.
    ///
    /// XG5000 stores ladder bodies as base64, usually bzip2-compressed, binary
    /// blobs. This returns a partial decode that preserves the bytes and extracts
    /// embedded UTF-16 strings and likely instruction calls.
    pub fn ladder_programs(&self) -> Vec<Result<LadderProgramData, XgwxError>> {
        self.doc.ladder_programs()
    }
    /// Decode only the selected program, without inflating unrelated payloads.
    /// The index uses the same document order as `ladder_programs`.
    pub fn ladder_program(&self, index: usize) -> Option<Result<LadderProgramData, XgwxError>> {
        self.doc.ladder_program(index)
    }
    /// Languages whose creation layout is validated for this CPU and project mode.
    pub fn program_languages(&self) -> Vec<&'static str> {
        self.doc.program_languages()
    }
    /// Read standalone source. Unknown layouts remain visible but cannot be written.
    pub fn text_programs(&self) -> Vec<TextProgram> {
        self.doc.text_programs()
    }
    /// Decode XGK ladder IL projections, distinct from standalone IEC IL source.
    #[cfg(all(feature = "write", feature = "il"))]
    pub fn vendor_il_programs(&self) -> Vec<VendorIlProgram> {
        self.doc.vendor_il_programs()
    }
    /// Read native SFC XML independently of binary ladder program bodies.
    /// Unknown entity kinds and property attributes remain available to callers.
    pub fn sfc_programs(&self) -> Vec<SfcProgram> {
        self.doc.sfc_programs()
    }
    /// Short form of [`Self::programs`].
    pub fn list(&self) -> Vec<ProgramSummary> {
        self.programs()
    }
    /// Short form of [`Self::program_languages`].
    pub fn languages(&self) -> Vec<&'static str> {
        self.program_languages()
    }
}

impl VariableView<'_> {
    /// Decode the global variable symbol table.
    ///
    /// XG5000 stores `<Symbols>` as base64, usually bzip2-compressed, binary
    /// data. The observed symbol table contains fixed seven-string records.
    pub fn variables(&self) -> Result<Vec<VariableSummary>, XgwxError> {
        self.doc.variables()
    }
    /// Decode each program's IEC `PB50` local symbol table in program order.
    /// A missing table is represented by an empty list for that program.
    pub fn iec_local_symbols(&self) -> Vec<Result<Vec<IecLocalSymbol>, XgwxError>> {
        self.doc.iec_local_symbols()
    }
    /// Decode regular IEC declarations while retaining the two captured SFC system records.
    pub fn sfc_variables(
        &self,
        program_index: usize,
    ) -> Result<Vec<SfcVariable>, crate::XgwxError> {
        self.doc.sfc_variables(program_index)
    }
    /// Preview native-style digital I/O variables for the current XGK modules.
    #[cfg(feature = "write")]
    pub fn preview_io_variables(&self) -> Result<Vec<IoVariableGenerationRow>, XgwxError> {
        self.doc.preview_io_variables()
    }
    /// Short form of [`Self::variables`].
    pub fn globals(&self) -> Result<Vec<VariableSummary>, XgwxError> {
        self.variables()
    }
    /// Short form of [`Self::iec_local_symbols`].
    pub fn locals(&self) -> Vec<Result<Vec<IecLocalSymbol>, XgwxError>> {
        self.iec_local_symbols()
    }
}

#[cfg(feature = "write")]
impl ProgramEditor<'_> {
    /// Return high-level summaries for all `<Task>` elements.
    pub fn tasks(&self) -> Vec<TaskSummary> {
        self.doc.tasks()
    }
    /// Return high-level summaries for all `<Program>` elements.
    pub fn programs(&self) -> Vec<ProgramSummary> {
        self.doc.programs()
    }
    /// Decode ladder `<ProgramData>` payloads for all `<Program>` elements.
    ///
    /// XG5000 stores ladder bodies as base64, usually bzip2-compressed, binary
    /// blobs. This returns a partial decode that preserves the bytes and extracts
    /// embedded UTF-16 strings and likely instruction calls.
    pub fn ladder_programs(&self) -> Vec<Result<LadderProgramData, XgwxError>> {
        self.doc.ladder_programs()
    }
    /// Decode only the selected program, without inflating unrelated payloads.
    /// The index uses the same document order as `ladder_programs`.
    pub fn ladder_program(&self, index: usize) -> Option<Result<LadderProgramData, XgwxError>> {
        self.doc.ladder_program(index)
    }
    /// Languages whose creation layout is validated for this CPU and project mode.
    pub fn program_languages(&self) -> Vec<&'static str> {
        self.doc.program_languages()
    }

    /// Read standalone source. Unknown layouts remain visible but cannot be written.
    pub fn text_programs(&self) -> Vec<TextProgram> {
        self.doc.text_programs()
    }

    /// Decode XGK ladder IL projections, distinct from standalone IEC IL source.
    #[cfg(all(feature = "write", feature = "il"))]
    pub fn vendor_il_programs(&self) -> Vec<VendorIlProgram> {
        self.doc.vendor_il_programs()
    }

    /// Read native SFC XML independently of binary ladder program bodies.
    /// Unknown entity kinds and property attributes remain available to callers.
    pub fn sfc_programs(&self) -> Vec<SfcProgram> {
        self.doc.sfc_programs()
    }

    /// Short form of [`Self::programs`].
    pub fn list(&self) -> Vec<ProgramSummary> {
        self.programs()
    }
    /// Short form of [`Self::program_languages`].
    pub fn languages(&self) -> Vec<&'static str> {
        self.program_languages()
    }
}

#[cfg(feature = "write")]
impl VariableEditor<'_> {
    /// Decode the global variable symbol table.
    ///
    /// XG5000 stores `<Symbols>` as base64, usually bzip2-compressed, binary
    /// data. The observed symbol table contains fixed seven-string records.
    pub fn variables(&self) -> Result<Vec<VariableSummary>, XgwxError> {
        self.doc.variables()
    }
    /// Decode each program's IEC `PB50` local symbol table in program order.
    /// A missing table is represented by an empty list for that program.
    pub fn iec_local_symbols(&self) -> Vec<Result<Vec<IecLocalSymbol>, XgwxError>> {
        self.doc.iec_local_symbols()
    }

    /// Decode regular IEC declarations while retaining the two captured SFC system records.
    pub fn sfc_variables(
        &self,
        program_index: usize,
    ) -> Result<Vec<SfcVariable>, crate::XgwxError> {
        self.doc.sfc_variables(program_index)
    }

    /// Preview native-style digital I/O variables for the current XGK modules.
    #[cfg(feature = "write")]
    pub fn preview_io_variables(&self) -> Result<Vec<IoVariableGenerationRow>, XgwxError> {
        self.doc.preview_io_variables()
    }
    /// Generate digital I/O variables, replacing duplicate names or addresses.
    /// Unrelated symbol records and every program payload remain untouched.
    #[cfg(feature = "write")]
    pub fn generate_io_variables(&mut self) -> Result<(), XgwxError> {
        self.doc.generate_io_variables()
    }

    /// Short form of [`Self::variables`].
    pub fn globals(&self) -> Result<Vec<VariableSummary>, XgwxError> {
        self.variables()
    }
    /// Short form of [`Self::iec_local_symbols`].
    pub fn locals(&self) -> Vec<Result<Vec<IecLocalSymbol>, XgwxError>> {
        self.iec_local_symbols()
    }
}
