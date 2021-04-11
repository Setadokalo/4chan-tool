use std::{sync::{Arc, Mutex, Once}, thread, time::Duration};

use chan_data::{BoardsResponse, Thread};
use chrono::Utc;
use config::ThreadConfig;
use cursive::{Vec2, View, direction::Orientation, theme::{BaseColor, Color}, traits::*, view::SizeConstraint, views::{Button, LinearLayout, SelectView, Panel, ResizedView, ScrollView, TextView}};

mod chan_data;

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
		for (i, line) in raw_config.split("\n").enumerate().filter(|(_, s)| !s.is_empty()) {
			let mut config_iter = line.trim().splitn(2, |c: char| c.is_whitespace());
			if let Some(thread_config) = config_iter.next() {
				let target: Vec<&str> = thread_config.split("/").collect();
				if target.len() != 3 {
					println!("Unrecognized board link structure at line {}: {}", i, thread_config);
					continue;
				}
				// the rest of the string is in the second iterator value (or there isn't one, in which case we default to the thread config string)
				let name = config_iter.next().unwrap_or(thread_config).trim().to_string();
				
				new_config.push(ThreadConfig {
					board: target[1].to_string(), 
					id: target[2].to_string(), 
					name, 
					// the unix epoch
					last_modified: get_unix_epoch()
				});
			}
		}
		new_config
	}	
}

trait ResizableWeak: Boxable {
	/// returns the self in a double resized view wrapper, which forces the `self` to request space (but not force it)
	/// up until it's SizeConstraint limit.
	fn resized_weak(self, width: SizeConstraint, height: SizeConstraint) -> ResizedView<ResizedView<Self>> {
		self.resized(SizeConstraint::Full, SizeConstraint::Full).resized(width, height)
	}
	/// same as `resized_weak`, but automatically uses `SizeConstraint::Free` for the height
	fn resized_weak_w(self, width: SizeConstraint) -> ResizedView<ResizedView<Self>> {
		self.resized(SizeConstraint::Full, SizeConstraint::Full).resized(width, SizeConstraint::Free)
	}
	/// same as `resized_weak`, but automatically uses `SizeConstraint::Free` for the width
	fn resized_weak_h(self, height: SizeConstraint) -> ResizedView<ResizedView<Self>> {
		self.resized(SizeConstraint::Full, SizeConstraint::Full).resized(SizeConstraint::Free, height)
	}
}

impl<T> ResizableWeak for T where T: Boxable {}
trait Panelable: View {
	// returns the self in a double resized view wrapper, which forces the `self` to request space (but not force it)
	// up until it's SizeConstraint limit.
	fn in_panel(self) -> Panel<Self> where Self: Sized {
		Panel::new(self)
	}
}

impl<T> Panelable for T where T: View {}

static mut CLIENT: Option<reqwest::blocking::Client> = None;
static CLIENT_ONCE: Once = Once::new();

fn get_client() -> &'static reqwest::blocking::Client {
	unsafe {
		CLIENT_ONCE.call_once(|| CLIENT = Some(reqwest::blocking::Client::new()));
		CLIENT.as_ref().unwrap()
	}
}
	
