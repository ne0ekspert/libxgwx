//! Native user-defined ST functions and stateful function blocks.
use crate::{XgwxDocument, XgwxError, XmlElement};

/// A user-defined function or function block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub enum UserPouKind {
    /// Stateless function with a return value.
    Function,
    /// Stateful function block, instantiated by a declaration.
    FunctionBlock,
}

/// Storage class of a native user-POU declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub enum UserPouScope {
    /// Private local variable.
    Local,
    /// Input parameter.
    Input,
    /// Output parameter.
    Output,
    /// Parameter passed by reference.
    InOut,
    /// Function return variable; its name matches the function.
    Return,
}
impl UserPouScope {
    #[cfg(feature = "write")]
    fn code(self) -> u32 {
        match self {
            Self::Local => 1,
            Self::Input => 3,
            Self::Output => 4,
            Self::InOut => 5,
            Self::Return => 10,
        }
    }
    fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            1 => Self::Local,
            3 => Self::Input,
            4 => Self::Output,
            5 => Self::InOut,
            10 => Self::Return,
            _ => return None,
        })
    }
    #[cfg(feature = "write")]
    fn order(self) -> u8 {
        match self {
            Self::Input => 0,
            Self::InOut => 1,
            Self::Return => 2,
            Self::Output => 3,
            Self::Local => 4,
        }
    }
}

/// One scalar parameter or local variable in a user-defined POU.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct UserPouVariable {
    /// IEC identifier, unique without regard to case within the POU.
    pub name: String,
    /// Captured scalar IEC primitive type name.
    pub data_type: String,
    /// Native parameter/local storage class.
    pub scope: UserPouScope,
    /// Human-readable declaration comment.
    #[cfg_attr(feature = "wasm", serde(default))]
    pub description: String,
}

/// Owned inspection snapshot. Unsupported native layouts remain visible.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct UserPou {
    /// Index in the independent user-function namespace, not a scan-program index.
    pub index: usize,
    /// Native identifier used to guard edits.
    pub object_id: String,
    /// User-defined type/call name.
    pub name: String,
    /// Native function/FB kind; absent for an unknown type.
    pub kind: Option<UserPouKind>,
    /// Native source language, when recognized.
    pub language: Option<String>,
    /// Decoded ST source, when supported.
    pub source: Option<String>,
    /// Function result type; absent for a function block.
    pub return_type: Option<String>,
    /// Declarations in native allocation/interface order, including the function result.
    pub variables: Vec<UserPouVariable>,
    /// Native definition comment.
    pub comment: String,
    /// Whether this definition has the captured editable ST layout.
    pub editable: bool,
    /// Reason a definition is read only.
    pub reason: Option<String>,
}

/// Create a native ST user function or function block.
#[cfg(feature = "write")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct NewUserPou {
    /// New unique IEC type/call name.
    pub name: String,
    /// Function or stateful function block.
    pub kind: UserPouKind,
    /// Unused native object GUID.
    pub object_id: String,
    /// Distinct unused symbol-table GUID.
    pub symbol_id: String,
    /// Required for a function, absent for an FB. The result declaration is added automatically.
    pub return_type: Option<String>,
    /// Parameters and locals; omit the automatically generated function result.
    pub variables: Vec<UserPouVariable>,
    /// ST body, without FUNCTION/VAR wrappers.
    pub source: String,
    /// Definition comment.
    #[cfg_attr(feature = "wasm", serde(default))]
    pub comment: String,
}

