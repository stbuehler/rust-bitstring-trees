//! generic map of bit strings prefixes to values
//!
//! This is a very generic abstraction and therefore not easy to use.
//!
//! Look for other containers in this crate that offer specific use cases.

use alloc::boxed::Box;
use bitstring::BitString;
use core::{
	fmt,
	mem::{
		replace,
		swap,
		take,
	},
	ptr::NonNull,
};
use goto::LookupStepWith;

use crate::walk_mut::NodeOrTree;

use self::goto::NodeRef as _;

pub use self::{
	goto::{
		InsertPosition,
		InsertPositionWith,
	},
	iter::{
		IterInOrder,
		IterLeaf,
		IterLeafFull,
		IterPostOrder,
		IterPreOrder,
	},
	mut_borrowed::{
		IterMutBorrowedInOrder,
		IterMutBorrowedLeaf,
		IterMutBorrowedLeafFull,
		IterMutBorrowedPostOrder,
		IterMutBorrowedPreOrder,
		IterWalkMutBorrowedPath,
		WalkMutBorrowed,
		WalkMutBorrowedPath,
	},
	mut_gen::IterMutPath,
	mut_owned::{
		IterMutOwnedInOrder,
		IterMutOwnedLeaf,
		IterMutOwnedLeafFull,
		IterMutOwnedPostOrder,
		IterMutOwnedPreOrder,
		IterWalkMutOwnedPath,
		WalkMutOwned,
		WalkMutOwnedPath,
	},
	path::{
		IterPath,
		MutPath,
	},
	walk::Walk,
	walk_dir::WalkedDirection,
};

mod goto;
mod iter;
mod mut_borrowed;
mod mut_gen;
mod mut_owned;
mod path;
mod walk;
mod walk_dir;

/// Define Tree behavior
pub trait TreeProperties {
	/// Bitstring key
	type Key: BitString + Clone + Eq;
	/// Value attached to all inner and leaf nodes
	type Value: Default;
	/// Value attached to leaf nodes only
	type LeafValue: Clone + Default;

	/// Used to compare leaf values to allow combining of leafs.
	/// (Only used when LEAF_EMPTY=false);
	type LeafValueComparer: LeafValueComparer<Self::LeafValue>;

	/// Whether value is insignificant (inner nodes can be removed)
	///
	/// When true `Value` should be `()`.
	const EMPTY: bool;

	/// Whether leaf value is insignificant (leafs won't
	/// cloned down a path to insert a new leaf - new leaf
	/// gets ignored if a parent leaf is present).
	///
	/// Most operations won't touch the set of covered bitstrings
	/// by leaf nodes unless that is their explicit goal.
	///
	/// When true `LeafValue` should be `()`.
	const LEAF_EMPTY: bool;

	/// Whether to completely ignore leafs and the bitstrings they cover.
	///
	/// Use this if you only care about the inner `Value`s.
	///
	/// If set `LEAF_EMPTY` must be set too.
	const IGNORE_LEAFS: bool;
}

const fn tp_valid<TP: TreeProperties>() -> bool {
	if TP::IGNORE_LEAFS && !TP::LEAF_EMPTY {
		return false;
	}

	if TP::EMPTY && TP::IGNORE_LEAFS {
		return false;
	} // useless tree

	// if TP::EMPTY && !is_empty_tuple::<TP::Value>() { return false; }
	// if TP::LEAF_EMPTY && !is_empty_tuple::<TP::LeafValue>() { return false; }

	true
}

/// Define how to compare leaf values in tree
pub trait LeafValueComparer<V> {
	/// Whether two leaf values are equal and can be merged if they are neighbors keys
	fn eq(a: &V, b: &V) -> bool;
}

/// Use [`Eq`] for [`LeafValueComparer`]
pub struct DefaultCompare;

impl<V: Eq> LeafValueComparer<V> for DefaultCompare {
	#[inline]
	fn eq(a: &V, b: &V) -> bool {
		a == b
	}
}

/// Define no leaf values to be equal for [`LeafValueComparer`]
pub struct NoEqual;

impl<V> LeafValueComparer<V> for NoEqual {
	#[inline]
	fn eq(_a: &V, _b: &V) -> bool {
		false
	}
}

/// Node in tree
pub struct Node<TP: TreeProperties> {
	key: TP::Key,
	value: TP::Value,
	state: NodeState<TP>,
}

