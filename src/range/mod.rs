//! functions operating on prefix ranges
use bitstring::BitString;

/// Iterator type for [`range_to_prefixes`](#method.range_to_prefixes).
pub struct RangeToPrefixes<S> {
	start: S,
	end: S,
	shared_len: usize,
}

impl<S: BitString+Clone> Iterator for RangeToPrefixes<S> {
	type Item = S;

	fn next(&mut self) -> Option<Self::Item> {
		if self.shared_len > self.end.len() { return None; }

		// => without shared prefix `start` and `end` now look like this:
		// - ("0.*1", "1.*")
		// - ("", ".+")      (`end` is actually either "1" or ".*0")
		// - ("", "")

		// first reduce `start` to an empty part:
		let mut start_len = self.start.len();
		if start_len > self.shared_len {
			// starts with a 0 and ends with 1
			debug_assert!(start_len > self.shared_len + 1);
			debug_assert_eq!(false, self.start.get(self.shared_len));
			debug_assert_eq!(true, self.start.get(start_len-1));

			// current `start` is one of the prefixes we want
			let result = Some(self.start.clone());

			// now "increment by one", i.e. drop all trailing `1`s, then
			// flip the last bit from a `0` to `1`. we know the first
			// bit in `start` (after the shared prefix) is a `0`, so
			// this won't touch the shared prefix.
			start_len -= 1; // we know the last bit was a `1`, always drop it
			debug_assert_eq!(true, self.start.get(start_len));
			self.start.flip(start_len);
			while self.start.get(start_len-1) {
				start_len -= 1;
			}
			debug_assert!(start_len >= self.shared_len);
			self.start.clip(start_len);
			self.start.flip(start_len-1);
			debug_assert_eq!(true, self.start.get(start_len-1));

			if start_len == self.shared_len + 1 {
				debug_assert_eq!(true, self.end.get(self.shared_len));
				// now `start` is just a single "1". `end` also starts with "1":
				self.shared_len += 1;
				debug_assert_eq!(start_len, self.shared_len);
			}

			return result;
		}
		debug_assert_eq!(start_len, self.shared_len);

		let end_len = self.end.len();
		// now reduce `end` to an empty part by increasing the length of the
		// shared part (in other words: `start`).
		if self.shared_len < end_len {
			while self.shared_len < end_len {
				if true == self.end.get(self.shared_len) {
					// need to add "SHARED_PART | 0" to the list:
					let mut item = self.end.clone();
					item.flip(self.shared_len);
					item.clip(self.shared_len + 1);
					self.shared_len += 1;
					return Some(item);
				} else {
					self.shared_len += 1;
				}
				// no need to update start anymore, but it would look like:
				// self.start = self.end.clone(); self.start.clip(self.shared_len);
			}
		}

		// now all that remains is a shared prefix. also needed in the list:
		let result = Some(self.end.clone());

		// now prevent further items:
		self.end.clip(0);
		self.shared_len = !0;

		result
	}
}

/// Convert a range given by a `start` and a `end` prefix into the
/// shortest list of prefixes covering all bit strings `s` with `(start
/// | 0*) <= s <= (end | 1*)` (`0` and `1` representing `false` and
/// `true`).
///
/// If `start` and `end` are already at "full length", these are all bit
/// strings `s` with `start <= s <= end`.
///
/// E.g. for IP addresses this results in the smalles list of CIDR
/// blocks covering a range.
pub fn range_to_prefixes<S: BitString+Clone>(mut start: S, mut end: S) -> RangeToPrefixes<S> {

	let mut start_len = start.len();
	let mut end_len = end.len();
	let mut shared_len = start.shared_prefix_len(&end);

	if shared_len < start_len && shared_len < end_len && end.get(shared_len) < start.get(shared_len) {
		// first bit after shared_len bits is `0` in end and `1` in start - empty range.
		start.clip(0);
		end.clip(0);
		return RangeToPrefixes{
			start: start,
			end: end,
			shared_len: !0,
		}
	}
	// otherwise range is never empty

	// according to the second definition we can trim/append trailing
	// `0`s of/to `start` and `s` of/to `end`:

	// trim '0' bits from end of `start`, but don't short beyond shared_len
	while start_len > shared_len && false == start.get(start_len - 1) {
		start_len -= 1;
	}
	start.clip(start_len);

	// trim '1' bits from end of `end`, but don't short beyond shared_len
	while end_len > shared_len && true == end.get(end_len - 1) {
		end_len -= 1;
	}
	end.clip(end_len);

	// trimming didn't change `shared_len`!

	// if end is just the shared prefix, and start is longer, take the
	// following `1*0?` bits from start and append them to end as `1`s.
	// we might end up appending a `1` we just trimmed, but that's ok.
	//
	// appending `1`s to `end` again is no semantic change.
	if end_len == shared_len && start_len > shared_len {
		while end_len < start_len && true == start.get(end_len) {
			end_len += 1;
			// copying a `1` also increased the shared prefix length
			shared_len += 1;
		}
		// copy bits from start by cloning and trimming. end was a
		// prefix of start, we don't drop any bits!
		if end_len < start_len {
			// if the loop was aborted due to a `0` bit take that too, but
			// make it a `1`.
			end = start.clone();
			// "transform-appending" `0` to `1` does NOT increase the shared prefix length
			end.flip(end_len);
			end_len += 1;
			end.clip(end_len);
		} else {
			end = start.clone();
			end.clip(end_len);
		}
	}

	// Obvervations without shared prefix:
	//
	// `start` cannot start with a `1`: it would have been appened to
	// `end` too.
	//
	// If `start` is not empty it must therefor start with a `0` and
	// must end with a `1`, because we trimmed all `0`s, and `end` must
	// start with a `1`, because if it was empty, we would have appended
	// a `1`, and it can't be `0`, because the shared prefix would be
	// longer otherwise.
	//
	// If `start` is empty, `end` can be almost anything: but it can
	// only end in a `1` if it is just a `1` (otherwise it would have
	// been trimmed).
	//
	// => without shared prefix `start` and `end` now look like this:
	// - ("0.*1", "1.*")
	// - ("", ".+")      (`end` is actually either "1" or ".*0")
	// - ("", "")
	//
	// first reduce `start` to an empty part, then `end`

	RangeToPrefixes{
		start: start,
		end: end,
		shared_len: shared_len,
	}
}

#[cfg(test)]
mod tests {
	use super::range_to_prefixes;
	use std::net::{Ipv4Addr,Ipv6Addr};
	use bitstring::{BitLengthString};

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
		for p in range_to_prefixes(c4("192.168.0.6", 32), c4("192.168.0.6", 32)) {
			println!("{:?}", p);
		}
	}

	#[test]
	fn testv6_1() {
		for p in range_to_prefixes(c6("::f0:4", 128), c6("::f0:10", 128)) {
			println!("{:?}", p);
		}
	}
}
