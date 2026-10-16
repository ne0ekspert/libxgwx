#![deny(missing_docs)]
//! Selected programs borrow their document; summaries remain detached metadata.
use crate::*;

/// A shared view of one program, selected in document order.
#[derive(Debug, Clone, Copy)]
pub struct ProgramRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
}

impl<'a> ProgramView<'a> {
    /// Select one program without cloning the document.
    pub fn get(self, index: usize) -> Result<ProgramRef<'a>, XgwxError> {
        if self.doc.program_element(index).is_none() {
            return Err(XgwxError::ProgramNotFound { index });
        }
        Ok(ProgramRef {
            doc: self.doc,
            index,
        })
    }
}

impl ProgramRef<'_> {
    /// Return the zero-based program index bound to this view or editor.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Read current program metadata as a detached owned snapshot.
    pub fn summary(&self) -> ProgramSummary {
        self.doc
            .program_summary(self.index)
            .expect("selected program exists")
    }
    /// Decode the selected native ladder payload without borrowing its bytes.
    pub fn ladder(&self) -> Result<LadderProgramData, XgwxError> {
        self.doc
            .ladder_program(self.index)
            .ok_or(XgwxError::ProgramNotFound { index: self.index })?
    }
    /// Read standalone ST/IL source for this program; reject other languages.
    pub fn text(&self) -> Result<TextProgram, XgwxError> {
        self.doc.text_program(self.index).ok_or_else(|| {
            XgwxError::TextProgramEdit("selected program is not standalone ST/IL".into())
        })
    }
    /// Read the native SFC charts for this program; reject programs without SFC blocks.
    pub fn sfc(&self) -> Result<SfcProgram, XgwxError> {
        self.doc
            .sfc_program(self.index)
            .ok_or_else(|| XgwxError::SfcEdit("selected program is not SFC".into()))
    }
    /// Decode this program's local IEC symbol table.
    pub fn variables(&self) -> Result<Vec<IecLocalSymbol>, XgwxError> {
        self.doc
            .iec_local_symbols_for_program(self.index)
            .ok_or(XgwxError::ProgramNotFound { index: self.index })?
    }
    /// Decode this program's local declarations, including native system records.
    pub fn declarations(&self) -> Result<Vec<SfcVariable>, XgwxError> {
        self.doc.sfc_variables(self.index)
    }
}

/// An exclusively borrowed program with a captured identity.
/// Metadata updates preserve the identity. Deletion consumes this handle.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct Program<'a> {
    doc: &'a mut XgwxDocument,
    index: usize,
    object_id: String,
}

#[cfg(feature = "write")]
impl<'a> ProgramEditor<'a> {
    /// Select a program only when its native ObjectId matches a captured
    /// identity. Empty or stale identities leave the document unchanged.
    pub fn get_if(self, index: usize, expected_object_id: &str) -> Result<Program<'a>, XgwxError> {
        let program = self.get(index)?;
        if expected_object_id.is_empty() || program.object_id != expected_object_id {
            return Err(XgwxError::ProgramReorder(
                "selected program identity changed".into(),
            ));
        }
        Ok(program)
    }
    /// Select one editable program, binding its identity and document index.
    /// This consumes the collection borrow so `doc.program_mut().get(index)?`
    /// can be stored directly without an intermediate collection variable.
    pub fn get(self, index: usize) -> Result<Program<'a>, XgwxError> {
        let summary = self
            .doc
            .program_summary(index)
            .ok_or(XgwxError::ProgramNotFound { index })?;
        let object_id = summary.object_id.filter(|id| !id.is_empty()).ok_or(
            XgwxError::MissingProgramAttribute {
                index,
                attribute: "ObjectId",
            },
        )?;
        Ok(Program {
            doc: self.doc,
            index,
            object_id,
        })
    }
    /// Create a program in this collection.
    pub fn create(&mut self, patch: &NewProgram) -> Result<(), XgwxError> {
        self.doc.create_program(patch)
    }
}

