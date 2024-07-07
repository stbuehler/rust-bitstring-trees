use crate::{
	iter::{
		iter_between,
		IterBetween,
	},
	tree::{
		mut_gen::{
			WalkMut,
			WalkMutPath,
		},
		MutPath,
		Node,
		TreeProperties,
		WalkedDirection,
	},
};

use super::walk::OwnedTreeMarker;

// safety: must only call once per node per iterator lifetime
unsafe fn extract_from_node<'r, TP: TreeProperties>(
	node: &mut Node<TP>,
) -> (
	&'r TP::Key,
	&'r mut TP::Value,
	Option<&'r mut TP::LeafValue>,
) {
	let key: *const <TP as TreeProperties>::Key = node.get_key() as *const _;
	let value = node.get_value_mut() as *mut _;
	let leaf_value = node.get_leaf_value_mut().map(|v| v as *mut _);
	(
		unsafe { &*key },
		unsafe { &mut *value },
		leaf_value.map(|v| unsafe { &mut *v }),
	)
}

/// Iterate over keys and mutable values of tree that are a prefix of target key
pub struct IterMutPath<'r, TP: TreeProperties> {
	path: MutPath<'r, TP>,
}

impl<'r, TP: TreeProperties> IterMutPath<'r, TP> {
	pub(in crate::tree) fn new(path: MutPath<'r, TP>) -> Self {
		Self { path }
	}
}

impl<'r, TP: TreeProperties> Iterator for IterMutPath<'r, TP> {
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.path.next()?;
		// safety: only once per node per iteration
		Some(unsafe { extract_from_node(node) })
	}
}

/// Iterate over all nodes that are a prefix of target key in a [`WalkMut`] stack
pub struct IterWalkMutPath<'r, 'w, TP, O, D = ()>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D, ()>,
{
	path: WalkMutPath<'r, 'w, TP, O, D>,
}

impl<'r, 'w, TP, O, D> IterWalkMutPath<'r, 'w, TP, O, D>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D, ()>,
{
	pub(in crate::tree) fn new(path: WalkMutPath<'r, 'w, TP, O, D>) -> Self {
		Self { path }
	}
}

impl<'r, TP, O, D> Iterator for IterWalkMutPath<'r, '_, TP, O, D>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D, ()>,
	D: From<WalkedDirection>,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.path.next()?;
		// safety: only once per node per iteration
		Some(unsafe { extract_from_node(node) })
	}
}

/// Iterate over keys and mutable values of tree depth-first pre-order
pub(in crate::tree) struct IterMutPreOrder<'r, TP, O>
where
	TP: TreeProperties,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	pub(super) walk: WalkMut<'r, TP, O, WalkedDirection>,
}

impl<'r, TP, O> Iterator for IterMutPreOrder<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.walk.next_pre_order()?;
		// safety: only once per node per iteration
		Some(unsafe { extract_from_node(node) })
	}
}

/// Iterate over keys and mutable values of tree depth-first in-order
pub(in crate::tree) struct IterMutInOrder<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	pub(super) walk: WalkMut<'r, TP, O, WalkedDirection>,
}

impl<'r, TP, O> Iterator for IterMutInOrder<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.walk.next_in_order()?;
		// safety: only once per node per iteration
		Some(unsafe { extract_from_node(node) })
	}
}

/// Iterate over keys and mutable values of tree depth-first post-order
pub(in crate::tree) struct IterMutPostOrder<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	pub(super) walk: WalkMut<'r, TP, O, WalkedDirection>,
}

impl<'r, TP, O> Iterator for IterMutPostOrder<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.walk.next_post_order()?;
		// safety: only once per node per iteration
		Some(unsafe { extract_from_node(node) })
	}
}

/// Iterate over keys and mutable leaf values of tree in-order
pub(in crate::tree) struct IterMutLeaf<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	pub(super) walk: WalkMut<'r, TP, O, WalkedDirection>,
}

impl<'r, TP, O> Iterator for IterMutLeaf<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	type Item = (&'r TP::Key, &'r mut TP::LeafValue);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.walk.next_leaf()?;
		// safety: only once per node per iteration
		let (key, _, leaf_value) = unsafe { extract_from_node(node) };
		Some((key, leaf_value.expect("leaf node")))
	}
}

/// Iterate over keys and mutable leaf values and uncovered keys of tree in-order
pub(in crate::tree) struct IterMutLeafFull<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	walk: Option<WalkMut<'r, TP, O, WalkedDirection>>,
	previous_key: Option<TP::Key>,
	uncovered: IterBetween<TP::Key>,
	next: Option<(TP::Key, &'r mut TP::LeafValue)>,
}

impl<'r, TP, O> IterMutLeafFull<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	pub(in crate::tree) fn new(walk: WalkMut<'r, TP, O, WalkedDirection>) -> Self {
		Self {
			walk: Some(walk),
			previous_key: None,
			uncovered: Default::default(),
			next: None,
		}
	}
}

impl<'r, TP, O> Iterator for IterMutLeafFull<'r, TP, O>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
{
	type Item = (TP::Key, Option<&'r mut TP::LeafValue>);

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
					let (key, _, leaf_value) = unsafe { extract_from_node(node) };
					let key = key.clone();
					let leaf_value = leaf_value.expect("leaf node");
					// return uncovered prefixes before
					let start = core::mem::replace(&mut self.previous_key, Some(key.clone()));
					self.uncovered = iter_between(start, Some(key.clone()));
					self.next = Some((key, leaf_value));
				},
			}
		}
	}
}
