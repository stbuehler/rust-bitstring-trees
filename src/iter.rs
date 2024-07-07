//! Iterators over bit string prefixes
#![allow(clippy::bool_comparison)]

use bitstring::BitString;

/// Generate the smallest (ordered) list of prefixes covering first..=last
// could also derive `Copy`, but iterators probably shouldn't be `Copy`?
#[derive(Clone, Debug)]
pub struct IterInclusive<K> {
	// cover all values between `first 0*` and `last 1*`

	// if iterator done: shared_len > first.len(), otherwise:
	// * first[..shared_len] == last[..shared_len]
	// * either:
	//   - shared_len == first.len() == last.len()
	//   - first[shared_len] == 0, no trailing "0"s after that in first and
	//     last[shared_len] == 1, no trailing "1"s after that in last
	first: K,          // no trailing "0"s (from shared_len+1 on)
	last: K,           // no trailing "1"s (from shared_len+1 on)
	shared_len: usize, // if longer than first: iterator done
}

impl<K> IterInclusive<K>
where
	K: BitString + Clone,
{
	fn empty() -> Self {
		Self {
			first: K::null(),
			last: K::null(),
			shared_len: 1,
		}
	}

	fn all() -> Self {
		Self {
			first: K::null(),
			last: K::null(),
			shared_len: 0,
		}
	}
}

impl<K> Default for IterInclusive<K>
where
	K: BitString + Clone,
{
	fn default() -> Self {
		Self::empty()
	}
}

impl<K> Iterator for IterInclusive<K>
where
	K: BitString + Clone,
{
	type Item = K;

	fn next(&mut self) -> Option<Self::Item> {
		let first_len = self.first.len();
		if self.shared_len > first_len {
			return None;
		}

		// without shared prefix (of length shared_len) we should have one
		// of these scenarios (first, last):
		// 1. (""(0*), ""(1*)) -> yield final ""
		// 2. ("0.*01*"(0*), "1.*"(1*)) -> yield first, -> "increment" first to "0.*1" (don't care about last)
		// 3. ("01*"(0*), "10*"(1*)) -> yield first, -> set first = last
		// 4. ("01*"(0*), "10*|1.*"(1*)) -> yield first, -> set first to "10*|0" from last (flipped bit after "|")

		if first_len == self.shared_len {
			let last_len = self.last.len();
			if last_len == self.shared_len {
				// scenario 1: yield final shared prefix
				// mark as done
				self.shared_len = !0;
				self.first.clip(0);
				return Some(self.last.clone());
			} else {
				debug_assert!(
					last_len == self.shared_len,
					"first was shared prefix, but last was longer"
				);
				return None; // invalid state
			}
		}
		// scenario 2-4
		let result = self.first.clone();
		// increment first; drop all trailing "1"s, then flip trailing "0" to "1"
		for pos in (self.shared_len + 1..first_len).rev() {
			if false == self.first.get(pos) {
				// scenario 2
				self.first.clip(pos + 1); // drop trailing "1"s
				self.first.flip(pos); // flip trailing "0" to "1"
				return Some(result);
			}
		}

		// scenario 3-4
		if true == self.first.get(self.shared_len) {
			debug_assert!(
				!self.first.get(self.shared_len),
				"first should have a '0' after shared prefix"
			);
			return None; // invalid state
		}
		if false == self.last.get(self.shared_len) {
			debug_assert!(
				self.last.get(self.shared_len),
				"last should have a '1' after shared prefix"
			);
			return None; // invalid state
		}

		// copy first "1" and then as many "0"s as possible; flip next 1 if present, otherwise cut
		self.first = self.last.clone();
		let check_from = self.shared_len + 1; // skip leading "1"
		self.shared_len = self.last.len(); // in case we don't find another "1" - take all (scenario 3)
		for pos in check_from..self.shared_len {
			if self.first.get(pos) {
				// scenario 4
				self.first.clip(pos + 1);
				self.first.flip(pos);
				self.shared_len = pos;
				break;
			}
		}

		Some(result)
	}
}

