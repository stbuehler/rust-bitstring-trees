use bitstring::BitString;
use std::option::Option;
use super::*;

#[derive(Clone,Copy,PartialEq,Eq)]
enum Direction {
	Left,
	Right,
	Up,
}
use self::Direction::*;

/// Iterate over tree
pub struct Iter<'a, S: BitString+'a, V: 'a> {
	stack: Vec<(Direction, &'a Node<S, V>)>,
}

impl<'a, S: BitString+Clone, V> Iter<'a, S, V> {
	/// new iterator
	pub fn new(tree: &'a RadixMap<S, V>) -> Self {
		match tree.root() {
			None => Iter{
				stack: Vec::new(),
			},
			Some(node) => Iter{
				stack: vec![(Left, node)],
			},
		}
	}
}

impl<'a, S: BitString+Clone, V> Iterator for Iter<'a, S, V> {
	type Item = (&'a S, &'a V);

	fn next(&mut self) -> Option<Self::Item> {
		if self.stack.is_empty() {
			return None;
		}

		// go up in tree from last visited node
		while Up == self.stack[self.stack.len()-1].0 {
			if 1 == self.stack.len() {
				self.stack.clear();
				return None;
			}

			self.stack.pop();
			// stack cannot be empty yet!
			debug_assert!(!self.stack.is_empty());
		}

		loop {
			let top = self.stack.len() - 1;
			let (dir, node) = self.stack[top];

			debug_assert!(!self.stack.is_empty());
			// go down in tree to next node
			match dir {
				Left => {
					match *node {
						Node::InnerNode(ref inner) => {
							self.stack[top].0 = Right;
							self.stack.push((Left, inner.left()));
						},
						Node::Leaf(ref leaf) => {
							self.stack[top].0 = Up;
							return Some((&leaf.key, &leaf.value));
						}
					}
				},
				Right => {
					match *node {
						Node::InnerNode(ref inner) => {
							self.stack[top].0 = Up;
							self.stack.push((Left, inner.right()));
						},
						Node::Leaf(_) => unreachable!(),
					}
				},
				Up => unreachable!(),
			}
		}
	}
}
