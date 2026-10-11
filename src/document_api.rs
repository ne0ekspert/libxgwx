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
    doc: &'a XgwxDocument,
}
/// Program edits borrow the document exclusively and use the existing guards.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct ProgramEditor<'a> {
    pub(crate) doc: &'a mut XgwxDocument,
}
/// Variable edits borrow the document exclusively and use the existing guards.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct VariableEditor<'a> {
    doc: &'a mut XgwxDocument,
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
    #[cfg(feature = "write")]
    /// Move a program to a final list index without changing its task or contents.
    #[doc(hidden)]
    pub fn move_program(
        &mut self,
        from: usize,
        to: usize,
        expected_object_id: &str,
        expected_target_id: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .move_program(from, to, expected_object_id, expected_target_id)
    }
    #[cfg(feature = "write")]
    /// Remove the selected program, checking its identity before changing XML.
    #[doc(hidden)]
    pub fn delete_program(
        &mut self,
        program_index: usize,
        expected_object_id: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_program(program_index, expected_object_id)
    }
    /// Append an empty scan program without copying existing application code.
    #[cfg(feature = "write")]
    pub fn create_program(&mut self, patch: &NewProgram) -> Result<(), XgwxError> {
        self.doc.create_program(patch)
    }
    /// Read standalone source. Unknown layouts remain visible but cannot be written.
    pub fn text_programs(&self) -> Vec<TextProgram> {
        self.doc.text_programs()
    }
    #[cfg(feature = "write")]
    /// Replace only the native source payload, with identity and stale-source checks.
    #[doc(hidden)]
    pub fn edit_text_program(&mut self, patch: &TextProgramPatch) -> Result<(), XgwxError> {
        self.doc.edit_text_program(patch)
    }
    /// Decode XGK ladder IL projections, distinct from standalone IEC IL source.
    #[cfg(all(feature = "write", feature = "il"))]
    pub fn vendor_il_programs(&self) -> Vec<VendorIlProgram> {
        self.doc.vendor_il_programs()
    }
    #[cfg(all(feature = "write", feature = "il"))]
    #[doc(hidden)]
    pub fn edit_vendor_il(&mut self, patch: &VendorIlPatch) -> Result<(), XgwxError> {
        self.doc.edit_vendor_il(patch)
    }
    /// Read native SFC XML independently of binary ladder program bodies.
    /// Unknown entity kinds and property attributes remain available to callers.
    pub fn sfc_programs(&self) -> Vec<SfcProgram> {
        self.doc.sfc_programs()
    }
    #[cfg(feature = "write")]
    /// Edit captured SFC step comments or direct BOOL M transitions.
    /// Identity and old-value checks reject stale selections; all other XML is preserved.
    #[doc(hidden)]
    pub fn edit_sfc_entity(&mut self, patch: &SfcEntityPatch) -> Result<(), crate::XgwxError> {
        self.doc.edit_sfc_entity(patch)
    }
    #[cfg(feature = "write")]
    /// Replace a captured linear chart atomically, retaining program identities and local symbols.
    #[doc(hidden)]
    pub fn replace_sfc_sequence(
        &mut self,
        patch: &SfcSequencePatch,
    ) -> Result<(), crate::XgwxError> {
        self.doc.replace_sfc_sequence(patch)
    }
    #[cfg(feature = "write")]
    /// Bounded browser edit: one existing IEC kind byte or one operand string.
    /// All XML outside the selected ProgramData and all other payload bytes
    /// are verified unchanged before the candidate is committed.
    #[doc(hidden)]
    pub fn edit_browser_iec(
        &mut self,
        index: usize,
        patch: &BrowserIecPatch,
    ) -> Result<(), XgwxError> {
        self.doc.edit_browser_iec(index, patch)
    }
    #[cfg(feature = "write")]
    /// Update editable XML metadata for one program selected by document order.
    #[doc(hidden)]
    pub fn update_program(
        &mut self,
        program_index: usize,
        patch: &ProgramPatch,
    ) -> Result<(), XgwxError> {
        self.doc.update_program(program_index, patch)
    }
    #[cfg(feature = "write")]
    /// Give one captured IEC function block a separate instance by cloning its
    /// PB50 instance declaration, assigning a new automatic-memory range, and changing only that block's
    /// instance marker. This is useful after copying a network containing a
    /// stateful function such as R_TRIG.
    #[doc(hidden)]
    pub fn duplicate_iec_ld_function_instance(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_instance: &str,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        self.doc.duplicate_iec_ld_function_instance(
            program_index,
            block_offset,
            expected_instance,
            new_instance,
        )
    }
    #[cfg(feature = "write")]
    /// Replace decoded ladder text. Recognized application instructions allow
    /// variable-length operands and catalog-backed mnemonic changes, synchronizing
    /// opcodes, decomposed strings, occupied cells and wires. Other records retain the
    /// same-length restriction to preserve their undecoded binary layout.
    #[doc(hidden)]
    pub fn update_ladder_cell_text(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_ladder_cell_text(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Replace the text of a captured IEC LD comment. XG5000 stores its UTF-16
    /// length in the marker byte; the surrounding record has no length field.
    /// Symbol and operand strings are deliberately excluded from this operation.
    #[doc(hidden)]
    pub fn update_iec_ld_comment(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_comment(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Replace the variable reference of a captured IEC LD rising-edge
    /// contact (`FF 08`).
    #[doc(hidden)]
    pub fn update_iec_ld_rising_contact_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_rising_contact_operand(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Replace an operand in a captured IEC LD contact or output coil record.
    /// This accepts only the record codes identified by `iec_ld::element_operands`.
    /// Classified replacements must be BOOL; coil destinations must be writable.
    /// XG5000 must still validate unclassified expressions.
    #[doc(hidden)]
    pub fn update_iec_ld_element_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_element_operand(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Change a captured IEC contact among the six addressed contact kinds.
    #[doc(hidden)]
    pub fn update_iec_ld_contact_kind(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_contact_kind(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Change a captured IEC coil among output, inverse, set, reset, rising,
    /// and falling kinds while preserving its operand and position.
    #[doc(hidden)]
    pub fn update_iec_ld_coil_kind(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_coil_kind(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Add or remove one captured IEC LD vertical branch segment between two
    /// adjacent rows that already belong to the same native row group.
    ///
    /// Removal preserves another connection between the rows, or deletes a
    /// supported lower branch row and shifts later rows as in native Delete Line.
    /// General group splitting remains unsupported.
    /// Addition is limited to rows without comments or function records and
    /// preserves the group's existing row envelope.
    #[doc(hidden)]
    pub fn edit_iec_ld_branch_segment(
        &mut self,
        program_index: usize,
        group_index: usize,
        start_row_index: u16,
        end_row_index: u16,
        x: u8,
        expected: bool,
        present: bool,
    ) -> Result<(), XgwxError> {
        self.doc.edit_iec_ld_branch_segment(
            program_index,
            group_index,
            start_row_index,
            end_row_index,
            x,
            expected,
            present,
        )
    }
    #[cfg(feature = "write")]
    /// Toggle a paired vertical wire while retaining both row envelopes and
    /// every other record. Open electrical endpoints are valid intermediate
    /// layouts; typed function bindings must remain intact. Addition joins two
    /// existing electrical points in the same group, including a deleted gap.
    #[doc(hidden)]
    pub fn edit_iec_ld_vertical_wire(
        &mut self,
        program_index: usize,
        group_index: usize,
        start_row_index: u16,
        end_row_index: u16,
        x: u8,
        expected: bool,
        present: bool,
    ) -> Result<(), XgwxError> {
        self.doc.edit_iec_ld_vertical_wire(
            program_index,
            group_index,
            start_row_index,
            end_row_index,
            x,
            expected,
            present,
        )
    }
    #[cfg(feature = "write")]
    /// Join consecutive stored IEC groups through a vertical wire between
    /// their adjacent boundary rows. Existing row and function records remain
    /// unchanged; the group envelopes are rebuilt and subsequent groups renumbered.
    /// The joined network is disabled if either source group is disabled.
    #[doc(hidden)]
    pub fn connect_iec_ld_groups(
        &mut self,
        program_index: usize,
        upper_group_index: usize,
        expected_upper_row: u16,
        lower_group_index: usize,
        expected_lower_row: u16,
        x: u8,
    ) -> Result<(), XgwxError> {
        self.doc.connect_iec_ld_groups(
            program_index,
            upper_group_index,
            expected_upper_row,
            lower_group_index,
            expected_lower_row,
            x,
        )
    }
    #[cfg(feature = "write")]
    /// Extend a stored group's final row into the adjacent implicit blank row.
    /// The new row contains the vertical endpoint only; subsequent element
    /// placement can complete the circuit. Existing rows are not shifted.
    #[doc(hidden)]
    pub fn extend_iec_ld_vertical_wire(
        &mut self,
        program_index: usize,
        group_index: usize,
        start_row_index: u16,
        x: u8,
    ) -> Result<(), XgwxError> {
        self.doc
            .extend_iec_ld_vertical_wire(program_index, group_index, start_row_index, x)
    }
    #[cfg(feature = "write")]
    /// Separate a stored IEC group at an adjacent row boundary with no
    /// crossing wire, function body or function reference. Coordinates and
    /// all row-local records are preserved. Both groups inherit the current
    /// network execution setting.
    #[doc(hidden)]
    pub fn split_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_upper_row: u16,
        expected_lower_row: u16,
    ) -> Result<(), XgwxError> {
        self.doc.split_iec_ld_group(
            program_index,
            group_index,
            expected_upper_row,
            expected_lower_row,
        )
    }
    #[cfg(feature = "write")]
    /// Insert one implicit blank IEC LD row after `after_row_index`, matching
    /// XG5000 Ctrl+L on a blank row boundary. The payload does not gain a row
    /// record; later row envelopes and every decoded Y coordinate move down by
    /// one four-unit grid row.
    #[doc(hidden)]
    pub fn insert_iec_ld_blank_row(
        &mut self,
        program_index: usize,
        after_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_iec_ld_blank_row(program_index, after_row_index)
    }
    #[cfg(feature = "write")]
    /// Compatibility wrapper for a normally open parallel contact.
    #[doc(hidden)]
    pub fn insert_iec_ld_parallel_contact(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_parallel_contact(
            program_index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            parallel_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Add an addressed contact in parallel with the leading contact of a
    /// captured one-row addressed contact/wire/output-coil rung. The operation creates
    /// a second row, connects it at the contact's right edge, and shifts later
    /// rows down. The complete two-row network is validated before writing.
    #[doc(hidden)]
    pub fn insert_iec_ld_parallel_contact_kind(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_kind: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_parallel_contact_kind(
            program_index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            parallel_kind,
            parallel_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete one implicit IEC LD blank row and move every later decoded Y
    /// coordinate up by one row, matching XG5000 Ctrl+D on an empty line.
    #[doc(hidden)]
    pub fn delete_iec_ld_blank_row(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_blank_row(program_index, blank_row_index)
    }
    #[cfg(feature = "write")]
    /// Create the compatibility normally-open-contact to output-coil IEC LD
    /// rung. See [`Self::insert_iec_ld_rung`] for the other decoded kinds.
    #[doc(hidden)]
    pub fn insert_iec_ld_linear_rung(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
        contact_variable: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_linear_rung(
            program_index,
            blank_row_index,
            contact_variable,
            coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Create a native three-record IEC LD rung in one implicit blank row.
    /// The captured shape is an addressed BOOL contact at the left rail, a
    /// long wire, and a BOOL coil at the right rail. A new single-row group is
    /// inserted without moving any existing row, including after the last
    /// stored network, extending the stored row range across sparse gaps when necessary.
    #[doc(hidden)]
    pub fn insert_iec_ld_rung(
        &mut self,
        program_index: usize,
        blank_row_index: u16,
        contact_kind: &str,
        contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_rung(
            program_index,
            blank_row_index,
            contact_kind,
            contact_variable,
            coil_kind,
            coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Insert one addressed contact or coil into an unoccupied IEC grid cell.
    /// No other element or wire is created. The resulting row may be incomplete.
    #[doc(hidden)]
    pub fn insert_iec_ld_single_element(
        &mut self,
        program_index: usize,
        row_index: u16,
        raw_x: u8,
        category: &str,
        kind: &str,
        operand: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_single_element(
            program_index,
            row_index,
            raw_x,
            category,
            kind,
            operand,
        )
    }
    #[cfg(feature = "write")]
    /// Delete the compatibility normally-open-contact to output-coil rung.
    /// See [`Self::delete_iec_ld_rung`] for the other decoded kinds.
    #[doc(hidden)]
    pub fn delete_iec_ld_linear_rung(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_linear_rung(
            program_index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete one exact addressed-contact, wire, and coil single-row group,
    /// restoring its row to an implicit blank gap without moving other rows.
    #[doc(hidden)]
    pub fn delete_iec_ld_rung(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_rung(
            program_index,
            row_index,
            expected_contact_kind,
            expected_contact_variable,
            expected_coil_kind,
            expected_coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete the terminal wire and coil from a captured IEC row.
    /// Keep its leading contact or incoming branch endpoint as the sole record.
    #[doc(hidden)]
    pub fn delete_iec_ld_terminal_coil(
        &mut self,
        program_index: usize,
        coil_record_offset: usize,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_terminal_coil(program_index, coil_record_offset, expected_variable)
    }
    #[cfg(feature = "write")]
    /// Finish a contact-only one-row IEC rung with a native long wire and coil.
    /// The row must have the exact shape left by terminal-coil deletion.
    #[doc(hidden)]
    pub fn insert_iec_ld_terminal_coil(
        &mut self,
        program_index: usize,
        contact_record_offset: usize,
        expected_contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_terminal_coil(
            program_index,
            contact_record_offset,
            expected_contact_variable,
            coil_kind,
            coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete one exact simple IEC LD rung and close its row, matching
    /// XG5000 Ctrl+D on an occupied line. Both edits are checked before the
    /// document is changed.
    #[doc(hidden)]
    pub fn delete_iec_ld_simple_row(
        &mut self,
        program_index: usize,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_simple_row(
            program_index,
            row_index,
            expected_contact_kind,
            expected_contact_variable,
            expected_coil_kind,
            expected_coil_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete the upper line of a captured two-row IEC branch. The lower
    /// contact-only line remains and all later rows move up, as in XG5000
    /// Ctrl+D on the upper branch line.
    #[doc(hidden)]
    pub fn delete_iec_ld_branch_top_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_branch_top_row(program_index, group_index, row_index)
    }
    #[cfg(feature = "write")]
    /// Delete a contact-only middle line from a nested IEC branch, reconnecting
    /// the outer x3 branch and removing the two branches local to that line.
    #[doc(hidden)]
    pub fn delete_iec_ld_nested_contact_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_nested_contact_branch_row(program_index, group_index, row_index)
    }
    #[cfg(feature = "write")]
    /// Delete a contact-only middle line in a chain of x3 branches. The
    /// adjacent vertical segments become one segment across the removed line.
    #[doc(hidden)]
    pub fn delete_iec_ld_chained_contact_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_chained_contact_branch_row(program_index, group_index, row_index)
    }
    #[cfg(feature = "write")]
    /// Delete a middle IEC row that only carries an incoming and outgoing
    /// vertical branch. The two segments are joined across the row gap.
    #[doc(hidden)]
    pub fn delete_iec_ld_empty_branch_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_empty_branch_row(program_index, group_index, row_index)
    }
    #[cfg(feature = "write")]
    /// Reproduce XG5000 Delete Line on the lower output row of the captured
    /// two-row FF branch. Native deletion also removes the connected FF block
    /// and its output reference from the upper row.
    #[doc(hidden)]
    pub fn delete_iec_ld_ff_branch_output_row(
        &mut self,
        program_index: usize,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_ff_branch_output_row(program_index, group_index, row_index)
    }
    #[cfg(feature = "write")]
    /// Insert a normally open contact into a captured linear IEC LD wire.
    /// This compatibility wrapper delegates to [`Self::insert_iec_ld_contact`].
    #[doc(hidden)]
    pub fn insert_iec_ld_no_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_no_contact(
            program_index,
            wire_offset,
            raw_x,
            expected_wire_start_x,
            expected_wire_end_x,
            variable,
        )
    }
    #[cfg(feature = "write")]
    /// Insert an addressed contact into the empty x1 cell of a captured
    /// two-row IEC branch upper line, matching XG5000 F3/F4 insertion.
    #[doc(hidden)]
    pub fn insert_iec_ld_leading_contact(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_leading_contact(
            program_index,
            insertion_offset,
            contact_kind,
            variable,
        )
    }
    #[cfg(feature = "write")]
    /// Insert one of the six addressed IEC contact kinds into a captured long
    /// wire. The record shape follows native XG5000 insertion and contact-kind
    /// captures. The operand must resolve to a BOOL symbol or device address.
    #[doc(hidden)]
    pub fn insert_iec_ld_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_contact(
            program_index,
            wire_offset,
            raw_x,
            expected_wire_start_x,
            expected_wire_end_x,
            contact_kind,
            variable,
        )
    }
    #[cfg(feature = "write")]
    /// Replace one framed IEC short wire with an addressed contact. XG5000 F3
    /// preserves the row count and surrounding records for this cell edit.
    #[doc(hidden)]
    pub fn insert_iec_ld_short_wire_contact(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        expected_raw_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_short_wire_contact(
            program_index,
            wire_offset,
            expected_raw_x,
            contact_kind,
            variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete any decoded contact between two captured wire fragments, checking
    /// its expected kind before applying native XG5000 Delete semantics.
    #[doc(hidden)]
    pub fn delete_iec_ld_contact(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_contact(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
            expected_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete a decoded contact between two captured wire fragments.
    /// Native XG5000 Delete leaves both fragments in place and decrements the
    /// row record count. Retained for compatibility with the original NO/NC API.
    #[doc(hidden)]
    pub fn delete_iec_ld_no_contact(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_no_contact(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete any decoded contact with native XG5000 Cell Delete semantics,
    /// checking its expected kind before applying the edit.
    #[doc(hidden)]
    pub fn delete_iec_ld_contact_cell(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_contact_cell(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
            expected_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Delete a decoded contact with native XG5000 Cell Delete semantics.
    /// A leading branch contact moves the next contact to x1 and puts a short
    /// wire at x4. Contacts between long wires shift later cells left while
    /// the output coil stays fixed. Retained for API compatibility.
    #[doc(hidden)]
    pub fn delete_iec_ld_no_contact_cell(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_no_contact_cell(
            program_index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }
    #[cfg(feature = "write")]
    /// Remove a captured one-cell `FF 01` wire between two long wires.
    /// The resulting gap must be recognized by the guarded wire repair path.
    #[doc(hidden)]
    pub fn delete_iec_ld_horizontal_wire(
        &mut self,
        program_index: usize,
        wire_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_horizontal_wire(program_index, wire_offset, expected_raw_x)
    }
    #[cfg(feature = "write")]
    /// Delete an exact horizontal XGK wire without removing its row or elements.
    #[doc(hidden)]
    pub fn delete_ladder_horizontal_wire(
        &mut self,
        program_index: usize,
        raw_y: u32,
        start_x: u8,
        end_x: u8,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_ladder_horizontal_wire(program_index, raw_y, start_x, end_x)
    }
    #[cfg(feature = "write")]
    /// Delete an exact vertical XGK branch reference pair, preserving shared rows.
    #[doc(hidden)]
    pub fn delete_ladder_vertical_wire(
        &mut self,
        program_index: usize,
        raw_x: u8,
        start_y: u32,
        end_y: u32,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_ladder_vertical_wire(program_index, raw_x, start_y, end_y)
    }
    #[cfg(feature = "write")]
    /// Delete one framed IEC horizontal wire, retaining row headers and operands.
    #[doc(hidden)]
    pub fn delete_iec_ld_horizontal_wire_record(
        &mut self,
        program_index: usize,
        offset: usize,
        expected_start_x: u8,
        expected_end_x: u8,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_horizontal_wire_record(
            program_index,
            offset,
            expected_start_x,
            expected_end_x,
        )
    }
    #[cfg(feature = "write")]
    /// Remove an isolated contact or coil, leaving its physical row empty.
    #[doc(hidden)]
    pub fn delete_iec_ld_isolated_element(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_isolated_element(program_index, offset, expected)
    }
    #[cfg(feature = "write")]
    /// Draw one native F5 cell from a scalar function BOOL output or extend
    /// its contiguous short-wire feed. A comparison OUT assignment is replaced
    /// by the wire; numeric destinations are preserved and cannot be wired.
    #[doc(hidden)]
    pub fn insert_iec_ld_function_output_wire(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
        pin_name: &str,
        start_x: u8,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_function_output_wire(
            program_index,
            block_offset,
            expected_name,
            pin_name,
            start_x,
        )
    }
    #[cfg(feature = "write")]
    /// Reconnect a one-cell gap left by a captured IEC contact or trigger deletion.
    /// Native XG5000 F5 inserts an `FF 01` short-wire record, increments the
    /// row count, and resets the row header coordinate to the first contact.
    #[doc(hidden)]
    pub fn repair_iec_ld_horizontal_wire(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        self.doc
            .repair_iec_ld_horizontal_wire(program_index, insertion_offset, expected_raw_x)
    }
    #[cfg(feature = "write")]
    /// Remove an output assignment without removing the pin, reference, body,
    /// or physical row. Inputs cannot be cleared through this API.
    #[doc(hidden)]
    pub fn delete_iec_ld_function_output_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_function_output_operand(program_index, offset, expected)
    }
    #[cfg(feature = "write")]
    /// Assign a previously empty OUT pin. Reuse the existing operand validator
    /// on a private candidate so a bad type or constant never changes the document.
    #[doc(hidden)]
    pub fn assign_iec_ld_function_output_operand(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
        ordinal: u8,
        value: &str,
    ) -> Result<(), XgwxError> {
        self.doc.assign_iec_ld_function_output_operand(
            program_index,
            block_offset,
            expected_name,
            ordinal,
            value,
        )
    }
    #[cfg(feature = "write")]
    /// Replace a captured IEC LD function input/output expression (`FF 46`).
    /// Classified symbols and literals are checked against the decoded pin type;
    /// output pins also require a writable destination.
    #[doc(hidden)]
    pub fn update_iec_ld_function_operand(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_function_operand(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Delete a scalar from a horizontal chain or a captured staggered MOVE layout,
    /// preserving neighboring bodies and pin rows.
    #[doc(hidden)]
    pub fn delete_iec_ld_scalar_chain_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_scalar_chain_function(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Replace a supported scalar chain body atomically while preserving its neighbors.
    #[doc(hidden)]
    pub fn replace_iec_ld_scalar_chain_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.doc.replace_iec_ld_scalar_chain_function(
            program_index,
            block_offset,
            expected_name,
            function_name,
            operands,
        )
    }
    #[cfg(feature = "write")]
    /// Delete a terminal IEC function block using the captured XG5000 Delete
    /// shape. This removes its preceding wire and its otherwise empty pin rows,
    /// while retaining the leading contact as the only record in the group.
    #[doc(hidden)]
    pub fn delete_iec_ld_terminal_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_terminal_function(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Insert a terminal TON using an existing local instance declaration.
    #[doc(hidden)]
    pub fn insert_iec_ld_terminal_timer(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        instance: &str,
        preset: &str,
        elapsed: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_terminal_timer(
            program_index,
            contact_offset,
            instance,
            preset,
            elapsed,
        )
    }
    #[cfg(feature = "write")]
    /// Restore a native two-pin MOVE after a retained terminal contact.
    /// Operand edits run on a clone, so the operation is atomic.
    #[doc(hidden)]
    pub fn insert_iec_ld_terminal_move(
        &mut self,
        program_index: usize,
        contact_offset: usize,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_terminal_move(
            program_index,
            contact_offset,
            input_operand,
            output_operand,
        )
    }
    #[cfg(feature = "write")]
    /// Delete a standalone IEC function group using the captured XG5000
    /// Delete shape. The group's stored rows become an implicit blank gap and
    /// all later group ordinals are decremented without changing row indices.
    /// A contiguous final group trims the total row count at its first row.
    #[doc(hidden)]
    pub fn delete_iec_ld_standalone_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_standalone_function(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Remove one complete decoded IEC LD network while leaving its row range
    /// as an implicit blank gap. Later network ordinals are renumbered; their
    /// rows, records, and coordinates remain unchanged.
    #[doc(hidden)]
    pub fn delete_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_group(program_index, group_index, expected_first_row_index)
    }
    #[cfg(feature = "write")]
    /// Replace an occupied IEC LD network with a copy of another network in
    /// the same program. Both edits are checked on a private document, so a
    /// failed copy never leaves the destination deleted.
    #[doc(hidden)]
    pub fn replace_iec_ld_group(
        &mut self,
        program_index: usize,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc.replace_iec_ld_group(
            program_index,
            source_group_index,
            expected_source_first_row_index,
            destination_group_index,
            expected_destination_first_row_index,
        )
    }
    #[cfg(feature = "write")]
    /// Replace a destination network with a network from another IEC program.
    /// Missing source locals and captured function instances can be copied in
    /// the same atomic edit when requested.
    #[doc(hidden)]
    pub fn replace_iec_ld_group_from_program(
        &mut self,
        source_program_index: usize,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_program_index: usize,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
        copy_missing_locals: bool,
    ) -> Result<(), XgwxError> {
        self.doc.replace_iec_ld_group_from_program(
            source_program_index,
            source_group_index,
            expected_source_first_row_index,
            destination_program_index,
            destination_group_index,
            expected_destination_first_row_index,
            copy_missing_locals,
        )
    }
    #[cfg(feature = "write")]
    /// Move a complete decoded IEC LD network into an empty row range.
    /// Its internal branches, function pins, and operand links move together.
    #[doc(hidden)]
    pub fn move_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc.move_iec_ld_group(
            program_index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
        )
    }
    #[cfg(feature = "write")]
    /// Copy a complete decoded IEC LD network into an empty row range,
    /// retaining the original group. The copied operands and function instance
    /// names can be edited separately after insertion.
    #[doc(hidden)]
    pub fn copy_iec_ld_group(
        &mut self,
        program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc.copy_iec_ld_group(
            program_index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
        )
    }
    #[cfg(feature = "write")]
    /// Copy a complete IEC network into an empty row range in another
    /// program. Referenced local symbols and function instances must already
    /// exist with compatible types in the destination program.
    #[doc(hidden)]
    pub fn copy_iec_ld_group_to_program(
        &mut self,
        source_program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc.copy_iec_ld_group_to_program(
            source_program_index,
            group_index,
            expected_first_row_index,
            destination_program_index,
            destination_first_row_index,
        )
    }
    #[cfg(feature = "write")]
    /// Copy a network and create its missing primitive locals and captured
    /// function-instance declarations in the destination. Supported BOOL
    /// mappings retain their source address.
    /// A private document makes declaration insertion and network copying atomic.
    #[doc(hidden)]
    pub fn copy_iec_ld_group_to_program_with_locals(
        &mut self,
        source_program_index: usize,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.doc.copy_iec_ld_group_to_program_with_locals(
            source_program_index,
            group_index,
            expected_first_row_index,
            destination_program_index,
            destination_first_row_index,
        )
    }
    #[cfg(feature = "write")]
    /// Create a standalone IEC comment in an empty row by copying a captured
    /// one-record comment group from the same program and replacing its text.
    /// The source group remains untouched; failures leave the document intact.
    #[doc(hidden)]
    pub fn insert_iec_ld_comment(
        &mut self,
        program_index: usize,
        destination_row_index: u16,
        text: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_iec_ld_comment(program_index, destination_row_index, text)
    }
    #[cfg(feature = "write")]
    /// Delete a scalar block on a branch row, retaining its branch scaffold.
    #[doc(hidden)]
    pub fn delete_iec_ld_branch_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_branch_function(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Replace a captured branch scalar function as one atomic document edit.
    /// A scaffold grows if needed and keeps surplus native branch rows; a completed
    /// branch tail uses only the replacement block's own pin rows.
    #[doc(hidden)]
    pub fn replace_iec_ld_branch_function(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.doc.replace_iec_ld_branch_function(
            program_index,
            block_offset,
            expected_name,
            function_name,
            operands,
        )
    }
    #[cfg(feature = "write")]
    /// Place a scalar MOVE, arithmetic, or comparison function in available IEC cells.
    #[doc(hidden)]
    pub fn insert_iec_ld_function(
        &mut self,
        program_index: usize,
        row_index: u16,
        raw_x: u8,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_iec_ld_function(program_index, row_index, raw_x, function_name, operands)
    }
    #[cfg(feature = "write")]
    /// Insert the captured standalone WORD_TO_UDINT shape at a three-row group gap.
    #[doc(hidden)]
    pub fn insert_iec_ld_standalone_function(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        function_name: &str,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_standalone_function(
            program_index,
            insertion_offset,
            function_name,
            input_operand,
            output_operand,
        )
    }
    #[cfg(feature = "write")]
    /// Insert the captured connected single-output FF function cell into a
    /// guarded two-row gap. Both stored rows and their surrounding records are
    /// retained; one function record and one output-link record are added.
    #[doc(hidden)]
    pub fn insert_iec_ld_function_cell(
        &mut self,
        program_index: usize,
        insertion_offset: usize,
        function_name: &str,
        instance_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc.insert_iec_ld_function_cell(
            program_index,
            insertion_offset,
            function_name,
            instance_name,
        )
    }
    #[cfg(feature = "write")]
    /// Delete a captured IEC function cell while retaining unrelated records
    /// in the same stored rows. The block and all of its pin-link records are
    /// removed and each affected row's record count is decremented.
    #[doc(hidden)]
    pub fn delete_iec_ld_function_cell(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_function_cell(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete the first EQ in the captured four-block comparison chain.
    /// XG5000 closes the fourth row of that block and joins the x12 feed to
    /// the next EQ; deleting only the function records leaves a stale branch.
    #[doc(hidden)]
    pub fn delete_iec_ld_eq_chain_head(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_eq_chain_head(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete the first comparison block in the captured heating/cooling
    /// chain. Native Delete Line on its first pin row removes the four-row
    /// block body, joins the two long branch feeds, and shifts later rows.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_head(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_heating_chain_head(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete one of the two captured middle EQ blocks in the heating chain.
    /// Native Delete Line on its first pin row keeps the x3/x15 branch feeds.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_middle(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_heating_chain_middle(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete an x15-fed EQ in the captured heating comparison chain by its
    /// first pin row. The shape guard also accepts the next comparison after
    /// an earlier deletion has shifted its row index.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_x15_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_heating_chain_x15_eq(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete the captured contact-fed L62 EQ by its first pin row.
    /// XG5000 removes its x6/x12 contact branches and retains the x15 feed.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_contact_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_heating_chain_contact_eq(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete the x3-fed EQ and its orphan x15 feed as one valid edit.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_x3_eq_repaired(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc.delete_iec_ld_heating_chain_x3_eq_repaired(
            program_index,
            block_offset,
            expected_name,
        )
    }
    #[cfg(feature = "write")]
    /// Reproduce XG5000 Delete Line on the L59 pin row of the L58 EQ.
    /// This native edit leaves one input/output error until the circuit is repaired.
    #[doc(hidden)]
    pub fn delete_iec_ld_heating_chain_x3_eq(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_heating_chain_x3_eq(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Remove the dangling x15 feed segments after deleting an x3-fed
    /// comparison. The unique orphan chain is found from decoded rows and
    /// branch endpoints, restoring a valid circuit while retaining the x3 feed.
    #[doc(hidden)]
    pub fn repair_iec_ld_heating_chain_x3_eq_deletion(
        &mut self,
        program_index: usize,
    ) -> Result<(), XgwxError> {
        self.doc
            .repair_iec_ld_heating_chain_x3_eq_deletion(program_index)
    }
    #[cfg(feature = "write")]
    /// Delete a captured connected ADD/SUB in a five-row R_TRIG/arithmetic/MOVE group.
    /// XG5000 retains the R_TRIG and MOVE circuits as two separate groups.
    #[doc(hidden)]
    pub fn delete_iec_ld_connected_arithmetic(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_connected_arithmetic(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Delete a four-row scalar arithmetic block sharing an external branch spine.
    /// Matches native Delete Line on its final continuation row, retaining the
    /// spine and neighboring functions while shifting later rows up once.
    #[doc(hidden)]
    pub fn delete_iec_ld_branched_arithmetic(
        &mut self,
        program_index: usize,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_ld_branched_arithmetic(program_index, block_offset, expected_name)
    }
    #[cfg(feature = "write")]
    /// Change a captured three-operand IEC ADD/SUB/MUL/DIV block.
    /// The native capture shows a paired function opcode and UTF-16 name.
    #[doc(hidden)]
    pub fn update_iec_ld_arithmetic_function(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_arithmetic_function(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Change a captured three-operand IEC comparison block among EQ/GT/GE/LT/LE.
    #[doc(hidden)]
    pub fn update_iec_ld_comparison_function(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .update_iec_ld_comparison_function(program_index, offset, expected, replacement)
    }
    #[cfg(feature = "write")]
    /// Insert, replace, or delete a contact/coil record in the captured
    /// LD format. Unlike text clearing, deletion removes the actual record.
    /// Unsupported topology is rejected without changing the document.
    #[doc(hidden)]
    pub fn edit_ladder_cell(
        &mut self,
        program_index: usize,
        edit: &LadderCellEdit,
    ) -> Result<(), XgwxError> {
        self.doc.edit_ladder_cell(program_index, edit)
    }
    #[cfg(feature = "write")]
    /// Insert a fixed-arity catalog instruction at an XGK row's output end.
    #[doc(hidden)]
    pub fn insert_ladder_instruction(
        &mut self,
        program_index: usize,
        raw_y: u32,
        mnemonic: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_ladder_instruction(program_index, raw_y, mnemonic, operands)
    }
    #[cfg(feature = "write")]
    /// Insert a comparison contact spanning three cells at the selected column.
    #[doc(hidden)]
    pub fn insert_ladder_comparison(
        &mut self,
        program_index: usize,
        raw_y: u32,
        column: u8,
        mnemonic: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_ladder_comparison(program_index, raw_y, column, mnemonic, operands)
    }
    #[cfg(feature = "write")]
    /// Delete a verified XGK comparison contact, leaving its occupied wiring gap.
    #[doc(hidden)]
    pub fn delete_ladder_comparison(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_ladder_comparison(program_index, offset, expected)
    }
    #[cfg(feature = "write")]
    /// Delete a catalog application from an unbranched output row, including its feed wire.
    #[doc(hidden)]
    pub fn delete_ladder_instruction(
        &mut self,
        program_index: usize,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_ladder_instruction(program_index, offset, expected)
    }
    #[cfg(feature = "write")]
    /// Create or edit a native rung/output comment record.
    #[doc(hidden)]
    pub fn edit_ladder_comment(
        &mut self,
        program_index: usize,
        edit: &LadderCommentEdit,
    ) -> Result<(), XgwxError> {
        self.doc.edit_ladder_comment(program_index, edit)
    }
    #[cfg(feature = "write")]
    /// Delete a native rung-comment row and close the resulting coordinate gap.
    #[doc(hidden)]
    pub fn delete_ladder_rung_comment(
        &mut self,
        program_index: usize,
        raw_y: u32,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_ladder_rung_comment(program_index, raw_y, expected)
    }
    #[cfg(feature = "write")]
    /// Add or remove a supported vertical connection between adjacent rows.
    #[doc(hidden)]
    pub fn edit_ladder_branch(
        &mut self,
        program_index: usize,
        edit: &LadderBranchEdit,
    ) -> Result<(), XgwxError> {
        self.doc.edit_ladder_branch(program_index, edit)
    }
    #[cfg(feature = "write")]
    /// Insert a physical blank row before the selected row, as native Ctrl+L.
    #[doc(hidden)]
    pub fn insert_ladder_row(&mut self, program_index: usize, raw_y: u32) -> Result<(), XgwxError> {
        self.doc.insert_ladder_row(program_index, raw_y)
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
    #[cfg(feature = "write")]
    /// Edit regular source-program declarations using the captured IEC PB50 encoder.
    #[doc(hidden)]
    pub fn edit_text_variable(
        &mut self,
        expected_object_id: &str,
        patch: &crate::SfcVariablePatch,
    ) -> Result<(), XgwxError> {
        self.doc.edit_text_variable(expected_object_id, patch)
    }
    /// Decode regular IEC declarations while retaining the two captured SFC system records.
    pub fn sfc_variables(
        &self,
        program_index: usize,
    ) -> Result<Vec<SfcVariable>, crate::XgwxError> {
        self.doc.sfc_variables(program_index)
    }
    #[cfg(feature = "write")]
    /// Insert, update, or remove captured program-local SFC declarations.
    #[doc(hidden)]
    pub fn edit_sfc_variable(&mut self, patch: &SfcVariablePatch) -> Result<(), crate::XgwxError> {
        self.doc.edit_sfc_variable(patch)
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
    /// Update one global variable, rebuilding bounded name/comment strings.
    #[cfg(feature = "write")]
    pub fn update_variable(
        &mut self,
        variable_index: usize,
        patch: &VariablePatch,
    ) -> Result<(), XgwxError> {
        self.doc.update_variable(variable_index, patch)
    }
    #[cfg(feature = "write")]
    /// Change a captured IEC local bit or numeric memory mapping.
    /// PB50 stores the address text, bit offset, and allocation width.
    /// Existing mappings keep their address area. Clearing or assigning a mapping
    /// also updates its storage class and allocation metadata.
    #[doc(hidden)]
    pub fn update_iec_local_symbol_address(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_address: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc.update_iec_local_symbol_address(
            program_index,
            symbol_index,
            expected_name,
            expected_address,
            replacement,
        )
    }
    #[cfg(feature = "write")]
    /// Rename one IEC program-local symbol and every captured LD operand or
    /// matching function-instance marker. Reject unknown reference records so
    /// the edit stays atomic.
    #[doc(hidden)]
    pub fn rename_iec_local_symbol(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .rename_iec_local_symbol(program_index, symbol_index, expected_name, replacement)
    }
    #[cfg(feature = "write")]
    /// Clone one captured IEC function-instance declaration into another
    /// program, assigning an unused automatic-memory range in that program.
    /// The source must contain a block using the declaration's function type.
    #[doc(hidden)]
    pub fn copy_iec_instance_declaration_to_program(
        &mut self,
        source_program_index: usize,
        source_instance: &str,
        destination_program_index: usize,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        self.doc.copy_iec_instance_declaration_to_program(
            source_program_index,
            source_instance,
            destination_program_index,
            new_instance,
        )
    }
    #[cfg(feature = "write")]
    /// Edit one captured IEC program-local symbol description. The PB50
    /// description is a length-prefixed UTF-16 field, independent of program
    /// operands and allocation metadata.
    #[doc(hidden)]
    pub fn update_iec_local_symbol_description(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_description: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc.update_iec_local_symbol_description(
            program_index,
            symbol_index,
            expected_name,
            expected_description,
            replacement,
        )
    }
    #[cfg(feature = "write")]
    /// Insert an unallocated primitive variable in a captured IEC program's
    /// local PB50 table. XG5000 sorts this table by case-insensitive name.
    #[doc(hidden)]
    pub fn insert_iec_local_symbol(
        &mut self,
        program_index: usize,
        name: &str,
        data_type: &str,
        description: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .insert_iec_local_symbol(program_index, name, data_type, description)
    }
    #[cfg(feature = "write")]
    /// Remove an IEC program-local PB50 record when the program does not
    /// contain a marker string referring to that symbol.
    #[doc(hidden)]
    pub fn delete_iec_local_symbol(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.doc
            .delete_iec_local_symbol(program_index, symbol_index, expected_name)
    }
    #[cfg(feature = "write")]
    /// Change a program-local automatic variable's primitive IEC type.
    /// XG5000 clears its allocation on a type change, even when the new
    /// type has the same storage width. Program references are retained.
    #[doc(hidden)]
    pub fn update_iec_local_symbol_type(
        &mut self,
        program_index: usize,
        symbol_index: usize,
        expected_name: &str,
        expected_type: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.doc.update_iec_local_symbol_type(
            program_index,
            symbol_index,
            expected_name,
            expected_type,
            replacement,
        )
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