/// Generate the smallest (ordered) list of prefixes covering first..=last
///
/// Generate smallest ordered list of prefixes to cover all
/// values `v` with `start 0*` <= `end 1*`.
///
/// E.g. for IP addresses this results in the smallest list of CIDR
/// blocks exactly covering a range.
pub fn iter_inclusive<K>(mut first: K, mut last: K) -> IterInclusive<K>
where
	K: BitString + Clone,
{
	// trailing "0"s in `first` and trailing "1"s in `last` are semantically
	// not important; but establish certain invariants during iteration.
	// Also see struct notes.

	// clip trailing "0"s from first
	let mut first_len = first.len();
	while first_len > 0 && false == first.get(first_len - 1) {
		first_len -= 1;
	}
	first.clip(first_len);

	// clip trailing "1"s from last
	let mut last_len = last.len();
	while last_len > 0 && true == last.get(last_len - 1) {
		last_len -= 1;
	}
	last.clip(last_len);

	let mut shared_len = first.shared_prefix_len(&last);

	if shared_len == first_len {
		// first is a prefix of last; include further "0"s from last into shared prefix
		while shared_len < last_len && false == last.get(shared_len) {
			shared_len += 1;
		}
		// copy "0"s to first
		first = last.clone();

		if shared_len == last_len {
			// first == last, yield once
			first.clip(shared_len);
		} else {
			// last continues with a "1...", make sure first continues with a "0"
			first.clip(shared_len + 1);
			first.flip(shared_len);
		}
	} else if shared_len == last_len {
		// last is a prefix of first; include further "1"s from first into shared prefix
		while shared_len < first_len && true == first.get(shared_len) {
			shared_len += 1;
		}
		// copy "1"s to last
		last = first.clone();

		if shared_len == first_len {
			// last == first, yield once
			last.clip(shared_len);
		} else {
			// first continues with a "0...", make sure last continues with a "1"
			last.clip(shared_len + 1);
			last.flip(shared_len);
		}
	} else if first.get(shared_len) > last.get(shared_len) {
		// wrong order: yield nothing
		return IterInclusive::empty();
	}
	IterInclusive {
		first,
		last,
		shared_len,
	}
}

/// Generate smallest set of prefixes covering values between other prefixes
///
/// See [`iter_uncovered_prefixes`].
#[derive(Clone, Debug)]
pub struct IterBetween<K> {
	range: IterInclusive<K>,
}

impl<K> Default for IterBetween<K>
where
	K: BitString + Clone,
{
	fn default() -> Self {
		Self {
			range: IterInclusive::empty(),
		}
	}
}

impl<K> Iterator for IterBetween<K>
where
	K: BitString + Clone,
{
	type Item = K;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.range.next()
	}
}

fn increment<K>(key: &mut K) -> bool
where
	K: BitString + Clone,
{
	// clip trailing "1"s, flip (then) trailing "0" to "1"
	for pos in (0..key.len()).rev() {
		if false == key.get(pos) {
			key.clip(pos + 1);
			key.flip(pos);
			return true;
		}
	}
	// only found "1"s (possibly empty key)
	false
}

fn decrement<K>(key: &mut K) -> bool
where
	K: BitString + Clone,
{
	// clip trailing "0"s, flip (then) trailing "1" to "0"
	for pos in (0..key.len()).rev() {
		if true == key.get(pos) {
			key.clip(pos + 1);
			key.flip(pos);
			return true;
		}
	}
	// only found "0"s (possibly empty key)
	false
}

/// Generate smallest set of prefixes covering values between `start` and `end`
///
/// Pass `None` to cover all values before or after a prefix, or simply all values.
pub fn iter_between<K>(mut after: Option<K>, mut before: Option<K>) -> IterBetween<K>
where
	K: BitString + Clone,
{
	if let Some(start) = after.as_mut() {
		if !increment(start) {
			return IterBetween {
				range: IterInclusive::empty(),
			};
		}
	}
	if let Some(end) = before.as_mut() {
		if !decrement(end) {
			return IterBetween {
				range: IterInclusive::empty(),
			};
		}
	}

	let range = match (after, before) {
		(Some(first), Some(last)) => iter_inclusive(first, last),
		(Some(first), None) => {
			let mut last = first.clone();
			// find first "0" and flip to "1" (and clip). no "0" -> only "1"s, yield first == last
			for pos in 0..last.len() {
				if false == last.get(pos) {
					last.clip(pos + 1);
					last.flip(pos);
					break;
				}
			}
			iter_inclusive(first, last)
		},
		(None, Some(last)) => {
			let mut first = last.clone();
			// find first "1" and flip to "0" (and clip). no "1" -> only "0"s, yield first == last
			for pos in 0..first.len() {
				if true == first.get(pos) {
					first.clip(pos + 1);
					first.flip(pos);
					break;
				}
			}
			iter_inclusive(first, last)
		},
		(None, None) => IterInclusive::all(),
	};
	IterBetween { range }
}

#[cfg(test)]
mod tests {
	use super::iter_inclusive;
	use bitstring::BitLengthString;
	use std::net::{
		Ipv4Addr,
		Ipv6Addr,
	};

	type Ipv4Cidr = BitLengthString<Ipv4Addr>;
	type Ipv6Cidr = BitLengthString<Ipv6Addr>;

	fn c4(a: &str, net: usize) -> Ipv4Cidr {
		Ipv4Cidr::new(a.parse().unwrap(), net)
	}

	fn c6(a: &str, net: usize) -> Ipv6Cidr {
		Ipv6Cidr::new(a.parse().unwrap(), net)
	}

	#[test]
	fn testv4_1() {
		assert_eq!(
			iter_inclusive(c4("192.168.0.6", 32), c4("192.168.0.6", 32)).collect::<Vec<_>>(),
			vec![c4("192.168.0.6", 32),]
		);
	}

	#[test]
	fn testv6_1() {
		assert_eq!(
			iter_inclusive(c6("::f0:4", 128), c6("::f0:10", 128)).collect::<Vec<_>>(),
			vec![c6("::f0:4", 126), c6("::f0:8", 125), c6("::f0:10", 128),]
		);
	}
}
