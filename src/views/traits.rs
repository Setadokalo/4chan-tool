use cursive::{traits::{Resizable, View}, view::SizeConstraint, views::{Panel, ResizedView}};


pub trait ResizableWeak: Resizable {
	/// returns the self in a double resized view wrapper, which forces the `self` to request space (but not force it)
	/// up until it's SizeConstraint limit.
	fn resized_weak(
		self,
		width: SizeConstraint,
		height: SizeConstraint,
	) -> ResizedView<ResizedView<Self>> {
		self
			.resized(SizeConstraint::Full, SizeConstraint::Full)
			.resized(width, height)
	}
	/// same as `resized_weak`, but automatically uses `SizeConstraint::Free` for the height
	fn resized_weak_w(self, width: SizeConstraint) -> ResizedView<ResizedView<Self>> {
		self
			.resized(SizeConstraint::Full, SizeConstraint::Full)
			.resized(width, SizeConstraint::Free)
	}
	/// same as `resized_weak`, but automatically uses `SizeConstraint::Free` for the width
	fn resized_weak_h(self, height: SizeConstraint) -> ResizedView<ResizedView<Self>> {
		self
			.resized(SizeConstraint::Full, SizeConstraint::Full)
			.resized(SizeConstraint::Free, height)
	}
}

impl<T> ResizableWeak for T where T: Resizable {}

pub trait Panelable: View {
	// returns the self in a double resized view wrapper, which forces the `self` to request space (but not force it)
	// up until it's SizeConstraint limit.
	fn in_panel(self) -> Panel<Self>
	where
		Self: Sized,
	{
		Panel::new(self)
	}
}

impl<T> Panelable for T where T: View {}
