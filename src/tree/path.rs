use bitstring::BitString as _;

use super::{
	goto::{
		LookupStepWith,
		NodeRef as _,
	},
	IterMutPath,
	Node,
	TreeProperties,
};

/// Iterate over all nodes that are a prefix of target key
pub struct MutPath<'r, TP: TreeProperties> {
	start: bool,
	current: Option<&'r mut Node<TP>>,
	target: TP::Key,
	target_len: usize,
}

impl<'r, TP: TreeProperties> MutPath<'r, TP> {
	pub(in crate::tree) fn new(root: Option<&'r mut Node<TP>>, key: TP::Key) -> Self {
		Self {
			start: true,
			current: root,
			target_len: key.len(),
			target: key,
		}
	}

	/// Next step towards target node
	#[allow(clippy::should_implement_trait)] // iterator doesn't allow using lifetime of itself in item
	pub fn next(&mut self) -> Option<&mut Node<TP>> {
		let lookup_step = if self.start {
			self.start = false;
			self.current
				.take()?
				.lookup_initial_step(&self.target, self.target_len)
		} else {
			self.current
				.take()?
				.lookup_step(&self.target, self.target_len)
		};

		match lookup_step {
			LookupStepWith::Found(node, _) => Some(node),
			LookupStepWith::Path(node, _) => {
				self.current = Some(node);
				Some(self.current.as_mut()?)
			},
			LookupStepWith::Miss => None,
		}
	}
}

impl<'r, TP: TreeProperties> IntoIterator for MutPath<'r, TP> {
	type IntoIter = IterMutPath<'r, TP>;
	type Item = (
		&'r TP::Key,
		&'r mut TP::Value,
		Option<&'r mut TP::LeafValue>,
	);

	fn into_iter(self) -> Self::IntoIter {
		IterMutPath::new(self)
	}
}

/// Iterate over all nodes that are a prefix of target key
pub struct IterPath<'r, TP: TreeProperties> {
	start: bool,
	current: Option<&'r Node<TP>>,
	target: TP::Key,
	target_len: usize,
}

impl<'r, TP: TreeProperties> IterPath<'r, TP> {
	pub(in crate::tree) fn new(node: Option<&'r Node<TP>>, key: TP::Key) -> Self {
		Self {
			start: true,
			current: node,
			target_len: key.len(),
			target: key,
		}
	}
}

impl<'r, TP: TreeProperties> Iterator for IterPath<'r, TP> {
	type Item = &'r Node<TP>;

	fn next(&mut self) -> Option<&'r Node<TP>> {
		let current = self.current.take()?;
		let lookup_step = if self.start {
			self.start = false;
			current.lookup_initial_step(&self.target, self.target_len)
		} else {
			current.lookup_step(&self.target, self.target_len)
		};

		match lookup_step {
			LookupStepWith::Found(node, _) => Some(node),
			LookupStepWith::Path(node, _) => {
				self.current = Some(node);
				Some(node)
			},
			LookupStepWith::Miss => None,
		}
	}
}
