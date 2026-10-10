//! Native program-language capabilities shared by writers and frontends.
use crate::XgwxDocument;

impl XgwxDocument {
    pub(crate) fn xgk_auto_allocation(&self) -> bool {
        let configs = self.configurations();
        configs.len() == 1
            && configs[0]
                .type_code
                .and_then(crate::cpu::cpu_for_type)
                .is_some_and(|c| c.family == "XGK")
            && configs[0].attribute.is_some_and(|a| a & 0x20 != 0)
            && self
                .root
                .descendants_named("Parameter")
                .filter(|p| p.attribute("Type") == Some("AUTO ALLOCATION PARAMETER"))
                .count()
                == 1
    }
    /// Languages whose creation layout is validated for this CPU and project mode.
    pub fn program_languages(&self) -> Vec<&'static str> {
        let configs = self.configurations();
        if configs.len() != 1 {
            return Vec::new();
        }
        match configs[0].type_code {
            Some(0 | 1 | 3 | 4 | 5 | 14 | 16 | 17) if self.xgk_auto_allocation() => {
                vec!["LD", "ST"]
            }
            Some(0 | 1 | 3 | 4 | 5 | 14 | 16 | 17) => {
                let mut languages = vec!["LD"];
                if cfg!(all(feature = "write", feature = "il")) {
                    languages.push("IL");
                }
                languages
            }
            Some(103 | 108 | 109 | 112 | 113 | 114 | 115 | 116) => vec!["ST", "IL"],
            Some(101) => vec!["ST", "IL"],
            Some(100 | 102 | 104 | 106 | 107 | 111) => vec!["LD", "SFC", "ST", "IL"],
            // CPUS/P source serialization has not yet been captured natively.
            Some(110) => vec!["LD"],
            _ => Vec::new(),
        }
    }
}