fn fail(reason: &str) -> XgwxError {
    XgwxError::UserPouEdit(reason.into())
}
fn kind(node: &XmlElement) -> Option<UserPouKind> {
    match node.attribute("Type") {
        Some("1") => Some(UserPouKind::Function),
        Some("2") => Some(UserPouKind::FunctionBlock),
        _ => None,
    }
}
fn variable_tag(k: UserPouKind) -> &'static str {
    match k {
        UserPouKind::Function => "UserFunctionVar",
        UserPouKind::FunctionBlock => "UserFunctionBlockVar",
    }
}
fn width(ty: &str) -> Option<u32> {
    Some(match ty {
        "BOOL" => 1,
        "BYTE" | "SINT" | "USINT" => 8,
        "WORD" | "INT" | "UINT" | "DATE" => 16,
        "DWORD" | "DINT" | "UDINT" | "REAL" | "TIME" | "TIME_OF_DAY" => 32,
        "LWORD" | "LINT" | "ULINT" | "LREAL" | "DATE_AND_TIME" => 64,
        _ => return None,
    })
}
fn word(bytes: &[u8], at: usize) -> Result<u32, XgwxError> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or_else(|| fail("truncated user-POU symbol record"))?
            .try_into()
            .unwrap(),
    ))
}
fn declarations(node: &XmlElement, k: UserPouKind) -> Result<Vec<UserPouVariable>, XgwxError> {
    let table = node
        .children
        .iter()
        .find(|n| n.name == variable_tag(k))
        .and_then(|n| n.children.iter().find(|n| n.name == "Symbols"))
        .ok_or_else(|| fail("missing user-POU symbol table"))?;
    let bytes =
        crate::decode_base64_payload(&table.text, table.attribute("Compressed") == Some("1"))?.data;
    let count = table
        .attribute("Count")
        .and_then(|n| n.parse::<usize>().ok())
        .ok_or_else(|| fail("invalid user-POU declaration count"))?;
    if count > 1024 {
        return Err(fail("too many user-POU declarations"));
    }
    let fields = crate::extract_utf16_marker_strings(&bytes, false, true);
    let marker = if k == UserPouKind::Function {
        "UF50"
    } else {
        "UB50"
    };
    let starts: Vec<_> = fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (f.value == marker).then_some(i))
        .collect();
    if starts.len() != count
        || (!bytes.is_empty()
            && (starts.first() != Some(&0)
                || fields[0].offset != 0
                || fields.last().unwrap().end_offset != bytes.len()))
    {
        return Err(fail("unsupported user-POU symbol framing"));
    }
    let mut out = Vec::new();
    for (i, start) in starts.iter().enumerate() {
        let f = &fields[*start..starts.get(i + 1).copied().unwrap_or(fields.len())];
        let function = k == UserPouKind::Function;
        if f.len() != if function { 5 } else { 8 } || f[2].offset != f[1].end_offset + 16 {
            return Err(fail("unsupported user-POU symbol layout"));
        }
        let header = f[1].end_offset;
        let scope = UserPouScope::from_code(word(&bytes, header)?)
            .ok_or_else(|| fail("unsupported user-POU declaration class"))?;
        let data_type = crate::iec_symbols::iec_primitive_type_name(word(&bytes, header + 4)?)
            .filter(|t| width(t).is_some())
            .ok_or_else(|| fail("unsupported user-POU declaration type"))?
            .to_owned();
        if word(&bytes, header + 8)? != 0
            || word(&bytes, header + 12)? != if scope == UserPouScope::Return { 4 } else { 0 }
        {
            return Err(fail("user-POU flags or triggers are unsupported"));
        }
        let alloc = if function { 3 } else { 5 };
        if f[alloc].value != if function { "S" } else { "B" }
            || f[alloc + 1].offset != f[alloc].end_offset + 32
            || word(&bytes, f[alloc].end_offset + 4)? != width(&data_type).unwrap()
            || bytes[f[alloc].end_offset + 12..f[alloc + 1].offset]
                .iter()
                .any(|b| *b != 0)
            || f.last().unwrap().value != "[2:0]"
            || (!function
                && (!f[2].value.is_empty() || !f[3].value.is_empty() || !f[6].value.is_empty()))
        {
            return Err(fail(
                "mapped, initialized, or extended user-POU declarations are read only",
            ));
        }
        let ordinal = word(&bytes, f[alloc].end_offset + 8)?;
        out.push((
            ordinal,
            UserPouVariable {
                name: f[1].value.clone(),
                data_type,
                scope,
                description: f[if function { 2 } else { 4 }].value.clone(),
            },
        ));
    }
    out.sort_by_key(|(i, _)| *i);
    Ok(out.into_iter().map(|(_, v)| v).collect())
}

// XG5000 regenerates these @Temp compiler records from the ST body.
fn supported_temporaries(node: &XmlElement, kind: UserPouKind) -> bool {
    let Some(count) = node
        .attribute("Count")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n <= 1024)
    else {
        return false;
    };
    if !node.children.is_empty() {
        return false;
    }
    if count == 0 {
        return node.text.trim().is_empty();
    }
    let Ok(payload) =
        crate::decode_base64_payload(&node.text, node.attribute("Compressed") == Some("1"))
    else {
        return false;
    };
    let fields = crate::extract_utf16_marker_strings(&payload.data, false, true);
    let marker = if kind == UserPouKind::Function {
        "UF50"
    } else {
        "UB50"
    };
    let starts: Vec<_> = fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (f.value == marker).then_some(i))
        .collect();
    starts.len() == count
        && starts.iter().all(|i| {
            fields
                .get(i + 1)
                .is_some_and(|f| f.value.starts_with("@Temp"))
        })
}

