//! [`FullMap`] of bit string prefixes

use core::{
	cmp::Ordering,
	marker::PhantomData,
};

use bitstring::BitString;

use crate::tree::{
	DefaultCompare,
	Node,
	Tree,
	TreeProperties,
	WalkedDirection,
};

struct TpFullMap<K: BitString + Clone, V>(PhantomData<*const K>, PhantomData<*const V>);

impl<K: BitString + Clone, V> TreeProperties for TpFullMap<K, V> {
	type Key = K;
	type LeafValue = ();
	type LeafValueComparer = DefaultCompare;
	type Value = Option<V>;

	const EMPTY: bool = false;
	const IGNORE_LEAFS: bool = true;
	const LEAF_EMPTY: bool = true;
}

/// Map bit string prefixes to values
///
/// This allows overriding values based on "better matching" longer prefixes in an efficient way.
///
/// Network routing tables are usually implemented that way: there often is a default
/// route for `0.0.0.0/0` and then a "more specific" for the LAN, e.g. `192.168.0.0/24`.
/// (I.e. a route is a map entry for a prefix to a "nexthop specification", as in how
/// to forward a packet matching the entry. The "most specific" (longest) matching
/// route is used.)
///
/// This is implemented as a [`crate::tree::Tree`] where all nodes can have an optional value;
/// branches where no node has a value are pruned.
#[derive(Clone)]
pub struct FullMap<K: BitString + Clone, V> {
	tree: Tree<TpFullMap<K, V>>,
}

impl<K: BitString + Clone, V> Default for FullMap<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K, V> core::fmt::Debug for FullMap<K, V>
where
	K: BitString + Clone + core::fmt::Debug,
	V: core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		f.debug_map().entries(self.iter()).finish()
	}
}

impl<K, V> FullMap<K, V>
where
	K: BitString + Clone,
{
	/// New (empty) map.
	pub const fn new() -> Self {
		Self { tree: Tree::new() }
	}

	/// Gets the given key's corresponding entry in the map for in-place manipulation.
	pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
		let mut walk = self.tree.walk_mut();
		walk.goto_insert(&key);
		if let Some(node) = walk.current().node() {
			if node.get_key().len() == key.len() && node.get_value().is_some() {
				return Entry::Occupied(OccupiedEntry { walk });
			}
		}
		Entry::Vacant(VacantEntry { walk, key })
	}

	fn occupied<'s>(&'s mut self, key: &K) -> Option<OccupiedEntry<'s, K, V>> {
		let mut walk = self.tree.walk_mut();
		walk.goto_insert(key);
		if let Some(node) = walk.current().node() {
			if node.get_key().len() == key.len() && node.get_value().is_some() {
				return Some(OccupiedEntry { walk });
			}
		}
		None
	}

	/// Inserts a key-value pair into the map.
	///
	/// If the map did not have this key present, None is returned.
	///
	/// If the map did have this key present, the value is updated, and the old value is returned.
	pub fn insert(&mut self, key: K, value: V) -> Option<V> {
		self.entry(key).replace(value).1
	}

	/// Removes a key from the map, returning the stored key and value if the key
	/// was previously in the map.
	pub fn remove(&mut self, key: &K) -> Option<V> {
		Some(self.occupied(key)?.remove())
	}

	/// Returns a reference to the value corresponding to the key.
	pub fn get(&self, key: &K) -> Option<&V> {
		self.tree.get(key)?.get_value().as_ref()
	}

	/// Returns a mutable reference to the value corresponding to the key.
	pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
		self.tree.get_mut(key)?.get_value_mut().as_mut()
	}

	/// Returns a reference to the key-value pair for the longest prefix of the key in the map.
	pub fn most_specific(&self, key: &K) -> Option<(&K, &V)> {
		// TODO: could probably also implement it using walk.goto + check, or manually
		self.path(key.clone()).last()
	}

	/// Remove all prefixes equal or longer than given key
	pub fn remove_tree(&mut self, key: K) {
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

	/// Iterate over all prefixes and their values on the path to a key
	pub fn path(&self, key: K) -> IterPath<'_, K, V> {
		IterPath {
			iter: self.tree.iter_path(key),
		}
	}

	/// Iterate over all prefixes and their mutable values on the path to a key
	///
	// TODO: return a `WalkMutPath` wrapper with IntoIterator impl?
	pub fn path_mut(&mut self, key: K) -> IterPathMut<'_, K, V> {
		IterPathMut {
			iter: self.tree.iter_mut_path(key).into_iter(),
		}
	}

	/// Iterate over all (aggregated) prefixes and their values
	pub fn iter(&self) -> IterMap<'_, K, V> {
		IterMap {
			iter: self.tree.iter_in_order(),
		}
	}

	/// Iterate over all (aggregated) prefixes and their mutable values
	pub fn iter_mut(&mut self) -> IterMutMap<'_, K, V> {
		IterMutMap {
			iter: self.tree.iter_mut_in_order(),
		}
	}
}

