use core::ops::Deref;

use bitstring::BitString as _;

use super::{
	Node,
	TreeProperties,
	WalkedDirection,
};

pub(in crate::tree) enum GotoStepResult<N> {
	/// Search done; returns passed node
	Final(InsertPositionWith<N>),
	/// Move to next node (that might be the final node)
	///
	/// One of next node key and target key is a prefix of the other; in other words: they are not in different subtrees.
	Continue(N, WalkedDirection),
}

/// Result of key lookup in tree
///
/// Found node is passed somewhere else (probably remembered in a "walk" stack).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsertPosition {
	/// Found node that is a leaf; its key is a prefix of the target key (but not equal to it)
	///
	/// Inserting the target key must convert the found node into an inner node and insert the target key as leaf.
	BelowLeaf,
	/// Found node with target key
	AlreadyExists,
	/// Found node to replace with target key
	///
	/// Parent node is a prefix of target key, but this node is not.
	///
	/// To insert a new node needs to replace the current one, using the shared prefix of this node and the target key as node key.
	/// (This node key could still be the target key.)
	ReplaceNode,
}

impl<N> From<InsertPositionWith<N>> for InsertPosition {
	fn from(value: InsertPositionWith<N>) -> Self {
		match value {
			InsertPositionWith::BelowLeaf(_) => Self::BelowLeaf,
			InsertPositionWith::AlreadyExists(_) => Self::AlreadyExists,
			InsertPositionWith::ReplaceNode(_) => Self::ReplaceNode,
		}
	}
}

/// Result of finding position to insert a target key
#[derive(Debug)]
pub enum InsertPositionWith<N> {
	/// Found node that is a leaf; its key is a prefix of the target key (but not equal to it)
	///
	/// Inserting the target key must convert the found node into an inner node and insert the target key as leaf.
	BelowLeaf(N),
	/// Found node with target key
	AlreadyExists(N),
	/// Found node to replace with target key
	///
	/// Parent node is a prefix of target key, but this node is not.
	///
	/// To insert a new node needs to replace the current one, using the shared prefix of this node and the target key as node key.
	/// (This node key could still be the target key.)
	ReplaceNode(N),
}

pub(in crate::tree) enum LookupStep {
	Path,
	Found,
	Miss,
}

impl<N> From<&LookupStepWith<N>> for LookupStep {
	fn from(value: &LookupStepWith<N>) -> Self {
		match value {
			LookupStepWith::Path(_, _) => Self::Path,
			LookupStepWith::Found(_, _) => Self::Found,
			LookupStepWith::Miss => Self::Miss,
		}
	}
}

impl<N> From<LookupStepWith<N>> for LookupStep {
	fn from(value: LookupStepWith<N>) -> Self {
		match value {
			LookupStepWith::Path(_, _) => Self::Path,
			LookupStepWith::Found(_, _) => Self::Found,
			LookupStepWith::Miss => Self::Miss,
		}
	}
}

#[derive(Debug)]
pub(in crate::tree) enum LookupStepWith<N> {
	Path(N, WalkedDirection),
	Found(N, WalkedDirection),
	Miss,
}

// is `a` a prefix of `b` ?
// i.e. shared_prefix(a, b) == a ?
pub(in crate::tree) fn is_prefix<K>(a: &K, a_len: usize, b: &K, b_len: usize) -> bool
where
	K: bitstring::BitString + Clone,
{
	if a_len > b_len {
		return false;
	}
	let mut b_test = b.clone();
	b_test.clip(a_len);
	*a == b_test
}

pub(in crate::tree) trait NodeRef<'a, TP: TreeProperties>:
	Sized + Deref<Target = Node<TP>>
{
	fn _get_child(self, side: bool) -> Option<Self>;

	// first lookup step with tree root (doesn't walk down, only evaluates root node)
	fn _lookup_check_node(
		self,
		key: &TP::Key,
		key_len: usize,
		dir: WalkedDirection,
	) -> LookupStepWith<Self> {
		let self_key_len = self.key.len();
		if !is_prefix(&self.key, self_key_len, key, key_len) {
			LookupStepWith::Miss
		} else if self_key_len == key_len {
			LookupStepWith::Found(self, dir)
		} else {
			LookupStepWith::Path(self, dir)
		}
	}

	// first lookup step with tree root (doesn't walk down, only evaluates root node)
	fn lookup_initial_step(self, key: &TP::Key, key_len: usize) -> LookupStepWith<Self> {
		self._lookup_check_node(key, key_len, WalkedDirection::Down)
	}

	// only use if previous/initial step returned `LookupStep::Path`
	// i.e. self must be a (real) prefix of the key
	fn lookup_step(self, key: &TP::Key, key_len: usize) -> LookupStepWith<Self> {
		let self_key_len: usize = self.key.len();
		debug_assert!(is_prefix(&self.key, self_key_len, key, key_len));
		debug_assert!(self_key_len < key_len);
		let side = key.get(self_key_len);
		match self._get_child(side) {
			None => LookupStepWith::Miss,
			Some(c) => c._lookup_check_node(key, key_len, WalkedDirection::from_side(side)),
		}
	}

	fn goto_insert_step(self, key: &TP::Key, key_len: usize) -> GotoStepResult<Self> {
		let self_key_len: usize = self.key.len();
		if !is_prefix(&self.key, self_key_len, key, key_len) {
			return GotoStepResult::Final(InsertPositionWith::ReplaceNode(self));
		}
		if self_key_len < key_len {
			let side = key.get(self_key_len);
			if matches!(self.state, super::NodeState::InnerNode { .. }) {
				GotoStepResult::Continue(
					self._get_child(side).expect("inner node"),
					WalkedDirection::from_side(side),
				)
			} else {
				GotoStepResult::Final(InsertPositionWith::BelowLeaf(self))
			}
		} else {
			debug_assert_eq!(self_key_len, key_len);
			GotoStepResult::Final(InsertPositionWith::AlreadyExists(self))
		}
	}

	fn goto_insert(self, key: &TP::Key) -> InsertPositionWith<Self> {
		let key_len = key.len();
		let mut cursor = self;
		loop {
			cursor = match cursor.goto_insert_step(key, key_len) {
				GotoStepResult::Continue(c, _) => c,
				GotoStepResult::Final(f) => return f,
			};
		}
	}
}

impl<'a, TP: TreeProperties> NodeRef<'a, TP> for &'a Node<TP> {
	fn _get_child(self, side: bool) -> Option<Self> {
		self.get_child(side)
	}
}

impl<'a, TP: TreeProperties> NodeRef<'a, TP> for &'a mut Node<TP> {
	fn _get_child(self, side: bool) -> Option<Self> {
		self.get_child_mut(side)
	}
}
