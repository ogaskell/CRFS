use crate::conflict_res::ast_doc::types;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum TestTag {
    Parent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum TestLeaf {
    Content,
}

impl types::TagLike for TestTag {
    fn root() -> Self {
        Self::Parent
    }
}

type TestDoc = types::Doc<TestTag, TestLeaf>;

#[test]
fn bottom_up_test() {
    let doc = TestDoc{
        items: types::Container::from([
            (0, types::Node::Parent{
                id: 0, tag: TestTag::Parent, children: types::Children::from(
                    (Vec::from([1, 2, 3]).into_iter(), Uuid::nil())
                )
            }),
            (1, types::Node::Parent{
                id: 1, tag: TestTag::Parent, children: types::Children::from(
                    (Vec::from([4, 5]).into_iter(), Uuid::nil())
                )
            }),
            (2, types::Node::Leaf{
                id: 2, content: TestLeaf::Content,
            }),
            (3, types::Node::Parent{
                id: 3, tag: TestTag::Parent, children: types::Children::from(
                    (Vec::from([6]).into_iter(), Uuid::nil())
                )
            }),
            (4, types::Node::Parent{
                id: 3, tag: TestTag::Parent, children: types::Children::from(
                    (Vec::from([7]).into_iter(), Uuid::nil())
                )
            }),
            (5, types::Node::Leaf{
                id: 5, content: TestLeaf::Content,
            }),
            (6, types::Node::Leaf{
                id: 6, content: TestLeaf::Content,
            }),
            (7, types::Node::Leaf{
                id: 7, content: TestLeaf::Content,
            }),
        ]),
        root: 0,
    };

    let result = doc.bottom_up();

    dbg!(&result);

    assert_eq!(
        result,
        Vec::from([7, 4, 5, 1, 2, 6, 3, 0]),
    )
}
