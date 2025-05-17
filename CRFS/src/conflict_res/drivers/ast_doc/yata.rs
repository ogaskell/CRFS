use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::ops::{Index, IndexMut};

use serde::{Serialize, Deserialize};

// ID must implement Eq, Hash
pub type ID = u64;
fn unique() -> ID {
    rand::rng().random()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Ref {
    Item(ID),
    Left,
    Right,
}

impl Into<Option<ID>> for Ref {
    fn into(self) -> Option<ID> {
        match self {
            Ref::Item(i) => Some(i),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Insertion<T, C> {
    pub origin: Ref,
    pub left: Ref,
    pub right: Ref,
    pub content: T,
    pub creator: C,
    pub deleted: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Op<T, C> {
    Insertion(ID, Insertion<T, C>),
    Deletion(ID),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Array<T, C> {
    pub items: HashMap<ID, Insertion<T, C>>,
    // pub items: RefCell<Container>,
    pub head: Option<ID>,
    pub tail: Option<ID>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct HashableArray<T, C> (Vec<(ID, Insertion<T, C>)>);

impl<T, C> Into<HashableArray<T, C>> for Array<T, C> where T: Copy, C: Ord + Copy {
    fn into(self) -> HashableArray<T, C> {
        let in_order = self.in_order();
        return HashableArray(
            in_order.into_iter().map(|id| (id, self[id])).collect()
        )
    }
}

impl<T, C> std::hash::Hash for Array<T, C> where T: Copy + std::hash::Hash, C: Ord + Copy + std::hash::Hash {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        let hashable: HashableArray<_, _> = self.clone().into();
        return hashable.hash(state);
    }
}

impl<T, C> std::cmp::PartialEq for Array<T, C> where T: Copy + PartialEq, C: Ord + Copy + PartialEq {
    fn eq(&self, other: &Self) -> bool {
        let h1: HashableArray<_, _> = self.clone().into();
        let h2: HashableArray<_, _> = other.clone().into();
        return h1 == h2;
    }
}

impl<T, C> std::cmp::Eq for Array<T, C> where T: Copy + PartialEq, C: Ord + Copy + Eq {}

impl<T, C> Index<ID> for Array<T, C> {
    type Output = Insertion<T, C>;
    fn index(&self, index: ID) -> &Insertion<T, C> {
        return self.items.get(&index).expect(&format!("No item with ID {index}."));
    }
}

impl<T, C> IndexMut<ID> for Array<T, C> {
    fn index_mut(&mut self, index: ID) -> &mut Insertion<T, C> {
        return self.items.get_mut(&index).expect(&format!("No item with ID {index}."));
    }
}

impl<T, C> Index<usize> for Array<T, C> where T: Copy, C: Ord + Copy {
    type Output = Insertion<T, C>;
    fn index(&self, index: usize) -> &Insertion<T, C> {
        return &self[self.in_order()[index]]
    }
}

impl<T, C> IndexMut<usize> for Array<T, C> where T: Copy, C: Ord + Copy {
    fn index_mut(&mut self, index: usize) -> &mut Insertion<T, C> {
        let order = self.in_order();
        return &mut self[order[index]]
    }
}

impl<T, C> Index<Ref> for Array<T, C> where T: Copy, C: Ord + Copy {
    type Output = Insertion<T, C>;
    fn index(&self, index: Ref) -> &Insertion<T, C> {
        match index {
            Ref::Item(i) => &self[i],
            _ => panic!(),
        }
    }
}

impl<T, C> IndexMut<Ref> for Array<T, C> where T: Copy, C: Ord + Copy {
    fn index_mut(&mut self, index: Ref) -> &mut Insertion<T, C> {
        match index {
            Ref::Item(i) => &mut self[i],
            _ => panic!(),
        }
    }
}

impl<F, T, C> From<(F, C)> for Array<T, C> where F: Iterator<Item = T>, C: Ord + Copy {
    fn from(item: (F, C)) -> Self {
        let (arr, creator) = item;
        let mut result = Self::empty();
        let mut left = Ref::Left;
        for i in arr {
            left = Ref::Item(result.insert_simple(Insertion {
                origin: left, left,
                right: Ref::Right,
                creator,
                content: i,
                deleted: false,
            }, None).unwrap());
        }

        return result;
    }
}

impl<T, C> Array<T, C> where C: Ord {
    pub fn empty() -> Self {
        Self {
            items: HashMap::new(),
            head: None, tail: None,
        }
    }

    pub fn in_order(&self) -> Vec<ID> {
        self.verify();

        let mut next = self.head;
        let mut result = Vec::new();
        while let Some(n) = next {
            next = self.items[&n].right.into();
            result.push(n);
        }
        return result;
    }

    /// Panics if the linked list is broken/malformed.
    pub fn verify(&self) {
        let mut next = self.head;
        let mut visited = HashSet::new();

        while let Some(n) = next {
            if visited.contains(&n) {
                panic!("yata::Array::verify - Cycle detected.")
            }

            let current = &self.items[&n];

            if let Ref::Item(l) = current.left {
                if self.items[&l].right != Ref::Item(n) {
                    panic!("yata::Array::verify - Broken link found. n={}, self.left.right={:?}", n, self.items[&l].right);
                }
            }
            if let Ref::Item(r) = current.right {
                if self.items[&r].left != Ref::Item(n) {
                    panic!("yata::Array::verify - Broken link found. n={}, self.right.left={:?}", n, self.items[&r].left);
                }
            }

            next = self.items[&n].right.into();
            visited.insert(n);
        }
    }

    pub fn in_order_undel(&self) -> Vec<ID> {
        let mut in_order = self.in_order();
        in_order.retain(|id| !self[*id].deleted);
        return in_order;
    }

    pub fn len(&self) -> usize {
        return self.in_order().len();
    }

    pub fn len_undel(&self) -> usize {
        return self.in_order_undel().len();
    }

    pub fn get_index_id(&self, i: ID) -> Option<usize> {
        Some(self.in_order().iter().position(|i_| *i_ == i)?)
    }

    pub fn get_index_ref(&self, r: Ref) -> Option<isize> {
        match r {
            Ref::Item(i) => {
                let in_order = self.in_order();
                Some(in_order.iter().position(|i_| *i_ == i)?.try_into().unwrap())
            },
            Ref::Left => Some(-1),
            Ref::Right => Some(self.items.len().try_into().unwrap()),
        }
    }

    pub fn origin(&self, i: ID) -> Option<Ref> {
        Some(self.items.get(&i)?.origin)
    }

    /// Insert directly according to `left` and `right` in `ins`.
    /// Assumes `left` and `right` are adjacent elements.
    fn insert_simple(&mut self, ins: Insertion<T, C>, id_: Option<ID>) -> Option<ID> {
        let id = match id_ {
            Some(i) => i,
            None => unique(),
        };

        if let Ref::Item(l) = ins.left {self[l].right = Ref::Item(id)}
        else if Ref::Left == ins.left {self.head = Some(id);}

        if let Ref::Item(r) = ins.right {self[r].left = Ref::Item(id)}
        else if Ref::Right == ins.right {self.tail = Some(id);}

        self.items.insert(id, ins);

        self.verify();

        return Some(id);
    }

    pub fn get_insertion(&self, ind: usize, item: T, creator: C) -> (ID, Insertion<T, C>) {
        let in_order = self.in_order(); let len = self.items.len();

        if ind > len {
            panic!("Index {ind} greater than length {len}.");
        }

        let (left, right) = match ind {
            _ if len == 0 => (Ref::Left, Ref::Right),
            0 => (Ref::Left, Ref::Item(in_order[ind])),
            _ if ind == len => (Ref::Item(in_order[ind - 1]), Ref::Right),
            _ => (Ref::Item(in_order[ind - 1]), Ref::Item(in_order[ind]))
        };

        (unique(), Insertion {
            origin: left,
            left, right,
            content: item,
            deleted: false,
            creator,
        })
    }

    pub fn delete(&mut self, id: ID) {
        self[id].deleted = true;
    }
}

impl<T, C> Array<T, C> where C: Ord {
    /// Returns None if there is an error with the references in `ins`
    /// Else returns the new ID of `ins` in the list.
    pub fn insert(&mut self, ins: Insertion<T, C>, id_: Option<ID>) -> Option<ID> {
        let in_order = self.in_order();
        let (l, r) = (self.get_index_ref(ins.left)?, self.get_index_ref(ins.right)?);
        let n_conflicting = r - l - 1;

        let id_i = match id_ {
            Some(id) => id,
            None => unique()
        };

        if n_conflicting == 0 {
            return self.insert_simple(ins, Some(id_i));
        }

        let mut new_left = ins.left;
        for (ind, id_o) in self.in_order().iter().enumerate().skip((l + 1).try_into().unwrap()) {
            if ind >= r.try_into().unwrap() { break; }

            if (
                self.get_index_ref(ins.origin).unwrap() > ind.try_into().unwrap() ||
                self.get_index_ref(ins.origin).unwrap() <= self.get_index_ref(self[*id_o].origin).unwrap()
            ) && (
                self.get_index_ref(ins.origin).unwrap() != self.get_index_ref(self[*id_o].origin).unwrap() ||
                self[*id_o].creator < ins.creator
            ) {
                new_left = Ref::Item(*id_o);
            } else {
                if self.get_index_ref(ins.origin).unwrap() >= self.get_index_ref(self[*id_o].origin).unwrap() {
                    break;
                }
            }
        }

        let mut new_ins = ins; new_ins.left = new_left;
        // new_ins.right = self[<Ref as Into<Option<ID>>>::into(new_left).unwrap()].right;
        new_ins.right = match new_left {
            Ref::Item(i) => self[i].right,
            Ref::Right => Ref::Right,
            Ref::Left => Ref::Item(self.in_order()[0]),
        };
        return self.insert_simple(new_ins, Some(id_i));
    }

    pub fn apply(&mut self, op: Op<T, C>) {
        match op {
            Op::Insertion(id, ins) => {self.insert(ins, Some(id));},
            Op::Deletion(id) => {self.delete(id);},
        }
    }
}

impl<T, C> Array<T, C> where T: Copy, C: Ord + Copy {
    pub fn in_order_content(&self) -> Vec<T> {
        let in_order = self.in_order();
        return in_order.iter().map(|id| self.items[id].content).collect();
    }

    pub fn in_order_content_undel(&self) -> Vec<T> {
        let in_order = self.in_order_undel();
        return in_order.iter().map(|id| self.items[id].content).collect();
    }

    pub fn insert_at(&mut self, ind: usize, item: T, creator: C) -> (Insertion<T, C>, Option<ID>) {
        let (id, ins) = self.get_insertion(ind, item, creator);
        return (ins, self.insert(ins, Some(id)));
    }

    pub fn get_op(&self, other: &Self, creator: C) -> Option<Op<T, C>> {
        let (a, b) = (self.in_order_undel(), other.in_order_undel());
        let lcs = crate::helpers::lcs(a.as_slice(), b.as_slice());

        for a_item in a.iter() {
            if !lcs.contains(&a_item) {return Some(Op::Deletion(*a_item))}
        }

        for b_item in b.iter() {
            // Since we're finding the first element (from the left) not in a, we can assume it's left pointer is in a
            // However it's right pointer may not be. So we find the next item in other that is in self.
            if !lcs.contains(b_item) {
                let mut right = other[*b_item].right;

                while let Ref::Item(r) = right {
                    if a.contains(&r) {break;}
                    right = other[r].right;
                }

                return Some(Op::Insertion(*b_item, Insertion{
                    origin: other[*b_item].left, left: other[*b_item].left,
                    right, content: other[*b_item].content, creator, deleted: false
                }))
            }
        }

        return None;
    }
}

impl<T, C> Array<T, C> where T: Eq {
    /// NOTE that this method assumes there is no duplicate content!!
    pub fn rename_against(&mut self, other: &Self) {
        // Map of (current/old ID) -> (new ID/ID in other)
        let mut renames = HashMap::new();

        for (id, ins) in self.items.iter() {
            if let Some((new_id, _)) = other.items.iter().find(
                |(new_id, x)| x.content == ins.content && renames.values().all(|i| *i != **new_id)
            ) {
                renames.insert(*id, *new_id);
            }
        }

        for (old, new) in renames.iter() {
            let ins = self.items.remove(old).unwrap();
            self.items.insert(*new, ins);
        }

        for (_, ins) in self.items.iter_mut() {
            if let Ref::Item(old) = ins.left {
                if let Some(new) = renames.get(&old) {
                    ins.left = Ref::Item(*new);
                }
            }
            if let Ref::Item(old) = ins.right {
                if let Some(new) = renames.get(&old) {
                    ins.right = Ref::Item(*new);
                }
            }
            if let Ref::Item(old) = ins.origin {
                if let Some(new) = renames.get(&old) {
                    ins.origin = Ref::Item(*new);
                }
            }
        }

        if let Some(head) = self.head {
            if let Some(new) = renames.get(&head) {self.head = Some(*new)}
        }
        if let Some(tail) = self.tail {
            if let Some(new) = renames.get(&tail) {self.tail = Some(*new)}
        }
    }
}

impl<T, C> Array<T, C> where T: Eq + Clone + Copy + std::fmt::Debug, C: Ord + Clone + Copy + std::fmt::Debug {
    pub fn get_ops(&self, other: &mut Self, creator: C) -> Vec<Op<T, C>> {
        let mut a = self.clone();
        let mut ops = Vec::new();
        other.rename_against(self);

        dbg!(&a, &other);

        while let Some(op) = a.get_op(&other, creator) {
            ops.push(op);

            match op {
                Op::Insertion(id, ins) => {a.insert(ins, Some(id));},
                Op::Deletion(id) => {
                    a.delete(id);
                    let mut to_insert = a[id].clone();

                    if let Ref::Item(r) = to_insert.right {
                        let mut right: ID = r;

                        dbg!(right);

                        while !other.items.contains_key(&right) {
                            if let Ref::Item(r) = a[right].right {
                                right = r;
                            } else {
                                to_insert.right = Ref::Right;
                                break;
                            }
                        }

                        if to_insert.right != Ref::Right {to_insert.right = Ref::Item(right)}
                    }

                    other.insert(to_insert, Some(id));
                }
            }
        }

        return ops;
    }

    pub fn rename_creators(&mut self, other: &Self) {
        for (id, ins) in self.items.iter_mut() {
            if let Some(other_ins) = other.items.get(id) {
                ins.creator = other_ins.creator;
            }
        }
    }

    pub fn eq_content(&self, other: &Self) -> bool {
        return std::iter::zip(
            self.in_order_content_undel().iter(), other.in_order_content_undel().iter()
        ).all(|(a, b)| a == b);
    }
}
