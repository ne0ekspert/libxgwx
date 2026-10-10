use super::*;

/// Inclusive native array bounds, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcArrayBound {
    pub lower: i32,
    pub upper: i32,
}

/// Captured advanced metadata for one program-local declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase", deny_unknown_fields))]
pub struct SfcDeclaration {
    #[cfg_attr(feature = "wasm", serde(default))]
    pub dimensions: Vec<SfcArrayBound>,
    #[cfg_attr(feature = "wasm", serde(default))]
    pub initial_value: String,
    #[cfg_attr(feature = "wasm", serde(default))]
    pub retain: bool,
}

fn records(bytes: &[u8]) -> Result<Vec<Vec<crate::LadderString>>, crate::XgwxError> {
    let fields = crate::extract_utf16_marker_strings(bytes, false, true);
    let starts = fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| (f.value == "PB50").then_some(i))
        .collect::<Vec<_>>();
    if !bytes.is_empty()
        && (starts.first() != Some(&0) || fields.first().is_none_or(|f| f.offset != 0))
    {
        return Err(fail());
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(n, start)| {
            fields[*start..starts.get(n + 1).copied().unwrap_or(fields.len())].to_vec()
        })
        .collect())
}
fn fail() -> crate::XgwxError {
    crate::XgwxError::SfcEdit("unsupported advanced declaration metadata".into())
}
fn word(bytes: &[u8], at: usize) -> Result<u32, crate::XgwxError> {
    Ok(u32::from_le_bytes(
        bytes.get(at..at + 4).ok_or_else(fail)?.try_into().unwrap(),
    ))
}
fn string(bytes: &[u8], at: &mut usize) -> Result<String, crate::XgwxError> {
    if bytes.get(*at..*at + 3) != Some(&[0xff, 0xfe, 0xff]) {
        return Err(fail());
    }
    let n = usize::from(*bytes.get(*at + 3).ok_or_else(fail)?);
    let end = *at + 4 + n * 2;
    let units = bytes
        .get(*at + 4..end)
        .ok_or_else(fail)?
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    *at = end;
    String::from_utf16(&units).map_err(|_| fail())
}
fn element_count(bounds: &[SfcArrayBound]) -> Option<usize> {
    bounds
        .iter()
        .try_fold(1usize, |n, d| {
            let width = i64::from(d.upper) - i64::from(d.lower) + 1;
            (width > 0)
                .then_some(width as usize)
                .and_then(|v| n.checked_mul(v))
        })
        .filter(|n| *n <= 65536)
}
fn element_name(name: &str, bounds: &[SfcArrayBound], mut index: usize) -> String {
    let mut indices = Vec::new();
    for d in bounds.iter().rev() {
        let width = (i64::from(d.upper) - i64::from(d.lower) + 1) as usize;
        indices.push((i64::from(d.lower) + (index % width) as i64).to_string());
        index /= width;
    }
    indices.reverse();
    format!("{name}[{}]", indices.join(","))
}
fn compact_values(values: &[String]) -> String {
    let mut parts = Vec::new();
    let mut at = 0;
    while at < values.len() {
        let end = at
            + values[at..]
                .iter()
                .take_while(|v| *v == &values[at])
                .count();
        let n = end - at;
        parts.push(if n > 1 {
            format!("{n}({})", values[at])
        } else {
            values[at].clone()
        });
        at = end;
    }
    parts.join(",")
}
pub(super) fn normalize(bytes: &mut Vec<u8>) -> Result<BTreeMap<String, usize>, crate::XgwxError> {
    let items = records(bytes)?;
    let mut normalized = Vec::new();
    let mut offsets = BTreeMap::new();
    for (n, f) in items.iter().enumerate() {
        if f.len() < 8 {
            return Err(fail());
        }
        let code = word(bytes, f[1].end_offset + 4)?;
        let reference = matches!(code, 22 | 23 | 24);
        let flag = if reference {
            f[2].end_offset
        } else {
            f[1].end_offset + 8
        };
        let next = if reference { f[3].offset } else { f[2].offset };
        if flag + 4 != next {
            return Err(fail());
        }
        let flags = word(bytes, flag)?;
        if !["TRANS", "GOTO_INIT"].contains(&f[1].value.as_str()) && flags & !1 != 0 {
            return Err(fail());
        }
        let start = f[0].offset;
        let end = items.get(n + 1).map_or(bytes.len(), |f| f[0].offset);
        if offsets.insert(f[1].value.clone(), start).is_some() {
            return Err(fail());
        }
        let begin = normalized.len();
        if code == 22 {
            let class = 6;
            normalized.extend_from_slice(&bytes[start..f[class].end_offset]);
            normalized.extend_from_slice(&bytes[f[class].end_offset..f[class].end_offset + 12]);
            normalized.extend([0; 24]);
            normalized.extend_from_slice(&bytes[f[f.len() - 2].offset..end]);
        } else {
            if f.len() != if reference { 9 } else { 8 } {
                return Err(fail());
            }
            normalized.extend_from_slice(&bytes[start..end]);
        }
        normalized[begin + flag - start..begin + flag - start + 4].fill(0);
    }
    *bytes = normalized;
    Ok(offsets)
}
pub(super) fn metadata(
    bytes: &[u8],
) -> Result<BTreeMap<String, (String, SfcDeclaration)>, crate::XgwxError> {
    let mut result = BTreeMap::new();
    for f in records(bytes)? {
        if f.len() < 8 {
            return Err(fail());
        }
        let code = word(bytes, f[1].end_offset + 4)?;
        let reference = matches!(code, 22 | 23 | 24);
        let flag = if reference {
            f[2].end_offset
        } else {
            f[1].end_offset + 8
        };
        let class = if reference { 6 } else { 5 };
        let mut options = SfcDeclaration {
            retain: word(bytes, flag)? & 1 != 0,
            initial_value: f[class - 2].value.clone(),
            ..Default::default()
        };
        let ty = if code == 22 {
            let (ty, bounds) = parse_array(&f[2].value).ok_or_else(fail)?;
            let count = element_count(&bounds).ok_or_else(fail)?;
            options.dimensions = bounds;
            if !options.initial_value.is_empty() {
                return Err(fail());
            }
            let mut at = f[class].end_offset + 12;
            let mut entries = BTreeMap::new();
            // PB50 has six counted member-override maps. The third stores
            // initial values. Other member overrides remain guarded.
            for map in 0..6 {
                let n = word(bytes, at)? as usize;
                at += 4;
                if map != 2 && n != 0 || n > count {
                    return Err(fail());
                }
                for _ in 0..n {
                    let key = string(bytes, &mut at)?;
                    let value = string(bytes, &mut at)?;
                    if entries.insert(key, value).is_some() {
                        return Err(fail());
                    }
                }
            }
            let mut values = Vec::new();
            let mut gap = false;
            for index in 0..count {
                if let Some(value) =
                    entries.remove(&element_name(&f[1].value, &options.dimensions, index))
                {
                    if gap {
                        return Err(fail());
                    }
                    values.push(value);
                } else {
                    gap = true;
                }
            }
            if !entries.is_empty() {
                return Err(fail());
            }
            let _ = string(bytes, &mut at)?;
            let _ = string(bytes, &mut at)?;
            if at != f.last().unwrap().end_offset {
                return Err(fail());
            }
            options.initial_value = compact_values(&values);
            ty
        } else if reference {
            f[2].value.clone()
        } else {
            crate::iec_symbols::iec_primitive_type_name(code)
                .unwrap_or("")
                .to_owned()
        };
        if result.insert(f[1].value.clone(), (ty, options)).is_some() {
            return Err(fail());
        }
    }
    Ok(result)
}
fn parse_array(text: &str) -> Option<(String, Vec<SfcArrayBound>)> {
    let rest = text.strip_prefix("ARRAY[")?;
    let (bounds, ty) = rest.split_once("] OF ")?;
    let dims = bounds
        .split(',')
        .map(|s| {
            let (l, u) = s.trim().split_once("..")?;
            Some(SfcArrayBound {
                lower: l.trim().parse().ok()?,
                upper: u.trim().parse().ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    (!dims.is_empty()).then(|| (ty.to_owned(), dims))
}

#[cfg(feature = "write")]
pub(super) fn record(patch: &SfcVariablePatch) -> Result<Vec<u8>, crate::XgwxError> {
    let fail = |s: &str| crate::XgwxError::SfcEdit(s.into());
    let options = patch.declaration.clone().unwrap_or_default();
    let fb = matches!(
        patch.data_type.as_str(),
        "TON"
            | "TOF"
            | "TP"
            | "CTU_DINT"
            | "CTD_DINT"
            | "CTUD_DINT"
            | "R_TRIG"
            | "F_TRIG"
            | "RS"
            | "SR"
    );
    let primitive = crate::iec_symbols::iec_primitive_type_code(&patch.data_type);
    if primitive.is_none() && !fb && patch.data_type != "STRING" {
        return Err(fail("unsupported variable type"));
    }
    if options.dimensions.len() > 3
        || options
            .dimensions
            .iter()
            .any(|d| d.lower > d.upper || d.lower != 0 || d.upper > 65535)
    {
        return Err(fail(
            "use up to three zero-based array bounds, with upper from 0 to 65535",
        ));
    }
    let count = options
        .dimensions
        .iter()
        .try_fold(1u64, |n, d| {
            n.checked_mul((i64::from(d.upper) - i64::from(d.lower) + 1) as u64)
        })
        .filter(|n| *n <= 65536)
        .ok_or_else(|| fail("array declarations support at most 65536 elements"))?;
    if fb && (!options.dimensions.is_empty() || !options.initial_value.is_empty()) {
        return Err(fail(
            "function blocks do not support array bounds or initial values here",
        ));
    }
    if patch.data_type == "STRING" && !options.dimensions.is_empty() {
        return Err(fail("arrays of STRING are not captured"));
    }
    let length = 32;
    if options.initial_value.encode_utf16().count() > 255
        || options.initial_value.chars().any(char::is_control)
    {
        return Err(fail(
            "initial value is too long or contains control characters",
        ));
    }
    if !options.initial_value.is_empty()
        && !valid_initial(
            &patch.data_type,
            &options.initial_value,
            count,
            !options.dimensions.is_empty(),
            length,
        )
    {
        return Err(fail(
            "use a valid literal initial value matching the declaration type and bounds",
        ));
    }
    let (code, reference) = if fb {
        (24, Some(patch.data_type.clone()))
    } else if !options.dimensions.is_empty() {
        (
            22,
            Some(format!(
                "ARRAY[{}] OF {}",
                options
                    .dimensions
                    .iter()
                    .map(|d| format!("{}..{}", d.lower, d.upper))
                    .collect::<Vec<_>>()
                    .join(","),
                patch.data_type
            )),
        )
    } else {
        (primitive.unwrap(), None)
    };
    let mut bytes = Vec::new();
    field(&mut bytes, "PB50");
    field(&mut bytes, &patch.name);
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(code.to_le_bytes());
    if let Some(reference) = reference {
        field(&mut bytes, &reference);
    }
    bytes.extend(u32::from(options.retain).to_le_bytes());
    field(&mut bytes, "");
    field(
        &mut bytes,
        if options.dimensions.is_empty() {
            &options.initial_value
        } else {
            ""
        },
    );
    field(&mut bytes, &patch.description);
    field(&mut bytes, "");
    bytes.extend(u32::MAX.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(u32::MAX.to_le_bytes());
    if options.dimensions.is_empty() {
        bytes.extend([0; 24]);
    } else {
        let values = expand_values(&options.initial_value);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
        bytes.extend((values.len() as u32).to_le_bytes());
        for (index, value) in values.iter().enumerate() {
            field(
                &mut bytes,
                &element_name(&patch.name, &options.dimensions, index),
            );
            field(&mut bytes, value);
        }
        bytes.extend([0; 12]);
    }
    field(&mut bytes, "");
    field(&mut bytes, "");
    Ok(bytes)
}
#[cfg(feature = "write")]
fn field(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend([0xff, 0xfe, 0xff, value.encode_utf16().count() as u8]);
    bytes.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
}

#[cfg(feature = "write")]
fn valid_initial(ty: &str, text: &str, count: u64, array: bool, length: u32) -> bool {
    if !array {
        return literal(ty, text.trim(), length);
    }
    let mut total = 0u64;
    for item in text.split(',') {
        let item = item.trim();
        let (repeat, value) = if let Some((n, value)) = item.split_once('(') {
            let Ok(n) = n.trim().parse::<u64>() else {
                return false;
            };
            let Some(value) = value.strip_suffix(')') else {
                return false;
            };
            if n == 0 {
                return false;
            }
            (n, value.trim())
        } else {
            (1, item)
        };
        if !literal(ty, value, length) {
            return false;
        }
        let Some(n) = total.checked_add(repeat) else {
            return false;
        };
        total = n;
    }
    total > 0 && total <= count
}
#[cfg(feature = "write")]
fn literal(ty: &str, text: &str, length: u32) -> bool {
    if ty == "STRING" {
        let Some(inner) = text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) else {
            return false;
        };
        return !inner.contains(['\'', '$']) && inner.is_ascii() && inner.len() <= length as usize;
    }
    if ty == "BOOL" {
        return matches!(
            text,
            "TRUE" | "FALSE" | "BOOL#TRUE" | "BOOL#FALSE" | "0" | "1"
        );
    }
    if ty == "TIME" {
        return super::valid_action_time(text);
    }
    if matches!(ty, "REAL" | "LREAL") {
        let value = text.strip_prefix(&format!("{ty}#")).unwrap_or(text);
        return value
            .parse::<f64>()
            .is_ok_and(|v| v.is_finite() && (ty != "REAL" || (v as f32).is_finite()));
    }
    let value = text.strip_prefix(&format!("{ty}#")).unwrap_or(text);
    let value = value.replace('_', "");
    let n = if let Some((radix, value)) = value.split_once('#') {
        let Ok(radix) = radix.parse::<u32>() else {
            return false;
        };
        if !matches!(radix, 2 | 8 | 16) {
            return false;
        }
        i128::from_str_radix(value, radix).ok()
    } else {
        value.parse::<i128>().ok()
    };
    let Some(n) = n else {
        return false;
    };
    let (min, max) = match ty {
        "SINT" => (i8::MIN as i128, i8::MAX as i128),
        "INT" => (i16::MIN as i128, i16::MAX as i128),
        "DINT" => (i32::MIN as i128, i32::MAX as i128),
        "LINT" => (i64::MIN as i128, i64::MAX as i128),
        "BYTE" | "USINT" => (0, u8::MAX as i128),
        "WORD" | "UINT" => (0, u16::MAX as i128),
        "DWORD" | "UDINT" => (0, u32::MAX as i128),
        "LWORD" | "ULINT" => (0, u64::MAX as i128),
        _ => return false,
    };
    (min..=max).contains(&n)
}

#[cfg(feature = "write")]
fn expand_values(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut values = Vec::new();
    for item in text.split(',') {
        let item = item.trim();
        if let Some((n, value)) = item.split_once('(') {
            let n = n.trim().parse::<usize>().unwrap();
            values.extend(std::iter::repeat_n(
                value.strip_suffix(')').unwrap().trim().to_owned(),
                n,
            ));
        } else {
            values.push(item.to_owned());
        }
    }
    values
}

#[cfg(all(test, feature = "write"))]
mod tests {
    use super::*;
    #[test]
    fn duplicate_names_cannot_redirect_original_record_offsets() {
        let patch = SfcVariablePatch {
            program_index: 0,
            expected_variables: vec![],
            name: "Value".into(),
            data_type: "DINT".into(),
            description: "".into(),
            remove: false,
            update: false,
            declaration: None,
        };
        let one = record(&patch).unwrap();
        let mut bytes = [one.clone(), one].concat();
        let original = bytes.clone();
        assert!(metadata(&bytes).is_err());
        assert!(normalize(&mut bytes).is_err());
        assert_eq!(bytes, original);
    }
}

// XGK Auto-allocation scalar IDs captured from XG5000 4.82.1.
const XGK_SCALARS: &[(u32, &str)] = &[
    (1, "BOOL"),
    (3, "BYTE"),
    (4, "WORD"),
    (5, "DWORD"),
    (6, "LWORD"),
    (16, "SINT"),
    (9, "INT"),
    (10, "DINT"),
    (11, "LINT"),
    (17, "USINT"),
    (18, "UINT"),
    (19, "UDINT"),
    (20, "ULINT"),
    (7, "REAL"),
    (8, "LREAL"),
];
pub(super) fn normalize_xgk_types(bytes: &mut [u8]) -> Result<(), crate::XgwxError> {
    for fields in records(bytes)? {
        let at = fields.get(1).ok_or_else(fail)?.end_offset + 4;
        let code = word(bytes, at)?;
        let name = XGK_SCALARS
            .iter()
            .find(|(id, _)| *id == code)
            .map(|(_, name)| *name)
            .ok_or_else(fail)?;
        let code = crate::iec_symbols::iec_primitive_type_code(name).ok_or_else(fail)?;
        bytes[at..at + 4].copy_from_slice(&code.to_le_bytes());
        // Native Auto-allocation assigns D storage after Check Program. The
        // shared symbol reader uses M for allocated internal scalar storage;
        // normalize only the decoding copy, retaining the original D record.
        if fields.len() == 8 && fields[5].value == "D" {
            bytes[fields[5].offset + 4..fields[5].end_offset]
                .copy_from_slice(&('M' as u16).to_le_bytes());
        }
    }
    Ok(())
}
#[cfg(feature = "write")]
pub(super) fn xgk_record(patch: &SfcVariablePatch) -> Result<Vec<u8>, crate::XgwxError> {
    if patch.declaration.clone().unwrap_or_default() != SfcDeclaration::default() {
        return Err(crate::XgwxError::SfcEdit(
            "XGK advanced declarations have not been captured".into(),
        ));
    }
    let code = XGK_SCALARS
        .iter()
        .find(|(_, name)| *name == patch.data_type)
        .map(|(id, _)| *id)
        .ok_or_else(fail)?;
    let mut bytes = record(patch)?;
    let at = records(&bytes)?[0][1].end_offset + 4;
    bytes[at..at + 4].copy_from_slice(&code.to_le_bytes());
    Ok(bytes)
}
