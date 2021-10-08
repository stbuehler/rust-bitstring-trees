//! map of bit strings prefixes to values
use bitstring::BitString;
use std::{
	boxed::Box,
	fmt,
	option::Option,
};

pub use self::{
	iter::*,
	iter_full::*,
};

mod iter;
mod iter_full;

/// `RadixMap` is a binary tree with path-shortening; leafs mark
/// prefixes mapping to a value, inner nodes have no semantic meaning.
///
/// If a prefix maps to a value set, all strings prefixed by it are also
/// considered to map to that value.
///
/// If an inner node would have only a single child, the paths to and
/// from it could be shortened - therefor all inner nodes have two
/// children.
#[derive(Clone)]
pub struct RadixMap<S: BitString, V> {
	node: Option<Node<S, V>>,
}

impl<S: BitString + fmt::Debug, V: fmt::Debug> fmt::Debug for RadixMap<S, V> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.node {
			None => {
				write!(f, "RadixMap {{ }}")
			},
			Some(ref node) => {
				write!(f, "RadixMap {{ {:?} }}", node)
			},
		}
	}
}

impl<S: BitString, V> Default for RadixMap<S, V> {
	fn default() -> Self {
		RadixMap { node: None }
	}
}

/// Nodes of a RadixMap can be either an InnerNode (with two children)
/// or a leaf node.
#[derive(Clone)]
pub enum Node<S: BitString, V> {
	/// Inner node
	InnerNode(InnerNode<S, V>),
	/// Leaf node
	Leaf(Leaf<S, V>),
}

/// Leaf nodes represent prefixes part of the set
#[derive(Clone, Debug)]
pub struct Leaf<S: BitString, V> {
	key: S,
	value: V,
}

/// Inner node with two direct children.
#[derive(Clone, Debug)]
pub struct InnerNode<S: BitString, V> {
	key: S,
	children: Box<Children<S, V>>,
}

#[derive(Clone, Debug)]
struct Children<S: BitString, V> {
	left: Node<S, V>,
	right: Node<S, V>,
}

impl<S: BitString, V> Leaf<S, V> {
	/// The prefix the leaf represents
	pub fn key(&self) -> &S {
		&self.key
	}
}

impl<S: BitString, V> InnerNode<S, V> {
	fn pick_side<'a>(&'a mut self, subkey: &S) -> &'a mut Node<S, V> {
		if subkey.get(self.key.len()) {
			&mut self.children.right
		} else {
			&mut self.children.left
		}
	}

	/// The longest shared prefix of the two contained child nodes.
	pub fn key(&self) -> &S {
		&self.key
	}

	/// The left branch; all prefixes in this sub tree have a `false`
	/// bit after `self.key()`.
	pub fn left(&self) -> &Node<S, V> {
		&self.children.left
	}

	/// The left branch; all prefixes in this sub tree have a `true`
	/// bit after `self.key()`.
	pub fn right(&self) -> &Node<S, V> {
		&self.children.right
	}
}

impl<S: BitString + fmt::Debug, V: fmt::Debug> fmt::Debug for Node<S, V> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Node::Leaf(ref leaf) => write!(f, "Leaf {{ key: {:?} }}", leaf.key),
			Node::InnerNode(ref inner) => write!(
				f,
				"InnerNode {{ key: {:?}, left: {:?}, right: {:?} }}",
				inner.key,
				inner.left(),
				inner.right()
			),
		}
	}
}

impl<S: BitString + Clone, V> Node<S, V> {
	fn new_leaf(key: S, value: V) -> Self {
		Node::Leaf(Leaf { key, value })
	}

	fn new_children_unknown_order(
		shared_prefix_len: usize,
		a: Node<S, V>,
		b: Node<S, V>,
	) -> Box<Children<S, V>> {
		let a_right = a.key().get(shared_prefix_len);
		assert_eq!(!a_right, b.key().get(shared_prefix_len));
		if a_right {
			Box::new(Children { left: b, right: a })
		} else {
			Box::new(Children { left: a, right: b })
		}
	}