impl<TP> Clone for Node<TP>
where
	TP: TreeProperties,
	TP::Value: Clone,
{
	fn clone(&self) -> Self {
		Self {
			key: self.key.clone(),
			value: self.value.clone(),
			state: self.state.clone(),
		}
	}
}

impl<TP> fmt::Debug for Node<TP>
where
	TP: TreeProperties,
	TP::Key: fmt::Debug,
	TP::Value: fmt::Debug,
	TP::LeafValue: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.state {
			NodeState::Leaf { ref value } => write!(
				f,
				"Leaf {{ key: {:?}, inner: {:?}, value: {:?} }}",
				self.key, self.value, value,
			),
			NodeState::InnerNode { ref children } => write!(
				f,
				"InnerNode {{ key: {:?}, inner: {:?}, left: {:?}, right: {:?} }}",
				self.key, self.value, children.left, children.right,
			),
		}
	}
}

impl<TP: TreeProperties> Node<TP> {
	#[inline]
	fn _is_prefix_of(&self, key: &TP::Key, key_len: usize) -> bool {
		goto::is_prefix(&self.key, self.key.len(), key, key_len)
	}

	/// Get key of node
	#[inline]
	pub fn get_key(&self) -> &TP::Key {
		&self.key
	}

	/// Get value of node
	#[inline]
	pub fn get_value(&self) -> &TP::Value {
		&self.value
	}

	/// Get mutable value of node
	#[inline]
	pub fn get_value_mut(&mut self) -> &mut TP::Value {
		&mut self.value
	}

	/// Whether node is a leaf
	#[inline]
	pub fn is_leaf(&self) -> bool {
		matches!(self.state, NodeState::Leaf { .. })
	}

	/// Return reference to leaf value if node is a leaf
	#[inline]
	pub fn get_leaf_value(&self) -> Option<&TP::LeafValue> {
		match self.state {
			NodeState::Leaf { ref value } => Some(value),
			_ => None,
		}
	}

	/// Return mutable reference to leaf value if node is a leaf
	#[inline]
	pub fn get_leaf_value_mut(&mut self) -> Option<&mut TP::LeafValue> {
		match self.state {
			NodeState::Leaf { ref mut value } => Some(value),
			_ => None,
		}
	}

	/// Make node a leaf node (i.e. drop potential child nodes) and set leaf value
	#[inline]
	pub fn set_leaf_value(&mut self, value: TP::LeafValue) -> &mut TP::LeafValue {
		self.state = NodeState::Leaf { value };
		match self.state {
			NodeState::Leaf { ref mut value } => value,
			_ => unreachable!(),
		}
	}

	/// Return mutable reference to leaf value
	///
	/// If node isn't a leaf, make it one and initialize it with given constructor
	pub fn get_or_make_leaf_value_with<F>(&mut self, f: F) -> &mut TP::LeafValue
	where
		F: FnOnce() -> TP::LeafValue,
	{
		match self.state {
			NodeState::Leaf { .. } => (),
			NodeState::InnerNode { .. } => {
				self.state = NodeState::Leaf { value: f() };
			},
		}
		match self.state {
			NodeState::Leaf { ref mut value } => value,
			_ => unreachable!(),
		}
	}

	/// Return reference to (left, right) child nodes unless node is a leaf
	#[inline]
	pub fn get_children(&self) -> Option<(&Self, &Self)> {
		match self.state {
			NodeState::InnerNode { ref children } => Some((&children.left, &children.right)),
			_ => None,
		}
	}

	/// Return mutable reference to (left, right) child nodes unless node is a leaf
	#[inline]
	pub fn get_children_mut(&mut self) -> Option<(&mut Self, &mut Self)> {
		match self.state {
			NodeState::InnerNode { ref mut children } => {
				Some((&mut children.left, &mut children.right))
			},
			_ => None,
		}
	}

	/// Return reference to left child node unless node is a leaf
	#[inline]
	pub fn get_left(&self) -> Option<&Self> {
		match self.state {
			NodeState::InnerNode { ref children } => Some(&children.left),
			_ => None,
		}
	}

	/// Return mutable reference to left child node unless node is a leaf
	#[inline]
	pub fn get_left_mut(&mut self) -> Option<&mut Self> {
		match self.state {
			NodeState::InnerNode { ref mut children } => Some(&mut children.left),
			_ => None,
		}
	}

