use std::{cell::RefCell, ops::Deref, rc::Rc, sync::{Arc, Mutex, Once}, thread, time::Duration};

use image::imageops::FilterType;
use serde::{Deserialize, Serialize};


use chan_data::{BoardsResponse, Post, Thread};
use chrono::Utc;
use config::ThreadConfig;
use cursive::{Cursive, Vec2, View, menu::{MenuItem, MenuTree}, theme::{BaseColor, Color}, traits::*, view::SizeConstraint, views::{Button, LinearLayout, ResizedView, ScrollView, SelectView, TextView}};


mod chan_data;
mod views;

use views::{Divider, ImageView, traits::{Panelable, ResizableWeak}};

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
					println!(
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
fn get_threads_for_board(board: impl Into<String>) -> Vec<Post> {
	#[derive(Serialize, Deserialize)]

	struct CatalogPage {
		page: usize,
		threads: Vec<Post>,
	}
	let req = get_client()
		.get(format!("https://a.4cdn.org/{}/catalog.json", board.into()))
		.build()
		.expect("Failed to build threads list request");
	let resp = get_client()
		.execute(req)
		.expect("Error requesting threads list");

	resp
		.json::<Vec<CatalogPage>>()
		.expect("Failed to parse threads list")
		.remove(0).threads
}

#[allow(dead_code)]
fn watch_threads(thread_list: Arc<Mutex<Vec<ThreadConfig>>>) -> ! {
	println!("Watch daemon started with these threads:");
	for config in (*thread_list).lock().unwrap().iter() {
		println!(
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
					println!("{:#?}", post);
				}
			} else {
				println!("Failed to parse response - assuming empty response body")
			}
			thread_cfg.last_modified = Utc::now();
			// avoid spamming the API
			thread::sleep(Duration::from_secs(1));
		}
		thread::sleep(Duration::from_secs(20));
	}
}


struct SettingsAndData {
	// settings
	show_nsfw: bool,
	// data
	boards: BoardsResponse,
	scale_mode: FilterType,
}

fn main() {
	let mut siv = cursive::default();

	let settings = Rc::new(RefCell::new(SettingsAndData {
		boards: load_4chan_boards(),
		show_nsfw: false,
		scale_mode: FilterType::Triangle,
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
			"Scale Mode", 
			MenuTree::new()
				.leaf("Nearest Neighbor", |c| {
					c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = FilterType::Nearest;
				})
				.leaf("Linear", |c| {
					c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = FilterType::Triangle;
				})
				.leaf("Cubic", |c| {
					c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = FilterType::CatmullRom;
				})
				.leaf("Gaussian", |c| {
					c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = FilterType::Gaussian;
				})
				.leaf("Lanczos", |c| {
					c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow_mut().scale_mode = FilterType::Lanczos3;
				})
		),
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
							let bl = scrollable_board_list.get_inner_mut();
							// the selected child of the boards list
							let bl_focus = bl.selected_id().unwrap();
							let i = {
								// * Starting from the current focus, find the first item that
								//   match the char.
								// * Cycle back to the beginning of the list when we reach the end.
								// * This is achieved by chaining twice the iterator.
								let iter = bl.iter().chain(bl.iter());

								if let Some((i, _)) =
									iter
										.enumerate()
										.skip(bl_focus + 1)
										.find(|&(_, (label, _))| {
											label
												.to_lowercase()
												.splitn(2, "/")
												.nth(1)
												.unwrap()
												.starts_with(ch)
										}) {
									i % bl.len()
								} else {
									bl_focus
								}
							};
							cb = Some(bl.set_selection(i));
							scrollable_board_list.scroll_to_important_area()
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

/// Creates a LinearLayout for
fn create_and_add_thread_panel(op: &Post, board: impl AsRef<str>, img_scale_method: FilterType) -> LinearLayout {
	let mut thread_panel = LinearLayout::horizontal();
	if let Some(attachment) = &op.attachment {
		if !attachment.filedeleted {
			thread_panel.add_child(ImageView::new(format!("https://i.4cdn.org/{}/{}s.jpg", board.as_ref(), &attachment.tim), Vec2::new(20, 10), img_scale_method));
		}
	}
	//TODO: implement thread viewing
	thread_panel.add_child(Button::new(op.op.as_ref().unwrap().sub.as_ref().unwrap_or(&"Thread".to_string()), |_| {}));
	
	thread_panel
}

fn create_board_view(c: &mut Cursive) -> impl View {
	let mut layout = SelectView::new();

	add_boards_to_select(&get_boards(c).unwrap(), &mut layout);

	layout.set_on_submit(|c, board: &String| {
		let scale_method =  c.user_data::<Rc<RefCell<SettingsAndData>>>().unwrap().borrow().scale_mode;
		c.call_on_name("threads_list", |threads_view: &mut LinearLayout| {
			//TODO: There's probably a more idiomatic way to clear the LinearLayout
			while threads_view.get_child(0).is_some() { threads_view.remove_child(0); };
			let threads = get_threads_for_board(board);
			let mut iter = threads.iter();
			

			if let Some(post) = iter.next() {
				threads_view.add_child(create_and_add_thread_panel(post, board, scale_method));
			}

			for post in iter {
				threads_view.add_child(Divider::horizontal());
				// if i > 5 {break} // TODO: Remove this
				threads_view.add_child(create_and_add_thread_panel(post, board, scale_method));
			}
		});
	});
	layout.scrollable().with_name("boards_list")
}

fn add_boards_to_select(settings: &SettingsAndData, layout: &mut SelectView) {
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
	let req = get_client()
		.get("https://a.4cdn.org/boards.json")
		.build()
		.expect("Failed to build boards list request");
	let resp = get_client()
		.execute(req)
		.expect("Error requesting boards list");

	resp
		.json::<BoardsResponse>()
		.expect("Failed to parse boards list")
}

fn get_boards(c: &mut Cursive) -> Option<impl Deref<Target = SettingsAndData> + '_> {
	if let Some(settings) = c.user_data::<Rc<RefCell<SettingsAndData>>>() {
		(**settings).try_borrow().ok()
	} else {
		None
	}
}


