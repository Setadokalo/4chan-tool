use std::{fmt::Debug, sync::{Arc, Mutex}, thread, time::Duration};

use chrono::{DateTime, NaiveDateTime, Utc};
use cursive::{direction::Orientation, theme::{BaseColor, Color, Theme}, views::{Button, LinearLayout, Panel, ResizedView, TextView}};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ThreadConfig {
	pub board: String,
	pub id: String,
	pub name: String,
	#[serde(skip, default = "get_unix_epoch")]
	pub last_modified: DateTime<Utc>,
}

fn get_unix_epoch() -> DateTime<Utc> {
	DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc)
}

#[derive(Serialize, Deserialize, Debug)]
struct Thread {
	posts: Vec<Post>,
}
#[derive(Serialize, Deserialize, Debug)]
struct Post {
	#[serde(flatten)]
	op: Option<OpData>,                 // contains data specific to the OP if this post is the OP
	no: isize,                          // post ID
	resto: isize,                       // ID of the thread (or 0 if this is the OP)
	now: String, 
	time: isize,                        // time post was created
	name: String,                       // user name
	trip: Option<String>,               // user tripcode, whatever that is
	id: Option<String>,                 // user ID?
	capcode: Option<String>,            // post capcode, whatever that is
	country: Option<String>,            // country code
	country_name: Option<String>,       // country name
	com: Option<String>,                // comment
	#[serde(flatten)]
	attachment: Option<AttachmentData>, // data for post's attachment if present
	since4pass: Option<isize>,          // year 4chan pass bought
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	m_img: bool,                        // if post has mobile optimized image
}

#[derive(Serialize, Deserialize, Debug)]
struct AttachmentData {
	tim: isize,                          // image upload timestamp
	filename: String,                    // file name
	ext: String,                         // file extension
	fsize: isize,                        // file size
	md5: String,                         // md5 of file
	w: isize,                            // image width
	h: isize,                            // image height
	tn_w: isize,                         // thumbnail width
	tn_h: isize,                         // thumbnail height
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	filedeleted: bool,                   // if the file has been deleted
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	spoiler: bool,                       // if the file is spoilered
	custom_spoiler: Option<isize>,       // custom spoiler ID
}

#[derive(Serialize, Deserialize, Debug)]
struct OpData {
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	sticky: bool,                        // if the thread is pinned
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	closed: bool,                        // if the thread is closed to replies
	sub: Option<String>,                 // subject text
	replies:isize,                       // total number of replies
	images: isize,                       // total number of image replies
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	bumplimit: bool,                     // if the thread has reached the bump limit
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	imagelimit: bool,                    // if the thread has reached the image limit
	tag: Option<String>,                 // (/f/ only) category of the .swf upload
	semantic_url: String,                // SEO URL slug for thread
	unique_ips: Option<isize>,           // Number of unique posters in thread

	#[serde(default, deserialize_with = "opt_int_to_bool")]
	archived: bool,                      // if the thread has been archived
	archived_on: Option<isize>,          // archived date
}

pub fn opt_int_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
	D: Deserializer<'de>,
{
	if let Ok(res) = Option::<isize>::deserialize(deserializer) {
		match res  {
			Some(1) => Ok(true),
			_ => Ok(false),
		}
	} else {
		Ok(false)
	}
}

fn load_config(raw_config: String) -> Vec<ThreadConfig> {
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
	});

	siv.add_fullscreen_layer(
		LinearLayout::new(Orientation::Vertical)
		   .child(
				LinearLayout::new(Orientation::Horizontal)
					.child(Panel::new(ResizedView::with_full_screen(TextView::new("Hello, Panel!"))))
		         .child(Panel::new(ResizedView::with_full_screen(TextView::new("Hello, Other Panel!"))))
			)
		   .child(
				LinearLayout::new(Orientation::Horizontal)
					.child(Button::new("Quit", |c| c.quit()))
			)
	);
	siv.run();
}

#[cfg(test)]
mod tests {
	use crate::Thread;
	use serde::{Deserialize, Serialize};
	#[test]
	fn test_load() {
		let bc = crate::load_config("
			\t  4chan/board/47357       garbage that is a name\n
					 4chan/board/23612           some bullshit
			4chan/board/42672
			".to_string());
		assert!(bc[0].name == "garbage that is a name");
		assert!(bc[1].name == "some bullshit");
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