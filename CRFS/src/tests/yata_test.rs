use crate::conflict_res::ast_doc::yata;
use std::collections::HashMap;

#[test]
fn yata_test1() {
    let mut arr = yata::Array {
        items: HashMap::from([
            (1, yata::Insertion{origin: yata::Ref::Left, left: yata::Ref::Left, right: yata::Ref::Right, content: 1, deleted: false, creator: 0})
        ]),
        head: Some(1),
        tail: Some(1),
    };

    arr.insert(yata::Insertion{origin: yata::Ref::Item(1), left: yata::Ref::Item(1), right: yata::Ref::Right, content: 2, deleted: false, creator: 0}, None);

    // dbg!(&arr);
    // dbg!(&arr.in_order());
}

#[test]
fn yata_test2() {
    let mut arr1 = yata::Array::from(([1, 2, 3, 4, 5].into_iter(), 0));
    let mut arr2 = arr1.clone();

    let ops = [
        arr1.get_insertion(0, 0, 0),
        arr1.get_insertion(2, 6, 0),
        arr1.get_insertion(5, 7, 0),
    ];

    let _:Vec<_> = ops.iter().map(|(id, op)| arr1.insert(*op, Some(*id))).collect();
    let _:Vec<_> = ops.iter().rev().map(|(id, op)| arr2.insert(*op, Some(*id))).collect();

    // dbg!(arr1.in_order_content(), arr2.in_order_content());

    assert_eq!(arr1.in_order(), arr2.in_order());
}

#[test]
fn yata_test3() {
    let mut arr1 = yata::Array::from(([10, 20, 30, 40, 50].into_iter(), 0));
    let mut arr2 = arr1.clone();

    let ops1 = [
        (2, 25),
        (3, 26),
        (4, 27),
        (2, 28),
        (3, 29),
        (4, 31),
        (2, 32),
        (3, 33),
        (4, 34),
    ];

    let ops2 = [
        (2, 22),
        (3, 23),
        (2, 21),
        (3, 35),
        (4, 36),
    ];

    let ins1: Vec<_> = ops1.iter().map(|(i, x)| arr1.insert_at(*i, *x, 0)).collect();
    let ins2: Vec<_> = ops2.iter().map(|(i, x)| arr2.insert_at(*i, *x, 1)).collect();

    for (ins, id) in ins1.iter() {
        arr2.insert(*ins, *id);
    }

    for (ins, id) in ins2.iter() {
        arr1.insert(*ins, *id);
    }

    // dbg!(&arr1.in_order_content(), &arr2.in_order_content());

    assert_eq!(arr1.in_order(), arr2.in_order());
}

#[test]
fn yata_test4() {
    let mut arr1 = yata::Array::from(([10, 20, 30, 40, 50].into_iter(), 0));
    let mut arr2 = yata::Array::from(([10, 15, 16, 40, 50].into_iter(), 0));

    arr2.rename_against(&arr1);

    dbg!(arr2.in_order());

    while let Some(op) = arr1.get_op(&arr2, 0) {
        println!("Got op: {:#?}", op);
        match op {
            yata::Op::Insertion(id, ins) => {arr1.insert(ins, Some(id));},
            yata::Op::Deletion(id) => {
                arr1.delete(id);
                let mut to_insert = arr1[id].clone();

                if let yata::Ref::Item(r) = to_insert.right {
                    let mut right: yata::ID = r;

                    dbg!(right);

                    while !arr2.items.contains_key(&right) {
                        if let yata::Ref::Item(r) = arr1[right].right {
                            right = r;
                        } else {
                            to_insert.right = yata::Ref::Right;
                            break;
                        }
                    }

                    if to_insert.right != yata::Ref::Right {to_insert.right = yata::Ref::Item(right)}
                }

                dbg!(&to_insert);

                arr2.insert(to_insert, Some(id));
                dbg!(arr1.items.keys(), arr2.items.keys());
            }
        }
    }

    assert_eq!(arr1.in_order(), arr2.in_order());
    assert_eq!(arr1.in_order_undel(), arr2.in_order_undel());

    assert_eq!(arr1.in_order_content(), arr2.in_order_content());
    assert_eq!(arr1.in_order_content_undel(), arr2.in_order_content_undel());

    assert_eq!(arr2.in_order_content_undel(), Vec::from([10, 15, 16, 40, 50]));
}

#[test]
fn yata_test5() {
    let mut arr1 = yata::Array::from(([10, 20, 30, 40, 50].into_iter(), 0));
    let mut arr2 = yata::Array::from(([10, 15, 16, 40, 50].into_iter(), 0));

    let ops = arr1.get_ops(&mut arr2, 0);

    dbg!(&ops);

    for op in ops.iter() {
        arr1.apply(*op);
    }

    assert_eq!(arr1.in_order_content_undel(), arr2.in_order_content_undel());
}
