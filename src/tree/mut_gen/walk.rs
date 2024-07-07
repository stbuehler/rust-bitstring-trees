use bitstring::BitString;

use crate::{
	tree::{
		goto::{
			GotoStepResult,
			LookupStep,
			LookupStepWith,
			NodeRef,
		},
		InsertPosition,
		Node,
		Tree,
		TreeProperties,
		WalkedDirection,
	},
	walk_mut::NodeOrTree,
};

use super::{
	IterMutInOrder,
	IterMutLeaf,
	IterMutLeafFull,
	IterMutPostOrder,
	IterMutPreOrder,
	IterWalkMutPath,
};

pub(in crate::tree) trait OwnedTreeMarker<'r, TP: TreeProperties, D = (), A = ()> {
	type WalkMut;

	fn current(walk: &Self::WalkMut) -> NodeOrTree<Option<&Node<TP>>, &Node<TP>>;
	fn current_mut(walk: &mut Self::WalkMut) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>>;
	fn into_current_mut(
		walk: Self::WalkMut,
	) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>>;
	fn up(walk: &mut Self::WalkMut) -> Option<(D, A)>;

	fn try_walk<F, E>(walk: &mut Self::WalkMut, with: F) -> Result<(), E>
	where
		F: for<'n> FnOnce(
			NodeOrTree<Option<&'n mut Node<TP>>, &'n mut Node<TP>>,
		) -> Result<(&'n mut Node<TP>, (D, A)), E>;
}

/// [`WalkMut`] variant that owns (a mutable reference) to a tree and can create and delete the root node
pub(in crate::tree) struct Owned;

impl<'r, TP: TreeProperties + 'r, D, A> OwnedTreeMarker<'r, TP, D, A> for Owned {
	type WalkMut = crate::walk_mut::WalkMut<'r, Option<Node<TP>>, Node<TP>, (D, A)>;

	fn current(walk: &Self::WalkMut) -> NodeOrTree<Option<&Node<TP>>, &Node<TP>> {
		walk.current().map_tree(Option::as_ref)
	}

	fn current_mut(walk: &mut Self::WalkMut) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>> {
		walk.current_mut().map_tree(Option::as_mut)
	}

	fn into_current_mut(
		walk: Self::WalkMut,
	) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>> {
		walk.into_current_mut().map_tree(Option::as_mut)
	}

	fn up(walk: &mut Self::WalkMut) -> Option<(D, A)> {
		walk.pop()
	}

	// `with` shouldn't return a node for an empty tree (`Borrowed` will ignore that and not actually walk down)
	fn try_walk<F, E>(walk: &mut Self::WalkMut, with: F) -> Result<(), E>
	where
		F: for<'n> FnOnce(
			NodeOrTree<Option<&'n mut Node<TP>>, &'n mut Node<TP>>,
		) -> Result<(&'n mut Node<TP>, (D, A)), E>,
	{
		walk.try_walk(|node_or_tree| with(node_or_tree.map_tree(Option::as_mut)))
	}
}

/// [`WalkMut`] variant that only borrows a tree or sub tree, and can't create or delete the root node
pub(in crate::tree) struct Borrowed;

impl<'r, TP: TreeProperties + 'r, D, A> OwnedTreeMarker<'r, TP, D, A> for Borrowed {
	type WalkMut = Option<crate::walk_mut::WalkMut<'r, Node<TP>, Node<TP>, (D, A)>>;

	fn current(walk: &Self::WalkMut) -> NodeOrTree<Option<&Node<TP>>, &Node<TP>> {
		match walk {
			Some(walk) => walk.current().map_tree(Some),
			None => NodeOrTree::Tree(None),
		}
	}

	fn current_mut(walk: &mut Self::WalkMut) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>> {
		match walk {
			Some(walk) => walk.current_mut().map_tree(Some),
			None => NodeOrTree::Tree(None),
		}
	}

	fn into_current_mut(
		walk: Self::WalkMut,
	) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>> {
		match walk {
			Some(walk) => walk.into_current_mut().map_tree(Some),
			None => NodeOrTree::Tree(None),
		}
	}

	fn up(walk: &mut Self::WalkMut) -> Option<(D, A)> {
		walk.as_mut()?.pop()
	}

	fn try_walk<F, E>(walk: &mut Self::WalkMut, with: F) -> Result<(), E>
	where
		F: for<'n> FnOnce(
			NodeOrTree<Option<&'n mut Node<TP>>, &'n mut Node<TP>>,
		) -> Result<(&'n mut Node<TP>, (D, A)), E>,
	{
		match walk {
			Some(walk) => walk.try_walk(|node_or_tree| with(node_or_tree.map_tree(Some))),
			// `with` really shouldn't find a node in an empty tree, but the typesystem can easily deal with it:
			// (it could only return something a static lifetime though, so this really shouldn't happen)
			None => with(NodeOrTree::Tree(None)).map(|_| ()),
		}
	}
}