	/// Return reference to right child node unless node is a leaf
	#[inline]
	pub fn get_right(&self) -> Option<&Self> {
		match self.state {
			NodeState::InnerNode { ref children } => Some(&children.right),
			_ => None,
		}
	}

	/// Return mutable reference to right child node unless node is a leaf
	#[inline]
	pub fn get_right_mut(&mut self) -> Option<&mut Self> {
		match self.state {
			NodeState::InnerNode { ref mut children } => Some(&mut children.right),
			_ => None,
		}
	}

	/// Return reference to requested child node unless node is a leaf
	///
	/// `false` returns left and `true` returns right node.
	#[inline]
	pub fn get_child(&self, side_bit: bool) -> Option<&Node<TP>> {
		match self.state {
			NodeState::InnerNode { ref children } => Some({
				if side_bit {
					&children.right
				} else {
					&children.left
				}
			}),
			_ => None,
		}
	}

	/// Return mutable reference to requested child node unless node is a leaf
	///
	/// `false` returns left and `true` returns right node.
	#[inline]
	pub fn get_child_mut(&mut self, side_bit: bool) -> Option<&mut Node<TP>> {
		match self.state {
			NodeState::InnerNode { ref mut children } => Some({
				if side_bit {
					&mut children.right
				} else {
					&mut children.left
				}
			}),
			_ => None,
		}
	}

	fn new_leaf(key: TP::Key, inner: TP::Value, value: TP::LeafValue) -> Self {
		Self {
			key,
			value: inner,
			state: NodeState::Leaf { value },
		}
	}

	// test whether two nodes can be combined because their leaf values are equal
	//
	// with real values (TP::EMPTY = false) we should never combines leaf nodes.
	// if leaf values are empty too we don't need to actually compare data.
	fn leaf_value_eq(a: &TP::LeafValue, b: &TP::LeafValue) -> bool {
		TP::EMPTY && (TP::LEAF_EMPTY || TP::LeafValueComparer::eq(a, b))
	}

	// panic-safe modification
	// always insert leaf! (no compression check)
	fn insert_leaf_sibling(
		&mut self,
		shared_prefix_len: usize,
		key: TP::Key,
		value: TP::LeafValue,
	) {
		debug_assert!(shared_prefix_len < self.key.len());
		debug_assert!(shared_prefix_len < key.len());
		debug_assert!(key.get(shared_prefix_len) != self.key.get(shared_prefix_len));

		// need to split path to this node; requires new parent
		let old_key: <TP as TreeProperties>::Key = self.key.clone();
		let new_leaf = Self::new_leaf(key.clone(), Default::default(), value);
		let tmp_node = NodeState::Leaf {
			value: Default::default(),
		};
		// need to move inner value down
		let new_inner: TP::Value = Default::default();

		// start modification; make it panic safe
		// * if this panics assume the key is left at its previous value:
		self.key.clip(shared_prefix_len);
		// * everything else shouldn't panic
		let old_inner = replace(&mut self.value, new_inner);
		let old_node = replace(&mut self.state, tmp_node);
		// TODO: new_inner_unknown_order calls BitString::get which might panic (but shouldn't)
		let old_state = replace(
			&mut self.state,
			NodeState::new_inner_unknown_order(
				shared_prefix_len,
				Self {
					key: old_key,
					value: old_inner,
					state: old_node,
				},
				new_leaf,
			),
		);
		// modification done, allow panics again
		drop(old_state);
	}

	// create chain of nodes to final leaf {key, value}; every shorter path from parent_key_len
	// to it gets an inner node with a side leaf {side_value}
	fn linear_split(
		parent_key_len: usize,
		side_value: TP::LeafValue,
		mut key: TP::Key,
		value: TP::LeafValue,
	) -> NodeState<TP> {
		let mut new_node = NodeState::Leaf { value };
		for l_minus1 in (parent_key_len..key.len()).rev() {
			key.clip(l_minus1 + 1);
			let mut other_key = key.clone();
			other_key.flip(l_minus1);
			new_node = NodeState::new_inner_unknown_order(
				l_minus1,
				Node {
					key: key.clone(),
					value: Default::default(),
					state: new_node,
				},
				Node::new_leaf(other_key, Default::default(), side_value.clone()),
			);
		}
		new_node
	}

