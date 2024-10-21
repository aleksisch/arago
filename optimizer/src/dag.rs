use crate::devices::AragoSpec;
use crate::operations::Operation;
use std::cell::RefCell;
use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::iter::zip;
use std::rc::Rc;


pub(crate) type Id = usize;

pub struct Dag {
    inputs: Vec<Rc<RefCell<Node>>>,
    pub all_nodes: Vec<Rc<RefCell<Node>>>,
}

pub struct Node {
    pub(crate) op: Option<Operation>,
    pub(crate) id: Id,
    pub(crate) users: Vec<Id>,
    pub sources: Vec<Id>,
}

impl Node {

    pub fn new(op: Option<Operation>, id: Id) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            op,
            id,
            users: Vec::new(),
            sources: Vec::new(),
        }))
    }

    pub fn add(source: &Rc<RefCell<Self>>, user: &Rc<RefCell<Self>>) {
        source.borrow_mut().users.push(user.borrow().id);
        user.borrow_mut().sources.push(source.borrow().id);
    }
}

impl Dag {
    pub fn new(in_nodes: Vec<Option<Operation>>, edges: Vec<(Id, Id)>) -> Self {
        let all_nodes: Vec<_> = zip(0..in_nodes.len(), in_nodes)
            .map(|(n, op)| Node::new(op, n))
            .collect();
        let mut used = HashSet::new();
        for (from, to) in edges {
            Node::add(&all_nodes[from], &all_nodes[to]);
            used.insert(to);
        }
        let inputs = (0..all_nodes.len())
            .filter(|n| !used.contains(n))
            .map(|i| all_nodes[i].clone())
            .collect();
        Self {
            inputs,
            all_nodes
        }
    }


    /// Implementation of Kunh algorithm.
    /// Find blocks of vertices, which are ordered topologically,
    /// vertices inside same block can be reordered
    /// To get some top sort you can make any permutations inside blocks
    pub fn top_sort(&self) -> Vec<Vec<Id>> {
        // Nodes without input edges (dependencies)
        let mut free: Vec<_> = self.inputs.iter().map(|x| x.as_ref().borrow().id).collect();
        // How many dependencies handled
        let mut done = vec![0; self.all_nodes.len()];

        let mut res = Vec::new();
        while !free.is_empty() {
            let mut next_free = Vec::new();
            for id in &free {
                for user_id in &self.all_nodes[*id].borrow().users {
                    done[*user_id] += 1;
                    if done[*user_id] == self.all_nodes[*user_id].borrow().sources.len() {
                        next_free.push(*user_id);
                    }
                }
            }
            res.push(free);
            free = next_free;
        }
        res
    }

    /// Solves Dijkstra for maximum cost from the end.
    /// With these costs we can find critical path
    fn get_costs(&self) -> Vec<u32> {
        let mut res = vec![0u32; self.all_nodes.len()];
        let device = AragoSpec::default();
        let empty_nodes: VecDeque<_> = self.all_nodes
            .iter()
            .filter(|n| n.borrow().users.is_empty())
            .cloned().collect();
        let mut finish_nodes: BTreeMap<_, _> = BTreeMap::new();

        let mut visited: HashSet<Id> = HashSet::new();

        finish_nodes.insert(0, empty_nodes);

        while !finish_nodes.is_empty() {
            let (t, v) = finish_nodes.iter_mut().next().unwrap();
            let t_copy = *t;
            let node = v.pop_front().unwrap();
            if v.is_empty() {
                finish_nodes.pop_first();
            }

            if visited.contains(&node.borrow().id) {
                // It's not critical path
                continue;
            }
            visited.insert(node.borrow().id);
            let new_cost = if let Some(op) = &node.borrow().op {
                t_copy + device.get_cost(op)
            } else {
                t_copy
            };
            assert!(res[node.borrow().id] <= new_cost);
            res[node.borrow().id] = new_cost;
            for src in &node.borrow().sources {
                finish_nodes.entry(new_cost).or_insert(VecDeque::new()).push_back(self.all_nodes[*src].clone());
            }
        }
        res
    }

    /// Heuristically chooses topological order with the lowest calculation time
    pub(crate) fn efficient_sort(&self) -> Vec<Id> {
        let mut res = Vec::with_capacity(self.all_nodes.len());
        let costs = self.get_costs();
        let mut cycles = 0;
        let mut active = BTreeMap::<u32, Vec<Id>>::new();
        let mut device = AragoSpec::default();
        let mut free: HashMap<Operation, BTreeMap<u32, Vec<Id>>> = HashMap::new();
        let mut done = vec![0; self.all_nodes.len()];
        for node in &self.inputs {
            res.push(node.borrow().id);
            active.entry(0).or_default().push(node.borrow().id);
        }
        while !active.is_empty() || !free.is_empty() {
            let mut elem: Option<Id> = None;
            for (op, nodes) in &mut free {
                if nodes.is_empty() {
                    continue;
                }
                let core = device.get_core(op);
                core.update_time(cycles);
                let (_weight, v) = nodes.iter_mut().last().unwrap();
                let task_id = *v.last().unwrap();
                if core.try_add(task_id, cycles) {
                    elem = Some(task_id);
                    active.entry(cycles + device.get_cost(op)).or_default().push(task_id);
                    assert!(!res.contains(&task_id));
                    res.push(task_id);
                    v.pop();
                    if v.is_empty() {
                        nodes.pop_last();
                    }
                    break;
                }
            }
            if elem.is_some() {
                continue;
            }
            match active.pop_first() {
                None => break,
                Some((top, tasks)) => {
                    cycles = max(top, cycles);
                    assert!(!tasks.is_empty());
                    for task in tasks {
                        for user in &self.all_nodes[task].borrow().users {
                            let child = self.all_nodes[*user].borrow();
                            done[*user] += 1;
                            let srcs = &self.all_nodes[*user].borrow().sources;
                            assert!(done[*user] <= srcs.len());
                            if done[*user] == srcs.len() {
                                let entry = free.entry(child.op.clone().unwrap())
                                    .or_default();
                                entry.entry(costs[*user]).or_default().push(*user);
                            }
                        }
                    }
                },
            };
        }
        assert_eq!(res.len(), self.all_nodes.len());
        res
    }
}