/// Walk mutable tree up and down
///
/// Some algorithms need to remember how they reached the current node via [`WalkedDirection`] as `D`.
///
/// When walking manually it might be useful to be able to store additional data via `A`.
pub(in crate::tree) struct WalkMut<
	'r,
	TP: TreeProperties,
	O: OwnedTreeMarker<'r, TP, D, A>,
	D = (),
	A = (),
> {
	pub(in crate::tree) walk: O::WalkMut,
}

impl<'r, TP: TreeProperties + 'r, D, A> WalkMut<'r, TP, Owned, D, A> {
	pub(in crate::tree) fn new(tree: &'r mut Tree<TP>) -> Self {
		Self {
			walk: crate::walk_mut::WalkMut::new(&mut tree.node),
		}
	}
}

impl<'r, TP, O, D, A> WalkMut<'r, TP, O, D, A>
where
	TP: TreeProperties,
	O: OwnedTreeMarker<'r, TP, D, A>,
{
	/// Walk up to parent node or tree if not at tree
	pub fn up(&mut self) -> Option<D> {
		Some(self.up_with()?.0)
	}

	/// Walk up to parent node or tree if not at tree
	pub fn up_with(&mut self) -> Option<(D, A)> {
		O::up(&mut self.walk)
	}

	/// Current node or tree
	pub fn current(&self) -> NodeOrTree<Option<&Node<TP>>, &Node<TP>> {
		O::current(&self.walk)
	}

	/// Current mutable node or tree
	///
	/// If you need the result to outlive the destruction of the [`WalkMut`] value, see [`into_current_mut`].
	///
	/// [`into_current_mut`]: WalkMut::into_current_mut
	pub fn current_mut(&mut self) -> NodeOrTree<Option<&mut Node<TP>>, &mut Node<TP>> {
		O::current_mut(&mut self.walk)
	}

	/// Extract mutable node or tree
	///
	/// Also see [`current_mut`]
	///
	/// [`current_mut`]: WalkMut::current_mut
	pub fn into_current_mut(self) -> NodeOrTree<Option<&'r mut Node<TP>>, &'r mut Node<TP>> {
		O::into_current_mut(self.walk)
	}
}

impl<'r, TP> WalkMut<'r, TP, Owned, WalkedDirection, ()>
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
	/// [`up_with`]: WalkMut::up_with
	pub fn delete_current(&mut self) -> Option<WalkedDirection> {
		Some(self.delete_current_with()?.0)
	}
}

impl<'r, TP, A> WalkMut<'r, TP, Owned, WalkedDirection, A>
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
	/// [`up_with`]: WalkMut::up_with
	pub fn delete_current_with(&mut self) -> Option<(WalkedDirection, A)> {
		if let Some(walked) = self.up_with() {
			match walked.0 {
				WalkedDirection::Down => (), // delete full tree below
				WalkedDirection::Left | WalkedDirection::Right => {
					let delete_right = walked.0 == WalkedDirection::Right;
					match self.walk.current_mut() {
						NodeOrTree::Node(node) => {
							node.delete_side(delete_right);
							return Some(walked);
						},
						// shouldn't have been able to walk left/right from tree;
						// anyway: delete previous node == full tree.
						NodeOrTree::Tree(tree) => {
							*tree = None;
							return Some(walked);
						},
					}
				},
			}
		}
		// either up() already was at tree, or explicit fallthrough above
		*self.walk.pop_all() = None;
		None
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
		match self.walk.current_mut() {
			NodeOrTree::Tree(tree) => {
				if let Some(root) = tree {
					if is_empty(&root.value) {
						*tree = None;
					}
				}
			},
			NodeOrTree::Node(node) => {
				if !is_empty(&node.value) {
					return;
				}
				if let Some((left, right)) = node.get_children() {
					if left.is_leaf() && is_empty(&left.value) {
						node.delete_side(false); // delete left empty leaf
					} else if right.is_leaf() && is_empty(&right.value) {
						node.delete_side(true); // delete right empty leaf
					}
					// even if we deleted an empty child node, assume that at least one wasn't an empty leaf,
					// otherwise we shouldn't have needed an inner node at the previous `node`
				} else {
					// `node` is an empty leaf node, check parent and sibling
					let dir: WalkedDirection = self.up().expect("shouldn't be at tree");
					let delete_side = match dir {
						WalkedDirection::Down => {
							// node was last node in tree, drop it
							*self.walk.pop_all() = None;
							return;
						},
						WalkedDirection::Left => false,
						WalkedDirection::Right => true,
					};
					match self.walk.current_mut() {
						NodeOrTree::Tree(tree) => {
							// should have gotten `WalkedDirection::Down` above, but clear tree anyway
							*tree = None;
						},
						NodeOrTree::Node(node) => {
							if is_empty(&node.value) {
								node.delete_side(delete_side);
							} else {
								let sibling =
									node.get_child(!delete_side).expect("sibling should exist");
								if sibling.is_leaf() && is_empty(&sibling.value) {
									// both child nodes are empty leafs: make a leaf node
									node.state = Default::default();
								}
							}
						},
					}
				}
			},
		}
	}
}

