//! Native comparison contacts, separate from the generated application catalog.
use crate::instruction_catalog::LadderInstructionSpec;

/// Comparison contacts validated by native XG5000 Check Program and Save As.
/// These use the
/// instruction envelope with contact flags instead of output flags.
pub fn ladder_comparison_catalog() -> &'static [LadderInstructionSpec] {
    &[
        LadderInstructionSpec {
            mnemonic: "=",
            opcode: 1254,
            operand_count: 2,
        },
        LadderInstructionSpec {
            mnemonic: ">",
            opcode: 1256,
            operand_count: 2,
        },
        LadderInstructionSpec {
            mnemonic: "<",
            opcode: 1258,
            operand_count: 2,
        },
        LadderInstructionSpec {
            mnemonic: ">=",
            opcode: 1260,
            operand_count: 2,
        },
        LadderInstructionSpec {
            mnemonic: "<=",
            opcode: 1262,
            operand_count: 2,
        },
        LadderInstructionSpec {
            mnemonic: "<>",
            opcode: 1264,
            operand_count: 2,
        },
    ]
}
