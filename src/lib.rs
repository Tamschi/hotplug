//! TODO_DOCS_DESCRIPTION
//!
//! [![Zulip Chat](https://img.shields.io/endpoint?label=chat&url=https%3A%2F%2Fiteration-square-automation.schichler.dev%2F.netlify%2Ffunctions%2Fstream_subscribers_shield%3Fstream%3Dproject%252Fhotplug)](https://iteration-square.schichler.dev/#narrow/stream/project.2Fhotplug)

#![doc(html_root_url = "https://docs.rs/hotplug/0.0.1")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(clippy::semicolon_if_nothing_returned)]

use std::{
	marker::PhantomData,
	ptr::null_mut,
	sync::atomic::{AtomicPtr, Ordering},
};

use higher_order_closure::higher_order_closure;
use tap::Pipe;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme {}

pub fn malleable<'a>(_a: (), b: &'a ()) -> &'a () {
	let malleable = higher_order_closure! {
		for<'a> |_a: (), b: &'a ()| -> &'a () {
			b
		}
	};

	{
		fn make_next(
			mut iter: Iter<
				for<'b> fn(
					(),
					&'b (),
					&(dyn Send + Sync + for<'c> Fn((), &'c ()) -> &'c ()),
				) -> &'b (),
			>,
			malleable: for<'b> fn((), &'b ()) -> &'b (),
		) -> impl Send + Sync + for<'a> Fn((), &'a ()) -> &'a () {
			let next = iter.next();
			let iter = iter;
			({
				fn __funnel__<__Closure>(f: __Closure) -> __Closure
				where
					__Closure: for<'a> Fn((), &'a ()) -> &'a (),
				{
					f
				}
				__funnel__::<_>
			})(move |a, b| match &next {
				None => malleable(a, b),
				Some(next) => next(a, b, &make_next(iter.clone(), malleable)),
			})
		}

		let first = make_next(malleable_detours.iter(), malleable);
		first(_a, b)
	}
}

pub static malleable_detours: HotSocket<
	for<'a> fn((), &'a (), &(dyn Send + Sync + for<'b> Fn((), &'b ()) -> &'b ())) -> &'a (),
> = HotSocket::new();

pub struct HotSocket<TDetour: 'static> {
	head: AtomicPtr<HotPlug<TDetour>>,
	_phantom: PhantomData<TDetour>,
}

impl<TDetour> Default for HotSocket<TDetour> {
	fn default() -> Self {
		Self::new()
	}
}

impl<TDetour> HotSocket<TDetour> {
	/// Creates a new instance of [`HotSocket<TDetour>`].
	#[must_use]
	pub const fn new() -> Self {
		Self {
			head: AtomicPtr::new(null_mut()),
			_phantom: PhantomData,
		}
	}

	pub fn add_plug(&self, plug: TDetour) -> &'static HotPlug<TDetour> {
		let new_head = Box::leak(Box::new(HotPlug {
			next: None,
			this: plug,
		}));

		let mut updated = None;
		let updated_mut = &mut updated;
		self.head
			.fetch_update(Ordering::Release, Ordering::Acquire, move |previous| {
				new_head.next = unsafe {
					//SAFETY: This is a permanently leaked instance.
					previous.as_ref()
				};
				*updated_mut = Some(new_head as *mut _);
				*updated_mut
			})
			.expect("unreachable");
		updated
			.expect("unreachable")
			.pipe(|ptr| unsafe { ptr.as_ref() })
			.expect("unreachable")
	}

	#[must_use]
	pub fn iter(&self) -> Iter<TDetour>
	where
		TDetour: Copy,
	{
		Iter {
			next: self
				.head
				.load(Ordering::Acquire)
				.pipe(|ptr| unsafe { ptr.as_ref() }),
		}
	}
}

pub struct HotPlug<TDetour: ?Sized + 'static> {
	next: Option<&'static Self>,
	this: TDetour,
}

trait HotPluggable {
	fn into_hot_plug(self: Box<Self>) -> Box<HotPlug<Self>>;
}
impl<T> HotPluggable for T {
	fn into_hot_plug(self: Box<Self>) -> Box<HotPlug<Self>> {
		Box::new(HotPlug {
			next: None,
			this: *self,
		})
	}
}

#[derive(Clone)]
pub struct Iter<TDetour: 'static> {
	next: Option<&'static HotPlug<TDetour>>,
}
impl<TDetour: Copy + 'static> Iterator for Iter<TDetour> {
	type Item = TDetour;

	fn next(&mut self) -> Option<Self::Item> {
		let next = self.next;
		self.next = next.and_then(|next| next.next);
		next.map(|next| next.this)
	}
}
