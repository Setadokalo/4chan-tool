#![feature(test)]

extern crate test;

use std::{cell::RefCell, fs::File, ops::Deref, rc::Rc, time::Instant};

use bench_debug::log_bench;



use cursive::{Cursive, Vec2, View, menu::{MenuItem, MenuTree}, theme::{BaseColor, Color}, traits::*, view::SizeConstraint, views::{Button, LinearLayout, ResizedView, ScrollView, SelectView, TextView}};


use simplelog::{Config, LevelFilter, CombinedLogger, WriteLogger};
use log::*;

mod data;
mod views;
mod config;
mod net;

use views::{Divider, ImageView, RenderMode, ScaleMode, traits::{Panelable, ResizableWeak}};
use data::{BoardsResponse, Post};



pub struct SettingsAndData {
	// settings
	show_nsfw: bool,
	render_mode: RenderMode,
	scale_mode: ScaleMode,
	// data
	boards: BoardsResponse,
}


fn main() {
	// initialize logger
	CombinedLogger::init(
		vec![
			WriteLogger::new(LevelFilter::Info, Config::default(), File::create("chan_tui.log").unwrap()),
			// Disabled because cursive spams the log to hell with it's inane bullshit
			// #[cfg(debug_assertions)]
			// WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("chan_tui-debug.log").unwrap()),
		]
	).unwrap();

	let mut siv = cursive::default();

	let settings = Rc::new(RefCell::new(SettingsAndData {
		show_nsfw: false,
		render_mode: RenderMode::Color,
		scale_mode: ScaleMode::Linear,
		boards: net::load_4chan_boards(),
	}));

	siv.set_user_data(settings.clone());

	// load user theme TODO: load theme from file instead of hardcoded
	siv.update_theme(|theme| {
		let palette = &mut theme.palette;
		palette.set_color("background", Color::Dark(BaseColor::Black));
		palette.set_color("view", Color::Dark(BaseColor::Black));
		palette.set_color("primary", Color::Light(BaseColor::White));
		palette.set_color("secondary", Color::Light(BaseColor::Blue));
		palette.set_color("tertiary", Color::Light(BaseColor::Red));

		theme.shadow = false;
	});
	let board_view = create_board_view(&mut siv).in_panel();
	siv.add_fullscreen_layer(
		LinearLayout::horizontal()
			.child(board_view)
			.child(LinearLayout::vertical()
				.child(ResizedView::with_full_screen(
					TextView::new("Hello, Other Panel!"),
				))
				.child(Divider::horizontal())
				.child(TextView::new("Panels are interesting!")
					.resized_weak_h(SizeConstraint::AtMost(4)),
				)
				.with_name("threads_list")
				.scrollable()
				.in_panel(),
			)
			.with_name("root_layout"),
	);
	siv.set_autohide_menu(false);
	siv.menubar().add_leaf("Quit", |c| c.quit());
	let mut settings_subtree = MenuTree::new().leaf("Show NSFW Boards", move |c| {
		{
			let mut settings = (*settings).borrow_mut();
			if let MenuItem::Leaf(s, _) = c.menubar().get_subtree(1).unwrap().get_mut(0).unwrap() {
				if settings.show_nsfw { *s = "Show NSFW Boards".to_string(); }
				else { *s = "Hide NSFW Boards".to_string(); }
			} else {
				panic!("unknown menu state");
			}
			settings.show_nsfw = !settings.show_nsfw;
		}
		c.call_on_name(
			"boards_list",
			|b_scrollable: &mut ScrollView<SelectView>| {
				let b = b_scrollable.get_inner_mut();
				b.clear();
				add_boards_to_select(&(*settings).borrow(), b);
			},
		);
	});

	fn set_scale_mode(c: &mut Cursive, s: ScaleMode) {
		c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = s;
	}
	fn set_render_mode(c: &mut Cursive, r: RenderMode) {
		c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().render_mode = r;
	}
	
	settings_subtree.add_subtree(
		"Image Settings",
		MenuTree::new()
		.subtree(
			"Scale Mode", 
			MenuTree::new()
				.leaf("Fast Nearest",     |c| set_scale_mode(c, ScaleMode::FastNearest))
				.leaf("Nearest Neighbor", |c| set_scale_mode(c, ScaleMode::Nearest))
				.leaf("Linear",           |c| set_scale_mode(c, ScaleMode::Linear))
				.leaf("Cubic",            |c| set_scale_mode(c, ScaleMode::Cubic))
				.leaf("Gaussian",         |c| set_scale_mode(c, ScaleMode::Gaussian))
				.leaf("Lanczos",          |c| set_scale_mode(c, ScaleMode::Lanczos))
			).subtree(
				"Render Mode", 
				MenuTree::new()
					.leaf("Color",                |c| set_render_mode(c, RenderMode::Color))
					.leaf("Grayscale",            |c| set_render_mode(c, RenderMode::Grayscale))
					.leaf("X11 (unimplemented!)", |c| set_render_mode(c, RenderMode::Gui))
				)
	);

	siv.menubar().add_subtree("Settings", settings_subtree);
	siv.menubar()
		.add_leaf("Press [ESC] to access the menu", |_| {});
	siv.add_global_callback(cursive::event::Key::Esc, |c| c.select_menubar());
	
	add_board_key_nav_callbacks(&mut siv);
	
	siv.run();
}