#[cfg(feature = "write")]
impl Program<'_> {
    /// Create, rename or delete a native label in XGK or IEC ladder.
    pub fn edit_ld_label(&mut self, edit: &LadderLabelEdit) -> Result<(), XgwxError> {
        self.ensure_identity()?;
        self.doc.edit_ld_label(self.index, edit)
    }
    fn ensure_identity(&self) -> Result<(), XgwxError> {
        let program = self
            .doc
            .program_element(self.index)
            .ok_or(XgwxError::ProgramNotFound { index: self.index })?;
        if program.attribute("ObjectId") != Some(self.object_id.as_str()) {
            return Err(XgwxError::ProgramReorder(
                "selected program identity changed".into(),
            ));
        }
        Ok(())
    }
    fn reborrow(&mut self) -> Program<'_> {
        Program {
            doc: self.doc,
            index: self.index,
            object_id: self.object_id.clone(),
        }
    }
    /// Return the zero-based program index bound to this view or editor.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Borrow the native ObjectId captured when this program was selected.
    pub fn object_id(&self) -> &str {
        &self.object_id
    }
    /// Borrow a shared inspection view of the same selected program.
    pub fn view(&self) -> ProgramRef<'_> {
        ProgramRef {
            doc: self.doc,
            index: self.index,
        }
    }
    /// Read current program metadata as a detached owned snapshot.
    pub fn summary(&self) -> ProgramSummary {
        self.view().summary()
    }
    /// Update XML metadata using the existing bounded writer.
    pub fn update(&mut self, patch: &ProgramPatch) -> Result<(), XgwxError> {
        self.ensure_identity()?;
        self.doc.update_program(self.index, patch)
    }
    /// Rename the selected program using the guarded metadata writer.
    pub fn rename(&mut self, name: &str) -> Result<(), XgwxError> {
        self.update(&ProgramPatch {
            name: Some(name.into()),
            ..Default::default()
        })
    }
    /// Delete this program. The handle cannot be used afterwards.
    ///
    /// ```compile_fail
    /// fn delete_once(program: xgwx::Program<'_>) {
    ///     program.delete().unwrap();
    ///     let _ = program.summary();
    /// }
    /// ```
    pub fn delete(self) -> Result<(), XgwxError> {
        self.ensure_identity()?;
        self.doc.delete_program(self.index, &self.object_id)
    }
    /// Move to a final document index. This handle follows the same program.
    pub fn move_to(&mut self, to: usize) -> Result<(), XgwxError> {
        self.ensure_identity()?;
        let target = self
            .doc
            .program_summary(to)
            .ok_or(XgwxError::ProgramNotFound { index: to })?;
        let target_id = target.object_id.ok_or(XgwxError::MissingProgramAttribute {
            index: to,
            attribute: "ObjectId",
        })?;
        self.move_to_if(to, &target_id)
    }
    /// Move only if the destination still has the previously captured ObjectId.
    /// The handle follows its selected program after a successful move.
    pub fn move_to_if(&mut self, to: usize, expected_target_id: &str) -> Result<(), XgwxError> {
        self.ensure_identity()?;
        self.doc
            .move_program(self.index, to, &self.object_id, expected_target_id)?;
        self.index = to;
        Ok(())
    }
    /// Select the IEC ladder editor. XGK, SFC and ST/IL programs are rejected.
    pub fn iec_ld(&mut self) -> Result<IecLdEditor<'_>, XgwxError> {
        self.ensure_identity()?;
        if self.view().ladder()?.project_type != Some(2) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        Ok(IecLdEditor {
            program: self.reborrow(),
        })
    }
    /// Select the native XGK ladder editor.
    pub fn ladder(&mut self) -> Result<LadderEditor<'_>, XgwxError> {
        self.ensure_identity()?;
        if self.view().ladder()?.project_type != Some(1) {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        Ok(LadderEditor {
            program: self.reborrow(),
        })
    }
    /// Borrow the standalone ST/IL source editor; reject other program body types.
    pub fn text(&mut self) -> Result<TextEditor<'_>, XgwxError> {
        self.ensure_identity()?;
        self.view().text()?;
        Ok(TextEditor {
            program: self.reborrow(),
        })
    }
    /// Borrow the native SFC editor; reject programs without an SFC chart.
    pub fn sfc(&mut self) -> Result<SfcEditor<'_>, XgwxError> {
        self.ensure_identity()?;
        self.view().sfc()?;
        Ok(SfcEditor {
            program: self.reborrow(),
        })
    }
    /// Local IEC symbols and ST/SFC declarations for this program.
    pub fn variable_mut(&mut self) -> ProgramVariableEditor<'_> {
        ProgramVariableEditor {
            program: self.reborrow(),
        }
    }
    /// XGK IL projection. This is distinct from standalone IEC IL source.
    #[cfg(feature = "il")]
    pub fn vendor_il(&mut self) -> Result<VendorIlEditor<'_>, XgwxError> {
        self.ensure_identity()?;
        if !self
            .doc
            .vendor_il_programs()
            .iter()
            .any(|p| p.program_index == self.index)
        {
            return Err(XgwxError::UnsupportedLadderLayout);
        }
        Ok(VendorIlEditor {
            program: self.reborrow(),
        })
    }
}

