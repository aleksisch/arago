use crate::dag::Id;
use crate::operations::{ChipType, Operation};
use std::cmp::max;
use std::collections::{HashMap, HashSet, VecDeque};
use log::debug;

pub const TRANSFER_TIME: u32 = 1;
pub const POINTWISE_COST: u32 = 1000;
pub const OPAC_COST: u32 = 1;

type Time = u32;

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct Register(u8);

impl Register {
    pub fn new(value: u8) -> Register {
        Register(value)
    }
}

/// Emulator of arago device.
/// Main purpose is to measure execution time.
///
/// Note: Register set per core is not implemented yet,
/// although it's assumed in regalloc, so it may lead to some bugs here
/// (But I think on real device there will be separate registers, so
/// such regalloc will lead to better chips memory utilization)
pub struct AragoSpec {
    /// Number OPAC instructions can be run in parallel
    active_mult_cores: Core,

    /// Number of pointwise instructions can be run in parallel
    active_pointwise_cores: Core,

    /// Number of registers
    active_memory: HashMap<Id, Register>,
    registers: HashMap<Register, Id>,
    done_tasks: HashSet<Id>,
    last_task: Option<usize>,

    time: Time,
}

pub trait Chip {
    fn max_cores(&self) -> u32;
    fn max_reg(&self) -> u32;
    fn cost(&self) -> u32;
    fn chip(&self) -> ChipType;
    fn name(&self) -> &'static str;

}

pub struct Core {
    active: VecDeque<(Id, Time)>,
    pub(crate) info: Box<dyn Chip>,
}

impl Core {
    fn scalar_core() -> Self {
        Self {
            active: VecDeque::new(),
            info: Box::new(ScalarCore::new()),
        }
    }

    fn matrix_core() -> Self {
        Self {
            active: VecDeque::new(),
            info: Box::new(MatrixCore::new()),
        }
    }

    /// Add task to device
    /// Return None if there's free cores,
    /// otherwise task Id, which will finish first
    pub fn add(&mut self, id: Id, time: Time) -> Option<(Id, Time)> {
        let res = self.ensure_free();
        self.active.push_back((id, res.map(|x| x.1).unwrap_or(time) + self.info.cost()));
        res
    }

    fn ensure_free(&mut self) -> Option<(Id, Time)> {
        if self.is_full() {
            Some(self.active.pop_front().unwrap())
        } else {
            None
        }
    }

    /// Tries to add task to device, starting at time `time`.
    pub fn try_add(&mut self, id: Id, time: Time) -> bool {
        if !self.is_full() {
            self.active.push_back((id, time + self.info.cost()));
            true
        } else {
            false
        }
    }

    /// Sync core to time `time`, removes done tasks.
    /// Returns number of done tasks
    pub fn update_time(&mut self, time: Time) -> usize {
        let mut res = 0;
        while !self.active.is_empty() && self.active.front().unwrap().1 <= time {
            self.active.pop_front();
            res += 1;
        }
        res
    }

    pub fn usage(&self) -> usize {
        self.active.len()
    }

    pub fn is_full(&self) -> bool {
        assert!(self.active.len() <= self.info.max_cores() as usize);
        self.active.len() == self.info.max_cores() as usize
    }

}

struct ScalarCore {}
impl ScalarCore {
    fn new() -> ScalarCore { Self {} }
}

impl Chip for ScalarCore {

    fn max_cores(&self) -> u32 {
        1
    }

    fn max_reg(&self) -> u32 {
        16
    }

    fn cost(&self) -> u32 {
        POINTWISE_COST
    }

    fn chip(&self) -> ChipType {
        ChipType::Scalar
    }

    fn name(&self) -> &'static str {
        "scalar"
    }

}

struct MatrixCore {}
impl MatrixCore {
    fn new() -> MatrixCore { Self {} }
}

impl Chip for MatrixCore {
    fn max_cores(&self) -> u32 {
        1
    }

