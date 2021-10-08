use super::*;
use bitstring::BitString;
use std::option::Option;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
	Down,
	Left,
	Right,
	Up,
}
use self::Direction::*;

/// Iterate over tree
pub struct IterFull<'a, S: BitString + 'a, V: 'a> {
	stack: Vec<(Direction, &'a Node<S, V>)>,
	depth: usize,
}

impl<'a, S: BitString + Clone, V> IterFull<'a, S, V> {
	/// new iterator
	pub fn new(tree: &'a RadixMap<S, V>) -> Self {
		match tree.root() {
			None => IterFull {
				stack: Vec::new(),
				depth: 0,
			},
			Some(node) => IterFull {
				stack: vec![(Down, node)],
				depth: 0,
			},
		}
	}
}

impl<'a, S: BitString + Clone, V> Iterator for IterFull<'a, S, V> {
	type Item = (S, Option<&'a V>);

	fn next(&mut self) -> Option<Self::Item> {
		if self.stack.is_empty() {
			if self.depth == 0 {
				// empty tree and first call
				self.depth = !0;
				return Some((S::null(), None));
			} else {
				return None;
			}
		}

		// go up in tree from last visited node
		while Up == self.stack[self.stack.len() - 1].0 {
			if 0 == self.depth {
				// all done
				debug_assert_eq!(1, self.stack.len());
				self.stack.clear();
				self.depth = !0;
				return None;
			}

			if self.stack.len() > 1 {
				// next node up the tree must be an inner node, and
				// covers the first bit of both branches
				let up_len = self.stack[self.stack.len() - 2].1.key().len();
				if self.depth - 1 == up_len {
					// done walking up this branch
					self.stack.pop();
					self.depth = up_len;
					debug_assert!(!self.stack.is_empty());
					// stack cannot be empty yet!
					continue;
				}
			}

			// still walking up current branch
			let key = self.stack[self.stack.len() - 1].1.key();
			self.depth -= 1;
			if key.get(self.depth) {
				// already walked that side when going Down
			} else {
				let mut key = key.clone();
				key.clip(self.depth + 1);
				key.flip(self.depth);
				return Some((key, None));
			}
		}

		loop {
			let top = self.stack.len() - 1;
			let (dir, node) = self.stack[top];

			debug_assert!(!self.stack.is_empty());
			// go down in tree to next node
			match dir {
				Down => loop {
					let key = node.key();
					// next node up the tree must be an inner node, and
					// covers the first bit of both branches
					let key_len = key.len();
					if self.depth == key_len {
						// done walking down this branch
						self.stack[top].0 = Left;
						break;
					}

					debug_assert!(self.depth < key_len);

					// still walking down current branch
					if key.get(self.depth) {
						let mut key = key.clone();
						key.flip(self.depth);
						self.depth += 1;
						key.clip(self.depth);
						return Some((key, None));
					} else {
						// will walk that side when going Up
						self.depth += 1;
					}
				},
				Left => {
					debug_assert_eq!(self.depth, node.key().len());
					match *node {
						Node::InnerNode(ref inner) => {
							self.stack[top].0 = Right;
							self.stack.push((Down, inner.left()));
							self.depth += 1;
						},
						Node::Leaf(ref leaf) => {
							self.stack[top].0 = Up;
							return Some((leaf.key.clone(), Some(&leaf.value)));
						},
					}
				},
				Right => {
					debug_assert_eq!(self.depth, node.key().len());
					match *node {
						Node::InnerNode(ref inner) => {
							self.stack[top].0 = Up;
							self.stack.push((Down, inner.right()));
							self.depth += 1;
						},
						Node::Leaf(_) => unreachable!(),
					}
				},
				Up => unreachable!(),
			}
		}
	}
}
