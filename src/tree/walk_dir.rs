/// Remember which path was taken to reach this node from the parent in [`WalkMutOwned`] and [`WalkMutBorrowed`].
///
/// [`WalkMutOwned`]: super::WalkMutOwned
/// [`WalkMutBorrowed`]: super::WalkMutBorrowed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkedDirection {
	/// Entered first node in tree
	Down,
	/// Entered left node
	Left,
	/// Entered right node
	Right,
}

impl WalkedDirection {
	/// [`Self::Right`] if `side` is true otherwise [`Self::Left`]
	pub fn from_side(side: bool) -> Self {
		if side {
			Self::Right
		} else {
			Self::Left
		}
	}
}

impl From<WalkedDirection> for () {
	fn from(_: WalkedDirection) -> Self {}
}
