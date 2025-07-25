use super::GeneratorCasts;
use crate::asm::*;
use language::*;

impl<'a> GeneratorCasts<'a> {
    pub fn cast_return(&self, instr: &TacInstruction) -> Vec<AsmInstruction> {
        if let TacInstruction::Return { val } = instr {
            vec![
                AsmInstruction::Mov {
                    src: convert_operand(val),
                    dst: AsmOperand::new_reg(AsmReg::AX),
                },
                AsmInstruction::Ret,
            ]
        } else {
            unreachable!()
        }
    }
}
