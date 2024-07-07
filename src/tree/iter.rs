use super::{
	Node,
	Tree,
	TreeProperties,
	Walk,
	WalkedDirection,
};
use crate::iter::{
	iter_between,
	IterBetween,
};

/// Iterate over node of tree depth-first pre-order
pub struct IterPreOrder<'r, TP: TreeProperties> {
	walk: Walk<'r, TP, WalkedDirection>,
}

impl<'r, TP: TreeProperties> IterPreOrder<'r, TP> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self { walk: tree.walk() }
	}
}

impl<'r, TP: TreeProperties> Iterator for IterPreOrder<'r, TP> {
	type Item = &'r Node<TP>;

	fn next(&mut self) -> Option<Self::Item> {
		self.walk.next_pre_order()
	}
}

/// Iterate over node of tree depth-first in-order
pub struct IterInOrder<'r, TP: TreeProperties> {
	walk: Walk<'r, TP, WalkedDirection>,
}

impl<'r, TP: TreeProperties> IterInOrder<'r, TP> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self { walk: tree.walk() }
	}
}

impl<'r, TP: TreeProperties> Iterator for IterInOrder<'r, TP> {
	type Item = &'r Node<TP>;

	fn next(&mut self) -> Option<Self::Item> {
		self.walk.next_in_order()
	}
}

/// Iterate over node of tree depth-first post-order
pub struct IterPostOrder<'r, TP: TreeProperties> {
	walk: Walk<'r, TP, WalkedDirection>,
}

impl<'r, TP: TreeProperties> IterPostOrder<'r, TP> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self { walk: tree.walk() }
	}
}

impl<'r, TP: TreeProperties> Iterator for IterPostOrder<'r, TP> {
	type Item = &'r Node<TP>;

	fn next(&mut self) -> Option<Self::Item> {
		self.walk.next_post_order()
	}
}

/// Iterate over nodes and leaf values of tree in-order
pub struct IterLeaf<'r, TP: TreeProperties> {
	walk: Walk<'r, TP, WalkedDirection>,
}

impl<'r, TP: TreeProperties> IterLeaf<'r, TP> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self { walk: tree.walk() }
	}
}

impl<'r, TP: TreeProperties> Iterator for IterLeaf<'r, TP> {
	type Item = (&'r Node<TP>, &'r TP::LeafValue);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.walk.next_leaf()?;
		Some((node, node.get_leaf_value().expect("leaf node")))
	}
}

/// Iterate over keys and mutable leaf values and uncovered keys of tree in-order
pub struct IterLeafFull<'r, TP: TreeProperties> {
	walk: Option<Walk<'r, TP, WalkedDirection>>,
	previous_key: Option<TP::Key>,
	uncovered: IterBetween<TP::Key>,
	next: Option<(TP::Key, &'r TP::LeafValue)>,
}

impl<'r, TP: TreeProperties> IterLeafFull<'r, TP> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self {
			walk: Some(tree.walk()),
			previous_key: None,
			uncovered: Default::default(),
			next: None,
		}
	}
}

impl<'r, TP: TreeProperties> Iterator for IterLeafFull<'r, TP> {
	type Item = (TP::Key, Option<&'r TP::LeafValue>);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if let Some(k) = self.uncovered.next() {
				return Some((k, None));
			}
			if let Some((key, value)) = self.next.take() {
				return Some((key, Some(value)));
			}
			match self.walk.as_mut()?.next_leaf() {
				None => {
					self.walk = None;
					// return final uncovered prefixes
					self.uncovered = iter_between(self.previous_key.clone(), None);
				},
				Some(node) => {
					// safety: only once per node per iteration
					let key = node.get_key().clone();
					let leaf_value = node.get_leaf_value().expect("leaf node");
					// return uncovered prefixes before
					let start = core::mem::replace(&mut self.previous_key, Some(key.clone()));
					self.uncovered = iter_between(start, Some(key.clone()));
					self.next = Some((key, leaf_value));
				},
			}
		}
	}
}