impl<'r, TP, O, D, A> WalkMut<'r, TP, O, D, A>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D, A>,
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if at tree and not empty)
	pub fn down_root_with(&mut self, add: A) -> bool {
		O::try_walk(&mut self.walk, |root_or_node| {
			let node = match root_or_node {
				NodeOrTree::Node(_) => return Err(()),
				NodeOrTree::Tree(None) => return Err(()),
				NodeOrTree::Tree(Some(r)) => r,
			};
			Ok((node, (WalkedDirection::Down.into(), add)))
		})
		.is_ok()
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left_with(&mut self, add: A) -> bool {
		self.down_with(false, add)
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right_with(&mut self, add: A) -> bool {
		self.down_with(true, add)
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down_with(&mut self, side: bool, add: A) -> bool {
		O::try_walk(&mut self.walk, |root_or_node| {
			let node = root_or_node.flatten_optional().ok_or(())?;
			Ok::<_, ()>((
				node.get_child_mut(side).ok_or(())?,
				(WalkedDirection::from_side(side).into(), add),
			))
		})
		.is_ok()
	}
}

impl<'r, TP, O, D> WalkMut<'r, TP, O, D, ()>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D>,
	D: From<WalkedDirection>,
{
	/// Walk down from tree to root node (if at tree and not empty)
	pub fn down_root(&mut self) -> bool {
		self.down_root_with(())
	}

	/// Walk down to left node if present and not currently at tree
	pub fn down_left(&mut self) -> bool {
		self.down_left_with(())
	}

	/// Walk down to right node if present and not currently at tree
	pub fn down_right(&mut self) -> bool {
		self.down_right_with(())
	}

	/// Walk down to specified node if present and not currently at tree
	///
	/// `false` picks left and `true` picks right.
	pub fn down(&mut self, side: bool) -> bool {
		self.down_with(side, ())
	}
}

impl<'r, TP, O, D> WalkMut<'r, TP, O, D>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D>,
	D: From<WalkedDirection>,
{
	fn lookup_step_initial(&mut self, key: &TP::Key, key_len: usize) -> LookupStep {
		self.down_root(); // ensure we are at least at root unless tree is empty
		let current = match self.current_mut().node() {
			Some(node) => node,
			None => return LookupStep::Miss,
		};

		current.lookup_initial_step(key, key_len).into()
	}

	fn lookup_step(&mut self, key: &TP::Key, key_len: usize) -> LookupStep {
		// need to extract different "success" values too
		let mut result = LookupStep::Miss;
		let _ = O::try_walk(&mut self.walk, |node_or_tree| {
			let node = node_or_tree.node().expect("should be at node");
			let lookup = node.lookup_step(key, key_len);
			result = (&lookup).into();
			let (next, dir) = match lookup {
				LookupStepWith::Path(next, dir) => (next, dir),
				LookupStepWith::Found(next, dir) => (next, dir),
				LookupStepWith::Miss => return Err(()),
			};
			Ok((next, (dir.into(), ())))
		});
		result
	}

	/// Start iterator to walk to deepest node that is a prefix of the target key
	///
	/// While consuming the iterator the stack is updated with the position of the returned nodes.
	///
	/// When `self` was in a mismatching subtree (i.e. not a prefix of the target key) before
	/// the iterator won't find anything.
	pub fn path(&mut self, key: TP::Key) -> WalkMutPath<'r, '_, TP, O, D> {
		WalkMutPath {
			start: true,
			done: false,
			walk: self,
			target_len: key.len(),
			target: key,
		}
	}

	// first need go up until current_node.key is a prefix of key (or we are at the root)
	fn goto_clean(&mut self, key: &TP::Key) {
		let key_len = key.len();
		while let NodeOrTree::Node(node) = self.current() {
			if node._is_prefix_of(key, key_len) {
				return;
			}
			self.up_with();
		}
	}

	// if not in the correct subtree call goto_clean first.
	fn goto_insert_step(
		&mut self,
		key: &TP::Key,
		key_len: usize,
	) -> Result<(), Option<InsertPosition>> {
		O::try_walk(&mut self.walk, |root_or_node| {
			let (node, dir) = match root_or_node {
				NodeOrTree::Tree(None) => return Err(None),
				NodeOrTree::Tree(Some(root)) => (root, WalkedDirection::Down),
				NodeOrTree::Node(node) => match node.goto_insert_step(key, key_len) {
					GotoStepResult::Final(r) => return Err(Some(r.into())),
					GotoStepResult::Continue(node, dir) => (node, dir),
				},
			};
			Ok((node, (dir.into(), ())))
		})
	}

	// if not in the correct subtree call goto_clean first.
	fn goto_insert_down(&mut self, key: &TP::Key) -> Option<InsertPosition> {
		let key_len = key.len();
		loop {
			match self.goto_insert_step(key, key_len) {
				Ok(()) => (),       // continue
				Err(r) => return r, // reached target
			}
		}
	}

	/// Walk to node where we'd have to insert key at
	///
	/// Returns `None` if tree is empty.
	pub fn goto_insert(&mut self, key: &TP::Key) -> Option<InsertPosition> {
		self.goto_clean(key);
		self.goto_insert_down(key)
	}
}

impl<'r, TP, D> WalkMut<'r, TP, Owned, D>
where
	TP: TreeProperties + 'r,
	D: From<WalkedDirection>,
{
	/// Insert new (possibly inner) node with exact key in tree, walk to it and return reference to it
	pub fn insert(&mut self, key: TP::Key) -> &mut Node<TP> {
		match self.goto_insert(&key) {
			Some(InsertPosition::BelowLeaf) => {
				let node = self.walk.current_mut().node().expect("should be at leaf");
				node.insert_sub_leaf(key.clone(), Default::default());
				self.goto_insert_down(&key);
			},
			Some(InsertPosition::AlreadyExists) => (),
			Some(InsertPosition::ReplaceNode) => {
				// node has longer key; need to insert inner node on top by adding leaf sibling
				let node = self.walk.current_mut().node().expect("should be at a node");
				let shared_prefix_len = key.shared_prefix_len(&node.key);
				if shared_prefix_len == key.len() {
					let mut sibling_key = node.key.clone();
					sibling_key.clip(shared_prefix_len + 1);
					sibling_key.flip(shared_prefix_len);
					node.insert_leaf_sibling(shared_prefix_len, sibling_key, Default::default());
					// already at correct node (that was replaced by the shared prefix == key)
				} else {
					node.insert_leaf_sibling(shared_prefix_len, key.clone(), Default::default());
					// move down to sibling we just inserted
					let side = key.get(shared_prefix_len);
					self.down(side);
				}
			},
			None => {
				let root: &mut Option<Node<TP>> = self.walk.pop_all();
				assert!(
					root.is_none(),
					"goto musn't end at tree with non-empty tree"
				);
				*root = Some(Node::new_leaf(key, Default::default(), Default::default()));
				self.down_root();
			},
		}

		self.walk
			.current_mut()
			.node()
			.expect("can't be empty after insert")
	}
}

impl<'r, TP, O> WalkMut<'r, TP, O, WalkedDirection, ()>
where
	O: OwnedTreeMarker<'r, TP, WalkedDirection>,
	TP: TreeProperties + 'r,
{
	/// Convert into iterator traversing depth-first pre-order
	pub fn into_iter_pre_order(self) -> IterMutPreOrder<'r, TP, O> {
		IterMutPreOrder { walk: self }
	}

	/// Tree traversal: depth-first pre-order
	pub fn next_pre_order(&mut self) -> Option<&mut Node<TP>> {
		match self.current_mut() {
			NodeOrTree::Tree(_) => {
				self.down_root();
			},
			NodeOrTree::Node(node) => {
				if node.is_leaf() {
					loop {
						match self.up()? {
							WalkedDirection::Down => {
								return None; // back up at tree
							},
							WalkedDirection::Left => {
								self.down_right();
								break;
							},
							WalkedDirection::Right => (), // continue further up
						}
					}
				} else {
					self.down_left();
				}
			},
		}
		return self.current_mut().node();
	}

	/// Convert into iterator traversing depth-first in-order
	pub fn into_iter_in_order(self) -> IterMutInOrder<'r, TP, O> {
		IterMutInOrder { walk: self }
	}

	/// Tree traversal: depth-first in-order
	pub fn next_in_order(&mut self) -> Option<&mut Node<TP>> {
		match self.current_mut() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(node) => {
				if node.is_leaf() {
					loop {
						match self.up()? {
							WalkedDirection::Down => {
								return None; // back up at tree
							},
							WalkedDirection::Left => {
								break;
							},
							WalkedDirection::Right => (), // continue further up
						}
					}
				} else {
					self.down_right();
					while self.down_left() {}
				}
			},
		}
		return self.current_mut().node();
	}

	/// Convert into iterator traversing depth-first post-order
	pub fn into_iter_post_order(self) -> IterMutPostOrder<'r, TP, O> {
		IterMutPostOrder { walk: self }
	}

	/// Tree traversal: depth-first post-order
	pub fn next_post_order(&mut self) -> Option<&mut Node<TP>> {
		match self.current_mut() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(_) => {
				match self.up()? {
					WalkedDirection::Down => {
						return None; // back up at tree
					},
					WalkedDirection::Left => {
						self.down_right();
						while self.down_left() {}
					},
					WalkedDirection::Right => (),
				}
			},
		}
		return self.current_mut().node();
	}

	/// Convert into iterator over all leafs
	pub fn into_iter_leafs(self) -> IterMutLeaf<'r, TP, O> {
		IterMutLeaf { walk: self }
	}

	/// Convert into iterator over all leafs and uncovered parts
	pub fn into_iter_full_leafs(self) -> IterMutLeafFull<'r, TP, O> {
		IterMutLeafFull::new(self)
	}

	/// Tree traversal: depth-first in-order leaf nodes only
	pub fn next_leaf(&mut self) -> Option<&mut Node<TP>> {
		match self.current_mut() {
			NodeOrTree::Tree(_) => {
				self.down_root();
				while self.down_left() {}
			},
			NodeOrTree::Node(_) => {
				loop {
					match self.up()? {
						WalkedDirection::Down => {
							return None; // back up at tree
						},
						WalkedDirection::Left => {
							self.down_right();
							while self.down_left() {}
							break;
						},
						WalkedDirection::Right => (), // continue further up
					}
				}
			},
		}
		return self.current_mut().node();
	}
}

