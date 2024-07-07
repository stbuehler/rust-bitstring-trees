use crate::{
	tree::{
		mut_gen::{
			Owned,
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
	IterMutOwnedInOrder,
	IterMutOwnedLeaf,
	IterMutOwnedLeafFull,
	IterMutOwnedPostOrder,
	IterMutOwnedPreOrder,
};

/// Walk owned mutable tree up and down
///
/// Some algorithms need to remember how they reached the current node via [`WalkedDirection`] as `D`.
///
/// When walking manually it might be useful to be able to store additional data via `A`.
pub struct WalkMutOwned<'r, TP: TreeProperties + 'r, D = (), A = ()> {
	pub(in crate::tree) inner: WalkMut<'r, TP, Owned, D, A>,
}

impl<'r, TP, D, A> WalkMutOwned<'r, TP, D, A>
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
	/// If you need the result to outlive the destruction of the [`WalkMutOwned`] value, see [`into_current_mut`].
	///
	/// [`into_current_mut`]: WalkMutOwned::into_current_mut
	pub fn current_mut(&mut self) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>> {
		self.inner.current_mut()
	}

	/// Extract mutable node or tree
	///
	/// Also see [`current_mut`]
	///
	/// [`current_mut`]: WalkMutOwned::current_mut
	pub fn into_current_mut(self) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>> {
		self.inner.into_current_mut()
	}
}

impl<'r, TP> WalkMutOwned<'r, TP, WalkedDirection, ()>
where
	TP: TreeProperties + 'r,
{
	/// Delete current node (or tree)
	///
	/// Afterwards the current node is the previous parent node, which was replaced by the sibling,
	/// or the (empty) tree when removing the last node.
	///
	/// Returns what [`up_with`] would have returned.
	///
	/// [`up_with`]: WalkMutOwned::up_with
	pub fn delete_current(&mut self) -> Option<WalkedDirection> {
		self.inner.delete_current()
	}
}

impl<'r, TP, A> WalkMutOwned<'r, TP, WalkedDirection, A>
where
	TP: TreeProperties + 'r,
{
	/// Delete current node (or tree)
	///
	/// Afterwards the current node is the previous parent node, which was replaced by the sibling,
	/// or the (empty) tree when removing the last node.
	///
	/// Returns what [`up_with`] would have returned.
	///
	/// [`up_with`]: WalkMutOwned::up_with
	pub fn delete_current_with(&mut self) -> Option<(WalkedDirection, A)> {
		self.inner.delete_current_with()
	}

	/// Remove empty leaf nodes if possible
	///
	/// A node is considered "empty" if the passed function considers its value empty.
	///
	/// Calls this if the current value might just have become empty.
	///
	/// Empty leafs can only be removed if the parent node is empty too, or
	/// both siblings are empty leafs (then the parent becomes a leaf node).
	///
	/// * if current node isn't empty nothing changes
	/// * if current node is an empty leaf node:
	///   * parent and sibling shouldn't have both been empty, as previous [`compact_if_empty`] calls would have cleaned that up
	///   * if parent is empty: remove leaf and parent, replace parent with sibling. `current` points to sibling afterwards.
	///   * if sibling is an empty leaf: make parent a leaf, `current` points to parent afterwards.
	///   * otherwise no tree changes, but `current` points to parent afterwards.
	/// * if current node has an empty leaf child node, remove that child node and current node.
	///   I.e. replace current node with other child; `current` points to that child afterwards.
	///   Other child node shouldn't be an empty leaf, as previous [`compact_if_empty`] calls would have cleaned that up.
	/// * if current points to tree or root node, clear tree if it is an empty node
	///
	/// [`compact_if_empty`]: Self::compact_if_empty
	pub fn compact_if_empty<F>(&mut self, is_empty: F)
	where
		F: Fn(&TP::Value) -> bool,
	{
		self.inner.compact_if_empty(is_empty)
	}
}

impl<'r, TP, D, A> WalkMutOwned<'r, TP, D, A>
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

impl<'r, TP, D> WalkMutOwned<'r, TP, D, ()>
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

impl<'r, TP, D> WalkMutOwned<'r, TP, D>
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
	pub fn path(&mut self, key: TP::Key) -> WalkMutOwnedPath<'r, '_, TP, D> {
		WalkMutOwnedPath {
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

impl<'r, TP, D> WalkMutOwned<'r, TP, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Insert new (possibly inner) node with exact key in tree, walk to it and return reference to it
	pub fn insert(&mut self, key: TP::Key) -> &mut Node<TP> {
		self.inner.insert(key)
	}
}

impl<'r, TP> WalkMutOwned<'r, TP, WalkedDirection, ()>
where
	TP: TreeProperties + 'r,
{
	/// Convert into iterator traversing depth-first pre-order
	pub fn into_iter_pre_order(self) -> IterMutOwnedPreOrder<'r, TP> {
		self.inner.into_iter_pre_order().into()
	}

	/// Tree traversal: depth-first pre-order
	pub fn next_pre_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_pre_order()
	}

	/// Convert into iterator traversing depth-first in-order
	pub fn into_iter_in_order(self) -> IterMutOwnedInOrder<'r, TP> {
		self.inner.into_iter_in_order().into()
	}

	/// Tree traversal: depth-first in-order
	pub fn next_in_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_in_order()
	}

	/// Convert into iterator traversing depth-first post-order
	pub fn into_iter_post_order(self) -> IterMutOwnedPostOrder<'r, TP> {
		self.inner.into_iter_post_order().into()
	}

	/// Tree traversal: depth-first post-order
	pub fn next_post_order(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_post_order()
	}

	/// Convert into iterator over all leafs
	pub fn into_iter_leafs(self) -> IterMutOwnedLeaf<'r, TP> {
		self.inner.into_iter_leafs().into()
	}

	/// Convert into iterator over all leafs and uncovered parts
	pub fn into_iter_full_leafs(self) -> IterMutOwnedLeafFull<'r, TP> {
		self.inner.into_iter_full_leafs().into()
	}

	/// Tree traversal: depth-first in-order leaf nodes only
	pub fn next_leaf(&mut self) -> Option<&mut Node<TP>> {
		self.inner.next_leaf()
	}
}

/// Iterate over all nodes that are a prefix of target key in a [`WalkMutOwned`] stack
pub struct WalkMutOwnedPath<'r, 'w, TP, D = ()>
where
	TP: TreeProperties + 'r,
{
	inner: WalkMutPath<'r, 'w, TP, Owned, D>,
}

impl<'r, 'w, TP, D> WalkMutOwnedPath<'r, 'w, TP, D>
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