	fn new_inner_unknown_order(
		shared_prefix_len: usize,
		a: Node<S, V>,
		b: Node<S, V>,
	) -> Node<S, V> {
		let mut key = a.key().clone();
		key.clip(shared_prefix_len);
		Node::InnerNode(InnerNode {
			key,
			children: Self::new_children_unknown_order(shared_prefix_len, a, b),
		})
	}

	/// The longest shared prefix of all nodes in this sub tree.
	pub fn key(&self) -> &S {
		match *self {
			Node::Leaf(ref leaf) => &leaf.key,
			Node::InnerNode(ref inner) => &inner.key,
		}
	}

	fn leaf_ref(&self) -> Option<&Leaf<S, V>> {
		match *self {
			Node::Leaf(ref leaf) => Some(leaf),
			_ => None,
		}
	}

	fn replace<F: FnOnce(Self) -> Self>(&mut self, with: F) {
		super::replace_at(self, with)
	}

	// convert self node to leaf with key clipped to key_len and given
	// value
	fn convert_leaf(&mut self, key_len: usize, value: V) {
		self.replace(|this| match this {
			Node::Leaf(mut leaf) => {
				leaf.key.clip(key_len);
				leaf.value = value;
				Node::Leaf(leaf)
			},
			Node::InnerNode(inner) => {
				let mut key = inner.key;
				key.clip(key_len);
				Self::new_leaf(key, value)
			},
		})
	}

	fn insert_uncompressed(&mut self, key: S, value: V)
	where
		V: Clone,
	{
		let (self_key_len, shared_prefix_len) = {
			let key_ref = self.key();
			(key_ref.len(), key_ref.shared_prefix_len(&key))
		};

		if shared_prefix_len == key.len() {
			// either key == self.key, or key is a prefix of self.key
			// => replace subtree
			self.convert_leaf(shared_prefix_len, value);
		} else if shared_prefix_len < self_key_len {
			debug_assert!(shared_prefix_len < key.len());
			// need to split path to current node; requires new parent
			self.replace(|this| {
				Self::new_inner_unknown_order(shared_prefix_len, this, Self::new_leaf(key, value))
			});
		} else {
			debug_assert!(shared_prefix_len == self_key_len);
			debug_assert!(shared_prefix_len < key.len());
			// new key below in tree
			match *self {
				Node::Leaf(_) => {
					// linear split of path down to leaf
					let old_value = self.leaf_ref().unwrap().value.clone();
					let mut new_node = Self::new_leaf(key.clone(), value);
					for l in (shared_prefix_len..key.len()).rev() {
						let mut other_key = key.clone();
						other_key.clip(l + 1);
						other_key.flip(l);
						new_node = Self::new_inner_unknown_order(
							l,
							new_node,
							Self::new_leaf(other_key, old_value.clone()),
						);
					}
					*self = new_node;
				},
				Node::InnerNode(ref mut inner) => {
					inner.pick_side(&key).insert_uncompressed(key, value);
				},
			}
		}
	}

