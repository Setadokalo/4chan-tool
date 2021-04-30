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
