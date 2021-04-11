use std::{collections::HashMap, fmt::Debug};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Thread {
	pub posts: Vec<Post>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
	#[serde(flatten)]
	pub op: Option<OpData>,                 // contains data specific to the OP if this post is the OP
	pub no: isize,                          // post ID
	pub resto: isize,                       // ID of the thread (or 0 if this is the OP)
	pub now: String, 
	pub time: isize,                        // time post was created
	pub name: String,                       // user name
	pub trip: Option<String>,               // user tripcode, whatever that is
	pub id: Option<String>,                 // user ID?
	pub capcode: Option<String>,            // post capcode, whatever that is
	pub country: Option<String>,            // country code
	pub country_name: Option<String>,       // country name
	pub com: Option<String>,                // comment
	#[serde(flatten)]
	pub attachment: Option<AttachmentData>, // data for post's attachment if present
	pub since4pass: Option<isize>,          // year 4chan pass bought
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub m_img: bool,                        // if post has mobile optimized image
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AttachmentData {
	pub tim: isize,                          // image upload timestamp
	pub filename: String,                    // file name
	pub ext: String,                         // file extension
	pub fsize: isize,                        // file size
	pub md5: String,                         // md5 of file
	pub w: isize,                            // image width
	pub h: isize,                            // image height
	pub tn_w: isize,                         // thumbnail width
	pub tn_h: isize,                         // thumbnail height
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub filedeleted: bool,                   // if the file has been deleted
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub spoiler: bool,                       // if the file is spoilered
	pub custom_spoiler: Option<isize>,       // custom spoiler ID
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpData {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Board {
	pub board: String,
	pub title: String,
	#[serde(rename = "ws_board", default, deserialize_with ="opt_int_to_bool")]
	pub sfw: bool,
	#[serde(rename = "per_page")]
	pub threads_per_page: usize,
	pub pages: usize,
	pub max_filesize: usize,
	pub max_webm_filesize: usize,
	pub max_comment_chars: usize,
	pub bump_limit: usize,
	pub image_limit: usize,
	// pub cooldowns: Vec<!> // this is just defined as "an array" on the docs...
	pub meta_description: String,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub spoilers: bool,
	pub custom_spoilers: Option<usize>,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub is_archived: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub troll_flags: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub country_flags: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub user_ids: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub oekaki: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub sjis_tags: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub code_tags: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub math_tags: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub text_only: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub forced_anon: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub webm_audio: bool,
	#[serde(default, deserialize_with = "opt_int_to_bool")]
	pub require_subject: bool,
	pub min_image_width: Option<usize>,
	pub min_image_height: Option<usize>,
	
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BoardsResponse {
	pub boards: Vec<Board>,
	pub troll_flags: Option<HashMap<String, String>>
}

#[test]
fn test_board_deserialize() {
	let test = std::fs::read_to_string("boards.json").unwrap();
	let tested: BoardsResponse = serde_json::de::from_str(&test).unwrap();
	assert!(tested.boards.len() == 79);
	assert!(tested.troll_flags.is_some())
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