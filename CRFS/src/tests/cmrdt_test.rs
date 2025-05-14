use crate::conflict_res::drivers::{directed_graph, file_tree::DriverID, CmRDT};
use CmRDT::Object;
use directed_graph::{Graph, GraphObject, GraphOp};

use std::collections::HashSet;

use uuid::Uuid;

#[test]
fn directed_graph_test() -> () {
    let mut obj1: GraphObject<u8> = GraphObject::init(DriverID::Driver(0));
    let mut obj2: GraphObject<u8> = GraphObject::init(DriverID::Driver(1));

    let mut ops: Vec<GraphOp<u8>> = Vec::new();

    let graph1 = Graph{
        v: HashSet::from([1, 2, 3, 4]),
        a: HashSet::from([(1, 2), (3, 4)]),
    };
    let graph2 = Graph{
        v: HashSet::from([2, 3]),
        a: HashSet::from([(2, 3)]),
    };

    // Read graph1 into obj1
    while let Some(op) = obj1.prep(&graph1, Uuid::from_u128(1)) {
        ops.push(op.clone());
        obj1.apply_op(&op).unwrap();
    };

    dbg!(&obj1.query());

    // Read graph2 into obj2
    while let Some(op) = obj2.prep(&graph2, Uuid::from_u128(2)) {
        ops.push(op.clone());
        obj2.apply_op(&op).unwrap();
    };

    // Merge obj1 and obj2
    for op in ops {
        obj1.apply_op(&op).unwrap();
        obj2.apply_op(&op).unwrap();
    }

    dbg!(&obj1.query());
    dbg!(&obj2.query());

    assert_eq!(obj1.query(), obj2.query());
}