    fn max_reg(&self) -> u32 {
        16
    }

    fn cost(&self) -> u32 {
        OPAC_COST
    }

    fn chip(&self) -> ChipType {
        ChipType::Opac
    }

    fn name(&self) -> &'static str {
        "matrix"
    }
}

impl AragoSpec {
    pub fn default() -> AragoSpec {
        AragoSpec {
            active_memory: HashMap::new(),
            registers: HashMap::new(),
            active_mult_cores: Core::matrix_core(),
            active_pointwise_cores: Core::scalar_core(),
            done_tasks: HashSet::new(),
            last_task: None,
            time: 0,
        }
    }

    /// Schedules task for execution and "execute" it.
    pub fn schedule(&mut self, op: &Operation, id: &Id, reg: &Register, inputs: &HashSet<Id>) {
        self.last_task = Some(*id);
        debug!("{:?} {:?}", inputs, self.active_memory);
        for input in inputs {
            assert!(self.active_memory.contains_key(input));
        }

        let time = self.time;
        let core = self.get_core(op);
        let exec = core.add(*id, time);
        self.store_memory(*id, reg.clone());
        match exec {
            Some((id_, time)) => {
                self.update_time(time);
                debug!("schedule id={:?} time={:?} {} {:?}", id, self.time.clone(), self.get_core(op).info.name(), self.get_core(op).usage());
                self.done_tasks.insert(id_);
            },
            None => {
                debug!("schedule id={:?} time={:?} {} {:?}", id, self.time.clone(), self.get_core(op).info.name(), self.get_core(op).usage());
            }
        }
    }

    /// Move input to device. Before execution all inputs must be presented in device memory
    pub fn to_device(&mut self, id: &Id, reg: &Register) {
        self.time += TRANSFER_TIME;
        debug!("{:?} -> {:?} | {:?}", id, reg, self.time);
        self.done_tasks.insert(*id);
        self.store_memory(*id, reg.clone());
    }

    pub fn reg_stat(&self) -> HashMap<ChipType, u32> {
        HashMap::from([
            (ChipType::Opac, self.active_mult_cores.info.max_reg()),
            (ChipType::Scalar, self.active_pointwise_cores.info.max_reg())
            ]
        )
    }

    /// How many times passed from start
    pub fn elapsed_time(&mut self) -> u32 {
        debug!("{}", self.time);
        while self.step().is_some() {}
        self.time
    }

    pub fn get_cost(&self, op: &Operation) -> u32 {
        match op {
            Operation::VAdd | Operation::VMin | Operation::VMax => POINTWISE_COST,
            Operation::VScaMul => OPAC_COST,
        }
    }


    fn store_memory(&mut self, id: Id, reg: Register) -> Option<()> {
        self.active_memory.insert(id, reg.clone());
        self.registers.insert(reg, id);
        Some(())
    }

    fn update_time(&mut self, time: u32) {
        self.time = max(time, self.time);
    }

    pub fn get_core(&mut self, op: &Operation) -> &mut Core {
        match op {
            Operation::VAdd | Operation::VMin | Operation::VMax => &mut self.active_pointwise_cores,
            Operation::VScaMul => &mut self.active_mult_cores,
        }
    }

    /// Picks first scheduled task, which is not done yet,
    /// updates time, so this task will be done
    fn step(&mut self) -> Option<Id> {
        let mult = self.active_mult_cores.active.front();
        let scalar = self.active_pointwise_cores.active.front();
        let core = match (mult, scalar) {
            (Some((_, t1)), Some((_, t2))) => {
                if t1 < t2 {
                    &mut self.active_mult_cores
                } else {
                    &mut self.active_pointwise_cores
                }
            }
            (None, Some(_)) => &mut self.active_pointwise_cores,
            (Some(_), None) => &mut self.active_mult_cores,
            (None, None) => return None,
        };
        core.active.pop_front().map(|(id, time)| {
            self.update_time(time);
            self.done_tasks.insert(id);
            id
        })
    }
}