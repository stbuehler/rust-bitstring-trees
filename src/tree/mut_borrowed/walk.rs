use crate::{
	tree::{
		mut_gen::{
			Borrowed,
			WalkMut,
			WalkMutPath,
		},
		InsertPosition,
		Node,
		TreeProperties,
		WalkedDirection,
	},
	walk_mut::NodeOrTree,
};

use super::{
	IterMutBorrowedInOrder,
	IterMutBorrowedLeaf,
	IterMutBorrowedLeafFull,
	IterMutBorrowedPostOrder,
	IterMutBorrowedPreOrder,
};

/// Walk borrowed mutable tree up and down
///
/// Some algorithms need to remember how they reached the current node via [`WalkedDirection`] as `D`.
///
/// When walking manually it might be useful to be able to store additional data via `A`.
pub struct WalkMutBorrowed<'r, TP: TreeProperties + 'r, D = (), A = ()> {
	pub(in crate::tree) inner: WalkMut<'r, TP, Borrowed, D, A>,
}

impl<'r, TP, D, A> WalkMutBorrowed<'r, TP, D, A>
where
	TP: TreeProperties,
{
	/// Walk up to parent node or tree if not at tree
	pub fn up(&mut self) -> Option<D> {
		self.inner.up()
	}

	/// Walk up to parent node or tree if not at tree
	pub fn up_with(&mut self) -> Option<(D, A)> {
		self.inner.up_with()
	}

	/// Current node or tree
	pub fn current(&self) -> NodeOrTree<Option<&Node<TP>>, &Node<TP>> {
		self.inner.current()
	}

	/// Current mutable node or tree
	///
	/// If you need the result to outlive the destruction of the [`WalkMutBorrowed`] value, see [`into_current_mut`].
	///
	/// [`into_current_mut`]: WalkMutBorrowed::into_current_mut
	pub fn current_mut(&mut self) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>> {
		self.inner.current_mut()
	}

	/// Extract mutable node or tree
	///
	/// Also see [`current_mut`]
	///
	/// [`current_mut`]: WalkMutBorrowed::current_mut
	pub fn into_current_mut(self) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>> {
		self.inner.into_current_mut()
	}
}

impl<'r, TP, D, A> WalkMutBorrowed<'r, TP, D, A>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if at tree and not empty)
	pub fn down_root_with(&mut self, add: A) -> bool {
		self.inner.down_root_with(add)
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left_with(&mut self, add: A) -> bool {
		self.inner.down_left_with(add)
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right_with(&mut self, add: A) -> bool {
		self.inner.down_right_with(add)
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down_with(&mut self, side: bool, add: A) -> bool {
		self.inner.down_with(side, add)
	}
}

impl<'r, TP, D> WalkMutBorrowed<'r, TP, D, ()>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if at tree and not empty)
	pub fn down_root(&mut self) -> bool {
		self.inner.down_root()
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left(&mut self) -> bool {
		self.inner.down_left()
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right(&mut self) -> bool {
		self.inner.down_right()
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down(&mut self, side: bool) -> bool {
		self.inner.down(side)
	}
}

impl<'r, TP, D> WalkMutBorrowed<'r, TP, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Start iterator to walk to deepest node that is a prefix of the target key
	///
	/// While consuming the iterator the stack is updated with the position of the returned nodes.
	///
	/// When `self` was in a mismatching subtree (i.e. not a prefix of the target key) before
	/// the iterator won't find anything.
	pub fn path(&mut self, key: TP::Key) -> WalkMutBorrowedPath<'r, '_, TP, D> {
		WalkMutBorrowedPath {
			inner: self.inner.path(key),
		}
	}

	/// Walk to node where we'd have to insert key at
	///
	/// Returns `None` if tree is empty.
	pub fn goto_insert(&mut self, key: &TP::Key) -> Option<InsertPosition> {
		self.inner.goto_insert(key)
	}
}

impl<'r, TP> WalkMutBorrowed<'r, TP, WalkedDirection, ()>
where
	TP: TreeProperties + 'r,
{
	/// Convert into iterator traversing depth-first pre-order
	pub fn into_iter_pre_order(self) -> IterMutBorrowedPreOrder<'r, TP> {
		self.inner.into_iter_pre_order().into()
	}

	/// Tree traversal: depth-first pre-order
	pub fn next_pre_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_pre_order()
	}

	/// Convert into iterator traversing depth-first in-order
	pub fn into_iter_in_order(self) -> IterMutBorrowedInOrder<'r, TP> {
		self.inner.into_iter_in_order().into()
	}

	/// Tree traversal: depth-first in-order
	pub fn next_in_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_in_order()
	}

	/// Convert into iterator traversing depth-first post-order
	pub fn into_iter_post_order(self) -> IterMutBorrowedPostOrder<'r, TP> {
		self.inner.into_iter_post_order().into()
	}

	/// Tree traversal: depth-first post-order
	pub fn next_post_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_post_order()
	}

	/// Convert into iterator over all leafs
	pub fn into_iter_leafs(self) -> IterMutBorrowedLeaf<'r, TP> {
		self.inner.into_iter_leafs().into()
	}

	/// Convert into iterator over all leafs and uncovered parts
	pub fn into_iter_full_leafs(self) -> IterMutBorrowedLeafFull<'r, TP> {
		self.inner.into_iter_full_leafs().into()
	}

	/// Tree traversal: depth-first in-order leaf nodes only
	pub fn next_leaf(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_leaf()
	}
}

/// Iterate over all nodes that are a prefix of target key in a [`WalkMutBorrowed`] stack
pub struct WalkMutBorrowedPath<'r, 'w, TP, D = ()>
where
	TP: TreeProperties + 'r,
{
	inner: WalkMutPath<'r, 'w, TP, Borrowed, D>,
}

impl<'r, 'w, TP, D> WalkMutBorrowedPath<'r, 'w, TP, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Next step towards target node
	#[allow(clippy::should_implement_trait)] // iterator doesn't allow using lifetime of itself in item
	pub fn next(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next()
	}
}

/*
impl<'r, 'w, TP, D> IntoIterator for WalkMutOwnedPath<'r, 'w, TP, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	type IntoIter = IterWalkMutOwnedPath<'r, 'w, TP, D>;
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn into_iter(self) -> Self::IntoIter {
		IterWalkMutOwnedPath::new(self)
	}
}
*/