	fn insert(&mut self, key: S, value: V)
	where
		V: Clone + Eq,
	{
		let (self_key_len, shared_prefix_len) = {
			let key_ref = self.key();
			(key_ref.len(), key_ref.shared_prefix_len(&key))
		};

		if shared_prefix_len == key.len() {
			// either key == self.key, or key is a prefix of self.key
			// => replace subtree
			self.convert_leaf(shared_prefix_len, value);
		// no need to compress
		} else if shared_prefix_len < self_key_len {
			debug_assert!(shared_prefix_len < key.len());
			if shared_prefix_len + 1 == self_key_len && shared_prefix_len + 1 == key.len() {
				if let Node::Leaf(ref mut this) = *self {
					if this.value == value {
						// we'd split this, and compress it below.
						// shortcut the allocations here
						this.key.clip(shared_prefix_len);
						return; // no need split path
					}
				}
			}

			// need to split path to current node; requires new parent
			self.replace(|this| {
				Self::new_inner_unknown_order(shared_prefix_len, this, Self::new_leaf(key, value))
			});
		// no need to compress - shortcut check above would
		// have found it
		} else {
			debug_assert!(shared_prefix_len == self_key_len);
			debug_assert!(shared_prefix_len < key.len());
			// new key below in tree
			match *self {
				Node::Leaf(_) => {
					// linear split of path down to leaf
					let new_node = {
						let old_value = &self.leaf_ref().unwrap().value;
						if *old_value == value {
							// below in tree, but same value - nothing new
							return;
						}
						let mut new_node = Self::new_leaf(key.clone(), value);
						for l in (shared_prefix_len..key.len()).rev() {
							let mut other_key = key.clone();
							other_key.clip(l + 1);
							other_key.flip(l);
							new_node = Self::new_inner_unknown_order(
								l,
								new_node,
								Self::new_leaf(other_key, old_value.clone()),
							);
						}
						new_node
					};
					*self = new_node;
					// we checked value before, nothing to compress
					return;
				},
				Node::InnerNode(ref mut inner) => {
					inner.pick_side(&key).insert(key, value);
				},
			}
			// after recursion check for compression
			self.compress();
		}
	}

	fn compress(&mut self)
	where
		V: Eq,
	{
		let self_key_len = self.key().len();

		// compress: if node has two children, and both sub keys are
		// exactly one bit longer than the key of the parent node, and
		// both child nodes are leafs and share the same value, make the
		// current node a leaf
		let compress = match *self {
			Node::InnerNode(ref inner) => {
				let left_value = match inner.children.left {
					Node::Leaf(ref leaf) if leaf.key.len() == self_key_len + 1 => &leaf.value,
					_ => return, // not a leaf or more than one bit longer
				};
				let right_value = match inner.children.right {
					Node::Leaf(ref leaf) if leaf.key.len() == self_key_len + 1 => &leaf.value,
					_ => return, // not a leaf or more than one bit longer
				};
				left_value == right_value
			},
			Node::Leaf(_) => return, // already compressed
		};
		if compress {
			self.replace(|this| match this {
				// move value from left
				Node::InnerNode(inner) => match inner.children.left {
					Node::Leaf(leaf) => Node::Leaf(Leaf {
						key: inner.key,
						value: leaf.value,
					}),
					_ => unreachable!(),
				},
				_ => unreachable!(),
			});
		}
	}
}

impl<S: BitString + Clone, V> RadixMap<S, V> {
	/// New (empty) map.
	pub fn new() -> Self {
		Default::default()
	}

	/// Add a new prefix => value mapping.
	///
	/// As values can't be compared for equality it cannot merge
	/// neighbour prefixes that map to the same value.
	pub fn insert_uncompressed(&mut self, key: S, value: V)
	where
		V: Clone,
	{
		match self.node {
			None => {
				self.node = Some(Node::new_leaf(key, value));
			},
			Some(ref mut node) => {
				node.insert_uncompressed(key, value);
			},
		}
	}

	/// Add a new prefix => value mapping.  (Partially) overwrites old
	/// mappings.
	pub fn insert(&mut self, key: S, value: V)
	where
		V: Clone + Eq,
	{
		match self.node {
			None => {
				self.node = Some(Node::new_leaf(key, value));
			},
			Some(ref mut node) => {
				node.insert(key, value);
			},
		}
	}

	/// Read-only access to the tree.
	///
	/// An empty map doesn't have any nodes (i.e. `None`).
	pub fn root(&self) -> Option<&Node<S, V>> {
		match self.node {
			None => None,
			Some(ref node) => Some(&node),
		}
	}

	/// Iterate over all values in the map
	pub fn iter(&self) -> Iter<S, V> {
		Iter::new(self)
	}

	/// Iterate over all values and missing values in the map
	pub fn iter_full(&self) -> IterFull<S, V> {
		IterFull::new(self)
	}
}
