use bitstring::BitString;

use crate::walk_mut::NodeOrTree;

use super::{
	goto::{
		GotoStepResult,
		NodeRef,
	},
	InsertPosition,
	Node,
	Tree,
	TreeProperties,
	WalkedDirection,
};

/// Walk tree
///
/// Some algorithms need to remember how they reached the current node via [`WalkedDirection`] as `D`.
///
/// When walking manually it might be useful to be able to store additional data via `A`; look for functions with the suffix `_with`.
pub struct Walk<'r, TP: TreeProperties, D = (), A = ()> {
	tree: Option<&'r Node<TP>>,
	stack: Vec<(&'r Node<TP>, (D, A))>,
}

impl<'r, TP: TreeProperties, D, A> Walk<'r, TP, D, A> {
	pub(in crate::tree) fn new(tree: &'r Tree<TP>) -> Self {
		Self {
			tree: tree.node.as_ref(),
			stack: Vec::new(),
		}
	}

	/// Walk up to parent node or tree if not at tree
	pub fn up(&mut self) -> Option<D> {
		Some(self.stack.pop()?.1 .0)
	}

	/// Walk up to parent node or tree if not at tree
	pub fn up_with(&mut self) -> Option<(D, A)> {
		Some(self.stack.pop()?.1)
	}

	/// Current node or tree
	pub fn current(&self) -> NodeOrTree<Option<&'r Node<TP>>, &'r Node<TP>> {
		match self.stack.last() {
			Some(&(node, _)) => NodeOrTree::Node(node),
			None => NodeOrTree::Tree(self.tree),
		}
	}
}

impl<'r, TP: TreeProperties, D, A> Walk<'r, TP, D, A>
where
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if present)
	pub fn down_root_with(&mut self, add: A) -> bool {
		if self.stack.is_empty() {
			if let Some(root) = self.tree {
				self.stack.push((root, (WalkedDirection::Down.into(), add)));
				return true;
			}
		}
		false
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left_with(&mut self, add: A) -> bool {
		self.down_with(false, add)
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right_with(&mut self, add: A) -> bool {
		self.down_with(true, add)
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down_with(&mut self, side: bool, add: A) -> bool {
		if let Some(&(node, _)) = self.stack.last() {
			if let Some(child) = node.get_child(side) {
				self.stack
					.push((child, (WalkedDirection::from_side(side).into(), add)));
				return true;
			}
		}
		false
	}
}

impl<'r, TP: TreeProperties, D> Walk<'r, TP, D, ()>
where
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if at tree and not empty)
	pub fn down_root(&mut self) -> bool {
		self.down_root_with(())
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left(&mut self) -> bool {
		self.down_left_with(())
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right(&mut self) -> bool {
		self.down_right_with(())
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down(&mut self, side: bool) -> bool {
		self.down_with(side, ())
	}
}

impl<'r, TP: TreeProperties, D> Walk<'r, TP, D>
where
	D: From<WalkedDirection>,
{
	// first need go up until current_node.key is a prefix of key (or we are at the root)
	fn goto_clean(&mut self, key: &TP::Key) {
		let key_len = key.len();
		while let Some(&(node, _)) = self.stack.last() {
			if node._is_prefix_of(key, key_len) {
				return;
			}
			self.stack.pop();
		}
	}

	// if not in the correct subtree call goto_clean first.
	fn goto_insert_step(
		&mut self,
		key: &TP::Key,
		key_len: usize,
	) -> Result<(), Option<InsertPosition>> {
		if let Some(&(node, _)) = self.stack.last() {
			match node.goto_insert_step(key, key_len) {
				GotoStepResult::Final(r) => Err(Some(r.into())),
				GotoStepResult::Continue(node, dir) => {
					self.stack.push((node, (dir.into(), ())));
					Ok(())
				},
			}
		} else if let Some(root) = self.tree {
			self.stack.push((root, (WalkedDirection::Down.into(), ())));
			Ok(())
		} else {
			Err(None)
		}
	}

	// if not in the correct subtree call goto_clean first.
	fn goto_insert_down(&mut self, key: &TP::Key) -> Option<InsertPosition> {
		let key_len = key.len();
		loop {
			match self.goto_insert_step(key, key_len) {
				Ok(()) => (),       // continue
				Err(r) => return r, // reached target
			}
		}
	}

	/// Walk to node where we'd have to insert key at
	///
	/// This can either be:
	/// - root if and only if the tree is empty
	/// - node with exactly matching key
	/// - node where the key is between the parent node (possibly root) and the node
	///   * to insert the key here we might have to create a new inner node and move the existing node down
	pub fn goto_insert(&mut self, key: &TP::Key) -> Option<InsertPosition> {
		self.goto_clean(key);
		self.goto_insert_down(key)
	}
}

impl<'r, TP: TreeProperties> Walk<'r, TP, WalkedDirection> {
	/// Tree traversal: depth-first pre-order
	pub fn next_pre_order(&mut self) -> Option<&'r Node<TP>> {
		match self.current() {
			NodeOrTree::Tree(_) => {
				self.down_root();
			},
			NodeOrTree::Node(node) => {
				if node.is_leaf() {
					loop {
						match self.up()? {
							WalkedDirection::Down => {
								return None;
							}, // back up at tree
							WalkedDirection::Left => {
								self.down_right();
								break;
							},
							WalkedDirection::Right => (), // continue further up
						}
					}
				} else {
					self.down_left();
				}
			},
		}
		return self.current().node();
	}

	/// Tree traversal: depth-first in-order
	pub fn next_in_order(&mut self) -> Option<&'r Node<TP>> {
		match self.current() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(node) => {
				if node.is_leaf() {
					loop {
						match self.up()? {
							WalkedDirection::Down => {
								return None;
							}, // back up at tree
							WalkedDirection::Left => {
								break;
							},
							WalkedDirection::Right => (), // continue further up
						}
					}
				} else {
					self.down_right();
					while self.down_left() {}
				}
			},
		}
		return self.current().node();
	}

	/// Tree traversal: depth-first in-order leaf nodes only
	pub fn next_leaf(&mut self) -> Option<&'r Node<TP>> {
		match self.current() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(_) => {
				loop {
					match self.up()? {
						WalkedDirection::Down => {
							return None; // back up at tree
						},
						WalkedDirection::Left => {
							self.down_right();
							while self.down_left() {}
							break;
						},
						WalkedDirection::Right => (), // continue further up
					}
				}
			},
		}
		return self.current().node();
	}

	/// Tree traversal: depth-first post-order
	pub fn next_post_order(&mut self) -> Option<&'r Node<TP>> {
		match self.current() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(_) => {
				match self.up()? {
					WalkedDirection::Down => {
						return None;
					}, // back up at tree
					WalkedDirection::Left => {
						self.down_right();
						while self.down_left() {}
					},
					WalkedDirection::Right => (),
				}
			},
		}
		return self.current().node();
	}
}