fn supported_interface(node: &XmlElement, kind: UserPouKind, vars: &[UserPouVariable]) -> bool {
    let Some(table) = node.children.iter().find(|n| n.name == variable_tag(kind)) else {
        return false;
    };
    if node.children.len() != 4
        || table.children.len() != 2
        || table.attribute("VARUseFBOnlineEdit") != Some("0")
        || node.attribute("UDFBUseFBOnlineEdit") != Some("0")
        || node.attribute("LibType") != Some("5")
    {
        return false;
    }
    if !table
        .children
        .iter()
        .any(|n| n.name == "TempVar" && supported_temporaries(n, kind))
    {
        return false;
    }
    let results: Vec<_> = vars
        .iter()
        .filter(|v| v.scope == UserPouScope::Return)
        .collect();
    match kind {
        UserPouKind::Function => {
            if results.len() != 1
                || results[0].name != node.text.trim()
                || table.attribute("FunName") != Some(node.text.trim())
                || table.attribute("ReturnType") != Some(results[0].data_type.as_str())
                || node.attribute("Return").and_then(|s| s.parse::<u32>().ok())
                    != crate::iec_symbols::iec_primitive_type_code(&results[0].data_type)
            {
                return false;
            }
        }
        UserPouKind::FunctionBlock => {
            if !results.is_empty() || table.attribute("FBInstRetain") != Some("0") {
                return false;
            }
        }
    }
    for (tag, input) in [("InputVariables", true), ("OutputVariables", false)] {
        let Some(list) = node.children.iter().find(|n| n.name == tag) else {
            return false;
        };
        let mut expected = vec![(
            if input { "EN" } else { "ENO" },
            if input { 0 } else { 2 },
            if input { 0x200001 } else { 1 },
        )];
        expected.extend(vars.iter().filter_map(|v| {
            let (class, mask) = match v.scope {
                UserPouScope::Input if input => (0, 0x200000),
                UserPouScope::InOut if input => (1, 0),
                UserPouScope::Output | UserPouScope::Return if !input => (2, 0),
                _ => return None,
            };
            Some((
                v.name.as_str(),
                class,
                mask | (1u32 << (crate::iec_symbols::iec_primitive_type_code(&v.data_type)? - 1)),
            ))
        }));
        if list.children.len() != expected.len() {
            return false;
        }
        for (actual, (name, class, ty)) in list.children.iter().zip(expected) {
            if actual.name != "Variable"
                || actual.text.trim() != name
                || actual.attribute("Type").and_then(|s| s.parse::<u32>().ok()) != Some(class)
                || actual
                    .attribute("DataType")
                    .and_then(|s| s.parse::<u32>().ok())
                    != Some(ty)
                || actual.attributes.len() != 6
                || actual.attribute("Trigger") != Some("0")
                || actual.attribute("UDTName") != Some("")
                || actual.attribute("Display") != Some("0")
                || actual.attribute("InstMemory") != Some("0")
            {
                return false;
            }
        }
    }
    true
}

/// Borrowed access to the independent user-function namespace.
pub struct UserPouView<'a> {
    doc: &'a XgwxDocument,
}
impl XgwxDocument {
    /// Inspect user-defined functions and function blocks independently of scan programs.
    pub fn user_pou(&self) -> UserPouView<'_> {
        UserPouView { doc: self }
    }
    /// Edit native user-defined ST functions and function blocks.
    #[cfg(feature = "write")]
    pub fn user_pou_mut(&mut self) -> UserPouEditor<'_> {
        UserPouEditor { doc: self }
    }
}
impl UserPouView<'_> {
    /// Whether native ST definition creation is captured for this document CPU.
    pub fn can_create(&self) -> bool {
        let configs = self.doc.configurations();
        configs.len() == 1
            && matches!(
                configs[0].type_code,
                Some(100 | 102 | 104 | 106 | 107 | 111)
            )
    }
    /// List definitions, retaining unknown layouts as read-only snapshots.
    pub fn list(&self) -> Vec<UserPou> {
        self.doc.root.descendants_named("UserFunction").enumerate().map(|(index,node)| {
            let kind=kind(node);
            let language=match node.attribute("Editor") {
                Some("4") => Some("ST"), Some("9") => Some("IL"),
                Some("0") => Some("LD"), Some("8") => Some("SFC"), _ => None,
            }.map(str::to_owned);
            let source=node.children.iter().find(|n|n.name=="Body").and_then(crate::sfc::read_st_source);
            let declarations=kind.ok_or_else(||fail("unknown user-POU kind")).and_then(|k|declarations(node,k));
            let return_type=declarations.as_ref().ok().and_then(|vars|vars.iter().find(|v|v.scope==UserPouScope::Return)).map(|v|v.data_type.clone());
            let editable=self.can_create() && language.as_deref()==Some("ST") && source.is_some()
                && node.attribute("Encrytption").unwrap_or("").is_empty()
                && node.attribute("En")==Some("1") && node.attribute("Eno")==Some("1")
                && node.attribute("ObjectId").is_some_and(|s|!s.is_empty())
                && kind.zip(declarations.as_ref().ok()).is_some_and(|(kind,vars)|supported_interface(node,kind,vars));
            UserPou {
                index, object_id:node.attribute("ObjectId").unwrap_or("").into(),
                name:node.text.trim().into(), kind, language, source, return_type,
                variables:declarations.unwrap_or_default(),comment:node.attribute("Comment").unwrap_or("").into(),
                editable,reason:(!editable).then(||"Only captured, unencrypted XGI ST definitions with EN/ENO and scalar declarations are editable.".into()),
            }
        }).collect()
    }
}

