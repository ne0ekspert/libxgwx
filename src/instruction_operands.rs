//! Operand types and device permissions from the local XGK instruction manual.
#[path = "instruction_operand_data.rs"]
mod data;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize))]
#[cfg_attr(feature = "wasm", serde(rename_all = "camelCase"))]
pub struct LadderOperandSpec {
    pub label: &'static str,
    pub data_types: &'static [&'static str],
    /// None means the device permissions were not unambiguously decoded.
    pub device_areas: Option<&'static [&'static str]>,
    pub allows_constant: Option<bool>,
    pub manual_page: &'static str,
}

/// Empty means this manual does not provide an unambiguous matching operand table.
/// Shared variant tables retain their union of types unless separately reviewed.
pub fn ladder_instruction_operand_rules(mnemonic: &str) -> &'static [LadderOperandSpec] {
    data::rules(mnemonic)
}

/// XGK integer symbols use storage-width names. REAL and LREAL stay distinct.
pub fn ladder_operand_type_matches(rule: &LadderOperandSpec, data_type: &str) -> bool {
    fn normalized(value: &str) -> &str {
        match value {
            "BOOL" => "BIT",
            "INT" | "UINT" => "WORD",
            "DINT" | "UDINT" => "DWORD",
            "LINT" | "ULINT" => "LWORD",
            _ => value,
        }
    }
    rule.data_types
        .iter()
        .any(|t| normalized(t) == normalized(data_type))
}

/// Check literal storage bounds without rounding integer constants through f64.
/// Hex/binary constants describe bit patterns even for signed instructions.
#[cfg(feature = "write")]
fn literal_fits(rule: &LadderOperandSpec, text: &str) -> bool {
    let radix = if text.starts_with('H') {
        16
    } else if text.starts_with('B') {
        2
    } else {
        10
    };
    let digits = if radix == 10 { text } else { &text[1..] };
    let integer = i128::from_str_radix(digits, radix).ok();
    rule.data_types.iter().any(|ty| {
        let (bits, signed, storage) = match *ty {
            "BIT" => (1, false, false),
            "NIBBLE" => (4, false, false),
            "BYTE" => (8, false, false),
            "WORD" => (16, true, true),
            "DWORD" => (32, true, true),
            "LWORD" => (64, true, true),
            "SINT" => (8, true, false),
            "INT" => (16, true, false),
            "DINT" => (32, true, false),
            "LINT" => (64, true, false),
            "USINT" => (8, false, false),
            "UINT" => (16, false, false),
            "UDINT" => (32, false, false),
            "ULINT" => (64, false, false),
            "REAL" => {
                return if radix == 10 {
                    text.parse::<f32>().is_ok_and(|value| value.is_finite())
                } else {
                    integer.is_some_and(|value| (0..=u32::MAX as i128).contains(&value))
                };
            }
            "LREAL" => {
                return if radix == 10 {
                    text.parse::<f64>().is_ok_and(|value| value.is_finite())
                } else {
                    integer.is_some_and(|value| (0..=u64::MAX as i128).contains(&value))
                };
            }
            _ => return true, // Unclassified manual types remain delegated to native checking.
        };
        let min = if signed { -(1i128 << (bits - 1)) } else { 0 };
        let max = if signed && !storage && radix == 10 {
            (1i128 << (bits - 1)) - 1
        } else {
            (1i128 << bits) - 1
        };
        integer.is_some_and(|value| value >= min && value <= max)
    })
}

