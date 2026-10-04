//! Reviewed instruction model restrictions. A model match does not establish
//! firmware, module compatibility, runtime behavior, or native edit validation.
#[path = "instruction_cpu_data.rs"]
mod data;

#[derive(Debug, Clone, Copy)]
pub struct LadderInstructionCpuRestriction {
    pub models: &'static [&'static str],
    pub manual_page: &'static str,
}

/// Return the reviewed XGK model table, if one is available.
pub fn ladder_instruction_cpu_restriction(name: &str) -> Option<LadderInstructionCpuRestriction> {
    data::restriction(&name.trim().to_ascii_uppercase())
}

/// `None` means an unreviewed command or CPU; `Some(true)` means only that the
/// reviewed model table permits this model. Firmware and modules are not checked.
pub fn ladder_instruction_cpu_allowed(name: &str, model: &str) -> Option<bool> {
    let cpu = crate::cpu_catalog()
        .iter()
        .find(|cpu| cpu.model.eq_ignore_ascii_case(model.trim()))?;
    if cpu.family != "XGK" {
        return None;
    }
    let restriction = ladder_instruction_cpu_restriction(name)?;
    Some(restriction.models.contains(&cpu.model))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_model_restrictions_cover_both_xgk_generations() {
        for cpu in crate::cpu_catalog()
            .iter()
            .filter(|cpu| cpu.family == "XGK")
        {
            let high_performance = matches!(cpu.model, "XGK-CPUUN" | "XGK-CPUHN" | "XGK-CPUSN");
            for command in ["INLATCH", "GETIP", "SETIP", "CPMSG"] {
                assert_eq!(
                    ladder_instruction_cpu_allowed(command, cpu.model),
                    Some(high_performance)
                );
            }
            assert_eq!(ladder_instruction_cpu_allowed("GET", cpu.model), Some(true));
            assert_eq!(
                ladder_instruction_cpu_allowed("GETCOMM", cpu.model),
                Some(false)
            );
        }
    }
    #[test]
    fn unreviewed_models_and_commands_remain_unknown() {
        assert_eq!(ladder_instruction_cpu_allowed("MOV", "XGK-CPUH"), None);
        assert_eq!(ladder_instruction_cpu_allowed("INLATCH", "XGB-XBMS"), None);
        assert_eq!(ladder_instruction_cpu_allowed("INLATCH", "XGI-CPUE"), None);
        assert_eq!(
            ladder_instruction_cpu_allowed("INLATCH", "future model"),
            None
        );
        assert_eq!(
            ladder_instruction_cpu_allowed(" inlatch ", "xgk-cpuhn"),
            Some(true)
        );
        assert_eq!(
            ladder_instruction_cpu_restriction("INLATCH")
                .unwrap()
                .manual_page,
            "42421inlatch.htm"
        );
    }
}
