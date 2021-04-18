#![feature(test)]

extern crate test;

use std::{cell::RefCell, fs::File, ops::Deref, rc::Rc, sync::{Arc, Mutex, Once}, thread, time::{Duration, Instant}};

use bench_debug::log_bench;
use serde::{Deserialize, Serialize};


use chan_data::{BoardsResponse, Post, Thread};
use chrono::Utc;
use config::ThreadConfig;
use cursive::{Cursive, Vec2, View, menu::{MenuItem, MenuTree}, theme::{BaseColor, Color}, traits::*, view::SizeConstraint, views::{Button, LinearLayout, ResizedView, ScrollView, SelectView, TextView}};


mod chan_data;
mod views;

use simplelog::{Config, LevelFilter, WriteLogger};
use log::*;

use views::{Divider, ImageView, RenderMode, ScaleMode, traits::{Panelable, ResizableWeak}};

mod config {
	use std::fmt::Debug;

	use chrono::{DateTime, NaiveDateTime, Utc};
	use serde::{Deserialize, Serialize};
	#[derive(Debug, Serialize, Deserialize)]
	pub struct ThreadConfig {
		pub board: String,
		pub id: String,
		pub name: String,
		#[serde(skip, default = "get_unix_epoch")]
		pub last_modified: DateTime<Utc>,
	}

	pub fn get_unix_epoch() -> DateTime<Utc> {
		DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc)
	}
	#[cfg(test)] // temporary to make the unused warning go away
	pub fn load_config(raw_config: String) -> Vec<ThreadConfig> {
		let mut new_config = Vec::new();
		for (i, line) in raw_config
			.split("\n")
			.enumerate()
			.filter(|(_, s)| !s.is_empty())
		{
			let mut config_iter = line.trim().splitn(2, |c: char| c.is_whitespace());
			if let Some(thread_config) = config_iter.next() {
				let target: Vec<&str> = thread_config.split("/").collect();
				if target.len() != 3 {
					log::info!(
						"Unrecognized board link structure at line {}: {}",
						i, thread_config
					);
					continue;
				}
				// the rest of the string is in the second iterator value (or there isn't one, in which case we default to the thread config string)
				let name = config_iter
					.next()
					.unwrap_or(thread_config)
					.trim()
					.to_string();

				new_config.push(ThreadConfig {
					board: target[1].to_string(),
					id: target[2].to_string(),
					name,
					// the unix epoch
					last_modified: get_unix_epoch(),
				});
			}
		}
		new_config
	}
}


static mut CLIENT: Option<reqwest::blocking::Client> = None;
static CLIENT_ONCE: Once = Once::new();

fn get_client() -> &'static reqwest::blocking::Client {
	unsafe {
		CLIENT_ONCE.call_once(|| CLIENT = Some(reqwest::blocking::Client::new()));
		CLIENT.as_ref().unwrap()
	}
}


#[log_bench]
fn get_threads_for_board(board: impl Into<String>) -> Vec<Post> {
	let s = board.into();
	#[derive(Serialize, Deserialize)]

	struct CatalogPage {
		page: usize,
		threads: Vec<Post>,
	}
	let now = Instant::now();
	let req = get_client()
		.get(format!("https://a.4cdn.org/{}/catalog.json", s.clone()))
		.build()
		.expect("Failed to build threads list request");
	let resp = get_client()
		.execute(req)
		.expect("Error requesting threads list");

	info!("Took {:.4} seconds to get /{}/ catalog", now.elapsed().as_secs_f64(), s);

	resp
		.json::<Vec<CatalogPage>>()
		.expect("Failed to parse threads list")
		.remove(0).threads
}