	// panic-safe modification
	// always insert leaf! (no compression check)
	// must be currently a leaf node, and `self.key` must be a prefix of `key`
	// will split current leaf value into chain if needed
	fn insert_sub_leaf(&mut self, key: TP::Key, value: TP::LeafValue) {
		let self_key_len = self.key.len(); // self.key is (shared) prefix of key!
									 // new value below in tree
		let old_value = self.get_leaf_value().expect("must be at leaf node").clone();

		let new_state = if TP::IGNORE_LEAFS {
			// leaf nodes not important; just create direct sibling
			let mut other_key = key.clone();
			other_key.clip(self_key_len + 1);
			other_key.flip(self_key_len);
			NodeState::new_inner_unknown_order(
				self_key_len,
				Node {
					key,
					value: Default::default(),
					state: NodeState::Leaf { value },
				},
				Node::new_leaf(other_key, Default::default(), old_value),
			)
		} else {
			// full chain of old leaf values
			Self::linear_split(self_key_len, old_value, key, value)
		};

		// now start modification; make it panic safe
		// * replacing state shouldn't panic
		let old_state = replace(&mut self.state, new_state);
		// panics allowed again
		drop(old_state);
	}

	// panic-safe modification
	fn clip_to_value(&mut self, key_len: usize, value: TP::LeafValue) {
		let mut old_inner = None;
		if key_len != self.key.len() {
			let new_inner = Default::default();

			// start modification; make it panic safe
			// * if this panics assume the key is left at its previous value:
			self.key.clip(key_len);
			// * everything else shouldn't panic
			old_inner = Some(replace(&mut self.value, new_inner));
		}
		let old_state = replace(&mut self.state, NodeState::Leaf { value });
		// modification done, allow panics again
		drop(old_state);
		drop(old_inner);
	}

	/// pre condition: self is the node to insert `key` at
	fn insert_leaf_value(&mut self, key: TP::Key, value: TP::LeafValue) {
		let key_len = key.len();
		let self_key_len = self.key.len();
		let shared_prefix_len = self.key.shared_prefix_len(&key);

		if shared_prefix_len == key_len {
			// either key == self.key, or key is a prefix of self.key
			// => replace subtree
			// panic-safe modification:
			self.clip_to_value(shared_prefix_len, value);
			return;
		}

		if shared_prefix_len < self_key_len {
			// need to insert new inner node at `self`, i.e. split path to this node
			debug_assert!(shared_prefix_len < key_len);

			// but first check a shortcut: if we could compress afterward, don't create
			// new nodes in the first place
			if TP::EMPTY {
				// we can merge inner nodes
				if self_key_len == key_len && self_key_len == shared_prefix_len + 1 {
					// we'd create direct neighbor nodes below
					if let Some(old_value) = self.get_leaf_value() {
						if Self::leaf_value_eq(&value, old_value) {
							// both nodes would be leaf nodes, and their values match
							// panic-safe modification:
							self.clip_to_value(shared_prefix_len, value.clone());
							return;
						}
					}
				}
			}

			self.insert_leaf_sibling(shared_prefix_len, key, value);
			return;
		}

		// otherwise: self.key is a (real) prefix of key
		// if self isn't a leaf we're not at the insert position (violiating precondition)
		debug_assert!(shared_prefix_len == self_key_len);
		debug_assert!(shared_prefix_len < key_len);

		// new value below in tree
		let old_value = self.get_leaf_value().expect("should be at leaf node");

		// borrow check is unhappy with putting this into the match below.
		if TP::LEAF_EMPTY {
			// we don't care about leaf values, and the key is already covered by a leaf.
			return;
		}
		if TP::LeafValueComparer::eq(old_value, &value) {
			// leaf values match, no need to create lots of nodes
			return;
		}
		self.insert_sub_leaf(key, value);
	}

	// return true when self is a leaf afterwards
	fn compress(&mut self) -> bool {
		let self_key_len = self.key.len();

		// compress: if node has two children, and both sub keys are
		// exactly one bit longer than the key of the parent node, and
		// both child nodes are leafs and share the same value, make the
		// current node a leaf
		let value = match self.state {
			NodeState::InnerNode { ref mut children } => {
				if children.left.key.len() != self_key_len + 1 {
					return false;
				}
				if children.right.key.len() != self_key_len + 1 {
					return false;
				}
				let left_value = match children.left.get_leaf_value() {
					Some(value) => value,
					None => return false, // not a leaf
				};
				let right_value = match children.right.get_leaf_value() {
					Some(value) => value,
					None => return false, // not a leaf
				};
				if !Self::leaf_value_eq(left_value, right_value) {
					return false; // values not equal
				}
				// clone value from left side
				left_value.clone()
			},
			NodeState::Leaf { .. } => return true, // already compressed
		};
		// now start modification; make it panic safe
		// (single assignment should be safe anyway, but make it explicit)
		let old_state = replace(&mut self.state, NodeState::Leaf { value });
		// drop afterwards
		drop(old_state);
		true
	}