#[cfg(feature = "write")]
mod write {
    use super::*;
    use base64::Engine;

    fn identifier(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 32
            && name
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || i > 0 && c.is_ascii_digit())
    }
    fn guid(s: &str) -> bool {
        s.len() == 36
            && s.bytes().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            })
    }
    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\r', "&#xD;")
            .replace('\n', "&#xA;")
            .replace('\t', "&#x9;")
    }
    fn field(b: &mut Vec<u8>, s: &str) {
        b.extend([0xff, 0xfe, 0xff, s.encode_utf16().count() as u8]);
        b.extend(s.encode_utf16().flat_map(u16::to_le_bytes));
    }
    fn encoded(b: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(b)
    }
    fn code(source: &str) -> Result<String, XgwxError> {
        let bytes: Vec<_> = source.encode_utf16().flat_map(u16::to_le_bytes).collect();
        if bytes.len() > 131072 || source.contains('\0') {
            return Err(fail(
                "source must contain at most 65536 UTF-16 units and no NUL",
            ));
        }
        Ok(format!(
            "<CodeList CodeCount=\"{}\" Compressed=\"0\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\">{}</CodeList>",
            bytes.len() / 2,
            encoded(&bytes)
        ))
    }
    fn payload(
        name: &str,
        k: UserPouKind,
        vars: &[UserPouVariable],
    ) -> Result<(Vec<u8>, String, String, u32), XgwxError> {
        if vars.len() > 128 {
            return Err(fail("at most 128 declarations are supported"));
        }
        let mut names = std::collections::BTreeSet::new();
        let mut returns = 0;
        for v in vars {
            if !identifier(&v.name)
                || !names.insert(v.name.to_ascii_lowercase())
                || ["EN", "ENO"].iter().any(|n| v.name.eq_ignore_ascii_case(n))
                || width(&v.data_type).is_none()
                || v.description.encode_utf16().count() > 255
                || v.description.chars().any(char::is_control)
            {
                return Err(fail(
                    "use unique scalar IEC identifiers and valid declaration comments",
                ));
            }
            if v.scope == UserPouScope::Return {
                returns += 1;
                if k != UserPouKind::Function || v.name != name {
                    return Err(fail("the function result must have the function name"));
                }
            }
            if k == UserPouKind::Function
                && v.scope != UserPouScope::Return
                && v.name.eq_ignore_ascii_case(name)
            {
                return Err(fail("function name is reserved for its return variable"));
            }
        }
        if returns != usize::from(k == UserPouKind::Function) {
            return Err(fail(
                "a function requires exactly one result; an FB has no result variable",
            ));
        }
        let mut ordered: Vec<_> = vars.iter().collect();
        ordered.sort_by_key(|v| v.scope.order());
        let mut offset = 8u32;
        let mut records = Vec::new();
        let mut inputs = String::from(
            "<InputVariables><Variable Type=\"0\" DataType=\"2097153\" Trigger=\"0\" UDTName=\"\" Display=\"0\" InstMemory=\"0\">EN</Variable>",
        );
        let mut outputs = String::from(
            "<OutputVariables><Variable Type=\"2\" DataType=\"1\" Trigger=\"0\" UDTName=\"\" Display=\"0\" InstMemory=\"0\">ENO</Variable>",
        );
        for (ordinal, v) in ordered.iter().enumerate() {
            let bits = width(&v.data_type).unwrap();
            offset = offset.div_ceil(bits) * bits;
            let ty = crate::iec_symbols::iec_primitive_type_code(&v.data_type).unwrap();
            let mut b = Vec::new();
            field(
                &mut b,
                if k == UserPouKind::Function {
                    "UF50"
                } else {
                    "UB50"
                },
            );
            field(&mut b, &v.name);
            for n in [
                v.scope.code(),
                ty,
                0,
                if v.scope == UserPouScope::Return {
                    4
                } else {
                    0
                },
            ] {
                b.extend(n.to_le_bytes());
            }
            if k == UserPouKind::Function {
                field(&mut b, &v.description);
                field(&mut b, "S");
            } else {
                field(&mut b, "");
                field(&mut b, "");
                field(&mut b, &v.description);
                field(&mut b, "B");
            }
            for n in [offset, bits, ordinal as u32, 0, 0, 0, 0, 0] {
                b.extend(n.to_le_bytes());
            }
            if k == UserPouKind::FunctionBlock {
                field(&mut b, "");
            }
            field(&mut b, "[2:0]");
            records.push((v.name.to_ascii_lowercase(), b));
            offset += bits;
            let mask = 1u32 << (ty - 1);
            let (dest, cls, data) = match v.scope {
                UserPouScope::Input => (&mut inputs, 0, mask | 0x200000),
                UserPouScope::InOut => (&mut inputs, 1, mask),
                UserPouScope::Output | UserPouScope::Return => (&mut outputs, 2, mask),
                _ => continue,
            };
            dest.push_str(&format!("<Variable Type=\"{cls}\" DataType=\"{data}\" Trigger=\"0\" UDTName=\"\" Display=\"0\" InstMemory=\"0\">{}</Variable>",escape(&v.name)));
        }
        if vars
            .iter()
            .filter(|v| matches!(v.scope, UserPouScope::Input | UserPouScope::InOut))
            .count()
            > 64
            || vars
                .iter()
                .filter(|v| matches!(v.scope, UserPouScope::Output | UserPouScope::Return))
                .count()
                > 64
        {
            return Err(fail(
                "at most 64 input and 64 output parameters are supported",
            ));
        }
        inputs.push_str("</InputVariables>");
        outputs.push_str("</OutputVariables>");
        records.sort_by(|a, b| a.0.cmp(&b.0));
        Ok((
            records.into_iter().flat_map(|(_, b)| b).collect(),
            inputs,
            outputs,
            offset.div_ceil(64) * 64,
        ))
    }

    /// Exclusively borrowed collection of native user-defined POUs.
    pub struct UserPouEditor<'a> {
        pub(super) doc: &'a mut XgwxDocument,
    }
    /// Selected user-defined POU with a captured native identity.
    pub struct UserPouHandle<'a> {
        doc: &'a mut XgwxDocument,
        index: usize,
        object_id: String,
    }
    impl<'a> UserPouEditor<'a> {
        /// Select a definition only if its identity still matches the caller's snapshot.
        pub fn get(
            self,
            index: usize,
            expected_object_id: &str,
        ) -> Result<UserPouHandle<'a>, XgwxError> {
            let selected = self
                .doc
                .user_pou()
                .list()
                .into_iter()
                .nth(index)
                .ok_or_else(|| fail("user-defined POU is absent"))?;
            if expected_object_id.is_empty()
                || selected.object_id != expected_object_id
                || !selected.editable
            {
                return Err(fail(
                    "user-defined POU identity changed or its layout is read only",
                ));
            }
            Ok(UserPouHandle {
                doc: self.doc,
                index,
                object_id: expected_object_id.into(),
            })
        }
        /// Append a new ST function/FB without changing existing program indices or definitions.
        pub fn create(&mut self, request: &NewUserPou) -> Result<(), XgwxError> {
            if !identifier(&request.name)
                || !guid(&request.object_id)
                || !guid(&request.symbol_id)
                || request.object_id.eq_ignore_ascii_case(&request.symbol_id)
                || request.comment.len() > 4096
                || request.comment.contains('\0')
            {
                return Err(fail("use an IEC name and two distinct unused GUIDs"));
            }
            // Initial native capture is XGI. Other families require their own native acceptance.
            if !self.doc.user_pou().can_create() {
                return Err(fail(
                    "user-defined ST POU creation is captured for XGI CPUs only",
                ));
            }
            let mut vars = request.variables.clone();
            if vars.iter().any(|v| v.scope == UserPouScope::Return) {
                return Err(fail(
                    "omit the automatic function result from a creation request",
                ));
            }
            match (request.kind, &request.return_type) {
                (UserPouKind::Function, Some(ty)) if width(ty).is_some() => {
                    vars.push(UserPouVariable {
                        name: request.name.clone(),
                        data_type: ty.clone(),
                        scope: UserPouScope::Return,
                        description: String::new(),
                    })
                }
                (UserPouKind::FunctionBlock, None) => {}
                _ => {
                    return Err(fail(
                        "a function requires a scalar result type; an FB has none",
                    ));
                }
            }
            let (bytes, input, output, size) = payload(&request.name, request.kind, &vars)?;
            let parsed = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
            if parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction") || n.has_tag_name("Program"))
                .any(|n| {
                    n.text()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case(&request.name)
                })
            {
                return Err(fail(
                    "a program or user-defined POU with that name already exists",
                ));
            }
            for a in parsed.descendants().flat_map(|n| n.attributes()) {
                let value = a.value().replace('_', "-");
                if value.eq_ignore_ascii_case(&request.object_id)
                    || value.eq_ignore_ascii_case(&request.symbol_id)
                {
                    return Err(fail("the requested identities are already used"));
                }
            }
            let lists: Vec<_> = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunctions"))
                .collect();
            if lists.len() != 1 {
                return Err(fail("missing or ambiguous UserFunctions container"));
            }
            let list = lists[0];
            let index = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction"))
                .filter_map(|n| n.attribute("Index")?.parse::<u32>().ok())
                .max()
                .map_or(Some(0x80000000), |i| i.checked_add(1))
                .ok_or_else(|| fail("user-defined POU index overflow"))?;
            let (template, old_name) = if request.kind == UserPouKind::Function {
                (
                    include_str!("user_pou_templates/function-st.xml"),
                    "NativeFunction",
                )
            } else {
                (
                    include_str!("user_pou_templates/block-st.xml"),
                    "NativeBlock",
                )
            };
            let native = roxmltree::Document::parse(template).map_err(XgwxError::Xml)?;
            let root = native.root_element();
            let symbol = root
                .children()
                .find(|n| n.has_tag_name(variable_tag(request.kind)))
                .unwrap()
                .attribute("SymDocGUID")
                .unwrap();
            let mut text = template
                .replace(root.attribute("ObjectId").unwrap(), &request.object_id)
                .replace(symbol, &request.symbol_id.replace('-', "_"))
                .replace(old_name, &request.name);
            let mut temp = self.doc.clone();
            // Populate the captured template through structural replacements, never string-match payloads.
            let p = roxmltree::Document::parse(&text).map_err(XgwxError::Xml)?;
            let r = p.root_element();
            let mut replacements = Vec::new();
            for (field, value) in [
                ("Index", index.to_string()),
                ("Size", size.to_string()),
                ("UDFBTotalAllocSize", size.to_string()),
                ("Completed", "1".into()),
                ("Comment", escape(&request.comment)),
            ] {
                replacements.push((
                    r.attributes()
                        .find(|a| a.name() == field)
                        .unwrap()
                        .range_value(),
                    value,
                ));
            }
            for (tag, value) in [
                ("InputVariables", input),
                ("OutputVariables", output),
                (
                    "Symbols",
                    format!(
                        "<Symbols Count=\"{}\" Compressed=\"0\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\">{}</Symbols>",
                        vars.len(),
                        encoded(&bytes)
                    ),
                ),
                ("CodeList", code(&request.source)?),
            ] {
                let n = r.descendants().find(|n| n.has_tag_name(tag)).unwrap();
                replacements.push((n.range(), value));
            }
            let vt = r
                .children()
                .find(|n| n.has_tag_name(variable_tag(request.kind)))
                .unwrap();
            for a in vt
                .attributes()
                .filter(|a| matches!(a.name(), "FunSize" | "FBSize" | "VARTotalAllocSize"))
            {
                replacements.push((a.range_value(), size.to_string()));
            }
            if let Some(ty) = &request.return_type {
                let a = vt.attributes().find(|a| a.name() == "ReturnType").unwrap();
                replacements.push((a.range_value(), ty.clone()));
                let a = r.attributes().find(|a| a.name() == "Return").unwrap();
                replacements.push((
                    a.range_value(),
                    crate::iec_symbols::iec_primitive_type_code(ty)
                        .unwrap()
                        .to_string(),
                ));
            }
            replacements.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
            for (range, value) in replacements {
                text.replace_range(range, &value);
            }
            let added = if list.children().any(|n| n.has_tag_name("UserFunction")) {
                3
            } else {
                4
            };
            let count = parsed
                .root_element()
                .attributes()
                .find(|a| a.name() == "WksNodeCount")
                .ok_or_else(|| fail("missing workspace node count"))?;
            let n = count
                .value()
                .parse::<u32>()
                .ok()
                .and_then(|n| n.checked_add(added))
                .ok_or_else(|| fail("invalid workspace node count"))?;
            let pos = list.children().last().map_or_else(
                || list.range().end - "</UserFunctions>".len(),
                |n| n.range().end,
            );
            let insertion = if self.doc.xml[list.range()].trim_end().ends_with("/>") {
                let original = &self.doc.xml[list.range()];
                (
                    list.range(),
                    format!("{}>{text}</UserFunctions>", &original[..original.len() - 2]),
                )
            } else {
                (pos..pos, text)
            };
            temp.apply_xml_replacements(vec![insertion, (count.range_value(), n.to_string())])?;
            if !temp.user_pou().list().last().is_some_and(|p| {
                p.editable && p.name == request.name && p.source.as_deref() == Some(&request.source)
            }) {
                return Err(fail("created user-defined POU did not round trip"));
            }
            temp.to_verified_bytes()?;
            *self.doc = temp;
            Ok(())
        }
    }
    impl UserPouHandle<'_> {
        /// Read the selected definition's current snapshot.
        pub fn summary(&self) -> UserPou {
            self.doc.user_pou().list()[self.index].clone()
        }
        fn check(&self, expected: &UserPou) -> Result<(), XgwxError> {
            if self.summary() != *expected
                || expected.object_id != self.object_id
                || !expected.editable
            {
                Err(fail(
                    "stale user-defined POU snapshot; reload before applying",
                ))
            } else {
                Ok(())
            }
        }
        /// Replace the ST body atomically while retaining native interface metadata.
        pub fn set_source(&mut self, expected: &UserPou, source: &str) -> Result<(), XgwxError> {
            self.check(expected)?;
            if expected.source.as_deref() == Some(source) {
                return Ok(());
            }
            let parsed = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
            let node = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction"))
                .nth(self.index)
                .unwrap();
            let code_node = node
                .descendants()
                .find(|n| n.has_tag_name("CodeList"))
                .ok_or_else(|| fail("missing source payload"))?;
            let mut candidate = self.doc.clone();
            let temporaries = node
                .descendants()
                .find(|n| n.has_tag_name("TempVar"))
                .unwrap();
            candidate.apply_xml_replacements(vec![
                (code_node.range(), code(source)?),
                (
                    temporaries.range(),
                    "<TempVar Count=\"0\"></TempVar>".into(),
                ),
            ])?;
            candidate.to_verified_bytes()?;
            *self.doc = candidate;
            Ok(())
        }
        /// Replace scalar parameters and locals together, rejecting interface changes while referenced.
        pub fn set_variables(
            &mut self,
            expected: &UserPou,
            variables: &[UserPouVariable],
        ) -> Result<(), XgwxError> {
            self.check(expected)?;
            if expected.variables == variables {
                return Ok(());
            }
            self.require_unused()?;
            for old in &expected.variables {
                if !variables.iter().any(|v| {
                    v.name == old.name && v.data_type == old.data_type && v.scope == old.scope
                }) && expected
                    .source
                    .as_deref()
                    .unwrap_or("")
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .any(|s| s.eq_ignore_ascii_case(&old.name))
                {
                    return Err(fail(
                        "a declaration used by the POU source cannot be removed, renamed or retyped",
                    ));
                }
            }
            let k = expected.kind.unwrap();
            let (bytes, input, output, size) = payload(&expected.name, k, variables)?;
            let old_return = expected
                .variables
                .iter()
                .find(|v| v.scope == UserPouScope::Return);
            let new_return = variables.iter().find(|v| v.scope == UserPouScope::Return);
            if old_return != new_return {
                return Err(fail(
                    "changing a function's result declaration is not supported",
                ));
            }
            let parsed = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
            let node = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction"))
                .nth(self.index)
                .unwrap();
            let mut replacements = Vec::new();
            for (tag, text) in [
                ("InputVariables", input),
                ("OutputVariables", output),
                (
                    "Symbols",
                    format!(
                        "<Symbols Count=\"{}\" Compressed=\"0\" dt:dt=\"bin.base64\" xmlns:dt=\"urn:schemas-microsoft-com:datatypes\">{}</Symbols>",
                        variables.len(),
                        encoded(&bytes)
                    ),
                ),
            ] {
                let n = node.descendants().find(|n| n.has_tag_name(tag)).unwrap();
                replacements.push((n.range(), text));
            }
            for a in node
                .attributes()
                .filter(|a| matches!(a.name(), "Size" | "UDFBTotalAllocSize"))
            {
                replacements.push((a.range_value(), size.to_string()));
            }
            let vt = node
                .children()
                .find(|n| n.has_tag_name(variable_tag(k)))
                .unwrap();
            for a in vt
                .attributes()
                .filter(|a| matches!(a.name(), "FunSize" | "FBSize" | "VARTotalAllocSize"))
            {
                replacements.push((a.range_value(), size.to_string()));
            }
            let temporaries = node
                .descendants()
                .find(|n| n.has_tag_name("TempVar"))
                .unwrap();
            replacements.push((
                temporaries.range(),
                "<TempVar Count=\"0\"></TempVar>".into(),
            ));
            let mut candidate = self.doc.clone();
            candidate.apply_xml_replacements(replacements)?;
            if !candidate.user_pou().list()[self.index].editable {
                return Err(fail("updated declarations did not round trip"));
            }
            candidate.to_verified_bytes()?;
            *self.doc = candidate;
            Ok(())
        }
        fn require_unused(&self) -> Result<(), XgwxError> {
            let name = self.summary().name.to_ascii_lowercase();
            // Include binary payload strings, ST, and native mirrored type attributes.
            let parsed = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
            let own = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction"))
                .nth(self.index)
                .unwrap()
                .range();
            for n in parsed.descendants().filter(|n| {
                n.is_element() && !(own.start <= n.range().start && n.range().end <= own.end)
            }) {
                if (n.has_tag_name("Program") || n.has_tag_name("UserFunction"))
                    && !n.attribute("Encrytption").unwrap_or("").is_empty()
                {
                    return Err(fail("references cannot be checked in an encrypted program"));
                }
                if n.has_tag_name("Program") && n.attribute("Kind") != Some("4") {
                    return Err(fail(
                        "interface changes and deletion require inspectable ST callers; another program language is present",
                    ));
                }
                for a in n.attributes() {
                    if a.value().eq_ignore_ascii_case(&name) {
                        return Err(fail(
                            "the user-defined POU is referenced; remove its calls and instances before changing its interface or deleting it",
                        ));
                    }
                }
                let Some(text) = n.children().find(|n| n.is_text()).and_then(|n| n.text()) else {
                    continue;
                };
                if matches!(n.tag_name().name(), "Symbols" | "ProgramData" | "CodeList") {
                    let p =
                        crate::decode_base64_payload(text, n.attribute("Compressed") == Some("1"))
                            .map_err(|_| {
                                fail(
                                    "references cannot be checked in an undecodable native payload",
                                )
                            })?;
                    {
                        let strings = crate::extract_utf16_marker_strings(&p.data, false, true);
                        if strings.iter().any(|s| s.value.eq_ignore_ascii_case(&name)) {
                            return Err(fail(
                                "the user-defined POU is referenced by a native payload",
                            ));
                        }
                        if n.has_tag_name("CodeList") {
                            let source = String::from_utf16_lossy(
                                &p.data
                                    .chunks_exact(2)
                                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                                    .collect::<Vec<_>>(),
                            );
                            if source
                                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                                .any(|s| s.eq_ignore_ascii_case(&name))
                            {
                                return Err(fail(
                                    "the user-defined POU is referenced by ST source",
                                ));
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        /// Delete an unused definition after checking its complete snapshot.
        pub fn delete(self, expected: &UserPou) -> Result<(), XgwxError> {
            self.check(expected)?;
            self.require_unused()?;
            let parsed = roxmltree::Document::parse(&self.doc.xml).map_err(XgwxError::Xml)?;
            let node = parsed
                .descendants()
                .filter(|n| n.has_tag_name("UserFunction"))
                .nth(self.index)
                .unwrap();
            let count = parsed
                .root_element()
                .attributes()
                .find(|a| a.name() == "WksNodeCount")
                .ok_or_else(|| fail("missing workspace node count"))?;
            let removed = if self.doc.user_pou().list().len() == 1 {
                4
            } else {
                3
            };
            let n = count
                .value()
                .parse::<u32>()
                .ok()
                .and_then(|n| n.checked_sub(removed))
                .ok_or_else(|| fail("invalid workspace node count"))?;
            let mut candidate = self.doc.clone();
            candidate.apply_xml_replacements(vec![
                (node.range(), String::new()),
                (count.range_value(), n.to_string()),
            ])?;
            candidate.to_verified_bytes()?;
            *self.doc = candidate;
            Ok(())
        }
    }
}
#[cfg(feature = "write")]
pub use write::{UserPouEditor, UserPouHandle};
