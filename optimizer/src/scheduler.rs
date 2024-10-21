use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::zip;

use crate::dag::{Id, Dag};
use crate::devices::{AragoSpec, Register};
use crate::instruction::Instruction;
use crate::regalloc::RegAlloc;

pub(crate) struct Scheduler {
    dag: Dag,
}

impl Scheduler {

    pub fn new(dag: Dag) -> Self {
        Self { dag }
    }

    /// Main logic, chooses best option to execute dag.
    /// 1. Finds critical path
    /// 2. Pick instructions which can be done, with priority to critical path instructions
    /// 3. allocates registers to avoid extra moves
    /// 4. todo: reshuffle again with new dependencies on registers
    pub fn optimal_execute(&self) -> (u32, Vec<Id>) {
        let mut device = AragoSpec::default();
        let stat = device.reg_stat();
        let order = self.dag.efficient_sort();
        let mut chip_order = Vec::new();
        for id in &order {
            let op = &self.dag.all_nodes[*id].borrow().op;
            let chip = op.clone().map(|x| device.get_core(&x).info.chip());
            chip_order.push((*id, chip));
        }

        let id_to_order = Self::map_id_to_order(&order);

        let inputs = self.dag.all_nodes
            .iter()
            .map(|x| (x.borrow().id, x.borrow().sources.clone()))
            .collect();
        let users = self.dag.all_nodes
            .iter()
            .map(|x| (
                x.borrow().id,
                x.borrow().users.iter()
                    .map(|x| id_to_order[x])
                    .collect()
                )
            ).collect();
        let regalloc = RegAlloc::new(stat);
        let moves = regalloc.regalloc(chip_order, inputs, users);
        (self.exec_time(moves, device), order)
    }

    /// Basic execution for comparison.
    /// 1. Any topological sort
    /// 2. Just moves everything to device before each instruction
    pub fn baseline_execute(&self) -> (u32, Vec<Id>) {
        let device = AragoSpec::default();

        let order: Vec<Id> = self.dag.top_sort().into_iter().flatten().collect();
        let mut moves = Vec::new();
        for id in &order {
            let srcs = self.dag.all_nodes[*id].borrow().sources.clone();
            let regs = (0..srcs.len()).map(|x| Register::new(x as u8));
            moves.push(Instruction::new(
                *id, Register::new(srcs.len() as u8), zip(srcs.into_iter(), regs).collect()
            ));
        }
        (self.exec_time(moves, device), order)
    }

    /// Executes vector of instructions,
    /// returns elapsed time
    fn exec_time(&self, moves: Vec<Instruction>, mut device: AragoSpec) -> u32 {
        assert_eq!(moves.len(), moves.len());

        let mut queue = VecDeque::new();
        for inst in &moves {
            let required = HashSet::<Id>::from_iter(self.dag.all_nodes[inst.id].borrow().sources.iter().cloned());
            for (id, op) in &inst.pre_move {
                device.to_device(id, op);
            }
            let op = &self.dag.all_nodes[inst.id].borrow().op;
            match op {
                Some(x) => device.schedule(x, &inst.id, &inst.res_reg, &required),
                None => device.to_device(&inst.id, &inst.res_reg),
            }
            queue.push_back(inst.id);
        }
        device.elapsed_time()
    }

    fn map_id_to_order(order: &[Id]) -> HashMap<Id, usize> {
        zip(0..order.len(), order.iter())
            .map(|(ord, id)| (*id, ord)).collect::<HashMap<Id, usize>>()
    }
}