#[allow(dead_code)]
fn watch_threads(thread_list: Arc<Mutex<Vec<ThreadConfig>>>) -> ! {
	info!("Watch daemon started with these threads:");
	for config in (*thread_list).lock().unwrap().iter() {
		info!(
			"Thread \"{}\" in board \"{}\" with name \"{}\"",
			config.id, config.board, config.name
		);
	}
	loop {
		for mut thread_cfg in (*thread_list).lock().unwrap().iter_mut() {
			let req = get_client()
				.get(format!(
					"https://a.4cdn.org/{}/thread/{}.json",
					thread_cfg.board, thread_cfg.id
				))
				.header(
					"If-Modified-Since",
					thread_cfg
						.last_modified
						.to_rfc2822()
						.replace("+0000", "GMT"),
				)
				.build()
				.expect("Failed to build request");
			let resp = get_client().execute(req).expect("Error requesting page");
			if let Ok(thread) = resp.json::<Thread>() {
				for post in thread.posts.iter() {
					info!("{:#?}", post);
				}
			} else {
				info!("Failed to parse response - assuming empty response body")
			}
			thread_cfg.last_modified = Utc::now();
			// avoid spamming the API
			thread::sleep(Duration::from_secs(1));
		}
		thread::sleep(Duration::from_secs(20));
	}
}


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
	WriteLogger::init(LevelFilter::Info, Config::default(), File::create("chantui.log").unwrap()).unwrap();

	let mut siv = cursive::default();

	let settings = Rc::new(RefCell::new(SettingsAndData {
		show_nsfw: false,
		render_mode: RenderMode::Color,
		scale_mode: ScaleMode::Linear,
		boards: load_4chan_boards(),
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
	siv.menubar().add_subtree(
		"Settings",
		MenuTree::new().leaf("Show NSFW Boards", move |c| {
			{
				let mut settings = (*settings).borrow_mut();
				if let MenuItem::Leaf(s, _) = c.menubar().get_subtree(1).unwrap().get_mut(0).unwrap() {
					if settings.show_nsfw {
						*s = "Show NSFW Boards".to_string();
					} else {
						*s = "Hide NSFW Boards".to_string();
					}
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
		}).subtree(
			"Image Settings",
			MenuTree::new()
			.subtree(
				"Scale Mode", 
				MenuTree::new()
					.leaf("Fast Nearest Neighbor", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::FastNearest;
					})
					.leaf("Nearest Neighbor", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::Nearest;
					})
					.leaf("Linear", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::Linear;
					})
					.leaf("Cubic", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::Cubic;
					})
					.leaf("Gaussian", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::Gaussian;
					})
					.leaf("Lanczos", |c| {
						c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = ScaleMode::Lanczos;
					})
				).subtree(
					"Render Mode", 
					MenuTree::new()
						.leaf("Color", |c| {
							c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().render_mode = RenderMode::Color;
						})
						.leaf("Grayscale", |c| {
							c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().render_mode = RenderMode::Grayscale;
						})
						.leaf("X11 (unimplemented!)", |c| {
							c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().render_mode = RenderMode::Gui;
						})
					)
		)
	);
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

fn scroll_to_matching_board(scrollable_board_list: &mut ScrollView<SelectView>, ch: char, cb: &mut Option<cursive::event::Callback>) -> cursive::event::EventResult {
    let bl = scrollable_board_list.get_inner_mut();
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
    scrollable_board_list.scroll_to_important_area()
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
	thread_panel.add_child(
		Button::new_raw(
			op.op_data.as_ref().unwrap().sub.as_ref()
				.unwrap_or(&"Thread".to_string()), 
			|_| {}
		)
	);
	thread_panel
}

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
			let threads = get_threads_for_board(board);
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

fn load_4chan_boards() -> BoardsResponse {
	let now = Instant::now();
	let req = get_client()
		.get("https://a.4cdn.org/boards.json")
		.build()
		.expect("Failed to build boards list request");
	let resp = get_client()
		.execute(req)
		.expect("Error requesting boards list");
	info!("Took {:.4} seconds to get 4chan boards", now.elapsed().as_secs_f64());

	resp
		.json::<BoardsResponse>()
		.expect("Failed to parse boards list")
}

fn get_settings(c: &mut Cursive) -> Option<impl Deref<Target = SettingsAndData> + '_> {
	if let Some(settings) = c.user_data::<Rc<RefCell<SettingsAndData>>>() {
		(**settings).try_borrow().ok()
	} else {
		None
	}
}