#[allow(dead_code)]
fn watch_threads(thread_list: Arc<Mutex<Vec<ThreadConfig>>>) -> ! {
	println!("Watch daemon started with these threads:");
	for config in (*thread_list).lock().unwrap().iter() {
		println!("Thread \"{}\" in board \"{}\" with name \"{}\"", config.id, config.board, config.name);
	}
	loop {
		for mut thread_cfg in (*thread_list).lock().unwrap().iter_mut() {
			let req = get_client().get(format!("https://a.4cdn.org/{}/thread/{}.json", thread_cfg.board, thread_cfg.id))
					.header("If-Modified-Since", thread_cfg.last_modified.to_rfc2822().replace("+0000", "GMT"))
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

struct Divider {
	orientation: Orientation
}

impl Divider {
	pub fn new(orientation: Orientation) -> Divider {
		Divider {orientation}
	}
	pub fn horizontal() -> Divider {
		Self::new(Orientation::Horizontal)
	}
	pub fn vertical() -> Divider {
		Self::new(Orientation::Vertical)
	}
}

impl View for Divider {
	fn draw(&self, printer: &cursive::Printer) {
		if self.orientation == Orientation::Horizontal {
			//  panic!("{}, {:?}, {:?}", std::iter::repeat("X").take(printer.size.y).collect::<String>(), printer.offset, printer.size);
			printer.print_hline(Vec2::zero(), printer.size.x, "-");
		} else {
			printer.print_vline(Vec2::zero(), printer.size.y, "|")
		}
	}
}

fn main() {
	let mut siv = cursive::default();
	siv.update_theme(|theme| {
		theme.palette.set_color("background", Color::Dark(BaseColor::Black));
		theme.palette.set_color("view", Color::Dark(BaseColor::Black));
		theme.palette.set_color("primary", Color::Light(BaseColor::White));
		theme.palette.set_color("secondary", Color::Light(BaseColor::Blue));
		theme.palette.set_color("tertiary", Color::Light(BaseColor::Red));
		theme.shadow = false;
	});
	type SiCon = SizeConstraint;
	siv.add_fullscreen_layer(
		LinearLayout::vertical()
		   .child(LinearLayout::horizontal()
				.child(create_board_view().in_panel())
				.child(LinearLayout::horizontal()
					.child(ResizedView::with_full_screen(TextView::new("Hello, Other Panel!").with_name("Board")))
					.child(Divider::vertical())
					.child(TextView::new("Panels are interesting!").resized_weak_w(SiCon::AtMost(20)))
					.in_panel()
				)
			)
		   .child(
				LinearLayout::horizontal()
					.child(Button::new("Quit", |c| c.quit()))
			)
	);

	siv.run();
}

fn create_board_view() -> ScrollView<SelectView> {

	let mut layout = SelectView::new();
	let boards = get_4chan_boards();
	for board in boards.boards.iter() {
		layout.add_item(format!("/{}/: {}", board.board, board.title), board.board.clone());
		//layout.add_child(Divider::horizontal());
	}
	layout.set_on_submit(|c, item: &String| {
		c.call_on_name("Board", |view: &mut TextView| {
			view.set_content(format!("User selected panel {}", item))
		});
	});
	layout.scrollable()
}

fn get_4chan_boards() -> BoardsResponse {
	let req = get_client().get("https://a.4cdn.org/boards.json")
					.build()
					.expect("Failed to build boards list request");
	let resp = get_client().execute(req).expect("Error requesting boards list");
	resp.json::<BoardsResponse>().expect("Failed to parse boards list")
}

#[cfg(test)]
mod tests {
	use crate::chan_data::Thread;
	#[test]
	fn test_load() {
		let bc = crate::config::load_config("
			\t  4chan/board/47357       garbage that is a name\n
					 4chan/board/23612           some text
			4chan/board/42672
			".to_string());
		assert!(bc[0].name == "garbage that is a name");
		assert!(bc[1].name == "some text");
		assert!(bc[2].name == "4chan/board/42672");
	}
	
	
	#[test]
	fn test_deser() {
		let test = std::fs::read_to_string("dummy.json").unwrap();
		let tested: Thread = serde_json::de::from_str(&test).unwrap();
		assert!(tested.posts.len() == 3);
		assert!(tested.posts.get(0).unwrap().op.is_some());
		assert!(tested.posts.get(0).unwrap().attachment.is_some());
		assert!(tested.posts.get(1).unwrap().op.is_none());
		assert!(tested.posts.get(1).unwrap().attachment.is_some());
		assert!(tested.posts.get(2).unwrap().op.is_none());
		assert!(tested.posts.get(2).unwrap().attachment.is_some());
	}
}