fn add_board_key_nav_callbacks(siv: &mut Cursive) {
	// register a global callback listener to listen for input events that aren't handled and check if the board list is focused
	// if it is, we'll go to the next board with a slug that starts with the pressed key

	// Cursive doesn't allow setting a callback on every char automatically, so we have to use this workaround
	// where we iterate through all alphanumeric chars and add a specialized callback for them
	for ch in ('a'..'z').chain('0'..'9') {
		siv.add_global_callback(ch, move |c| {
			// the SelectView will return a callback when it's focus is changed, but it can't be executed immediately
			// because the &mut Cursive (`c`) is in use (for the .call_on_name() it's occuring in).
			// So we store the callback here and call it after the call_on_name() returns.
			let mut cb = None;
			// Cursive also has no way of identifying what element has focus easily, so we use this workaround
			// where we check which of the root layout's children has focus instead
			c.call_on_name("root_layout", |root: &mut LinearLayout| {
				if root.get_focus_index() == root.find_child_from_name("boards_list").unwrap() {
					// if it DOES have focus, finally we can actually run the selection logic
					root.call_on_name(
						"boards_list",
						|scrollable_board_list: &mut ScrollView<SelectView>| {
							scroll_to_matching_board(scrollable_board_list, ch, &mut cb)
						},
					);
				}
			});
			if let Some(cb) = cb {
				cb(c);
			}
		});
	}
}

type EventCallback = cursive::event::Callback;

fn scroll_to_matching_board(scroll_wrap: &mut ScrollView<SelectView>, ch: char, cb: &mut Option<EventCallback>) -> cursive::event::EventResult {
    let bl = scroll_wrap.get_inner_mut();
    let bl_focus = bl.selected_id().unwrap();
    let i = {
		// Starting from the current focus, find the first item that
		//   matches the char.
		// Cycle back to the beginning of the list when we reach the end.
		// This is achieved by chaining the iterator.
		let mut iter = bl.iter()
			.skip(bl_focus + 1)
			.chain(bl.iter())
			.enumerate();

		if let Some((i, _)) =
			iter.find(|&(_, (label, _))| {
				label
					.to_lowercase()
					.trim_start_matches("/")
					.starts_with(ch)
				}) {
			i % bl.len()
		} else {
			bl_focus
		}
	};
    *cb = Some(bl.set_selection(i));
    scroll_wrap.scroll_to_important_area()
}

/// Creates a LinearLayout for
fn create_and_add_thread_panel(op: &Post, board: impl AsRef<str>, render_mode: RenderMode, img_scale_method: ScaleMode) -> LinearLayout {
	let mut thread_panel = LinearLayout::horizontal();
	if let Some(attachment) = &op.attachment {
		if !attachment.filedeleted {
			thread_panel.add_child(
				ImageView::new(
					format!("https://i.4cdn.org/{}/{}s.jpg", board.as_ref(), &attachment.tim), 
					Vec2::new(20, 10), 
					render_mode, 
					img_scale_method
				)
			);
		}
	}
	//TODO: implement thread viewing
	let mut text_pane = LinearLayout::vertical();
	text_pane.add_child(
		Button::new_raw(
			op.op_data.as_ref().unwrap().sub.as_ref()
				.unwrap_or(&"Thread".to_string()), 
			|_| {}
		)
	);
	text_pane.add_child(
		TextView::new(op.com.as_ref().unwrap_or(&"".to_string()))
	);

	thread_panel.add_child(text_pane);
	thread_panel
}

#[log_bench]
fn create_board_view(c: &mut Cursive) -> impl View {
	let mut layout = SelectView::new();

	add_boards_to_select(&get_settings(c).unwrap(), &mut layout);

	layout.set_on_submit(|c, board: &String| {
		let (scale_method, render_mode) = {
			let settings = c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow();
			(settings.scale_mode, settings.render_mode)
		};
		c.call_on_name("threads_list", |threads_view: &mut LinearLayout| {
			//TODO: There's probably a more idiomatic way to clear the LinearLayout
			while threads_view.get_child(0).is_some() { threads_view.remove_child(0); };
			let threads = net::get_threads_for_board(board);
			let mut iter = threads.iter();
			

			if let Some(post) = iter.next() {
				threads_view.add_child(create_and_add_thread_panel(post, board, render_mode, scale_method));
			}

			for post in iter {
				threads_view.add_child(Divider::horizontal());
				// if i > 5 {break} // TODO: Remove this
				threads_view.add_child(create_and_add_thread_panel(post, board, render_mode, scale_method));
			}
		});
	});
	layout.scrollable().with_name("boards_list")
}

pub fn add_boards_to_select(settings: &SettingsAndData, layout: &mut SelectView) {
	for board in settings.boards.boards.iter() {
		if settings.show_nsfw || board.sfw {
			layout.add_item(
				format!("/{}/: {}", board.board, board.title),
				board.board.clone(),
			);
		}
	}
}

fn get_settings(c: &mut Cursive) -> Option<impl Deref<Target = SettingsAndData> + '_> {
	if let Some(settings) = c.user_data::<Rc<RefCell<SettingsAndData>>>() {
		(**settings).try_borrow().ok()
	} else {
		None
	}
}


