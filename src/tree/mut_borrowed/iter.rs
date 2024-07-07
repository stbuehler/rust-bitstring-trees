use crate::tree::{
	mut_gen,
	TreeProperties,
	WalkedDirection,
};

/// Iterate over all nodes that are a prefix of target key in a [`WalkMutBorrowed`] stack
///
/// [`WalkMutBorrowed`]: crate::tree::WalkMutBorrowed
pub struct IterWalkMutBorrowedPath<'r, 'w, TP, D = ()>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterWalkMutPath<'r, 'w, TP, mut_gen::Borrowed, D>,
}

impl<'r, TP, D> Iterator for IterWalkMutBorrowedPath<'r, '_, TP, D>
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
pub struct IterMutBorrowedPreOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutPreOrder<'r, TP, mut_gen::Borrowed>,
}

impl<'r, TP> From<mut_gen::IterMutPreOrder<'r, TP, mut_gen::Borrowed>>
	for IterMutBorrowedPreOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutPreOrder<'r, TP, mut_gen::Borrowed>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutBorrowedPreOrder<'r, TP>
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
pub struct IterMutBorrowedInOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutInOrder<'r, TP, mut_gen::Borrowed>,
}

impl<'r, TP> From<mut_gen::IterMutInOrder<'r, TP, mut_gen::Borrowed>>
	for IterMutBorrowedInOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutInOrder<'r, TP, mut_gen::Borrowed>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutBorrowedInOrder<'r, TP>
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
pub struct IterMutBorrowedPostOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutPostOrder<'r, TP, mut_gen::Borrowed>,
}

impl<'r, TP> From<mut_gen::IterMutPostOrder<'r, TP, mut_gen::Borrowed>>
	for IterMutBorrowedPostOrder<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutPostOrder<'r, TP, mut_gen::Borrowed>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutBorrowedPostOrder<'r, TP>
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
pub struct IterMutBorrowedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutLeaf<'r, TP, mut_gen::Borrowed>,
}

impl<'r, TP> From<mut_gen::IterMutLeaf<'r, TP, mut_gen::Borrowed>> for IterMutBorrowedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutLeaf<'r, TP, mut_gen::Borrowed>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutBorrowedLeaf<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (&'r TP::Key, &'r mut TP::LeafValue);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Iterate over keys and mutable leaf values and uncovered keys of tree in-order
pub struct IterMutBorrowedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	inner: mut_gen::IterMutLeafFull<'r, TP, mut_gen::Borrowed>,
}

impl<'r, TP> From<mut_gen::IterMutLeafFull<'r, TP, mut_gen::Borrowed>>
	for IterMutBorrowedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	fn from(inner: mut_gen::IterMutLeafFull<'r, TP, mut_gen::Borrowed>) -> Self {
		Self { inner }
	}
}

impl<'r, TP> Iterator for IterMutBorrowedLeafFull<'r, TP>
where
	TP: TreeProperties + 'r,
{
	type Item = (TP::Key, Option<&'r mut TP::LeafValue>);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}
