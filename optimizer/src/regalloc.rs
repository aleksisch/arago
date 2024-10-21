use crate::dag::Id;
use crate::devices::Register;
use crate::operations::ChipType;
use std::collections::{HashMap, VecDeque};
use crate::instruction::Instruction;

/// Simple furthest live Register Allocation.
/// Return Vec of Vec, which holds values which should be moved to register at step i.
pub struct RegAlloc {
    free_regs: HashMap<ChipType, Vec<Register>>,
    memory: HashMap<ChipType, HashMap<Id, Register>>,
}

impl RegAlloc {

    pub fn new(reg_num: HashMap<ChipType, u32>) -> RegAlloc {
        let free_regs = reg_num.iter().map(|(k, v)| (k.clone(), (0..*v).map(|x| Register::new(x as u8)).collect())).collect();
        let memory = HashMap::<ChipType, HashMap<Id, Register>>::from_iter(
            reg_num.keys().map(|chip| (chip.clone(), HashMap::new()))
        );

        RegAlloc { free_regs, memory }
    }

    /// In current model we assume:
    /// - Each chip has its own set of register
    /// - However, all cores on chip share registers
    ///
    /// These requirements allow us to not care about return value overwrite
    ///
    /// Otherwise, we can't rely on runtime scheduler behaviour,
    /// and it will be complex to allocate registers.
    pub fn regalloc(mut self, order: Vec<(Id, Option<ChipType>)>, inputs: HashMap<Id, Vec<Id>>, mut users: HashMap<Id, VecDeque<usize>>) -> Vec<Instruction> {
        let _ = self.memory.iter_mut().map(|(_chip, reg)| reg.clear());
        let mut res = Vec::new();

        for (i, (inst, chip)) in order.iter().enumerate() {
            self.sync_memory(&mut users, i);
            match chip {
                None => {
                    assert!(inputs[inst].is_empty());
                    // Global input data
                    let reg = self.alloc_reg(&ChipType::Scalar, &users);
                    let prev = self.memory.get_mut(&ChipType::Scalar).unwrap().insert(*inst, reg.clone());
                    assert!(prev.is_none());
                    res.push(Instruction::new(*inst, reg, Vec::new()));
                }
                Some(chip) => {
                    // Regular instruction
                    let mut ins = Vec::new();
                    for id in &inputs[inst] {
                        if !self.memory.get_mut(chip).unwrap().contains_key(id) {
                            let reg = self.alloc_reg(chip, &users);
                            let prev = self.memory.get_mut(chip).unwrap().insert(*id, reg.clone());
                            assert!(prev.is_none());
                            ins.push((*id, reg));
                        }
                    }
                    let reg = self.alloc_reg(chip, &users);
                    res.push(Instruction::new(*inst, reg.clone(), ins));
                    self.memory.get_mut(chip).unwrap().insert(*inst, reg);
                }
            }
        }
        res

    }

    /// Remove all `used` users of task
    fn sync_memory(&mut self, users: &mut HashMap<Id, VecDeque<usize>>, step: usize) {
        for (_chip, regs) in self.memory.iter_mut() {
            for (id, _) in regs.iter_mut() {
                while !users[id].is_empty() && users[id].front().unwrap() < &step {
                    users.get_mut(id).unwrap().pop_front();
                }
            }
        }
    }

    /// Finds free register or removes some data
    fn alloc_reg(&mut self, chip: &ChipType, users: &HashMap<Id, VecDeque<usize>>) -> Register {
        let free_regs = self.free_regs.get_mut(chip).unwrap();
        if free_regs.is_empty() {
            let mem_ref = self.memory.get_mut(chip).unwrap();
            let (id, reg) = mem_ref.iter_mut().max_by(|(id1, _), (id2, _)| users[id1].cmp(&users[id2])).unwrap();
            let idd = *id;
            let regg = reg.clone();
            mem_ref.remove(&idd).unwrap();
            free_regs.push(regg);
        }
        free_regs.pop().unwrap()
    }
}

