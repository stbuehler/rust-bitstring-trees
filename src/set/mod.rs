//! set of bit strings prefixes
use bitstring::BitString;
use std::boxed::Box;
use std::option::Option;
use std::fmt;

pub use self::iter::*;
pub use self::iter_full::*;

mod iter;
mod iter_full;

/// `RadixSet` is a binary tree with path-shortening; leafs mark
/// prefixes included in the set, inner nodes have no semantic value.
///
/// If a prefix is in the set, all strings prefixed by it are also
/// considered part of the set.
///
/// If an inner node would have only a single child, the paths to and
/// from it could be shortened - therefor all inner nodes have two
/// children.
#[derive(Clone)]
pub struct RadixSet<S: BitString> {
	node: Option<Node<S>>,
}

impl<S: BitString+fmt::Debug> fmt::Debug for RadixSet<S> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.node {
			None => {
				write!(f, "RadixSet {{ }}")
			},
			Some(ref node) => {
				write!(f, "RadixSet {{ {:?} }}", node)
			},
		}
	}
}

impl<S: BitString> Default for RadixSet<S> {
	fn default() -> RadixSet<S> {
		return RadixSet::<S>{
			node: None,
		}
	}
}

/// Nodes of a RadixSet can be either an InnerNode (with two children)
/// or a leaf node.
#[derive(Clone)]
pub enum Node<S: BitString> {
	/// Inner node
	InnerNode(InnerNode<S>),
	/// Leaf node
	Leaf(Leaf<S>),
}

/// Leaf nodes represent prefixes part of the set
#[derive(Clone,Debug)]
pub struct Leaf<S: BitString> {
	key: S,
}

/// Inner node with two direrct children.
#[derive(Clone,Debug)]
pub struct InnerNode<S: BitString> {
	key: S,
	children: Box<Children<S>>,
}

#[derive(Clone,Debug)]
struct Children<S: BitString> {
	left: Node<S>,
	right: Node<S>,
}

impl<S: BitString> Leaf<S> {
	/// The prefix the leaf represents
	pub fn key(&self) -> &S {
		&self.key
	}
}

impl<S: BitString> InnerNode<S> {
	fn pick_side<'a>(&'a mut self, subkey: &S) -> &'a mut Node<S> {
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
	pub fn left(&self) -> &Node<S> {
		&self.children.left
	}

	/// The left branch; all prefixes in this sub tree have a `true`
	/// bit after `self.key()`.
	pub fn right(&self) -> &Node<S> {
		&self.children.right
	}
}

impl<S: BitString+fmt::Debug> fmt::Debug for Node<S> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Node::Leaf(ref leaf) => write!(f, "Leaf {{ key: {:?} }}", leaf.key),
			Node::InnerNode(ref inner) => write!(f, "InnerNode {{ key: {:?}, left: {:?}, right: {:?} }}", inner.key, inner.children.left, inner.children.right),
		}
	}
}

impl<S: BitString+Clone> Node<S> {
	fn new_leaf(key: S) -> Node<S> {
		Node::Leaf(Leaf{
			key: key,
		})
	}

	fn new_children_unknown_order(shared_prefix_len: usize, a: Node<S>, b: Node<S>) -> Box<Children<S>> {
		let a_right = a.key().get(shared_prefix_len);
		assert_eq!(!a_right, b.key().get(shared_prefix_len));
		if a_right {
			Box::new(Children{
				left: b,
				right: a,
			})
		} else {
			Box::new(Children{
				left: a,
				right: b,
			})
		}
	}

	fn new_inner_unknown_order(shared_prefix_len: usize, a: Node<S>, b: Node<S>) -> Node<S> {
		let mut key = a.key().clone();
		key.clip(shared_prefix_len);
		Node::InnerNode(InnerNode{
			key: key,
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

	fn replace<F: FnOnce(Self) -> Self>(&mut self, with: F) {
		super::replace_at_and_fallback(self, with, || {
			Self::new_leaf(S::null())
		})
	}

	// convert self node to leaf with key clipped to key_len
	fn convert_leaf(&mut self, key_len: usize) {
		self.replace(|this| match this {
			Node::Leaf(mut leaf) => {
				leaf.key.clip(key_len);
				Node::Leaf(leaf)
			},
			Node::InnerNode(inner) => {
				let mut key = inner.key;
				key.clip(key_len);
				Self::new_leaf(key)
			},
		})
	}

	fn insert(&mut self, key: S) {
		let (mut self_key_len, shared_prefix_len) = {
			let key_ref = self.key();
			(key_ref.len(), key_ref.shared_prefix_len(&key))
		};
		if shared_prefix_len == key.len() {
			// either key == self.key, or key is a prefix of self.key
			// => replace subtree
			self.convert_leaf(shared_prefix_len);
			return; // no need to compress below
		} else if shared_prefix_len < self_key_len {
			debug_assert!(shared_prefix_len < key.len());
			// need to split path to current node; requires new parent
			self.replace(|this| {
				Self::new_inner_unknown_order(
					shared_prefix_len,
					this,
					Self::new_leaf(key)
				)
			});
			// update self_key_len for compression handling below
			self_key_len = shared_prefix_len;
		} else if shared_prefix_len == self_key_len {
			debug_assert!(shared_prefix_len == self_key_len);
			debug_assert!(shared_prefix_len < key.len());
			// new key below in tree
			match *self {
				Node::Leaf(_) => {
					// -> already included
					// no changes, no compression handling
					return
				},
				Node::InnerNode(ref mut inner) => {
					inner.pick_side(&key).insert(key)
				},
			}
			// self_key_len didn't change
		}

		// compress: if node has two children, and both sub keys are
		// exactly one bit longer than the key of the parent node, and
		// both child nodes are leafs, make the current node a leaf
		let compress = match *self {
			Node::InnerNode(ref inner) => {
				let compress_left = match inner.children.left {
					Node::Leaf(ref left_leaf) => left_leaf.key.len() == self_key_len + 1,
					Node::InnerNode(_) => return, // must be leaf
				};
				let compress_right = match inner.children.right {
					Node::Leaf(ref right_leaf) => right_leaf.key.len() == self_key_len + 1,
					Node::InnerNode(_) => return, // must be leaf
				};
				compress_left && compress_right
			},
			Node::Leaf(_) => return, // already compressed
		};
		if compress {
			self.convert_leaf(self_key_len);
		}
	}
}

impl<S: BitString+Clone> RadixSet<S> {
	/// New (empty) set.
	pub fn new() -> Self {
		Default::default()
	}

	/// Add a new prefix to the set.
	pub fn insert(&mut self, key: S) {
		match self.node {
			None => {
				self.node = Some(Node::new_leaf(key));
			},
			Some(ref mut node) => {
				node.insert(key);
			},
		}
	}

	/// Read-only access to the tree.
	///
	/// An empty set doesn't have any nodes (i.e. `None`).
	pub fn root(&self) -> Option<&Node<S>> {
		match self.node {
			None => None,
			Some(ref node) => Some(&node),
		}
	}

	/// Iterate over all prefixes in the set
	pub fn iter(&self) -> Iter<S> {
		Iter::new(self)
	}

	/// Iterate over all prefixes and missing prefixes in the set
	pub fn iter_full(&self) -> IterFull<S> {
		IterFull::new(self)
	}
}