	// delete either left or right side
	fn delete_side(&mut self, delete_right: bool) {
		// start modification; make it panic safe
		// * take might panic when creation of default state fails - nothing else was modified
		let mut old_state = take(&mut self.state);
		// * swaps shouldn't panic
		match old_state {
			NodeState::Leaf { .. } => {
				// no children, not deleting anything. probably shouldn't end up here, but easy to handle.
				swap(&mut self.state, &mut old_state);
			},
			NodeState::InnerNode { ref mut children } => {
				if delete_right {
					// drop right, replace self with left
					swap(self, &mut children.left);
				} else {
					// drop left, replace self with right
					swap(self, &mut children.right);
				}
			},
		}
		// * modification done, panics allowed again
		drop(old_state);
	}
}

/// Nodes of a [`Tree`] can be either an InnerNode (with two children)
/// or a leaf node.
enum NodeState<TP: TreeProperties> {
	/// Inner node
	InnerNode { children: Box<Children<TP>> },
	/// Leaf node
	Leaf { value: TP::LeafValue },
}

impl<TP: TreeProperties> Default for NodeState<TP> {
	fn default() -> Self {
		Self::Leaf {
			value: Default::default(),
		}
	}
}

impl<TP> Clone for NodeState<TP>
where
	TP: TreeProperties,
	TP::Value: Clone,
{
	fn clone(&self) -> Self {
		match self {
			Self::InnerNode { children } => Self::InnerNode {
				children: children.clone(),
			},
			Self::Leaf { value } => Self::Leaf {
				value: value.clone(),
			},
		}
	}
}

impl<TP: TreeProperties> NodeState<TP> {
	fn new_inner_unknown_order(shared_prefix_len: usize, a: Node<TP>, b: Node<TP>) -> Self {
		let a_right = a.key.get(shared_prefix_len);
		assert_eq!(!a_right, b.key.get(shared_prefix_len));
		if a_right {
			Self::InnerNode {
				children: Box::new(Children { left: b, right: a }),
			}
		} else {
			Self::InnerNode {
				children: Box::new(Children { left: a, right: b }),
			}
		}
	}
}

struct Children<TP: TreeProperties> {
	left: Node<TP>,
	right: Node<TP>,
}

impl<TP> Clone for Children<TP>
where
	TP: TreeProperties,
	TP::Value: Clone,
{
	fn clone(&self) -> Self {
		Self {
			left: self.left.clone(),
			right: self.right.clone(),
		}
	}
}

/// [`Tree`] is a binary tree with path-shortening.
///
/// Nodes are either inner nodes with two child nodes, or leaf nodes.
/// Both node types carry keys and values, leaf nodes an additional leaf value (of different type).
pub struct Tree<TP: TreeProperties> {
	node: Option<Node<TP>>,
}

impl<TP: TreeProperties> Default for Tree<TP> {
	fn default() -> Self {
		Self::new()
	}
}

impl<TP> Clone for Tree<TP>
where
	TP: TreeProperties,
	TP::Value: Clone,
{
	fn clone(&self) -> Self {
		Self {
			node: self.node.clone(),
		}
	}
}

impl<TP> fmt::Debug for Tree<TP>
where
	TP: TreeProperties,
	TP::Key: fmt::Debug,
	TP::Value: fmt::Debug,
	TP::LeafValue: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.node {
			None => {
				write!(f, "Tree {{ }}")
			},
			Some(ref node) => {
				write!(f, "Tree {{ {:?} }}", node)
			},
		}
	}
}

impl<TP: TreeProperties> Tree<TP> {
	/// New (empty) tree.
	pub const fn new() -> Self {
		assert!(tp_valid::<TP>()); // TODO: make it a static assert somehow?
		Self { node: None }
	}