// basically copied from alloc::collections::btree::map::entry:
/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This enum is constructed from the [`entry`] method on [`FullMap`].
///
/// [`entry`]: FullMap::entry
pub enum Entry<'s, K: BitString + Clone, V> {
	/// A vacant entry.
	Vacant(VacantEntry<'s, K, V>),
	/// An occupied entry.
	Occupied(OccupiedEntry<'s, K, V>),
}

impl<'s, K: BitString + Clone, V> Entry<'s, K, V> {
	/// Ensures a value is in the entry by inserting the default if empty, and returns
	/// a mutable reference to the value in the entry.
	pub fn or_insert(self, default: V) -> &'s mut V {
		match self {
			Self::Occupied(entry) => entry.into_mut(),
			Self::Vacant(entry) => entry.insert(default),
		}
	}

	/// Ensures a value is in the entry by inserting the result of the default function if empty,
	/// and returns a mutable reference to the value in the entry.
	pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'s mut V {
		match self {
			Self::Occupied(entry) => entry.into_mut(),
			Self::Vacant(entry) => entry.insert(default()),
		}
	}

	/// Ensures a value is in the entry by inserting, if empty, the result of the default function.
	/// This method allows for generating key-derived values for insertion by providing the default
	/// function a reference to the key that was moved during the `.entry(key)` method call.
	///
	/// The reference to the moved key is provided so that cloning or copying the key is
	/// unnecessary, unlike with `.or_insert_with(|| ... )`.
	#[inline]
	pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'s mut V {
		match self {
			Self::Occupied(entry) => entry.into_mut(),
			Self::Vacant(entry) => {
				let value = default(entry.key());
				entry.insert(value)
			},
		}
	}

	/// Returns a reference to this entry's key.
	pub fn key(&self) -> &K {
		match self {
			Self::Occupied(entry) => entry.key(),
			Self::Vacant(entry) => entry.key(),
		}
	}

	/// Provides in-place mutable access to an occupied entry before any
	/// potential inserts into the map.
	pub fn and_modify<F>(mut self, f: F) -> Self
	where
		F: FnOnce(&mut V),
	{
		if let Self::Occupied(ref mut entry) = self {
			f(entry.get_mut())
		}
		self
	}

	/// Ensures a value is in the entry by inserting the default value if empty,
	/// and returns a mutable reference to the value in the entry.
	pub fn or_default(self) -> &'s mut V
	where
		V: Default,
	{
		match self {
			Self::Occupied(entry) => entry.into_mut(),
			Self::Vacant(entry) => entry.insert(Default::default()),
		}
	}

	/// Sets or inserts the value of the entry with the [`Entry`]'s key,
	/// and returns a mutable reference to it.
	pub fn insert(self, value: V) -> &'s mut V {
		match self {
			Self::Occupied(entry) => {
				let vref = entry.into_mut();
				*vref = value;
				vref
			},
			Self::Vacant(entry) => entry.insert(value),
		}
	}

	/// Sets or inserts the value of the entry with the [`Entry`]'s key,
	/// and returns the occupied entry.
	pub fn set(self, value: V) -> OccupiedEntry<'s, K, V> {
		self.replace(value).0
	}

	/// Sets or inserts the value of the entry with the [`Entry`]'s key,
	/// and returns the occupied entry and previous value (if present).
	pub fn replace(self, value: V) -> (OccupiedEntry<'s, K, V>, Option<V>) {
		match self {
			Self::Occupied(mut entry) => {
				let old = entry.insert(value);
				(entry, Some(old))
			},
			Self::Vacant(entry) => {
				let VacantEntry { mut walk, key } = entry;
				walk.insert(key);
				let node = walk
					.current_mut()
					.node()
					.expect("after insert walk should be at a node");
				*node.get_value_mut() = Some(value);
				(OccupiedEntry { walk }, None)
			},
		}
	}
}

/// A view into a vacant entry in a [`FullMap`]. It is part of the [`Entry`] enum.
pub struct VacantEntry<'s, K: BitString + Clone + 's, V: 's> {
	walk: crate::tree::WalkMutOwned<'s, TpFullMap<K, V>, WalkedDirection>,
	key: K,
}

impl<'s, K: BitString + Clone, V> VacantEntry<'s, K, V> {
	/// Gets a reference to the key that would be used when inserting a value
	/// through the VacantEntry.
	pub fn key(&self) -> &K {
		&self.key
	}

	/// Take ownership of the key.
	pub fn into_key(self) -> K {
		self.key
	}

	/// Sets the value of the entry with the `VacantEntry`'s key,
	/// and returns a mutable reference to it.
	pub fn insert(self, value: V) -> &'s mut V {
		let Self { mut walk, key } = self;
		walk.insert(key);
		let node = walk
			.into_current_mut()
			.node()
			.expect("after insert walk should be at a node");
		*node.get_value_mut() = Some(value);
		node.get_value_mut().as_mut().expect("value can't be empty")
	}
}

