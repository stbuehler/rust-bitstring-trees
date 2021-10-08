//! provide trees based on bitstrings
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/bitstring-trees/0.1.0")]

extern crate bitstring;

pub mod map;
pub mod set;

// sometimes one wants to destruct and re-construct a value, but only
// has a mutable reference.
//
// if re-constructing the value panics we end up with a really fucked up
// memory state - we need to kill the process.
//
// use AssertUnwindSafe quite heavily internally - we abort anyway if
// something panics.
fn replace_at<T, F>(location: &mut T, with: F)
where
	T: Sized,
	F: FnOnce(T) -> T,
{
	use std::{
		panic::*,
		process,
	};

	let with = AssertUnwindSafe(with);

	let old = AssertUnwindSafe(unsafe { std::ptr::read(location) });
	let new = catch_unwind(move || AssertUnwindSafe(with.0(old.0))).unwrap_or_else(move |_e| {
		// we're screwed, give up
		process::abort();
	});
	unsafe { std::ptr::write(location, new.0) }
}

// similar to replace_at, but allow for a second chance through
// `fallback` to construct a value to restore the memory state to
// something sane - then we can continue unwinding the stack.
//
// use AssertUnwindSafe quite heavily internally - pulling UnwindSafe
// trait on all generics is quite annoying. so this is actually
// "unsafe".
fn replace_at_and_fallback<T, F, G>(location: &mut T, with: F, fallback: G)
where
	T: Sized,
	F: FnOnce(T) -> T,
	G: FnOnce() -> T,
{
	use std::{
		panic::*,
		process,
	};

	let with = AssertUnwindSafe(with);
	let fallback = AssertUnwindSafe(fallback);

	let old = AssertUnwindSafe(unsafe { std::ptr::read(location) });
	let (new, panic_err) = catch_unwind(move || (AssertUnwindSafe(with.0(old.0)), None))
		.unwrap_or_else(move |e| {
			// remember panic so we can resume unwinding it
			// now give `fallback` a second chance to create a value
			let e = AssertUnwindSafe(e);
			catch_unwind(move || (AssertUnwindSafe(fallback.0()), Some(e.0))).unwrap_or_else(
				move |_e| {
					// if fallback panics too, give up
					process::abort();
				},
			)
		});
	unsafe { std::ptr::write(location, new.0) }
	if let Some(panic_err) = panic_err {
		resume_unwind(panic_err);
	}
}