/// Iterate over all nodes that are a prefix of target key in a [`WalkMut`] stack
pub struct WalkMutPath<'r, 'w, TP, O, D = ()>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D>,
{
	start: bool,
	done: bool,
	walk: &'w mut WalkMut<'r, TP, O, D>,
	target: TP::Key,
	target_len: usize,
}

impl<'r, 'w, TP, O, D> WalkMutPath<'r, 'w, TP, O, D>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D>,
	D: From<WalkedDirection>,
{
	/// Next step towards target node
	#[allow(clippy::should_implement_trait)] // iterator doesn't allow using lifetime of itself in item
	pub fn next(&mut self) -> Option<&mut Node<TP>> {
		if self.done {
			return None;
		}
		let lookup = if self.start {
			self.start = false;
			self.walk.lookup_step_initial(&self.target, self.target_len)
		} else {
			self.walk.lookup_step(&self.target, self.target_len)
		};
		match lookup {
			LookupStep::Path => (),
			LookupStep::Found => {
				self.done = true;
			},
			LookupStep::Miss => {
				self.done = true;
				return None;
			},
		}
		Some(self.walk.current_mut().node().expect("should be at node"))
	}
}

impl<'r, 'w, TP, O, D> IntoIterator for WalkMutPath<'r, 'w, TP, O, D>
where
	TP: TreeProperties + 'r,
	O: OwnedTreeMarker<'r, TP, D>,
	D: From<WalkedDirection>,
{
	type IntoIter = IterWalkMutPath<'r, 'w, TP, O, D>;
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn into_iter(self) -> Self::IntoIter {
		IterWalkMutPath::new(self)
	}
}