#[cfg(feature = "write")]
pub(crate) fn validate_raw_operands(
    mnemonic: &str,
    operands: &[&str],
) -> Result<(), crate::XgwxError> {
    for (rule, operand) in ladder_instruction_operand_rules(mnemonic)
        .iter()
        .zip(operands)
    {
        let text = operand.to_ascii_uppercase();
        let literal = text.parse::<f64>().is_ok()
            || ['H', 'B'].iter().any(|prefix| {
                text.strip_prefix(*prefix).is_some_and(|digits| {
                    !digits.is_empty()
                        && digits.chars().all(|c| {
                            if *prefix == 'H' {
                                c.is_ascii_hexdigit()
                            } else {
                                matches!(c, '0' | '1')
                            }
                        })
                })
            });
        if literal
            && (text.parse::<f64>().is_ok_and(|value| !value.is_finite())
                || !literal_fits(rule, &text))
        {
            return Err(crate::XgwxError::InvalidLadderEdit {
                reason: "instruction constant is outside the permitted operand type range",
            });
        }
        if literal && rule.allows_constant == Some(false) {
            return Err(crate::XgwxError::InvalidLadderEdit {
                reason: "instruction operand does not permit a constant",
            });
        }
        if literal
            && text.parse::<f64>().is_ok()
            && text.contains(['.', 'E'])
            && !rule
                .data_types
                .iter()
                .any(|t| matches!(*t, "REAL" | "LREAL"))
        {
            return Err(crate::XgwxError::InvalidLadderEdit {
                reason: "instruction operand requires an integer rather than a real constant",
            });
        }
        // Raw word devices are starting addresses: D100 can address WORD, DWORD,
        // LWORD or REAL storage. Do not infer a fixed width from their spelling.
        let area = text.get(..1).unwrap_or_default();
        if !matches!(
            area,
            "P" | "M" | "K" | "F" | "L" | "T" | "C" | "S" | "Z" | "U" | "N" | "D" | "R"
        ) {
            continue;
        }
        let tail = &text[1..];
        if area == "D" && tail.contains('.') {
            let valid = tail.split_once('.').is_some_and(|(word, bit)| {
                !word.is_empty()
                    && word.bytes().all(|c| c.is_ascii_digit())
                    && bit.len() == 1
                    && bit.bytes().all(|c| c.is_ascii_hexdigit())
            });
            if !valid || text.len() > 32 {
                return Err(crate::XgwxError::InvalidLadderEdit {
                    reason: "D register bit address requires a decimal word address and one bit index 0 through F",
                });
            }
            // Some instructions consume words from a bit-position starting address.
            // Preserve the manual's explicit D.x permission for those operands.
            if !rule
                .device_areas
                .is_some_and(|areas| areas.contains(&"D.x"))
                && !rule
                    .data_types
                    .iter()
                    .any(|t| matches!(*t, "BIT" | "BOOL" | "NIBBLE" | "BYTE"))
            {
                return Err(crate::XgwxError::InvalidLadderEdit {
                    reason: "D register bit address cannot be used as a word device",
                });
            }
        }
        if matches!(area, "P" | "M" | "K" | "F" | "L")
            && !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_hexdigit())
            && tail.chars().any(|c| matches!(c, 'A'..='F'))
            && !rule
                .data_types
                .iter()
                .any(|t| matches!(*t, "BIT" | "NIBBLE" | "BYTE"))
        {
            return Err(crate::XgwxError::InvalidLadderEdit {
                reason: "hexadecimal bit address cannot be used as a word device",
            });
        }
        if tail.is_empty() || !tail.chars().all(|c| c.is_ascii_hexdigit() || c == '.') {
            continue;
        }
        if let Some(areas) = rule.device_areas {
            let key = if matches!(area, "D" | "R") && tail.contains('.') {
                if area == "D" { "D.x" } else { "R.x" }
            } else if matches!(area, "P" | "M" | "K") {
                "PMK"
            } else {
                area
            };
            if !areas.contains(&key) {
                return Err(crate::XgwxError::InvalidLadderEdit {
                    reason: "instruction operand device area is not permitted by the manual",
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_variants_have_distinct_operand_types() {
        for (name, expected) in [
            ("MOV", &["WORD", "WORD"][..]),
            ("DMOV", &["DWORD", "DWORD"]),
            ("I2R", &["INT", "REAL"]),
            ("I2LP", &["INT", "LREAL"]),
            ("MUL", &["INT", "INT", "DINT"]),
            ("DMUL", &["DINT", "DINT", "LINT"]),
            ("RADD", &["REAL", "REAL", "REAL"]),
        ] {
            let rules = ladder_instruction_operand_rules(name);
            assert_eq!(rules.len(), expected.len(), "{name}");
            for (rule, ty) in rules.iter().zip(expected) {
                assert_eq!(rule.data_types, &[*ty]);
            }
        }
        assert!(ladder_operand_type_matches(
            &ladder_instruction_operand_rules("ADD")[0],
            "WORD"
        ));
        assert!(!ladder_operand_type_matches(
            &ladder_instruction_operand_rules("ADD")[0],
            "BIT"
        ));
        assert!(!ladder_operand_type_matches(
            &ladder_instruction_operand_rules("I2R")[1],
            "DWORD"
        ));
    }
    #[test]
    fn manual_rules_match_catalog_arity() {
        for spec in crate::ladder_instruction_catalog()
            .iter()
            .chain(crate::ladder_comparison_catalog())
        {
            let rules = ladder_instruction_operand_rules(spec.mnemonic);
            assert!(
                rules.is_empty() || rules.len() == spec.operand_count,
                "{}",
                spec.mnemonic
            );
        }
        for spec in crate::ladder_comparison_catalog() {
            let rules = ladder_instruction_operand_rules(spec.mnemonic);
            assert_eq!(rules.len(), 2, "{}", spec.mnemonic);
            assert!(rules.iter().all(|rule| rule.data_types == ["INT"]));
        }
    }
    #[cfg(feature = "write")]
    #[test]
    fn literal_boundaries_are_exact_and_real_constants_are_finite() {
        for value in ["-32768", "65535", "HFFFF", "B1111111111111111"] {
            assert!(
                validate_raw_operands("MOV", &[value, "D100"]).is_ok(),
                "{value}"
            );
        }
        for value in [
            "-32769",
            "65536",
            "H10000",
            "B10000000000000000",
            "NaN",
            "INF",
        ] {
            assert!(
                validate_raw_operands("MOV", &[value, "D100"]).is_err(),
                "{value}"
            );
        }
        assert!(validate_raw_operands("ADD", &["32767", "-32768", "D100"]).is_ok());
        assert!(validate_raw_operands("ADD", &["32768", "1", "D100"]).is_err());
        assert!(validate_raw_operands("ADD", &["HFFFF", "1", "D100"]).is_ok());
        assert!(validate_raw_operands("RADD", &["3.4E38", "D100", "D200"]).is_ok());
        assert!(validate_raw_operands("RADD", &["3.5E38", "D100", "D200"]).is_err());
        assert!(validate_raw_operands("LADD", &["3.5E38", "D100", "D200"]).is_ok());
        let lword = LadderOperandSpec {
            label: "S",
            data_types: &["LWORD"],
            device_areas: None,
            allows_constant: Some(true),
            manual_page: "",
        };
        assert!(literal_fits(&lword, "18446744073709551615"));
        assert!(!literal_fits(&lword, "18446744073709551616"));
    }

    #[cfg(feature = "write")]
    #[test]
    fn constants_device_permissions_and_raw_storage_are_checked() {
        assert!(validate_raw_operands("MOV", &["1", "D100"]).is_ok());
        assert!(validate_raw_operands("I2R", &["D100", "D200"]).is_ok());
        assert!(validate_raw_operands("MOV", &["D100", "1"]).is_err());
        assert!(validate_raw_operands("MOV", &["D100", "F100"]).is_err());
        assert!(validate_raw_operands("MOV", &["D100.1", "D200"]).is_err());
        assert!(validate_raw_operands("MOV", &["M0000A", "D200"]).is_err());
        assert!(validate_raw_operands("MOV4", &["M0000A", "D100.4"]).is_ok());
        assert!(validate_raw_operands("MOV8", &["M00000", "D100.8"]).is_ok());
        assert!(validate_raw_operands("MOV4", &["D0000.F", "D100.4"]).is_ok());
        for bad in ["D0000.10", "D0000.G", "D.0"] {
            assert!(validate_raw_operands("MOV4", &[bad, "D100.4"]).is_err());
        }

        assert!(validate_raw_operands("MOV", &["1.5", "D200"]).is_err());
        assert!(validate_raw_operands("RADD", &["1.5", "D100", "D200"]).is_ok());
    }
}
