use crate::tree::{
	mut_gen,
	TreeProperties,
	WalkedDirection,
};

/// Iterate over all nodes that are a prefix of target key in a [`WalkMutOwned`] stack
///
/// [`WalkMutOwned`]: crate::tree::WalkMutOwned
pub struct IterWalkMutOwnedPath<'r, 'w, TP, D = ()>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterWalkMutPath<'r, 'w, TP, mut_gen::Owned, D>,
}

impl<'r, TP, D> Iterator for IterWalkMutOwnedPath<'r, '_, TP, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable values of tree depth-first pre-order
pub struct IterMutOwnedPreOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutPreOrder<'r, TP, mut_gen::Owned>,
}

impl<'r, TP> From<mut_gen::IterMutPreOrder<'r, TP, mut_gen::Owned>> for IterMutOwnedPreOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutPreOrder<'r, TP, mut_gen::Owned>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutOwnedPreOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable values of tree depth-first in-order
pub struct IterMutOwnedInOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutInOrder<'r, TP, mut_gen::Owned>,
}

impl<'r, TP> From<mut_gen::IterMutInOrder<'r, TP, mut_gen::Owned>> for IterMutOwnedInOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutInOrder<'r, TP, mut_gen::Owned>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutOwnedInOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable values of tree depth-first post-order
pub struct IterMutOwnedPostOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutPostOrder<'r, TP, mut_gen::Owned>,
}

impl<'r, TP> From<mut_gen::IterMutPostOrder<'r, TP, mut_gen::Owned>>
	for IterMutOwnedPostOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutPostOrder<'r, TP, mut_gen::Owned>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutOwnedPostOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable leaf values of tree in-order
pub struct IterMutOwnedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutLeaf<'r, TP, mut_gen::Owned>,
}

impl<'r, TP> From<mut_gen::IterMutLeaf<'r, TP, mut_gen::Owned>> for IterMutOwnedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutLeaf<'r, TP, mut_gen::Owned>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutOwnedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (&'r TP::Key, &'r mut TP::LeafValue);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable leaf values and uncovered keys of tree in-order
pub struct IterMutOwnedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutLeafFull<'r, TP, mut_gen::Owned>,
}

impl<'r, TP> From<mut_gen::IterMutLeafFull<'r, TP, mut_gen::Owned>> for IterMutOwnedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutLeafFull<'r, TP, mut_gen::Owned>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutOwnedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (TP::Key, Option<&'r mut TP::LeafValue>);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}
