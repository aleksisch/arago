use crate::dag::Id;
use crate::devices::Register;

/// Right now we have only 1 instruction.
/// Which basically stores info abou:
/// - what inpus are needed to be transfered
/// - where to store result
pub struct Instruction {
    // instruction id
    pub id: Id,
    // register to store result
    pub res_reg: Register,
    // required moves before execution
    pub pre_move: Vec<(Id, Register)>,
}

impl Instruction {
    pub fn new(id: Id, res_reg: Register, pre_move: Vec<(Id, Register)>) -> Self {
        Self { id, res_reg, pre_move }
    }
}
