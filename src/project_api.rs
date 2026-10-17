#![deny(missing_docs)]
//! Project metadata, read-only option records, and captured-name renaming.
use crate::{ProjectInfo, ProjectOptionsSummary, XgwxDocument};
/// Borrowed project metadata and option records.
/// Legacy root readers are private; use this view for project metadata.
/// ```compile_fail
/// # use xgwx::XgwxDocument;
/// fn read(doc: &XgwxDocument) { let _ = doc.project_info(); }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ProjectView<'a> {
    doc: &'a XgwxDocument,
}
impl XgwxDocument {
    /// Borrow root project metadata and options.
    pub fn project(&self) -> ProjectView<'_> {
        ProjectView { doc: self }
    }
}
impl<'a> ProjectView<'a> {
    /// Root project metadata, including all preserved native attributes.
    pub fn info(self) -> ProjectInfo {
        self.doc.project_info()
    }
    /// Whether the captured workspace header has a project password.
    pub fn password_protected(self) -> Result<bool, crate::XgwxError> {
        self.doc.project_password_is_set()
    }
    /// Direct project name text, if nonempty.
    pub fn name(self) -> Option<&'a str> {
        self.doc.project_name()
    }
    /// Parsed option entries and their preserved raw encoding.
    /// Options are read-only until native option writes are validated.
    pub fn options(self) -> Option<ProjectOptionsSummary> {
        self.doc.project_options()
    }
}
#[cfg(feature = "write")]
mod editors {
    use super::*;
    use crate::XgwxError;
    /// Exclusive project editor. Other metadata and option fields remain read-only.
    pub struct ProjectEditor<'a> {
        doc: &'a mut XgwxDocument,
    }
    impl XgwxDocument {
        /// Exclusively borrow the project for supported metadata edits.
        pub fn project_mut(&mut self) -> ProjectEditor<'_> {
            ProjectEditor { doc: self }
        }
        pub(crate) fn ensure_consistent_api_tree(&self) -> Result<(), XgwxError> {
            if crate::parse_xml(&self.xml)? != self.root {
                return Err(XgwxError::DocumentEdit(
                    "XML and parsed tree disagree; reparse the document".into(),
                ));
            }
            Ok(())
        }
    }
    impl ProjectEditor<'_> {
        /// Set, change, or remove the project password. `None` removes it.
        /// Existing protection requires the current password.
        pub fn set_password(
            &mut self,
            current: Option<&str>,
            replacement: Option<&str>,
        ) -> Result<(), XgwxError> {
            self.doc.set_project_password(current, replacement)
        }
        /// Inspect current project metadata and options.
        pub fn view(&self) -> ProjectView<'_> {
            ProjectView { doc: self.doc }
        }
        /// Rename using the current name captured by this exclusive handle.
        /// Missing, ambiguous, blank and invalid names are rejected by the native writer.
        pub fn rename(&mut self, name: &str) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            let expected = self
                .view()
                .name()
                .ok_or_else(|| XgwxError::ProjectRename("project name is absent".into()))?
                .to_owned();
            self.rename_if(&expected, name)
        }
        /// Rename only if a previously inspected project name still matches.
        /// This form supports editor and MCP transactions with captured values.
        pub fn rename_if(&mut self, expected_name: &str, name: &str) -> Result<(), XgwxError> {
            self.doc.ensure_consistent_api_tree()?;
            self.doc.rename_project(expected_name, name)
        }
    }
}
#[cfg(feature = "write")]
pub use editors::ProjectEditor;