/// IEC ladder operations bound to one program.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct IecLdEditor<'a> {
    program: Program<'a>,
}
/// Native XGK ladder operations bound to one program.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct LadderEditor<'a> {
    program: Program<'a>,
}
/// Standalone ST/IL source bound to one program.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct TextEditor<'a> {
    program: Program<'a>,
}
/// Native SFC charts bound to one program.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct SfcEditor<'a> {
    program: Program<'a>,
}
/// Program-local IEC symbols and declarations bound to one program.
#[cfg(feature = "write")]
#[derive(Debug)]
pub struct ProgramVariableEditor<'a> {
    program: Program<'a>,
}
/// XGK IL projection bound to one program.
#[cfg(all(feature = "write", feature = "il"))]
#[derive(Debug)]
pub struct VendorIlEditor<'a> {
    program: Program<'a>,
}
#[cfg(feature = "write")]
impl IecLdEditor<'_> {
    /// Return the zero-based program index bound to this view or editor.
    pub fn index(&self) -> usize {
        self.program.index
    }
    /// Decode the current IEC ladder body as an owned inspection snapshot.
    pub fn data(&self) -> Result<LadderProgramData, XgwxError> {
        self.program.view().ladder()
    }
    /// Give one captured IEC function block a separate instance by cloning its
    /// PB50 instance declaration, assigning a new automatic-memory range, and changing only that block's
    /// instance marker. This is useful after copying a network containing a
    /// stateful function such as R_TRIG.
    pub fn duplicate_function_instance(
        &mut self,
        block_offset: usize,
        expected_instance: &str,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.duplicate_iec_ld_function_instance(
            self.program.index,
            block_offset,
            expected_instance,
            new_instance,
        )
    }
    /// Replace the text of a captured IEC LD comment. XG5000 stores its UTF-16
    /// length in the marker byte; the surrounding record has no length field.
    /// Symbol and operand strings are deliberately excluded from this operation.
    pub fn update_comment(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .update_iec_ld_comment(self.program.index, offset, expected, replacement)
    }
    /// Replace the variable reference of a captured IEC LD rising-edge
    /// contact (`FF 08`).
    pub fn update_rising_contact_operand(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_rising_contact_operand(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Replace an operand in a captured IEC LD contact or output coil record.
    /// This accepts only the record codes identified by `iec_ld::element_operands`.
    /// Classified replacements must be BOOL; coil destinations must be writable.
    /// XG5000 must still validate unclassified expressions.
    pub fn update_element_operand(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_element_operand(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Change a captured IEC contact among the six addressed contact kinds.
    pub fn update_contact_kind(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_contact_kind(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Change a captured IEC coil among output, inverse, set, reset, rising,
    /// and falling kinds while preserving its operand and position.
    pub fn update_coil_kind(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .update_iec_ld_coil_kind(self.program.index, offset, expected, replacement)
    }
    /// Add or remove one captured IEC LD vertical branch segment between two
    /// adjacent rows that already belong to the same native row group.
    ///
    /// Removal preserves another connection between the rows, or deletes a
    /// supported lower branch row and shifts later rows as in native Delete Line.
    /// General group splitting remains unsupported.
    /// Addition is limited to rows without comments or function records and
    /// preserves the group's existing row envelope.
    pub fn edit_branch_segment(
        &mut self,
        group_index: usize,
        start_row_index: u16,
        end_row_index: u16,
        x: u8,
        expected: bool,
        present: bool,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_iec_ld_branch_segment(
            self.program.index,
            group_index,
            start_row_index,
            end_row_index,
            x,
            expected,
            present,
        )
    }
    /// Toggle a paired vertical wire while retaining both row envelopes and
    /// every other record. Open electrical endpoints are valid intermediate
    /// layouts; typed function bindings must remain intact. Addition joins two
    /// existing electrical points in the same group, including a deleted gap.
    pub fn edit_vertical_wire(
        &mut self,
        group_index: usize,
        start_row_index: u16,
        end_row_index: u16,
        x: u8,
        expected: bool,
        present: bool,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_iec_ld_vertical_wire(
            self.program.index,
            group_index,
            start_row_index,
            end_row_index,
            x,
            expected,
            present,
        )
    }
    /// Join consecutive stored IEC groups through a vertical wire between
    /// their adjacent boundary rows. Existing row and function records remain
    /// unchanged; the group envelopes are rebuilt and subsequent groups renumbered.
    /// The joined network is disabled if either source group is disabled.
    pub fn connect_groups(
        &mut self,
        upper_group_index: usize,
        expected_upper_row: u16,
        lower_group_index: usize,
        expected_lower_row: u16,
        x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.connect_iec_ld_groups(
            self.program.index,
            upper_group_index,
            expected_upper_row,
            lower_group_index,
            expected_lower_row,
            x,
        )
    }
    /// Extend a stored group's final row into the adjacent implicit blank row.
    /// The new row contains the vertical endpoint only; subsequent element
    /// placement can complete the circuit. Existing rows are not shifted.
    pub fn extend_vertical_wire(
        &mut self,
        group_index: usize,
        start_row_index: u16,
        x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.extend_iec_ld_vertical_wire(
            self.program.index,
            group_index,
            start_row_index,
            x,
        )
    }
    /// Separate a stored IEC group at an adjacent row boundary with no
    /// crossing wire, function body or function reference. Coordinates and
    /// all row-local records are preserved. Both groups inherit the current
    /// network execution setting.
    pub fn split_group(
        &mut self,
        group_index: usize,
        expected_upper_row: u16,
        expected_lower_row: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.split_iec_ld_group(
            self.program.index,
            group_index,
            expected_upper_row,
            expected_lower_row,
        )
    }
    /// Insert one implicit blank IEC LD row after `after_row_index`, matching
    /// XG5000 Ctrl+L on a blank row boundary. The payload does not gain a row
    /// record; later row envelopes and every decoded Y coordinate move down by
    /// one four-unit grid row.
    pub fn insert_blank_row(&mut self, after_row_index: u16) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .insert_iec_ld_blank_row(self.program.index, after_row_index)
    }
    /// Compatibility wrapper for a normally open parallel contact.
    pub fn insert_parallel_contact(
        &mut self,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_parallel_contact(
            self.program.index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            parallel_variable,
        )
    }
    /// Add an addressed contact in parallel with the leading contact of a
    /// captured one-row addressed contact/wire/output-coil rung. The operation creates
    /// a second row, connects it at the contact's right edge, and shifts later
    /// rows down. The complete two-row network is validated before writing.
    pub fn insert_parallel_contact_kind(
        &mut self,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
        parallel_kind: &str,
        parallel_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_parallel_contact_kind(
            self.program.index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
            parallel_kind,
            parallel_variable,
        )
    }
    /// Delete one implicit IEC LD blank row and move every later decoded Y
    /// coordinate up by one row, matching XG5000 Ctrl+D on an empty line.
    pub fn delete_blank_row(&mut self, blank_row_index: u16) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_ld_blank_row(self.program.index, blank_row_index)
    }
    /// Create the compatibility normally-open-contact to output-coil IEC LD
    /// rung. See [`Self::insert_rung`] for the other decoded kinds.
    pub fn insert_linear_rung(
        &mut self,
        blank_row_index: u16,
        contact_variable: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_linear_rung(
            self.program.index,
            blank_row_index,
            contact_variable,
            coil_variable,
        )
    }
    /// Create a native three-record IEC LD rung in one implicit blank row.
    /// The captured shape is an addressed BOOL contact at the left rail, a
    /// long wire, and a BOOL coil at the right rail. A new single-row group is
    /// inserted without moving any existing row, including after the last
    /// stored network, extending the stored row range across sparse gaps when necessary.
    pub fn insert_rung(
        &mut self,
        blank_row_index: u16,
        contact_kind: &str,
        contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_rung(
            self.program.index,
            blank_row_index,
            contact_kind,
            contact_variable,
            coil_kind,
            coil_variable,
        )
    }
    /// Insert one addressed contact or coil into an unoccupied IEC grid cell.
    /// No other element or wire is created. The resulting row may be incomplete.
    pub fn insert_single_element(
        &mut self,
        row_index: u16,
        raw_x: u8,
        category: &str,
        kind: &str,
        operand: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_single_element(
            self.program.index,
            row_index,
            raw_x,
            category,
            kind,
            operand,
        )
    }
    /// Delete the compatibility normally-open-contact to output-coil rung.
    /// See [`Self::delete_rung`] for the other decoded kinds.
    pub fn delete_linear_rung(
        &mut self,
        row_index: u16,
        expected_contact_variable: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_linear_rung(
            self.program.index,
            row_index,
            expected_contact_variable,
            expected_coil_variable,
        )
    }
    /// Delete one exact addressed-contact, wire, and coil single-row group,
    /// restoring its row to an implicit blank gap without moving other rows.
    pub fn delete_rung(
        &mut self,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_rung(
            self.program.index,
            row_index,
            expected_contact_kind,
            expected_contact_variable,
            expected_coil_kind,
            expected_coil_variable,
        )
    }
    /// Delete the terminal wire and coil from a captured IEC row.
    /// Keep its leading contact or incoming branch endpoint as the sole record.
    pub fn delete_terminal_coil(
        &mut self,
        coil_record_offset: usize,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_terminal_coil(
            self.program.index,
            coil_record_offset,
            expected_variable,
        )
    }
    /// Finish a contact-only one-row IEC rung with a native long wire and coil.
    /// The row must have the exact shape left by terminal-coil deletion.
    pub fn insert_terminal_coil(
        &mut self,
        contact_record_offset: usize,
        expected_contact_variable: &str,
        coil_kind: &str,
        coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_terminal_coil(
            self.program.index,
            contact_record_offset,
            expected_contact_variable,
            coil_kind,
            coil_variable,
        )
    }
    /// Delete one exact simple IEC LD rung and close its row, matching
    /// XG5000 Ctrl+D on an occupied line. Both edits are checked before the
    /// document is changed.
    pub fn delete_simple_row(
        &mut self,
        row_index: u16,
        expected_contact_kind: &str,
        expected_contact_variable: &str,
        expected_coil_kind: &str,
        expected_coil_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_simple_row(
            self.program.index,
            row_index,
            expected_contact_kind,
            expected_contact_variable,
            expected_coil_kind,
            expected_coil_variable,
        )
    }
    /// Delete the upper line of a captured two-row IEC branch. The lower
    /// contact-only line remains and all later rows move up, as in XG5000
    /// Ctrl+D on the upper branch line.
    pub fn delete_branch_top_row(
        &mut self,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_ld_branch_top_row(self.program.index, group_index, row_index)
    }
    /// Delete a contact-only middle line from a nested IEC branch, reconnecting
    /// the outer x3 branch and removing the two branches local to that line.
    pub fn delete_nested_contact_branch_row(
        &mut self,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_nested_contact_branch_row(
            self.program.index,
            group_index,
            row_index,
        )
    }
    /// Delete a contact-only middle line in a chain of x3 branches. The
    /// adjacent vertical segments become one segment across the removed line.
    pub fn delete_chained_contact_branch_row(
        &mut self,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_chained_contact_branch_row(
            self.program.index,
            group_index,
            row_index,
        )
    }
    /// Delete a middle IEC row that only carries an incoming and outgoing
    /// vertical branch. The two segments are joined across the row gap.
    pub fn delete_empty_branch_row(
        &mut self,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_ld_empty_branch_row(self.program.index, group_index, row_index)
    }
    /// Reproduce XG5000 Delete Line on the lower output row of the captured
    /// two-row FF branch. Native deletion also removes the connected FF block
    /// and its output reference from the upper row.
    pub fn delete_ff_branch_output_row(
        &mut self,
        group_index: usize,
        row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_ff_branch_output_row(
            self.program.index,
            group_index,
            row_index,
        )
    }
    /// Insert a normally open contact into a captured linear IEC LD wire.
    /// This compatibility wrapper delegates to [`Self::insert_contact`].
    pub fn insert_no_contact(
        &mut self,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_no_contact(
            self.program.index,
            wire_offset,
            raw_x,
            expected_wire_start_x,
            expected_wire_end_x,
            variable,
        )
    }
    /// Insert an addressed contact into the empty x1 cell of a captured
    /// two-row IEC branch upper line, matching XG5000 F3/F4 insertion.
    pub fn insert_leading_contact(
        &mut self,
        insertion_offset: usize,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_leading_contact(
            self.program.index,
            insertion_offset,
            contact_kind,
            variable,
        )
    }
    /// Insert one of the six addressed IEC contact kinds into a captured long
    /// wire. The record shape follows native XG5000 insertion and contact-kind
    /// captures. The operand must resolve to a BOOL symbol or device address.
    pub fn insert_contact(
        &mut self,
        wire_offset: usize,
        raw_x: u8,
        expected_wire_start_x: u8,
        expected_wire_end_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_contact(
            self.program.index,
            wire_offset,
            raw_x,
            expected_wire_start_x,
            expected_wire_end_x,
            contact_kind,
            variable,
        )
    }
    /// Replace one framed IEC short wire with an addressed contact. XG5000 F3
    /// preserves the row count and surrounding records for this cell edit.
    pub fn insert_short_wire_contact(
        &mut self,
        wire_offset: usize,
        expected_raw_x: u8,
        contact_kind: &str,
        variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_short_wire_contact(
            self.program.index,
            wire_offset,
            expected_raw_x,
            contact_kind,
            variable,
        )
    }
    /// Delete any decoded contact between two captured wire fragments, checking
    /// its expected kind before applying native XG5000 Delete semantics.
    pub fn delete_contact(
        &mut self,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_contact(
            self.program.index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
            expected_variable,
        )
    }
    /// Delete a decoded contact between two captured wire fragments.
    /// Native XG5000 Delete leaves both fragments in place and decrements the
    /// row record count. Retained for compatibility with the original NO/NC API.
    pub fn delete_no_contact(
        &mut self,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_no_contact(
            self.program.index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }
    /// Delete any decoded contact with native XG5000 Cell Delete semantics,
    /// checking its expected kind before applying the edit.
    pub fn delete_contact_cell(
        &mut self,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_contact_kind: &str,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_contact_cell(
            self.program.index,
            contact_offset,
            expected_raw_x,
            expected_contact_kind,
            expected_variable,
        )
    }
    /// Delete a decoded contact with native XG5000 Cell Delete semantics.
    /// A leading branch contact moves the next contact to x1 and puts a short
    /// wire at x4. Contacts between long wires shift later cells left while
    /// the output coil stays fixed. Retained for API compatibility.
    pub fn delete_no_contact_cell(
        &mut self,
        contact_offset: usize,
        expected_raw_x: u8,
        expected_variable: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_no_contact_cell(
            self.program.index,
            contact_offset,
            expected_raw_x,
            expected_variable,
        )
    }
    /// Remove a captured one-cell `FF 01` wire between two long wires.
    /// The resulting gap must be recognized by the guarded wire repair path.
    pub fn delete_horizontal_wire(
        &mut self,
        wire_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_horizontal_wire(
            self.program.index,
            wire_offset,
            expected_raw_x,
        )
    }
    /// Delete one framed IEC horizontal wire, retaining row headers and operands.
    pub fn delete_horizontal_wire_record(
        &mut self,
        offset: usize,
        expected_start_x: u8,
        expected_end_x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_horizontal_wire_record(
            self.program.index,
            offset,
            expected_start_x,
            expected_end_x,
        )
    }
    /// Remove an isolated contact or coil, leaving its physical row empty.
    pub fn delete_isolated_element(
        &mut self,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_ld_isolated_element(self.program.index, offset, expected)
    }
    /// Draw one native F5 cell from a scalar function BOOL output or extend
    /// its contiguous short-wire feed. A comparison OUT assignment is replaced
    /// by the wire; numeric destinations are preserved and cannot be wired.
    pub fn insert_function_output_wire(
        &mut self,
        block_offset: usize,
        expected_name: &str,
        pin_name: &str,
        start_x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_function_output_wire(
            self.program.index,
            block_offset,
            expected_name,
            pin_name,
            start_x,
        )
    }
    /// Reconnect a one-cell gap left by a captured IEC contact or trigger deletion.
    /// Native XG5000 F5 inserts an `FF 01` short-wire record, increments the
    /// row count, and resets the row header coordinate to the first contact.
    pub fn repair_horizontal_wire(
        &mut self,
        insertion_offset: usize,
        expected_raw_x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.repair_iec_ld_horizontal_wire(
            self.program.index,
            insertion_offset,
            expected_raw_x,
        )
    }
    /// Remove an output assignment without removing the pin, reference, body,
    /// or physical row. Inputs cannot be cleared through this API.
    pub fn delete_function_output_operand(
        &mut self,
        offset: usize,
        expected: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_ld_function_output_operand(self.program.index, offset, expected)
    }
    /// Assign a previously empty OUT pin. Reuse the existing operand validator
    /// on a private candidate so a bad type or constant never changes the document.
    pub fn assign_function_output_operand(
        &mut self,
        block_offset: usize,
        expected_name: &str,
        ordinal: u8,
        value: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.assign_iec_ld_function_output_operand(
            self.program.index,
            block_offset,
            expected_name,
            ordinal,
            value,
        )
    }
    /// Replace a captured IEC LD function input/output expression (`FF 46`).
    /// Classified symbols and literals are checked against the decoded pin type;
    /// output pins also require a writable destination.
    pub fn update_function_operand(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_function_operand(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Delete a scalar from a horizontal chain or a captured staggered MOVE layout,
    /// preserving neighboring bodies and pin rows.
    pub fn delete_scalar_chain_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_scalar_chain_function(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Replace a supported scalar chain body atomically while preserving its neighbors.
    pub fn replace_scalar_chain_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.replace_iec_ld_scalar_chain_function(
            self.program.index,
            block_offset,
            expected_name,
            function_name,
            operands,
        )
    }
    /// Delete a terminal IEC function block using the captured XG5000 Delete
    /// shape. This removes its preceding wire and its otherwise empty pin rows,
    /// while retaining the leading contact as the only record in the group.
    pub fn delete_terminal_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_terminal_function(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Insert a terminal TON using an existing local instance declaration.
    pub fn insert_terminal_timer(
        &mut self,
        contact_offset: usize,
        instance: &str,
        preset: &str,
        elapsed: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_terminal_timer(
            self.program.index,
            contact_offset,
            instance,
            preset,
            elapsed,
        )
    }
    /// Restore a native two-pin MOVE after a retained terminal contact.
    /// Operand edits run on a clone, so the operation is atomic.
    pub fn insert_terminal_move(
        &mut self,
        contact_offset: usize,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_terminal_move(
            self.program.index,
            contact_offset,
            input_operand,
            output_operand,
        )
    }
    /// Delete a standalone IEC function group using the captured XG5000
    /// Delete shape. The group's stored rows become an implicit blank gap and
    /// all later group ordinals are decremented without changing row indices.
    /// A contiguous final group trims the total row count at its first row.
    pub fn delete_standalone_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_standalone_function(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Remove one complete decoded IEC LD network while leaving its row range
    /// as an implicit blank gap. Later network ordinals are renumbered; their
    /// rows, records, and coordinates remain unchanged.
    pub fn delete_group(
        &mut self,
        group_index: usize,
        expected_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_group(
            self.program.index,
            group_index,
            expected_first_row_index,
        )
    }
    /// Replace an occupied IEC LD network with a copy of another network in
    /// the same program. Both edits are checked on a private document, so a
    /// failed copy never leaves the destination deleted.
    pub fn replace_group(
        &mut self,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.replace_iec_ld_group(
            self.program.index,
            source_group_index,
            expected_source_first_row_index,
            destination_group_index,
            expected_destination_first_row_index,
        )
    }
    /// Replace a destination network with a network from another IEC program.
    /// Missing source locals and captured function instances can be copied in
    /// the same atomic edit when requested.
    pub fn replace_group_from_program(
        &mut self,
        source_program_index: usize,
        source_group_index: usize,
        expected_source_first_row_index: u16,
        destination_group_index: usize,
        expected_destination_first_row_index: u16,
        copy_missing_locals: bool,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.replace_iec_ld_group_from_program(
            source_program_index,
            source_group_index,
            expected_source_first_row_index,
            self.program.index,
            destination_group_index,
            expected_destination_first_row_index,
            copy_missing_locals,
        )
    }
    /// Move a complete decoded IEC LD network into an empty row range.
    /// Its internal branches, function pins, and operand links move together.
    pub fn move_group(
        &mut self,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.move_iec_ld_group(
            self.program.index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
        )
    }
    /// Copy a complete decoded IEC LD network into an empty row range,
    /// retaining the original group. The copied operands and function instance
    /// names can be edited separately after insertion.
    pub fn copy_group(
        &mut self,
        group_index: usize,
        expected_first_row_index: u16,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.copy_iec_ld_group(
            self.program.index,
            group_index,
            expected_first_row_index,
            destination_first_row_index,
        )
    }
    /// Copy a complete IEC network into an empty row range in another
    /// program. Referenced local symbols and function instances must already
    /// exist with compatible types in the destination program.
    pub fn copy_group_to_program(
        &mut self,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.copy_iec_ld_group_to_program(
            self.program.index,
            group_index,
            expected_first_row_index,
            destination_program_index,
            destination_first_row_index,
        )
    }
    /// Copy a network and create its missing primitive locals and captured
    /// function-instance declarations in the destination. Supported BOOL
    /// mappings retain their source address.
    /// A private document makes declaration insertion and network copying atomic.
    pub fn copy_group_to_program_with_locals(
        &mut self,
        group_index: usize,
        expected_first_row_index: u16,
        destination_program_index: usize,
        destination_first_row_index: u16,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.copy_iec_ld_group_to_program_with_locals(
            self.program.index,
            group_index,
            expected_first_row_index,
            destination_program_index,
            destination_first_row_index,
        )
    }
    /// Insert a native standalone IEC comment into an empty canvas row.
    /// No existing comment template is needed. Sparse tail rows extend the
    /// stored extent; failures leave the document intact.
    pub fn insert_comment(
        &mut self,
        destination_row_index: u16,
        text: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .insert_iec_ld_comment(self.program.index, destination_row_index, text)
    }
    /// Delete a scalar block on a branch row, retaining its branch scaffold.
    pub fn delete_branch_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_branch_function(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Replace a captured branch scalar function as one atomic document edit.
    /// A scaffold grows if needed and keeps surplus native branch rows; a completed
    /// branch tail uses only the replacement block's own pin rows.
    pub fn replace_branch_function(
        &mut self,
        block_offset: usize,
        expected_name: &str,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.replace_iec_ld_branch_function(
            self.program.index,
            block_offset,
            expected_name,
            function_name,
            operands,
        )
    }
    /// Place a scalar MOVE, arithmetic, or comparison function in available IEC cells.
    pub fn insert_function(
        &mut self,
        row_index: u16,
        raw_x: u8,
        function_name: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_function(
            self.program.index,
            row_index,
            raw_x,
            function_name,
            operands,
        )
    }
    /// Insert the captured standalone WORD_TO_UDINT shape at a three-row group gap.
    pub fn insert_standalone_function(
        &mut self,
        insertion_offset: usize,
        function_name: &str,
        input_operand: &str,
        output_operand: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_standalone_function(
            self.program.index,
            insertion_offset,
            function_name,
            input_operand,
            output_operand,
        )
    }
    /// Insert the captured connected single-output FF function cell into a
    /// guarded two-row gap. Both stored rows and their surrounding records are
    /// retained; one function record and one output-link record are added.
    pub fn insert_function_cell(
        &mut self,
        insertion_offset: usize,
        function_name: &str,
        instance_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_iec_ld_function_cell(
            self.program.index,
            insertion_offset,
            function_name,
            instance_name,
        )
    }
    /// Delete a captured IEC function cell while retaining unrelated records
    /// in the same stored rows. The block and all of its pin-link records are
    /// removed and each affected row's record count is decremented.
    pub fn delete_function_cell(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_function_cell(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete the first EQ in the captured four-block comparison chain.
    /// XG5000 closes the fourth row of that block and joins the x12 feed to
    /// the next EQ; deleting only the function records leaves a stale branch.
    pub fn delete_eq_chain_head(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_eq_chain_head(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete the first comparison block in the captured heating/cooling
    /// chain. Native Delete Line on its first pin row removes the four-row
    /// block body, joins the two long branch feeds, and shifts later rows.
    pub fn delete_heating_chain_head(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_head(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete one of the two captured middle EQ blocks in the heating chain.
    /// Native Delete Line on its first pin row keeps the x3/x15 branch feeds.
    pub fn delete_heating_chain_middle(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_middle(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete an x15-fed EQ in the captured heating comparison chain by its
    /// first pin row. The shape guard also accepts the next comparison after
    /// an earlier deletion has shifted its row index.
    pub fn delete_heating_chain_x15_eq(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_x15_eq(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete the captured contact-fed L62 EQ by its first pin row.
    /// XG5000 removes its x6/x12 contact branches and retains the x15 feed.
    pub fn delete_heating_chain_contact_eq(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_contact_eq(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete the x3-fed EQ and its orphan x15 feed as one valid edit.
    pub fn delete_heating_chain_x3_eq_repaired(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_x3_eq_repaired(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Reproduce XG5000 Delete Line on the L59 pin row of the L58 EQ.
    /// This native edit leaves one input/output error until the circuit is repaired.
    pub fn delete_heating_chain_x3_eq(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_heating_chain_x3_eq(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Remove the dangling x15 feed segments after deleting an x3-fed
    /// comparison. The unique orphan chain is found from decoded rows and
    /// branch endpoints, restoring a valid circuit while retaining the x3 feed.
    pub fn repair_heating_chain_x3_eq_deletion(&mut self) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .repair_iec_ld_heating_chain_x3_eq_deletion(self.program.index)
    }
    /// Delete a captured connected ADD/SUB in a five-row R_TRIG/arithmetic/MOVE group.
    /// XG5000 retains the R_TRIG and MOVE circuits as two separate groups.
    pub fn delete_connected_arithmetic(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_connected_arithmetic(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Delete a four-row scalar arithmetic block sharing an external branch spine.
    /// Matches native Delete Line on its final continuation row, retaining the
    /// spine and neighboring functions while shifting later rows up once.
    pub fn delete_branched_arithmetic(
        &mut self,
        block_offset: usize,
        expected_name: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.delete_iec_ld_branched_arithmetic(
            self.program.index,
            block_offset,
            expected_name,
        )
    }
    /// Change a captured three-operand IEC ADD/SUB/MUL/DIV block.
    /// The native capture shows a paired function opcode and UTF-16 name.
    pub fn update_arithmetic_function(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_arithmetic_function(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Change a captured three-operand IEC comparison block among EQ/GT/GE/LT/LE.
    pub fn update_comparison_function(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_ld_comparison_function(
            self.program.index,
            offset,
            expected,
            replacement,
        )
    }
    /// Apply a bounded browser record patch to this program.
    pub fn edit_browser(&mut self, patch: &BrowserIecPatch) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_browser_iec(self.program.index, patch)
    }
    /// Update an existing decoded cell text without changing topology.
    pub fn update_cell_text(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .update_ladder_cell_text(self.program.index, offset, expected, replacement)
    }
}
#[cfg(feature = "write")]
impl LadderEditor<'_> {
    /// Return the zero-based program index bound to this view or editor.
    pub fn index(&self) -> usize {
        self.program.index
    }
    /// Decode the current XGK ladder body as an owned inspection snapshot.
    pub fn data(&self) -> Result<LadderProgramData, XgwxError> {
        self.program.view().ladder()
    }
    /// Replace decoded ladder text. Recognized application instructions allow
    /// variable-length operands and catalog-backed mnemonic changes, synchronizing
    /// opcodes, decomposed strings, occupied cells and wires. Other records retain the
    /// same-length restriction to preserve their undecoded binary layout.
    pub fn update_cell_text(
        &mut self,
        offset: usize,
        expected: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .update_ladder_cell_text(self.program.index, offset, expected, replacement)
    }
    /// Delete an exact horizontal XGK wire without removing its row or elements.
    pub fn delete_horizontal_wire(
        &mut self,
        raw_y: u32,
        start_x: u8,
        end_x: u8,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_ladder_horizontal_wire(self.program.index, raw_y, start_x, end_x)
    }
    /// Delete an exact vertical XGK branch reference pair, preserving shared rows.
    pub fn delete_vertical_wire(
        &mut self,
        raw_x: u8,
        start_y: u32,
        end_y: u32,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_ladder_vertical_wire(self.program.index, raw_x, start_y, end_y)
    }
    /// Insert, replace, or delete a contact/coil record in the captured
    /// LD format. Unlike text clearing, deletion removes the actual record.
    /// Unsupported topology is rejected without changing the document.
    pub fn edit_cell(&mut self, edit: &LadderCellEdit) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_ladder_cell(self.program.index, edit)
    }
    /// Insert a fixed-arity catalog instruction at an XGK row's output end.
    pub fn insert_instruction(
        &mut self,
        raw_y: u32,
        mnemonic: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .insert_ladder_instruction(self.program.index, raw_y, mnemonic, operands)
    }
    /// Insert a comparison contact spanning three cells at the selected column.
    pub fn insert_comparison(
        &mut self,
        raw_y: u32,
        column: u8,
        mnemonic: &str,
        operands: &[String],
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.insert_ladder_comparison(
            self.program.index,
            raw_y,
            column,
            mnemonic,
            operands,
        )
    }
    /// Delete a verified XGK comparison contact, leaving its occupied wiring gap.
    pub fn delete_comparison(&mut self, offset: usize, expected: &str) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_ladder_comparison(self.program.index, offset, expected)
    }
    /// Delete a catalog application from an unbranched output row, including its feed wire.
    pub fn delete_instruction(&mut self, offset: usize, expected: &str) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_ladder_instruction(self.program.index, offset, expected)
    }
    /// Create or edit a native rung/output comment record.
    pub fn edit_comment(&mut self, edit: &LadderCommentEdit) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .edit_ladder_comment(self.program.index, edit)
    }
    /// Delete a native rung-comment row and close the resulting coordinate gap.
    pub fn delete_rung_comment(&mut self, raw_y: u32, expected: &str) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_ladder_rung_comment(self.program.index, raw_y, expected)
    }
    /// Add or remove a supported vertical connection between adjacent rows.
    pub fn edit_branch(&mut self, edit: &LadderBranchEdit) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .edit_ladder_branch(self.program.index, edit)
    }
    /// Insert a physical blank row before the selected row, as native Ctrl+L.
    pub fn insert_row(&mut self, raw_y: u32) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .insert_ladder_row(self.program.index, raw_y)
    }
}
#[cfg(feature = "write")]
impl ProgramVariableEditor<'_> {
    /// Return the zero-based program index bound to this view or editor.
    pub fn index(&self) -> usize {
        self.program.index
    }
    /// Change a captured IEC local bit or numeric memory mapping.
    /// PB50 stores the address text, bit offset, and allocation width.
    /// Existing mappings keep their address area. Clearing or assigning a mapping
    /// also updates its storage class and allocation metadata.
    pub fn update_address(
        &mut self,
        symbol_index: usize,
        expected_name: &str,
        expected_address: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_local_symbol_address(
            self.program.index,
            symbol_index,
            expected_name,
            expected_address,
            replacement,
        )
    }
    /// Rename one IEC program-local symbol and every captured LD operand or
    /// matching function-instance marker. Reject unknown reference records so
    /// the edit stays atomic.
    pub fn rename(
        &mut self,
        symbol_index: usize,
        expected_name: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.rename_iec_local_symbol(
            self.program.index,
            symbol_index,
            expected_name,
            replacement,
        )
    }
    /// Edit one captured IEC program-local symbol description. The PB50
    /// description is a length-prefixed UTF-16 field, independent of program
    /// operands and allocation metadata.
    pub fn update_description(
        &mut self,
        symbol_index: usize,
        expected_name: &str,
        expected_description: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_local_symbol_description(
            self.program.index,
            symbol_index,
            expected_name,
            expected_description,
            replacement,
        )
    }
    /// Insert an unallocated primitive variable in a captured IEC program's
    /// local PB50 table. XG5000 sorts this table by case-insensitive name.
    pub fn insert(
        &mut self,
        name: &str,
        data_type: &str,
        description: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .insert_iec_local_symbol(self.program.index, name, data_type, description)
    }
    /// Remove an IEC program-local PB50 record when the program does not
    /// contain a marker string referring to that symbol.
    pub fn delete(&mut self, symbol_index: usize, expected_name: &str) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program
            .doc
            .delete_iec_local_symbol(self.program.index, symbol_index, expected_name)
    }
    /// Change a program-local automatic variable's primitive IEC type.
    /// XG5000 clears its allocation on a type change, even when the new
    /// type has the same storage width. Program references are retained.
    pub fn update_type(
        &mut self,
        symbol_index: usize,
        expected_name: &str,
        expected_type: &str,
        replacement: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.update_iec_local_symbol_type(
            self.program.index,
            symbol_index,
            expected_name,
            expected_type,
            replacement,
        )
    }
    /// Decode the local IEC symbols of the selected program.
    pub fn list(&self) -> Result<Vec<IecLocalSymbol>, XgwxError> {
        self.program.view().variables()
    }
    /// Decode this program's local declarations, including native system records.
    pub fn declarations(&self) -> Result<Vec<SfcVariable>, XgwxError> {
        self.program.view().declarations()
    }
    /// Edit one declaration; the selected program supplies its index and identity.
    pub fn edit_declaration(&mut self, patch: &ProgramVariablePatch) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        let patch = SfcVariablePatch {
            program_index: self.program.index,
            expected_variables: patch.expected_variables.clone(),
            name: patch.name.clone(),
            data_type: patch.data_type.clone(),
            description: patch.description.clone(),
            remove: patch.remove,
            declaration: patch.declaration.clone(),
            update: patch.update,
        };
        if self
            .program
            .doc
            .text_programs()
            .iter()
            .any(|p| p.program_index == self.program.index)
        {
            self.program
                .doc
                .edit_text_variable(&self.program.object_id, &patch)
        } else {
            self.program.doc.edit_sfc_variable(&patch)
        }
    }
    /// Copy a declaration from this program to another program in this document.
    pub fn copy_instance_to_program(
        &mut self,
        instance: &str,
        destination_program_index: usize,
        new_instance: &str,
    ) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.copy_iec_instance_declaration_to_program(
            self.program.index,
            instance,
            destination_program_index,
            new_instance,
        )
    }
}

/// ST/IL source edit without a redundant program index or identity.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
pub struct TextSourcePatch {
    /// Exact previously read source, including line endings, used to reject stale edits.
    pub expected_source: String,
    /// Replacement source text; line endings are part of the stored value.
    pub source: String,
}

#[cfg(feature = "write")]
impl TextEditor<'_> {
    /// Read the current standalone source, editability metadata, and declarations.
    pub fn data(&self) -> Result<TextProgram, XgwxError> {
        self.program.view().text()
    }
    /// Read current standalone source, or `None` when the native payload is undecodable.
    pub fn source(&self) -> Result<Option<String>, XgwxError> {
        Ok(self.data()?.source)
    }
    /// Replace source only when the expected old source still matches; preserve program identity.
    pub fn update(&mut self, patch: &TextSourcePatch) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        let language = self.data()?.language;
        self.program.doc.edit_text_program(&TextProgramPatch {
            program_index: self.program.index,
            expected_object_id: self.program.object_id.clone(),
            expected_language: language,
            expected_source: patch.expected_source.clone(),
            source: patch.source.clone(),
        })
    }
}

/// SFC entity edit bound to the selected program.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
pub struct SfcEntityEdit {
    /// Zero-based block index within the selected SFC program.
    pub block_index: usize,
    /// Zero-based entity index within the selected SFC block.
    pub entity_index: usize,
    /// Previously read native entity type code.
    pub expected_type: u32,
    /// Previously read native entity row coordinate.
    pub expected_row: u32,
    /// Previously read native entity column coordinate.
    pub expected_column: u32,
    /// Supported entity property field name for the bounded SFC writer.
    pub field: String,
    /// Previously read field value that must still match before replacement.
    pub expected_value: String,
    /// Replacement value for the selected field.
    pub replacement: String,
}
/// SFC chart replacement bound to the selected program.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
pub struct SfcSequenceEdit {
    /// Zero-based block index within the selected SFC program.
    pub block_index: usize,
    /// Previously read entity list that must still match the chart.
    pub expected_entities: Vec<SfcEntity>,
    /// Replacement editable chart rows, in chart order.
    pub rows: Vec<SfcRow>,
    /// Previously read editable rows, or `None` when no editable row projection was available.
    pub expected_rows: Option<Vec<SfcRow>>,
}
#[cfg(feature = "write")]
impl SfcEditor<'_> {
    /// Read the current native SFC blocks and declaration metadata.
    pub fn data(&self) -> Result<SfcProgram, XgwxError> {
        self.program.view().sfc()
    }
    /// Edit a chart entity after checking its type, position, and expected field value.
    pub fn edit_entity(&mut self, patch: &SfcEntityEdit) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_sfc_entity(&SfcEntityPatch {
            program_index: self.program.index,
            block_index: patch.block_index,
            entity_index: patch.entity_index,
            expected_type: patch.expected_type,
            expected_row: patch.expected_row,
            expected_column: patch.expected_column,
            field: patch.field.clone(),
            expected_value: patch.expected_value.clone(),
            replacement: patch.replacement.clone(),
        })
    }
    /// Replace the selected chart after verifying its expected entities and editable rows.
    pub fn replace_sequence(&mut self, patch: &SfcSequenceEdit) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.replace_sfc_sequence(&SfcSequencePatch {
            program_index: self.program.index,
            block_index: patch.block_index,
            expected_entities: patch.expected_entities.clone(),
            rows: patch.rows.clone(),
            expected_rows: patch.expected_rows.clone(),
        })
    }
}

/// Local declaration edit without a redundant program index.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
pub struct ProgramVariablePatch {
    /// Previously read declaration list, including system records, used to reject stale edits.
    pub expected_variables: Vec<SfcVariable>,
    /// Name of the declaration or symbol.
    pub name: String,
    /// Native declaration type name, such as `BOOL`, `DINT`, or a supported instance type.
    pub data_type: String,
    /// Human-readable declaration or symbol description.
    pub description: String,
    /// Delete the named ordinary declaration; cannot be combined with `update`.
    pub remove: bool,
    /// Replacement array bounds, initial value, and retention settings.
    /// `None` uses default scalar metadata with no initial value or retention.
    pub declaration: Option<SfcDeclaration>,
    /// Modify the named existing declaration; false with `remove` false means insertion.
    pub update: bool,
}

/// Source edit for the native XGK IL projection.
#[cfg(all(feature = "write", feature = "il"))]
#[derive(Debug, Clone)]
pub struct VendorIlSourcePatch {
    /// Exact previously read source, including line endings, used to reject stale edits.
    pub expected_source: String,
    /// Replacement source text; line endings are part of the stored value.
    pub source: String,
}
#[cfg(all(feature = "write", feature = "il"))]
impl VendorIlEditor<'_> {
    /// Read the current textual XGK ladder IL projection and editability metadata.
    pub fn data(&self) -> Result<VendorIlProgram, XgwxError> {
        self.program
            .doc
            .vendor_il_programs()
            .into_iter()
            .find(|p| p.program_index == self.program.index)
            .ok_or(XgwxError::UnsupportedLadderLayout)
    }
    /// Replace source only when the expected old source still matches; preserve program identity.
    pub fn update(&mut self, patch: &VendorIlSourcePatch) -> Result<(), XgwxError> {
        self.program.ensure_identity()?;
        self.program.doc.edit_vendor_il(&VendorIlPatch {
            program_index: self.program.index,
            expected_object_id: self.program.object_id.clone(),
            expected_source: patch.expected_source.clone(),
            source: patch.source.clone(),
        })
    }
}
