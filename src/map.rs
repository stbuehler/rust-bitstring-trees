//! [`Map`] of bit string prefixes

use core::{
	cmp::Ordering,
	marker::PhantomData,
};

use bitstring::BitString;

use crate::tree::{
	DefaultCompare,
	Tree,
	TreeProperties,
};

struct TpMap<K, V>(PhantomData<*const K>, PhantomData<*const V>)
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq;

impl<K, V> TreeProperties for TpMap<K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	type Key = K;
	type LeafValue = V;
	type LeafValueComparer = DefaultCompare;
	type Value = ();

	const EMPTY: bool = true;
	const IGNORE_LEAFS: bool = false;
	const LEAF_EMPTY: bool = false;
}

/// Map of bit strings (combined to prefixes) to values
///
/// Each bit string can only have a single value; sibling bit strings
/// mapping to the same value are automatically merged internally.
///
/// This is implemented as a [`crate::tree::Tree`] where only leaf nodes carry values.
#[derive(Clone)]
pub struct Map<K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	tree: Tree<TpMap<K, V>>,
}

impl<K, V> Default for Map<K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<K, V> core::fmt::Debug for Map<K, V>
where
	K: BitString + Clone + Eq + core::fmt::Debug,
	V: Default + Clone + Eq + core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

impl<K, V> Map<K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	/// New (empty) map.
	pub const fn new() -> Self {
		Self { tree: Tree::new() }
	}

	/// Set new value for all bit strings with given prefix
	pub fn insert(&mut self, prefix: K, value: V) {
		self.tree.set_leaf_value(prefix, value);
	}

	/// Unset values for all bit strings with given prefix
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

	/// Lookup value for a bit string
	///
	/// If only a prefix for longer values is given this only finds
	/// an aggregated value, i.e. lookups should usually be done
	/// using a "full-length" bit string.
	/// (E.g. lookup single hosts in a CIDR-map.)
	pub fn get(&self, key: &K) -> Option<&V> {
		let mut walk = self.tree.walk::<(), ()>();
		walk.goto_insert(key);
		match walk.current().node() {
			None => None, // empty tree
			Some(node) => {
				match node.get_key().len().cmp(&key.len()) {
					Ordering::Less => {
						// node is a leaf and covers key
						Some(node.get_leaf_value().expect("node must be a leaf"))
					},
					Ordering::Equal => node.get_leaf_value(),
					Ordering::Greater => {
						// key not fully contained
						None
					},
				}
			},
		}
	}

	/// Iterate over all (aggregated) prefixes and their values
	pub fn iter(&self) -> IterMap<'_, K, V> {
		IterMap {
			iter: self.tree.iter_leaf(),
		}
	}

	/// Iterate over all (aggregated) prefixes and their mutable values
	pub fn iter_mut(&mut self) -> IterMutMap<'_, K, V> {
		IterMutMap {
			iter: self.tree.iter_mut_leaf(),
		}
	}

	/// Iterate over smallest list of bit strings that cover everything with a value or None if not mapped
	pub fn iter_full(&self) -> IterMapFull<'_, K, V> {
		IterMapFull {
			iter: self.tree.iter_leaf_full(),
		}
	}
}

/// Iterate over all (aggregated) prefixes and their values
pub struct IterMap<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	iter: crate::tree::IterLeaf<'s, TpMap<K, V>>,
}

impl<'s, K, V> Iterator for IterMap<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	type Item = (&'s K, &'s V);

	fn next(&mut self) -> Option<Self::Item> {
		let (node, value) = self.iter.next()?;
		Some((node.get_key(), value))
	}
}

/// Iterate over all (aggregated) prefixes and their mutable values
pub struct IterMutMap<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	iter: crate::tree::IterMutOwnedLeaf<'s, TpMap<K, V>>,
}

impl<'s, K, V> Iterator for IterMutMap<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	type Item = (&'s K, &'s mut V);

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
}

/// Iterate over smallest list of bit strings that cover everything with a value or None if not mapped
pub struct IterMapFull<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	iter: crate::tree::IterLeafFull<'s, TpMap<K, V>>,
}

impl<'s, K, V> Iterator for IterMapFull<'s, K, V>
where
	K: BitString + Clone + Eq,
	V: Default + Clone + Eq,
{
	type Item = (K, Option<&'s V>);

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
}
