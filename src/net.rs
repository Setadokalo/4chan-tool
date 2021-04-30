use std::{sync::{Arc, Mutex, Once}, thread, time::{Duration, Instant}};

use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
use chrono::Utc;

use log::*;

use bench_debug::log_bench;

use crate::data::{BoardsResponse, Post, Thread};
use crate::config::ThreadConfig;



static mut CLIENT: Option<reqwest::blocking::Client> = None;
static CLIENT_ONCE: Once = Once::new();

fn get_client() -> &'static reqwest::blocking::Client {
	unsafe {
		CLIENT_ONCE.call_once(|| CLIENT = Some(reqwest::blocking::Client::new()));
		CLIENT.as_ref().unwrap()
	}
}

#[log_bench(url)]
pub fn get_bytes(url: impl AsRef<str> + std::fmt::Debug) -> bytes::Bytes {
	let url = url.as_ref();
	let req = get_client()
		.get(url)
		.build()
		.expect(&*format!("Failed to build {} request", url));
	let resp = get_client()
		.execute(req)
		.expect(&*format!("Error during {} request", url));
	resp.bytes().unwrap()
}

pub fn load_4chan_boards() -> BoardsResponse {
	let now = Instant::now();
	let client = get_client();
	
	let resp = client.execute(
		client
			.get("https://a.4cdn.org/boards.json")
			.build()
			.expect("Failed to build boards list request")
		).expect("Error requesting boards list");

	info!("Took {:.4} seconds to get 4chan boards", now.elapsed().as_secs_f64());

	resp
		.json::<BoardsResponse>()
		.expect("Failed to parse boards list")
}

pub fn request_url(url: impl AsRef<str>) -> Response {
	let req = get_client()
	.get(url.as_ref())
	.build()
	.expect("Failed to build threads list request");
	
	get_client()
	.execute(req)
	.expect("Error requesting threads list")
}

pub fn get_threads_for_board(board: impl Into<String>) -> Vec<Post> {
	let s = board.into();
	#[derive(Serialize, Deserialize)]

	struct CatalogPage {
		page: usize,
		threads: Vec<Post>,
	}
	let now = Instant::now();
	
	let resp = request_url(format!("https://a.4cdn.org/{}/catalog.json", s.clone()));

	info!("Took {:.4} seconds to get /{}/ catalog", now.elapsed().as_secs_f64(), s);

	resp
		.json::<Vec<CatalogPage>>()
		.expect("Failed to parse threads list")
		.remove(0).threads
}

#[allow(dead_code)]
pub fn watch_threads(thread_list: Arc<Mutex<Vec<ThreadConfig>>>) -> ! {
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