	/// Set a new prefix => value mapping.
	///
	/// Leaf values are designed to split all values into prefixes
	/// that either have a leaf value or no leaf value set.
	///
	/// Sibling prefixes that share the same leaf value are merged.
	pub fn set_leaf_value(&mut self, key: TP::Key, value: TP::LeafValue) {
		let mut walk = self.walk_mut::<(), ()>();
		walk.goto_insert(&key);

		match walk.inner.walk.current_mut() {
			NodeOrTree::Tree(root) => {
				assert!(root.is_none());
				*root = Some(Node::new_leaf(key, Default::default(), value));
			},
			NodeOrTree::Node(node) => {
				node.insert_leaf_value(key, value);
			},
		}

		// compress while walking up the tree until compress fails
		if TP::EMPTY {
			while walk.up().is_some() {
				match walk.current_mut() {
					NodeOrTree::Tree(_) => break,
					NodeOrTree::Node(node) => {
						if !node.compress() {
							break;
						}
					},
				}
			}
		}
	}

	/// Get reference to root node
	pub fn root(&self) -> Option<&Node<TP>> {
		self.node.as_ref()
	}

	/// Get mutable reference to root node
	pub fn root_mut(&mut self) -> Option<&mut Node<TP>> {
		self.node.as_mut()
	}

