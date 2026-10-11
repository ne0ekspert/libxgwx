#![deny(missing_docs)]
//! Selected global symbol records. Program-local declarations use program editors.
use crate::{VariableSummary, VariableView, XgwxDocument, XgwxError};

/// A global variable selected by its record index in the unique symbol table.
/// The document is borrowed; decoded metadata is captured once at selection.
#[derive(Debug, Clone)]
pub struct GlobalVariableRef<'a> {
    doc: &'a XgwxDocument,
    index: usize,
    summary: VariableSummary,
}
fn globals(doc: &XgwxDocument) -> Result<Vec<VariableSummary>, XgwxError> {
    if doc.root.descendants_named("Symbols").count() > 1 {
        return Err(XgwxError::GlobalVariableEdit(
            "multiple symbol tables are ambiguous; select a single-table document".into(),
        ));
    }
    doc.variables()
}
impl<'a> VariableView<'a> {
    /// Decode the unique global table into borrowed record handles in native order.
    /// Missing tables produce an empty list; malformed or multiple tables fail.
    pub fn list(self) -> Result<Vec<GlobalVariableRef<'a>>, XgwxError> {
        Ok(globals(self.doc)?
            .into_iter()
            .enumerate()
            .map(|(index, summary)| GlobalVariableRef {
                doc: self.doc,
                index,
                summary,
            })
            .collect())
    }
    /// Select a global variable by native record index.
    pub fn get(self, index: usize) -> Result<GlobalVariableRef<'a>, XgwxError> {
        let summary = globals(self.doc)?
            .into_iter()
            .nth(index)
            .ok_or(XgwxError::VariableNotFound { index })?;
        Ok(GlobalVariableRef {
            doc: self.doc,
            index,
            summary,
        })
    }
    /// Select a uniquely named global variable using the writer's case-insensitive matching.
    /// Missing and duplicate names fail rather than selecting an arbitrary record.
    pub fn named(self, name: &str) -> Result<GlobalVariableRef<'a>, XgwxError> {
        let mut matches = self.list()?.into_iter().filter(|variable| {
            variable
                .summary
                .name
                .as_deref()
                .is_some_and(|value| value.to_lowercase() == name.to_lowercase())
        });
        let selected = matches.next().ok_or_else(|| {
            XgwxError::GlobalVariableEdit(format!("no global variable named {name}"))
        })?;
        if matches.next().is_some() {
            return Err(XgwxError::GlobalVariableEdit(format!(
                "multiple global variables named {name}"
            )));
        }
        Ok(selected)
    }
}
impl<'a> GlobalVariableRef<'a> {
    /// Native record index, distinct from program-local symbol indices.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Decoded metadata captured at selection.
    pub fn summary(&self) -> &VariableSummary {
        &self.summary
    }
    /// Native variable name, when decoded.
    pub fn name(&self) -> Option<&str> {
        self.summary.name.as_deref()
    }
    /// Return to the document-wide variable namespace.
    pub fn variables(&self) -> VariableView<'a> {
        self.doc.variable()
    }
}
#[cfg(feature = "write")]
mod editors {
    use super::*;
    use crate::{VariableEditor, VariablePatch};
    /// Exclusive global variable editor with a bound record index and decoded snapshot.
    /// Updating names and descriptions may resize native strings; other string fields
    /// retain the backend's encoded-length restrictions. Insertion and deletion are unsupported.
    pub struct GlobalVariableEditor<'a> {
        doc: &'a mut XgwxDocument,
        index: usize,
        expected: VariableSummary,
    }
    impl<'a> VariableEditor<'a> {
        /// Select one global record, binding its index and current decoded values.
        /// Consume the collection borrow for `doc.variable_mut().get(index)?` chaining.
        pub fn get(self, index: usize) -> Result<GlobalVariableEditor<'a>, XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            let expected = self.doc.variable().get(index)?.summary().clone();
            Ok(GlobalVariableEditor {
                doc: self.doc,
                index,
                expected,
            })
        }
        /// Select only if previously inspected decoded metadata still matches.
        /// Use after I/O generation or another operation that can reorder global records.
        pub fn get_if(
            self,
            index: usize,
            expected: &VariableSummary,
        ) -> Result<GlobalVariableEditor<'a>, XgwxError> {
            let selected = self.get(index)?;
            if &selected.expected != expected {
                return Err(XgwxError::GlobalVariableEdit(
                    "stale global variable selection; inspect again".into(),
                ));
            }
            Ok(selected)
        }
        /// Select a unique global name without case distinctions.
        pub fn named(self, name: &str) -> Result<GlobalVariableEditor<'a>, XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            let index = self.doc.variable().named(name)?.index();
            self.get(index)
        }
    }
    impl GlobalVariableEditor<'_> {
        /// Bound native global record index.
        pub fn index(&self) -> usize {
            self.index
        }
        /// Current decoded metadata, refreshed after each successful update.
        pub fn summary(&self) -> &VariableSummary {
            &self.expected
        }
        /// Borrow the selected global variable for inspection.
        pub fn view(&self) -> GlobalVariableRef<'_> {
            GlobalVariableRef {
                doc: self.doc,
                index: self.index,
                summary: self.expected.clone(),
            }
        }
        /// Update this record atomically, preserving every other decoded global record.
        /// The native writer enforces duplicate-name, UTF-16 length and layout guards.
        pub fn update(&mut self, patch: &VariablePatch) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            let before = globals(self.doc)?;
            if before.get(self.index) != Some(&self.expected) {
                return Err(XgwxError::GlobalVariableEdit(
                    "stale global variable selection; inspect again".into(),
                ));
            }
            let mut candidate = self.doc.clone();
            candidate.update_variable(self.index, patch)?;
            let after = globals(&candidate)?;
            if before.len() != after.len()
                || before
                    .iter()
                    .zip(&after)
                    .enumerate()
                    .any(|(index, (a, b))| index != self.index && a != b)
            {
                return Err(XgwxError::GlobalVariableEdit(
                    "non-target global records changed".into(),
                ));
            }
            self.expected = after[self.index].clone();
            *self.doc = candidate;
            Ok(())
        }
        /// Rename the selected variable with the same native uniqueness and length guards.
        pub fn rename(&mut self, name: &str) -> Result<(), XgwxError> {
            self.update(&VariablePatch {
                name: Some(name.into()),
                ..VariablePatch::default()
            })
        }
    }
}
#[cfg(feature = "write")]
pub use editors::GlobalVariableEditor;
