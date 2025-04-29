use std::path::PathBuf;
use std::collections::HashSet;

use crate::conflict_res::drivers::CmRDT::{DiskType, Object};
use crate::conflict_res::drivers::ast_doc;
use ast_doc::types::{Doc, Node, FileInterface, Children};
use ast_doc::crdt::{DocObject, DocOp};
use ast_doc::md;
use markdown_ast::ast_to_markdown;

use crate::storage;

use uuid::Uuid;

const TESTFILEDIR: &str = ".testfiles";

#[test]
fn read_test() {
    let path = PathBuf::from(format!("{}/test.md", TESTFILEDIR));
    let loc = storage::ObjectLocation::OnDisk(path.clone());

    let raw_md = std::fs::read_to_string(path).unwrap();
    let canon = markdown_ast::canonicalize(&raw_md);

    let int = *md::MDInterface::read(&loc).unwrap();

    let doc = int.generate(Uuid::nil());

    dbg!(&doc);

    let mdast = md::MDInterface::to_blocks(doc.get_root_children(), &doc);

    let new_md = markdown_ast::ast_to_markdown(&mdast);
    println!("{}", &new_md);

    assert_eq!(new_md, canon);
}

#[test]
fn eq_content_test() {
    let node1 = Node::<md::MDTag, md::MDLeaf>::Parent{
        id: 1,
        tag: md::MDTag::Heading(markdown_ast::HeadingLevel::H1),
        children: Children::from((Vec::from([2, 4]).into_iter(), Uuid::nil())),
    };

    assert!(node1.eq_content(&node1));
}

#[test]
fn generate_against_test() {
    let path = PathBuf::from(format!("{}/test.md", TESTFILEDIR));
    let loc = storage::ObjectLocation::OnDisk(path.clone());

    let int = *md::MDInterface::read(&loc).unwrap();

    let doc1 = int.generate(Uuid::from_u128(1));
    let doc2 = int.generate_against(&doc1, Uuid::from_u128(2));

    dbg!(&doc1);
    dbg!(&doc2);

    let keys1: HashSet<_> = doc1.items.keys().collect();
    let keys2: HashSet<_> = doc2.items.keys().collect();

    let diff1: HashSet<_> = keys1.difference(&keys2).collect();
    let diff_w_n1: HashSet<_> = diff1.into_iter().map(|x| (x, doc1.items.get(*x).unwrap())).collect();
    let diff2: HashSet<_> = keys2.difference(&keys1).collect();
    let diff_w_n2: HashSet<_> = diff2.into_iter().map(|x| (x, doc2.items.get(*x).unwrap())).collect();

    dbg!(diff_w_n1);
    dbg!(diff_w_n2);

    assert_eq!(doc1, doc2);
}

#[test]
fn md_merge_test() {
    let paths = [
        PathBuf::from(format!("{}/test1.md", TESTFILEDIR)),
        PathBuf::from(format!("{}/test2.md", TESTFILEDIR)),
        PathBuf::from(format!("{}/test3.md", TESTFILEDIR)),
        PathBuf::from(format!("{}/test4.md", TESTFILEDIR)),
    ];
    let locs: Vec<_> =
        paths.iter().map(|path| storage::ObjectLocation::OnDisk(path.clone())).collect();

    let ints: Vec<_> =
        locs.iter().map(|loc| *md::MDInterface::read(loc).unwrap()).collect();

    let mut object1 = md::MDObject::init(); let id1 = Uuid::from_u128(1);
    let mut object2 = md::MDObject::init(); let id2 = Uuid::from_u128(2);

    println!("Initialised.");

    let mut init_ops = Vec::new();

    while let Some(op) = object1.prep(&ints[0], id1) {
        object1.apply_op(&op);
        object2.apply_op(&op);
        init_ops.push(op);
    }

    dbg!(object1.query().get_canon());

    assert_eq!(object1.query().get_canon(), object2.query().get_canon());

    let mut ops2 = Vec::new();
    while let Some(op) = object1.prep(&ints[1], id1) {
        object1.apply_op(&op);
        ops2.push(op);
    }

    dbg!(object1.query().get_canon());

    let mut ops3 = Vec::new();
    while let Some(op) = object2.prep(&ints[2], id2) {
        object2.apply_op(&op);
        ops3.push(op);
    }

    dbg!(object2.query().get_canon());

    for op in ops2.iter() {object2.apply_op(op);}
    for op in ops3.iter() {object1.apply_op(op);}

    println!("Object 1:\n{}\n\nObject 2:\n{}\n\nReference:\n{}", object1.query().get_canon(), object2.query().get_canon(), ints[3].get_canon());
    assert_eq!(object1.query().get_canon(), object2.query().get_canon());
    // assert_eq!(object1.query().get_canon(), ints[3].get_canon());
}
