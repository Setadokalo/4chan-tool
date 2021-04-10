use std::{sync::{Arc, Mutex}, thread, time::Duration};

use chan_data::Thread;
use chrono::Utc;
use config::ThreadConfig;
use cursive::{Cursive, direction::Orientation, theme::{BaseColor, BorderStyle, Color}, traits::{Boxable, Nameable}, views::{Button, Dialog, EditView, LinearLayout, Panel, ResizedView, TextView}};

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
#[allow(dead_code)]
fn watch_threads(thread_list: Arc<Mutex<Vec<ThreadConfig>>>) -> ! {
	println!("Watch daemon started with these threads:");
	for config in (*thread_list).lock().unwrap().iter() {
		println!("Thread \"{}\" in board \"{}\" with name \"{}\"", config.id, config.board, config.name);
	}
	let client = reqwest::blocking::Client::new();
	loop {
		for mut thread_cfg in (*thread_list).lock().unwrap().iter_mut() {
			let req = client.get(format!("https://a.4cdn.org/{}/thread/{}.json", thread_cfg.board, thread_cfg.id))
					.header("If-Modified-Since", thread_cfg.last_modified.to_rfc2822().replace("+0000", "GMT"))
					.build()
					.expect("Failed to build request");
			let resp = client.execute(req).expect("Error requesting page");
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

	// siv.add_fullscreen_layer(
	// 	LinearLayout::new(Orientation::Vertical)
	// 	   .child(
	// 			LinearLayout::new(Orientation::Horizontal)
	// 				.child(Panel::new(ResizedView::with_full_screen(TextView::new("Hello, Panel!"))))
	// 	         .child(Panel::new(ResizedView::with_full_screen(TextView::new("Hello, Other Panel!"))))
	// 		)
	// 	   .child(
	// 			LinearLayout::new(Orientation::Horizontal)
	// 				.child(Button::new("Quit", |c| c.quit()))
	// 		)
	// );
	
	siv.add_layer(
		Dialog::new()
			 .title("Enter your name")
			 // Padding is (left, right, top, bottom)
			 .padding_lrtb(1, 1, 1, 0)
			 .content(
				  EditView::new()
						// Call `show_popup` when the user presses `Enter`
						.on_submit(show_popup)
						// Give the `EditView` a name so we can refer to it later.
						.with_name("name")
						// Wrap this in a `ResizedView` with a fixed width.
						// Do this _after_ `with_name` or the name will point to the
						// `ResizedView` instead of `EditView`!
						.fixed_width(50),
			 )
			 .button("Ok", |s| {
				  // This will run the given closure, *ONLY* if a view with the
				  // correct type and the given name is found.
				  let name = s
						.call_on_name("name", |view: &mut EditView| {
							 // We can return content from the closure!
							 view.get_content()
						})
						.unwrap();

				  // Run the next step
				  show_popup(s, &name);
			 }),
  );
	siv.run();
}

// This will replace the current layer with a new popup.
// If the name is empty, we'll show an error message instead.
fn show_popup(s: &mut Cursive, name: &str) {
	if name.is_empty() {
		 // Try again as many times as we need!
		 s.add_layer(Dialog::info("Please enter a name!"));
	} else {
		 let content = format!("Hello {}!", name);
		 // Remove the initial popup
		 s.pop_layer();
		 // And put a new one instead
		 s.add_layer(
			  Dialog::around(TextView::new(content))
					.button("Quit", |s| s.quit()),
		 );
	}
}

#[cfg(test)]
mod tests {
	use crate::chan_data::Thread;
	use serde::{Deserialize, Serialize};
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
		#[derive(Deserialize, Serialize, Debug)]
		struct Test {
			data: Option<isize>
		}
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