	/// Get reference to node with exact key
	pub fn get<'r>(&'r self, key: &TP::Key) -> Option<&'r Node<TP>> {
		match self.goto_insert(key)? {
			InsertPositionWith::AlreadyExists(n) => Some(n),
			_ => None,
		}
	}

	/// Get mutable reference to node with exact key
	pub fn get_mut<'r>(&'r mut self, key: &TP::Key) -> Option<&'r mut Node<TP>> {
		match self.goto_mut_insert(key)? {
			InsertPositionWith::AlreadyExists(n) => Some(n),
			_ => None,
		}
	}

	/// Goto insert position for given key
	pub fn goto_insert<'r>(&'r self, key: &TP::Key) -> Option<InsertPositionWith<&'r Node<TP>>> {
		Some(self.node.as_ref()?.goto_insert(key))
	}

	/// Goto mutable insert position for given key
	pub fn goto_mut_insert<'r>(
		&'r mut self,
		key: &TP::Key,
	) -> Option<InsertPositionWith<&'r mut Node<TP>>> {
		Some(self.node.as_mut()?.goto_insert(key))
	}

	/// Get a reference to the node with the longest prefix satisfying callback of the target key
	pub fn get_longest_prefix_with<'r, F>(
		&'r self,
		key: &TP::Key,
		mut callback: F,
	) -> Option<&'r Node<TP>>
	where
		F: FnMut(&Node<TP>) -> bool,
	{
		let key_len = key.len();
		let mut step = self.node.as_ref()?.lookup_initial_step(key, key_len);
		let mut result = None;
		loop {
			step = match step {
				LookupStepWith::Path(node, _) => {
					if callback(node) {
						result = Some(node);
					}
					node.lookup_step(key, key_len)
				},
				LookupStepWith::Found(node, _) => {
					if callback(node) {
						return Some(node);
					}
					return result;
				},
				LookupStepWith::Miss => {
					return result;
				},
			};
		}
	}

	/// Get a reference to the node with the longest prefix satisfying callback of the target key
	pub fn get_longest_prefix_mut_with<'r, F>(
		&'r mut self,
		key: &TP::Key,
		mut callback: F,
	) -> Option<&'r mut Node<TP>>
	where
		F: FnMut(&mut Node<TP>) -> bool,
	{
		let key_len = key.len();
		let mut step = self.node.as_mut()?.lookup_initial_step(key, key_len);
		let mut result = None;
		loop {
			step = match step {
				LookupStepWith::Path(node, _) => {
					if callback(node) {
						result = Some(NonNull::from(&mut *node));
					}
					node.lookup_step(key, key_len)
				},
				LookupStepWith::Found(node, _) => {
					if callback(node) {
						return Some(node);
					}
					break;
				},
				LookupStepWith::Miss => {
					break;
				},
			};
		}
		// safety: steps derived from result are not borrowed anymore
		return Some(unsafe { result?.as_mut() });
	}

	/// Get a reference to the node with the longest prefix of the target key
	pub fn get_most_specific<'r>(&'r self, key: &TP::Key) -> Option<&'r Node<TP>> {
		let key_len = key.len();
		let mut current = match self.node.as_ref()?.lookup_initial_step(key, key_len) {
			LookupStepWith::Path(node, _) => node,
			LookupStepWith::Found(node, _) => return Some(node),
			LookupStepWith::Miss => return None,
		};
		loop {
			current = match current.lookup_step(key, key_len) {
				LookupStepWith::Path(node, _) => node,
				LookupStepWith::Found(node, _) => return Some(node),
				LookupStepWith::Miss => return Some(current),
			};
		}
	}

	/// Get a mutable reference to the node with the longest prefix of the target key
	pub fn get_most_specific_mut<'r>(&'r mut self, key: &TP::Key) -> Option<&'r mut Node<TP>> {
		let key_len = key.len();
		let mut current = match self.node.as_mut()?.lookup_initial_step(key, key_len) {
			LookupStepWith::Path(node, _) => node,
			LookupStepWith::Found(node, _) => return Some(node),
			LookupStepWith::Miss => return None,
		};
		loop {
			let previous = current as *mut _;
			current = match current.lookup_step(key, key_len) {
				LookupStepWith::Path(node, _) => node,
				LookupStepWith::Found(node, _) => return Some(node),
				// safety: current isn't actually still be borrowed, but borrow checker fails (polonius should fix this).
				// LookupStep::Miss => return Some(current),
				LookupStepWith::Miss => return Some(unsafe { &mut *previous }),
			};
		}
	}

	/// Walk tree
	pub fn walk<D, A>(&self) -> Walk<'_, TP, D, A> {
		Walk::new(self)
	}

	/// Iterate over nodes of tree that are a prefix of target key
	pub fn iter_path(&self, key: TP::Key) -> IterPath<'_, TP> {
		IterPath::new(self.node.as_ref(), key)
	}

	/// Iterate over nodes of tree depth-first pre-order
	pub fn iter_pre_order(&self) -> IterPreOrder<'_, TP> {
		IterPreOrder::new(self)
	}

	/// Iterate over nodes of tree depth-first in-order
	pub fn iter_in_order(&self) -> IterInOrder<'_, TP> {
		IterInOrder::new(self)
	}

	/// Iterate over nodes of tree depth-first post-order
	pub fn iter_post_order(&self) -> IterPostOrder<'_, TP> {
		IterPostOrder::new(self)
	}

	/// Iterate over nodes and leaf values of tree in-order
	pub fn iter_leaf(&self) -> IterLeaf<'_, TP> {
		IterLeaf::new(self)
	}

	/// Iterate over nodes and leaf values and uncovered keys of tree in-order
	pub fn iter_leaf_full(&self) -> IterLeafFull<'_, TP> {
		IterLeafFull::new(self)
	}

	/// Walk mutable tree
	pub fn walk_mut<D, A>(&mut self) -> WalkMutOwned<'_, TP, D, A> {
		WalkMutOwned {
			inner: mut_gen::WalkMut::new(self),
		}
	}

	/// Iterate over keys and mutable values of tree that are a prefix of target key
	pub fn iter_mut_path(&mut self, key: TP::Key) -> MutPath<'_, TP> {
		MutPath::new(self.node.as_mut(), key)
	}

	/// Iterate over keys and mutable values of tree depth-first pre-order
	pub fn iter_mut_pre_order(&mut self) -> IterMutOwnedPreOrder<'_, TP> {
		self.walk_mut().into_iter_pre_order()
	}

	/// Iterate over keys and mutable values of tree depth-first in-order
	pub fn iter_mut_in_order(&mut self) -> IterMutOwnedInOrder<'_, TP> {
		self.walk_mut().into_iter_in_order()
	}

	/// Iterate over keys and mutable values of tree depth-first post-order
	pub fn iter_mut_post_order(&mut self) -> IterMutOwnedPostOrder<'_, TP> {
		self.walk_mut().into_iter_post_order()
	}

	/// Iterate over keys and mutable leaf values of tree in-order
	pub fn iter_mut_leaf(&mut self) -> IterMutOwnedLeaf<'_, TP> {
		self.walk_mut().into_iter_leafs()
	}

	/// Iterate over keys and mutable leaf values and uncovered keys of tree in-order
	pub fn iter_mut_leaf_full(&mut self) -> IterMutOwnedLeafFull<'_, TP> {
		self.walk_mut().into_iter_full_leafs()
	}
}
