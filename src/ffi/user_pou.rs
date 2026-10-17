/// A user-defined function or function block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserPouKind {
    /// Stateless function with a return value.
    Function,
    /// Stateful function block, instantiated by a declaration.
    FunctionBlock,
}

/// Storage class of a native user-POU declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
/// One scalar parameter or local variable in a user-defined POU.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPouVariable {
    /// IEC identifier, unique without regard to case within the POU.
    pub name: String,
    /// Captured scalar IEC primitive type name.
    pub data_type: String,
    /// Native parameter/local storage class.
    pub scope: UserPouScope,
    /// Human-readable declaration comment.
    #[serde(default)]
    pub description: String,
}

/// Owned inspection snapshot. Unsupported native layouts remain visible.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(feature = "write")]
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
    #[serde(default)]
    pub comment: String,
}

impl From<UserPouKind> for crate::UserPouKind {
    fn from(v: UserPouKind) -> Self {
        match v {
            UserPouKind::Function => crate::UserPouKind::Function,
            UserPouKind::FunctionBlock => crate::UserPouKind::FunctionBlock,
        }
    }
}
impl From<crate::UserPouKind> for UserPouKind {
    fn from(v: crate::UserPouKind) -> Self {
        match v {
            crate::UserPouKind::Function => UserPouKind::Function,
            crate::UserPouKind::FunctionBlock => UserPouKind::FunctionBlock,
        }
    }
}
impl From<UserPouScope> for crate::UserPouScope {
    fn from(v: UserPouScope) -> Self {
        match v {
            UserPouScope::Local => crate::UserPouScope::Local,
            UserPouScope::Input => crate::UserPouScope::Input,
            UserPouScope::Output => crate::UserPouScope::Output,
            UserPouScope::InOut => crate::UserPouScope::InOut,
            UserPouScope::Return => crate::UserPouScope::Return,
        }
    }
}
impl From<crate::UserPouScope> for UserPouScope {
    fn from(v: crate::UserPouScope) -> Self {
        match v {
            crate::UserPouScope::Local => UserPouScope::Local,
            crate::UserPouScope::Input => UserPouScope::Input,
            crate::UserPouScope::Output => UserPouScope::Output,
            crate::UserPouScope::InOut => UserPouScope::InOut,
            crate::UserPouScope::Return => UserPouScope::Return,
        }
    }
}
impl From<UserPouVariable> for crate::UserPouVariable {
    fn from(v: UserPouVariable) -> Self {
        Self {
            name: v.name,
            data_type: v.data_type,
            scope: v.scope.into(),
            description: v.description,
        }
    }
}
impl From<crate::UserPouVariable> for UserPouVariable {
    fn from(v: crate::UserPouVariable) -> Self {
        Self {
            name: v.name,
            data_type: v.data_type,
            scope: v.scope.into(),
            description: v.description,
        }
    }
}
impl From<UserPou> for crate::UserPou {
    fn from(v: UserPou) -> Self {
        Self {
            index: v.index,
            object_id: v.object_id,
            name: v.name,
            kind: v.kind.map(Into::into),
            language: v.language,
            source: v.source,
            return_type: v.return_type,
            variables: v.variables.into_iter().map(Into::into).collect(),
            comment: v.comment,
            editable: v.editable,
            reason: v.reason,
        }
    }
}
impl From<crate::UserPou> for UserPou {
    fn from(v: crate::UserPou) -> Self {
        Self {
            index: v.index,
            object_id: v.object_id,
            name: v.name,
            kind: v.kind.map(Into::into),
            language: v.language,
            source: v.source,
            return_type: v.return_type,
            variables: v.variables.into_iter().map(Into::into).collect(),
            comment: v.comment,
            editable: v.editable,
            reason: v.reason,
        }
    }
}
#[cfg(feature = "write")]
impl From<NewUserPou> for crate::NewUserPou {
    fn from(v: NewUserPou) -> Self {
        Self {
            name: v.name,
            kind: v.kind.into(),
            object_id: v.object_id,
            symbol_id: v.symbol_id,
            return_type: v.return_type,
            variables: v.variables.into_iter().map(Into::into).collect(),
            source: v.source,
            comment: v.comment,
        }
    }
}
#[cfg(feature = "write")]
impl From<crate::NewUserPou> for NewUserPou {
    fn from(v: crate::NewUserPou) -> Self {
        Self {
            name: v.name,
            kind: v.kind.into(),
            object_id: v.object_id,
            symbol_id: v.symbol_id,
            return_type: v.return_type,
            variables: v.variables.into_iter().map(Into::into).collect(),
            source: v.source,
            comment: v.comment,
        }
    }
}
