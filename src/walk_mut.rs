//! Walk tree structures without call stack

use std::{
	marker::PhantomData,
	ptr::NonNull,
};

/// Allows different node and tree types in [`WalkMut`].
pub enum NodeOrTree<T, N> {
	/// [`WalkMut`] is currently at a node
	Node(N),
	/// [`WalkMut`] is currently at tree
	Tree(T),
}

impl<T, N> NodeOrTree<T, N> {
	/// Map tree value
	pub fn map_tree<F, U>(self, f: F) -> NodeOrTree<U, N>
	where
		F: FnOnce(T) -> U,
	{
		match self {
			Self::Tree(r) => NodeOrTree::Tree(f(r)),
			Self::Node(n) => NodeOrTree::Node(n),
		}
	}

	/// Map node value
	pub fn map_node<F, U>(self, f: F) -> NodeOrTree<T, U>
	where
		F: FnOnce(N) -> U,
	{
		match self {
			Self::Tree(r) => NodeOrTree::Tree(r),
			Self::Node(n) => NodeOrTree::Node(f(n)),
		}
	}

	/// Return node
	pub fn node(self) -> Option<N> {
		match self {
			Self::Tree(_) => None,
			Self::Node(n) => Some(n),
		}
	}
}

impl<N> NodeOrTree<N, N> {
	/// If tree and node type are equivalent extract inner type.
	#[inline]
	pub fn flatten(self) -> N {
		match self {
			Self::Tree(r) => r,
			Self::Node(n) => n,
		}
	}
}

impl<N> NodeOrTree<Option<N>, N> {
	/// If tree and node type are equivalent extract inner type.
	#[inline]
	pub fn flatten_optional(self) -> Option<N> {
		match self {
			Self::Tree(r) => r,
			Self::Node(n) => Some(n),
		}
	}
}

/// Walk tree structures without call stack
///
/// Walking tree structures with mutable references usually
/// requires a recursive call stack to make the borrow-checker
/// happy.
///
/// This uses a stack ([`Vec`]) to keep track of the "current"
/// mutable reference (and hiding the previous ones).
///
/// (There is no way to implement this without `unsafe`, but the
/// abstraction should be safe.)
///
/// Each nested level can also track additional value of type `A`.
pub struct WalkMut<'r, T: ?Sized, N: ?Sized, A = ()> {
	_lifetime: PhantomData<&'r mut T>,
	tree: NonNull<T>,
	stack: Vec<(NonNull<N>, A)>,
}

impl<'r, T: ?Sized, N: ?Sized, A> WalkMut<'r, T, N, A> {
	/// Start a new tree walk at a tree
	pub fn new(tree: &'r mut T) -> Self {
		Self {
			_lifetime: PhantomData,
			tree: tree.into(),
			stack: Vec::new(),
		}
	}

	/// Walk down the tree one step
	///
	/// The step can fail by returning [`Err`].
	pub fn try_walk<F, E>(&mut self, with: F) -> Result<(), E>
	where
		F: for<'n> FnOnce(NodeOrTree<&'n mut T, &'n mut N>) -> Result<(&'n mut N, A), E>,
	{
		match with(self.current_mut()) {
			Err(e) => Err(e),
			Ok((next, add)) => {
				let next: NonNull<N> = next.into();
				self.stack.push((next, add));
				Ok(())
			},
		}
	}

	/// Walk up to the previous level.
	///
	/// Returns the associated data stored with the step,
	/// or [`None`] if already at the initial tree.
	pub fn pop(&mut self) -> Option<A> {
		Some(self.stack.pop()?.1)
	}

	/// Walk up to tree
	pub fn pop_all(&mut self) -> &mut T {
		self.stack.clear();
		unsafe { self.tree.as_mut() }
	}

	/// Get mutable reference to current node or tree
	///
	/// If you need the result to outlive the destruction of the [`WalkMut`] value, see [`into_current_mut`].
	///
	/// [`into_current_mut`]: WalkMut::into_current_mut
	pub fn current_mut(&mut self) -> NodeOrTree<&mut T, &mut N> {
		if let Some((cur, _)) = self.stack.last_mut() {
			NodeOrTree::Node(unsafe { cur.as_mut() })
		} else {
			NodeOrTree::Tree(unsafe { self.tree.as_mut() })
		}
	}

	/// Extract mutable reference to current node or tree
	///
	/// Also see [`current_mut`]
	///
	/// [`current_mut`]: WalkMut::current_mut
	pub fn into_current_mut(mut self) -> NodeOrTree<&'r mut T, &'r mut N> {
		// safety: dropping stack of references means nothing else can create
		// new references to them while 'r still blocks new references to the root.
		if let Some((cur, _)) = self.stack.last_mut() {
			NodeOrTree::Node(unsafe { cur.as_mut() })
		} else {
			NodeOrTree::Tree(unsafe { self.tree.as_mut() })
		}
	}

	/// Extract mutable reference to tree
	pub fn into_tree_mut(mut self) -> &'r mut T {
		self.stack.clear();
		// safety: same as pop_all() + into_current_mut() (which must return the tree)
		unsafe { self.tree.as_mut() }
	}

	/// Get reference to current node or tree
	pub fn current(&self) -> NodeOrTree<&T, &N> {
		if let Some((cur, _)) = self.stack.last() {
			NodeOrTree::Node(unsafe { cur.as_ref() })
		} else {
			NodeOrTree::Tree(unsafe { self.tree.as_ref() })
		}
	}
}