/// A view into an occupied entry in a [`FullMap`]. It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'s, K: BitString + Clone + 's, V: 's> {
	walk: crate::tree::WalkMutOwned<'s, TpFullMap<K, V>, WalkedDirection>,
}

impl<'s, K: BitString + Clone, V> OccupiedEntry<'s, K, V> {
	fn node(&self) -> &Node<TpFullMap<K, V>> {
		self.walk
			.current()
			.node()
			.expect("OccupiedEntry should have a node")
	}

	fn node_mut(&mut self) -> &mut Node<TpFullMap<K, V>> {
		self.walk
			.current_mut()
			.node()
			.expect("OccupiedEntry should have a node")
	}

	fn node_into(self) -> &'s mut Node<TpFullMap<K, V>> {
		self.walk
			.into_current_mut()
			.node()
			.expect("OccupiedEntry should have a node")
	}

	/// Gets a reference to the value in the entry.
	pub fn get(&self) -> &V {
		self.node()
			.get_value()
			.as_ref()
			.expect("OccupiedEntry should have a value")
	}

	/// Gets a mutable reference to the value in the entry.
	///
	/// If you need a reference to the [`OccupiedEntry`] that may outlive the destruction of the Entry value, see [`into_mut`].
	///
	/// [`into_mut`]: OccupiedEntry::into_mut
	pub fn get_mut(&mut self) -> &mut V {
		self.node_mut()
			.get_value_mut()
			.as_mut()
			.expect("OccupiedEntry should have a value")
	}

	/// Converts the entry into a mutable reference to its value.
	///
	/// If you need multiple references to the [`OccupiedEntry`], see [`get_mut`].
	///
	/// [`get_mut`]: OccupiedEntry::get_mut
	pub fn into_mut(self) -> &'s mut V {
		self.node_into()
			.get_value_mut()
			.as_mut()
			.expect("OccupiedEntry should have a value")
	}

	/// Gets a reference to the key in the entry.
	pub fn key(&self) -> &K {
		self.node().get_key()
	}

	/// Sets the value of the entry with the [`OccupiedEntry`]'s key,
	/// and returns the entry's old value.
	pub fn insert(&mut self, value: V) -> V {
		core::mem::replace(self.get_mut(), value)
	}

	/// Takes the value of the entry out of the map, and returns it.
	pub fn remove(mut self) -> V {
		let value = self
			.node_mut()
			.get_value_mut()
			.take()
			.expect("OccupiedEntry should have a value");
		self.walk.compact_if_empty(Option::is_none);
		value
	}
}

/// Iterate over all prefixes and their values on the path to a key
pub struct IterPath<'s, K: BitString + Clone, V> {
	iter: crate::tree::IterPath<'s, TpFullMap<K, V>>,
}

impl<'s, K: BitString + Clone, V> Iterator for IterPath<'s, K, V> {
	type Item = (&'s K, &'s V);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let node = self.iter.next()?;
			// skip (inner) nodes that don't have a value
			if let Some(value) = node.get_value() {
				return Some((node.get_key(), value));
			}
		}
	}
}

/// Iterate over all prefixes and their values on the path to a key
pub struct IterPathMut<'s, K: BitString + Clone, V> {
	iter: crate::tree::IterMutPath<'s, TpFullMap<K, V>>,
}

impl<'s, K: BitString + Clone, V> Iterator for IterPathMut<'s, K, V> {
	type Item = (&'s K, &'s mut V);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let (key, value, _) = self.iter.next()?;
			// skip (inner) nodes that don't have a value
			if let Some(value) = value {
				return Some((key, value));
			}
		}
	}
}

/// Iterate over all prefixes and their values
pub struct IterMap<'s, K: BitString + Clone, V> {
	iter: crate::tree::IterInOrder<'s, TpFullMap<K, V>>,
}

impl<'s, K: BitString + Clone, V> Iterator for IterMap<'s, K, V> {
	type Item = (&'s K, &'s V);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let node = self.iter.next()?;
			// skip (inner) nodes that don't have a value
			if let Some(value) = node.get_value() {
				return Some((node.get_key(), value));
			}
		}
	}
}

/// Iterate over all (aggregated) prefixes and their mutable values
pub struct IterMutMap<'s, K: BitString + Clone, V> {
	iter: crate::tree::IterMutOwnedInOrder<'s, TpFullMap<K, V>>,
}

impl<'s, K: BitString + Clone, V> Iterator for IterMutMap<'s, K, V> {
	type Item = (&'s K, &'s mut V);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let (key, value, _) = self.iter.next()?;
			// skip (inner) nodes that don't have a value
			if let Some(value) = value.as_mut() {
				return Some((key, value));
			}
		}
	}
}
