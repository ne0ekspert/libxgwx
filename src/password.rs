//! Password support for the captured XG5000 4.82 workspace layout.
use crate::*;
use std::collections::BTreeSet;
use std::fmt;

/// The native password prompt to which an error belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordScope {
    /// Password used when opening a workspace file.
    Project,
    /// Password used when opening a program block.
    Program {
        /// Zero-based program index in document order.
        index: usize,
    },
}

/// Password errors never contain a supplied password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordError {
    /// A protected project or program needs a password.
    Required(PasswordScope),
    /// The supplied password did not match the stored value.
    Incorrect(PasswordScope),
    /// A new password must contain one to eight characters without NUL.
    InvalidLength,
    /// The password cannot be represented losslessly in the supported native encoding.
    UnsupportedCharacters,
    /// The workspace or program protection layout is not captured.
    UnsupportedLayout,
}
impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required(scope) => write!(f, "password required for {scope:?}"),
            Self::Incorrect(scope) => write!(f, "incorrect password for {scope:?}"),
            Self::InvalidLength => write!(f, "password must contain 1 to 8 characters and no NUL"),
            Self::UnsupportedCharacters => {
                write!(f, "password is not supported by the native encoding")
            }
            Self::UnsupportedLayout => write!(f, "unsupported native password layout"),
        }
    }
}
impl std::error::Error for PasswordError {}

fn fail(error: PasswordError) -> XgwxError {
    XgwxError::Password(error)
}

// Prefix of the native RC4 stream, confirmed by independent known-password captures.
const STREAM: [u8; 16] = [
    0x45, 0xdd, 0xbc, 0xde, 0xc3, 0xff, 0xa2, 0x1d, 0xb5, 0x75, 0xf6, 0xbb, 0xc0, 0xcd, 0x7a, 0x88,
];
fn encode(password: &str) -> Result<Vec<u8>, XgwxError> {
    if password.is_empty() || password.chars().count() > 8 || password.contains('\0') {
        return Err(fail(PasswordError::InvalidLength));
    }
    let (bytes, _, had_errors) = encoding_rs::EUC_KR.encode(password);
    if had_errors || bytes.len() > STREAM.len() {
        return Err(fail(PasswordError::UnsupportedCharacters));
    }
    let encoded: Vec<_> = bytes.iter().zip(STREAM).map(|(b, k)| b ^ k).collect();
    // Native fields are NUL-terminated. Do not create ambiguous passwords.
    if encoded.contains(&0) {
        return Err(fail(PasswordError::UnsupportedCharacters));
    }
    Ok(encoded)
}
fn verify(stored: &[u8], password: Option<&str>, scope: PasswordScope) -> Result<(), XgwxError> {
    if stored.is_empty() || stored.iter().all(|b| *b == 0) {
        return Ok(());
    }
    let password = password.ok_or_else(|| fail(PasswordError::Required(scope)))?;
    let encoded = encode(password)?;
    let mut expected = vec![0; stored.len()];
    if encoded.len() > expected.len() {
        return Err(fail(PasswordError::Incorrect(scope)));
    }
    expected[..encoded.len()].copy_from_slice(&encoded);
    // Compare the entire stored field, including its padding.
    if stored
        .iter()
        .zip(expected)
        .fold(0, |diff, (a, b)| diff | (a ^ b))
        != 0
    {
        return Err(fail(PasswordError::Incorrect(scope)));
    }
    Ok(())
}
fn native_header(header: &XgwxHeader) -> bool {
    (header.raw.len() == 138 || (header.raw.len() == 142 && header.raw[134..138] == [0xff; 4]))
        && header.gzip_offset == header.raw.len()
        && matches!(header.signature, [b'X', b'G', 0, 4, 2 | 3, 0])
        && header.label.as_deref() == Some("XG5000 WORKSPACE FILE")
}
fn program_id(program: &XmlElement) -> Result<&str, XgwxError> {
    program
        .attribute("ObjectId")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| fail(PasswordError::UnsupportedLayout))
}
fn program_password(program: &XmlElement) -> Result<Vec<u8>, XgwxError> {
    let value = program.attribute("Encrytption").unwrap_or("");
    if value.len() > 32 || value.len() % 2 != 0 || !value.is_ascii() {
        return Err(fail(PasswordError::UnsupportedLayout));
    }
    let bytes: Vec<u8> = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(s, 16).map_err(|_| fail(PasswordError::UnsupportedLayout))
        })
        .collect::<Result<_, _>>()?;
    if bytes.contains(&0) {
        return Err(fail(PasswordError::UnsupportedLayout));
    }
    Ok(bytes)
}

