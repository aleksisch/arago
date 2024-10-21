use crate::dag::{Id, Dag};
use crate::operations::Operation;
use crate::scheduler::Scheduler;

mod dag;
mod operations;
mod scheduler;
mod devices;
mod regalloc;
mod instruction;

fn main() {
    let nodes = vec![
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VScaMul),
        Some(Operation::VMax),
        Some(Operation::VMax),
        Some(Operation::VMax),
        Some(Operation::VMax),
        Some(Operation::VMax),
        Some(Operation::VMax),
        Some(Operation::VMax),
        None,
    ];
    let edges: Vec<(Id, Id)> = [
        (14, 0),
        (14, 1),
        (14, 2),
        (14, 3),

        (14, 7),
        (14, 8),
        (14, 9),
        (14, 10),

        (0, 4),
        (1, 4),
        (2, 5),
        (3, 5),
        (4, 6),
        (5, 6),

        (7, 11),
        (8, 11),
        (9, 12),
        (10, 12),
        (11, 13),
        (12, 13),
    ].iter().map(|(a, b)| (*a as Id, *b as Id)).collect();

    let dag = Dag::new(nodes, edges);
    let scheduler = Scheduler::new(dag);
    println!("Baseline execution: {:?}", scheduler.baseline_execute());
    println!("Optimal execution: {:?}", scheduler.optimal_execute());
}
