//! [`Set`] of bit string prefixes

use core::cmp::Ordering;

use bitstring::BitString;

use crate::tree::{
	DefaultCompare,
	InsertPositionWith,
	Tree,
	TreeProperties,
};

mod hidden {
	use bitstring::BitString;
	use core::marker::PhantomData;

	/// make it public so we can use it in returned types, but don't make it directly accessible
	pub struct TpSet<K: BitString + Clone + Eq>(PhantomData<*const K>);
}
use hidden::TpSet;

impl<K: BitString + Clone + Eq> TreeProperties for TpSet<K> {
	type Key = K;
	type LeafValue = ();
	type LeafValueComparer = DefaultCompare;
	type Value = ();

	const EMPTY: bool = true;
	const IGNORE_LEAFS: bool = false;
	const LEAF_EMPTY: bool = true;
}

/// Set of bit string prefixes
///
/// Sibling prefixes are automatically merged.
///
/// This is implemented as a [`crate::tree::Tree`] where nodes don't carry
/// values at all, buf leaf nodes represent set membership of the associated
/// key.
#[derive(Clone)]
pub struct Set<K: BitString + Clone + Eq> {
	tree: Tree<TpSet<K>>,
}

impl<K: BitString + Clone + Eq> Default for Set<K> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: BitString + Clone + Eq + core::fmt::Debug> core::fmt::Debug for Set<K> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.debug_set().entries(self.iter()).finish()
	}
}

impl<K: BitString + Clone + Eq> Set<K> {
	/// New (empty) set.
	pub const fn new() -> Self {
		Self { tree: Tree::new() }
	}

	/// Access raw tree of set
	pub fn tree(&self) -> &Tree<TpSet<K>> {
		&self.tree
	}

	/// Insert prefix into set
	pub fn insert(&mut self, key: K) {
		self.tree.set_leaf_value(key, ());
	}

	/// Remove everything covered by prefix from set
	pub fn remove(&mut self, key: K) {
		let mut walk = self.tree.walk_mut();
		walk.goto_insert(&key);
		match walk.current().node() {
			None => (), // empty tree
			Some(node) => {
				match node.get_key().len().cmp(&key.len()) {
					Ordering::Less => {
						// node is a leaf and covers key; need to split and remove key
						// create explicit node with key we want to remove
						walk.insert(key);
						// now remove it
						walk.delete_current();
					},
					Ordering::Equal | Ordering::Greater => {
						// remove subtree
						walk.delete_current();
					},
				}
			},
		}
	}

	/// Whether prefix is (completely) contained in set
	pub fn contains(&self, key: &K) -> bool {
		match self.tree.goto_insert(key) {
			Some(InsertPositionWith::BelowLeaf(_)) => true,
			Some(InsertPositionWith::AlreadyExists(_)) => true,
			Some(InsertPositionWith::ReplaceNode(_)) => false,
			None => false,
		}
	}

	/// Iterate over all contained prefixes
	pub fn iter(&self) -> IterSet<'_, K> {
		IterSet {
			iter: self.tree.iter_leaf(),
		}
	}

	/// Iterate over smallest list of bit strings that cover everything with information whether they are part of the set or not
	pub fn iter_full(&self) -> IterSetFull<'_, K> {
		IterSetFull {
			iter: self.tree.iter_leaf_full(),
		}
	}
}

/// Iterate over all prefixes contained in a set
pub struct IterSet<'s, K: BitString + Clone + Eq> {
	iter: super::tree::IterLeaf<'s, TpSet<K>>,
}

impl<'s, K: BitString + Clone + Eq> Iterator for IterSet<'s, K> {
	type Item = &'s K;

	fn next(&mut self) -> Option<Self::Item> {
		Some(self.iter.next()?.0.get_key())
	}
}

/// Iterate over smallest list of bit strings that cover everything with information whether they are part of the set or not
pub struct IterSetFull<'s, K: BitString + Clone + Eq> {
	iter: super::tree::IterLeafFull<'s, TpSet<K>>,
}

impl<'s, K: BitString + Clone + Eq> Iterator for IterSetFull<'s, K> {
	type Item = (K, bool);

	fn next(&mut self) -> Option<Self::Item> {
		let (key, value) = self.iter.next()?;
		Some((key, value.is_some()))
	}
}