impl XgwxDocument {
    /// Reparse bytes in an authenticated session without retaining plaintext
    /// passwords. Authentication transfers only while stored credentials match;
    /// program authentication also requires the same unique native identity.
    pub fn parse_in_session(bytes: &[u8], previous: &Self) -> Result<Self, XgwxError> {
        let mut next = Self::parse_container(bytes)?;
        if next.verify_project_password(None).is_err()
            && (!native_header(&previous.header)
                || !native_header(&next.header)
                || next.header.raw[68..134] != previous.header.raw[68..134])
        {
            return Err(fail(PasswordError::Required(PasswordScope::Project)));
        }
        let previous_credentials = previous
            .root
            .descendants_named("Program")
            .filter_map(|program| {
                let id = program.attribute("ObjectId")?;
                if !previous.opened_programs.contains(id) {
                    return None;
                }
                Some((id, program.attribute("Encrytption")?))
            })
            .collect::<BTreeSet<_>>();
        let ids = next
            .root
            .descendants_named("Program")
            .filter_map(|program| {
                let id = program.attribute("ObjectId")?;
                let credential = program.attribute("Encrytption")?;
                (!credential.is_empty() && previous_credentials.contains(&(id, credential)))
                    .then(|| id.to_owned())
            })
            .collect();
        next.restore_open_programs(&ids)?;
        Ok(next)
    }
    /// Open a workspace using its project password. Program blocks remain locked
    /// until [`Self::unlock_program`] succeeds. Opening never removes a password.
    pub fn parse_with_password(bytes: &[u8], password: &str) -> Result<Self, XgwxError> {
        let doc = Self::parse_container(bytes)?;
        doc.verify_project_password(Some(password))?;
        Ok(doc)
    }
    /// Open a workspace file using its project password.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path_with_password(
        path: impl AsRef<std::path::Path>,
        password: &str,
    ) -> Result<Self, XgwxError> {
        let bytes = std::fs::read(path).map_err(XgwxError::Io)?;
        Self::parse_with_password(&bytes, password)
    }
    pub(crate) fn verify_project_password(&self, password: Option<&str>) -> Result<(), XgwxError> {
        if native_header(&self.header) {
            verify(&self.header.raw[68..134], password, PasswordScope::Project)
        } else {
            Ok(())
        }
    }
    pub(crate) fn project_password_is_set(&self) -> Result<bool, XgwxError> {
        if !native_header(&self.header) {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        Ok(self.header.raw[68..134].iter().any(|b| *b != 0))
    }
    pub(crate) fn program_is_locked(&self, index: usize) -> bool {
        self.program_element(index)
            .is_some_and(|program| self.program_element_is_locked(program))
    }
    // Bulk readers already hold the element; looking it up again by index would
    // restart the document traversal once for every program.
    pub(crate) fn program_element_is_locked(&self, program: &XmlElement) -> bool {
        !program.attribute("Encrytption").unwrap_or("").is_empty()
            && !program
                .attribute("ObjectId")
                .is_some_and(|id| self.opened_programs.contains(id))
    }
    pub(crate) fn require_open_program_element(
        &self,
        index: usize,
        program: &XmlElement,
    ) -> Result<(), XgwxError> {
        if self.program_element_is_locked(program) {
            return Err(fail(PasswordError::Required(PasswordScope::Program {
                index,
            })));
        }
        Ok(())
    }
    pub(crate) fn require_open_program(&self, index: usize) -> Result<(), XgwxError> {
        self.program_element(index).map_or(Ok(()), |program| {
            self.require_open_program_element(index, program)
        })
    }
    /// Authenticate a protected program, opening its real `Body2` source when
    /// the native layout uses a compatibility body.
    /// The session remembers only the program identity, never the password.
    /// Saving restores the native `Body`/`Body2` envelope and keeps protection.
    pub fn unlock_program(&mut self, index: usize, password: &str) -> Result<(), XgwxError> {
        let program = self
            .program_element(index)
            .ok_or(XgwxError::ProgramNotFound { index })?;
        let stored = program_password(program)?;
        verify(&stored, Some(password), PasswordScope::Program { index })?;
        if stored.is_empty() || !self.program_is_locked(index) {
            return Ok(());
        }
        let id = program_id(program)?.to_owned();
        if self
            .root
            .descendants_named("Program")
            .filter(|p| p.attribute("ObjectId") == Some(&id))
            .count()
            != 1
        {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        let xml = authenticated_program_xml(&self.xml, index)?;
        let root = parse_xml(&xml)?;
        self.xml = xml;
        self.root = root;
        self.opened_programs.insert(id);
        Ok(())
    }
    pub(crate) fn restore_open_programs(
        &mut self,
        ids: &BTreeSet<String>,
    ) -> Result<(), XgwxError> {
        if ids.is_empty() {
            return Ok(());
        }
        let indices = self
            .root
            .descendants_named("Program")
            .enumerate()
            .filter_map(|(index, program)| {
                (program
                    .attribute("ObjectId")
                    .is_some_and(|id| ids.contains(id))
                    && !program.attribute("Encrytption").unwrap_or("").is_empty())
                .then_some(index)
            })
            .collect::<Vec<_>>();
        for index in indices {
            let p = self.program_element(index).unwrap();
            if p.attribute("ObjectId").is_some_and(|id| ids.contains(id))
                && !p.attribute("Encrytption").unwrap_or("").is_empty()
            {
                let id = program_id(p)?.to_owned();
                if self
                    .root
                    .descendants_named("Program")
                    .filter(|p| p.attribute("ObjectId") == Some(&id))
                    .count()
                    != 1
                {
                    return Err(fail(PasswordError::UnsupportedLayout));
                }
                let xml = authenticated_program_xml(&self.xml, index)?;
                self.root = parse_xml(&xml)?;
                self.xml = xml;
                self.opened_programs.insert(id);
            }
        }
        Ok(())
    }
    #[cfg(feature = "write")]
    pub(crate) fn password_serialized_xml(&self) -> Result<std::borrow::Cow<'_, str>, XgwxError> {
        if self.opened_programs.is_empty() {
            return Ok(std::borrow::Cow::Borrowed(&self.xml));
        }
        let mut xml = self.xml.clone();
        for (index, p) in self.root.descendants_named("Program").enumerate() {
            if p.attribute("ObjectId")
                .is_some_and(|id| self.opened_programs.contains(id))
                && !p.attribute("Encrytption").unwrap_or("").is_empty()
            {
                xml = authenticated_program_xml(&xml, index)?;
            }
        }
        Ok(std::borrow::Cow::Owned(xml))
    }
    #[cfg(feature = "write")]
    fn update_protection_header(&mut self) -> Result<(), XgwxError> {
        if !native_header(&self.header) {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        let xml = roxmltree::Document::parse(&self.xml).map_err(XgwxError::Xml)?;
        let protected = xml.descendants().any(|n| {
            (n.has_tag_name("Program") || n.has_tag_name("UserFunction"))
                && n.attribute("Encrytption").is_some_and(|v| !v.is_empty())
        });
        let mut raw = self.header.raw.clone();
        if protected && raw.len() == 138 {
            raw.splice(134..134, [0xff; 4]);
        }
        if !protected && raw.len() == 142 {
            raw.drain(134..138);
        }
        let len = raw.len();
        self.header = XgwxHeader::parse(&raw, len);
        let checksum = crate::writer::workspace_checksum(
            &raw,
            self.header.compressed_size_hint.unwrap(),
            &self.main_gzip,
        );
        self.header.raw[64..68].copy_from_slice(&checksum.to_le_bytes());
        Ok(())
    }
    #[cfg(feature = "write")]
    pub(crate) fn set_project_password(
        &mut self,
        current: Option<&str>,
        replacement: Option<&str>,
    ) -> Result<(), XgwxError> {
        self.project_password_is_set()?;
        self.verify_project_password(current)?;
        let encoded = replacement.map(encode).transpose()?.unwrap_or_default();
        let mut next = self.clone();
        next.header.raw[68..134].fill(0);
        next.header.raw[68..68 + encoded.len()].copy_from_slice(&encoded);
        let aligned_len = next
            .header
            .compressed_size_hint
            .ok_or_else(|| fail(PasswordError::UnsupportedLayout))?;
        let checksum =
            crate::writer::workspace_checksum(&next.header.raw, aligned_len, &next.main_gzip);
        next.header.raw[64..68].copy_from_slice(&checksum.to_le_bytes());
        next.to_verified_bytes()?;
        *self = next;
        Ok(())
    }
    #[cfg(feature = "write")]
    pub(crate) fn set_program_password(
        &mut self,
        index: usize,
        current: Option<&str>,
        replacement: Option<&str>,
    ) -> Result<(), XgwxError> {
        let program = self
            .program_element(index)
            .ok_or(XgwxError::ProgramNotFound { index })?;
        let stored = program_password(program)?;
        verify(&stored, current, PasswordScope::Program { index })?;
        let encoded = replacement.map(encode).transpose()?.unwrap_or_default();
        let id = program_id(program)?.to_owned();
        if self
            .root
            .descendants_named("Program")
            .filter(|p| p.attribute("ObjectId") == Some(&id))
            .count()
            != 1
        {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        let mut next = self.clone();
        if !stored.is_empty() && next.program_is_locked(index) {
            next.unlock_program(index, current.unwrap())?;
        }
        let document = roxmltree::Document::parse(&next.xml).map_err(XgwxError::Xml)?;
        let p = document
            .descendants()
            .filter(|n| n.has_tag_name("Program"))
            .nth(index)
            .unwrap();
        let attr = p
            .attribute_node("Encrytption")
            .ok_or_else(|| fail(PasswordError::UnsupportedLayout))?;
        let hex: String = encoded.iter().map(|b| format!("{b:02X}")).collect();
        let mut changes = vec![(attr.range_value(), hex)];
        let bodies: Vec<_> = p.children().filter(|n| n.has_tag_name("Body")).collect();
        let backups: Vec<_> = p.children().filter(|n| n.has_tag_name("Body2")).collect();
        if bodies.len() != 1 {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        if stored.is_empty() && !encoded.is_empty() {
            if !backups.is_empty() {
                return Err(fail(PasswordError::UnsupportedLayout));
            }
            if !single_body_protection(p) {
                if next.configurations().len() != 1
                    || next
                        .configuration()
                        .get(0)?
                        .cpu()
                        .is_none_or(|c| c.family != "XGK")
                    || p.attribute("Kind") != Some("0")
                {
                    return Err(fail(PasswordError::UnsupportedLayout));
                }
                let body = &next.xml[bodies[0].range()];
                let opening_end = body
                    .find('>')
                    .ok_or_else(|| fail(PasswordError::UnsupportedLayout))?;
                let backup = format!(
                    "<Body2{}>{}</Body2>",
                    &body[5..opening_end],
                    include_str!("password_defaults/xgk-ld.xml")
                );
                let end = bodies[0].range().end;
                changes.push((end..end, backup));
            }
            next.opened_programs.insert(id.clone());
        } else if !stored.is_empty() && encoded.is_empty() {
            if backups.len() == 1 {
                changes.push((backups[0].range(), String::new()));
            } else if !backups.is_empty() || !single_body_protection(p) {
                return Err(fail(PasswordError::UnsupportedLayout));
            }
            next.opened_programs.remove(&id);
        }
        next.apply_xml_replacements(changes)?;
        next.update_protection_header()?;
        next.to_verified_bytes()?;
        *self = next;
        Ok(())
    }
}

// IEC programs retain their actual body when protected. Restrict the
// single-body form to captured native languages and their expected body type.
fn single_body_protection(program: roxmltree::Node<'_, '_>) -> bool {
    let children: Vec<_> = program
        .children()
        .filter(|n| n.has_tag_name("Body"))
        .collect();
    if children.len() != 1 || children[0].children().filter(|n| n.is_element()).count() != 1 {
        return false;
    }
    let Some(configuration) = program
        .ancestors()
        .find(|n| n.has_tag_name("Configuration"))
    else {
        return false;
    };
    let Some(cpu) = configuration
        .attribute("Type")
        .and_then(|v| v.parse().ok())
        .and_then(crate::cpu::cpu_for_type)
    else {
        return false;
    };
    cpu.family == "XGI"
        && match program.attribute("Kind") {
            Some("4" | "9") => children[0].children().any(|n| n.has_tag_name("ST_Program")),
            Some("3") => children[0]
                .children()
                .any(|n| n.has_tag_name("SFC_ProgramList")),
            Some("0") => children[0].children().any(|n| {
                n.has_tag_name("LDRoutine")
                    && n.children()
                        .filter(|c| c.has_tag_name("ProgramData"))
                        .count()
                        == 1
                    && n.children().any(|c| {
                        c.has_tag_name("ProgramData") && c.attribute("ProjectType") == Some("2")
                    })
            }),
            _ => false,
        }
}

fn authenticated_program_xml(xml: &str, index: usize) -> Result<String, XgwxError> {
    let document = roxmltree::Document::parse(xml).map_err(XgwxError::Xml)?;
    let program = document
        .descendants()
        .filter(|n| n.has_tag_name("Program"))
        .nth(index)
        .ok_or(XgwxError::ProgramNotFound { index })?;
    if !program.children().any(|n| n.has_tag_name("Body2")) && single_body_protection(program) {
        return Ok(xml.to_owned());
    }
    swap_bodies(xml, index)
}

fn swap_bodies(xml: &str, index: usize) -> Result<String, XgwxError> {
    let doc = roxmltree::Document::parse(xml).map_err(XgwxError::Xml)?;
    let p = doc
        .descendants()
        .filter(|n| n.has_tag_name("Program"))
        .nth(index)
        .ok_or(XgwxError::ProgramNotFound { index })?;
    let bodies: Vec<_> = p.children().filter(|n| n.has_tag_name("Body")).collect();
    let backups: Vec<_> = p.children().filter(|n| n.has_tag_name("Body2")).collect();
    if bodies.len() != 1 || backups.len() != 1 || bodies[0].range().end > backups[0].range().start {
        return Err(fail(PasswordError::UnsupportedLayout));
    }
    fn rename(body: &str, from: &str, to: &str) -> Result<String, XgwxError> {
        let closing = format!("</{from}>");
        if !body.ends_with(&closing) {
            return Err(fail(PasswordError::UnsupportedLayout));
        }
        Ok(format!(
            "<{to}{}</{to}>",
            &body[from.len() + 1..body.len() - closing.len()]
        ))
    }
    let first = rename(&xml[backups[0].range()], "Body2", "Body")?;
    let second = rename(&xml[bodies[0].range()], "Body", "Body2")?;
    let mut result = xml.to_owned();
    result.replace_range(backups[0].range(), &second);
    result.replace_range(bodies[0].range(), &first);
    Ok(result)
}
