use std::{convert::TryInto, error::Error, fmt::Display, time::Instant};

use string_builder::Builder as StringBuilder;
use wasmer_enumset::EnumSet;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Pixel, Rgba, imageops::FilterType};
use cursive::{Vec2, View, direction::Orientation, theme::{Color, ColorStyle, ColorType, Style}, utils::markup::StyledString};

use log::*;

use crate::get_client;

pub mod traits;
pub struct ImageView {
	rendered: ImageRenderable,
	size: Vec2,
}

#[derive(Debug)]
enum ImageRenderable {
	Styled(Vec<StyledString>),
	Raw(Vec<String>),
	// Gui(Window)
}

#[derive(Debug, Clone, Copy)]
pub enum RenderMode {
	// full 24-bit color
	Color,
	Grayscale,
	Gui,
}

#[derive(Debug, Clone, Copy)]
pub enum ConversionError {
	InvalidMode
}
impl Error for ConversionError {}

impl Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		//TODO: proper display function
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScaleMode {
    Nearest,
    Linear,
	Cubic,
    Gaussian,
    Lanczos,
	FastNearest,
}

impl TryInto<FilterType> for ScaleMode {
    type Error = ConversionError;

    fn try_into(self) -> Result<FilterType, Self::Error> {
        match self {
            ScaleMode::Nearest     => Ok(FilterType::Nearest),
            ScaleMode::Linear      => Ok(FilterType::Triangle),
            ScaleMode::Cubic       => Ok(FilterType::CatmullRom),
            ScaleMode::Gaussian    => Ok(FilterType::Gaussian),
            ScaleMode::Lanczos     => Ok(FilterType::Lanczos3),
			// Fast Nearest does not use the image library's scaling functions
			ScaleMode::FastNearest => Err(ConversionError::InvalidMode)
        }
    }
}

impl ImageView {
	pub fn new<'a>(url: impl AsRef<str>, dims: impl Into<Vec2>, render_mode: RenderMode, scale_method: ScaleMode) -> ImageView {
		let dims = dims.into();
		let now = Instant::now();
		let req = get_client()
			.get(url.as_ref())
			.build()
			.expect("Failed to build boards list request");
		let resp = get_client()
			.execute(req)
			.expect("Error requesting boards list");
		let bytes = resp.bytes().unwrap();

		info!("Took {:.4} seconds to get image from {}", now.elapsed().as_secs_f64(), url.as_ref());
		
		let img = decode_image(bytes.as_ref());
		let (size, rendered) = match render_mode {
		    RenderMode::Color => {
				let styled = Self::img_to_color_unicode(&img, dims, scale_method);
				(Vec2::new(styled[0].width(), styled.len()), ImageRenderable::Styled(styled))
			}
		    RenderMode::Grayscale => {
				let gray = Self::img_to_gray_unicode(img, dims, scale_method);
				(Vec2::new(gray[0].len(), gray.len()), ImageRenderable::Raw(gray))
			}
		    RenderMode::Gui => {todo!()}
		};
		ImageView {
			rendered,
			size,
		}
		// img.get_pixel(0, 0);
	}
	
	fn img_to_color_unicode(img: &DynamicImage, dims: Vec2, scale_method: ScaleMode) -> Vec<StyledString> {
		let resized = img.resize(dims.x as u32, dims.y as u32 * 2,scale_method.try_into().unwrap());
		let mut output = Vec::new();
		for y in 0..resized.height()/2 {
			let mut builder = StyledString::new();
			for x in 0..resized.width() {
				let top_pixel = resized.get_pixel(x, y * 2);
				let top_chnls = top_pixel.channels();
				let bottom_pixel = resized.get_pixel(x, y * 2 + 1);
				let bottom_chnls = bottom_pixel.channels();
				let style = Style{
					effects: EnumSet::new(),
					color: ColorStyle::new(
					ColorType::Color(Color::Rgb(bottom_chnls[0], bottom_chnls[1], bottom_chnls[2])), 
					ColorType::Color(Color::Rgb(top_chnls[0], top_chnls[1], top_chnls[2]))
				)};
				builder.append_styled('▄', style);
			}
			output.push(builder)
		}
		output
	}

	fn resized_i2g_unicode(img: &ImageBuffer<Luma<u8>, Vec<u8>>, dims: Vec2) -> Vec<String> {
		let mut output = Vec::new();
		for y in 0..img.height()/2 {
			let mut builder = StringBuilder::new(dims.x);
			for x in 0..img.width() {
				let top_pixel = img.get_pixel(x, y * 2);
				let top_luma = top_pixel.channels()[0] as usize;
				let bottom_pixel = img.get_pixel(x, y * 2 + 1);
				let bottom_luma = bottom_pixel.channels()[0] as usize;
				let luma = (top_luma + bottom_luma) / 2;
				let luma_char = match luma {
					0..=51 => ' ',
					52..=102 => '░',
					103..=153 => '▒',
					154..=204 => '▓',
					205..=256 => '█',
					_ => panic!("invalid luma")
				};
				builder.append(luma_char);
			}
			output.push(builder.string().unwrap())
		}
		output
	}
	
	fn rgb_to_luma(rgb: Rgba<u8>) -> usize {
		let channels = rgb.channels();
		(0.2126 * channels[0] as f64 + 0.7152 * channels[1] as f64 + 0.0722 * channels[2] as f64) as usize
	}

	fn n64_i2g_unicode(img: &DynamicImage, dims: Vec2) -> Vec<String> {
		let scale_x = img.width() as f64 / dims.x as f64;
		let scale_y = img.height() as f64 / dims.y as f64;
		let mut output = Vec::new();
		for y in 0..dims.y/2 {
			let mut builder = StringBuilder::new(dims.x);
			let scaled_top_y = ((y * 2) as f64 * scale_y) as u32;
			let scaled_bottom_y = ((y * 2 + 1) as f64 * scale_y) as u32;
			for x in 0..dims.x {
				let scaled_x = (x as f64 * scale_x) as u32;
				let top_luma = Self::rgb_to_luma(img.get_pixel(scaled_x, scaled_top_y));
				let bottom_luma = Self::rgb_to_luma(img.get_pixel(scaled_x, scaled_bottom_y));
				let luma = (top_luma + bottom_luma) / 2;
				let luma_char = match luma {
					0..=51 => ' ',
					52..=102 => '░',
					103..=153 => '▒',
					154..=204 => '▓',
					205..=256 => '█',
					_ => panic!("invalid luma")
				};
				builder.append(luma_char);
			}
			output.push(builder.string().unwrap())
		}
		output
	}
	
	fn img_to_gray_unicode(img: DynamicImage, dims: Vec2, scale_method: ScaleMode) -> Vec<String> {
		if let Ok(scale_method) = scale_method.try_into() { 
			Self::resized_i2g_unicode(
				&img.resize(dims.x as u32, dims.y as u32 * 2, scale_method).into_luma8(), 
				dims
			)
		} else {
			Self::n64_i2g_unicode(&img, dims)
		}

		
	}

}

impl View for ImageView {
	fn draw(&self, printer: &cursive::Printer) {
		match &self.rendered {
		    ImageRenderable::Styled(styled) => {
				for y in 0..printer.output_size.y {
					if let Some(line) = styled.get(y) {
						printer.print_styled((0, y), line.into());
					} else {
						// printer is bigger than our image; no further lines will be valid
						return;
					}
				}
			}
		    ImageRenderable::Raw(strs) => {
				for y in 0..printer.output_size.y {
					if let Some(line) = strs.get(y) {
						printer.print((0, y), line);
					} else {
						// printer is bigger than our image; no further lines will be valid
						return;
					}
				}
			}
		   //  ImageRenderable::Gui(_) => {}
		}
	}
	
	fn required_size(&mut self, _: Vec2) -> Vec2 {
		self.size
	}
}


fn decode_image(buffer: &[u8]) -> DynamicImage {
	image::load_from_memory(buffer).unwrap()
}

pub struct Divider {
	orientation: Orientation,
}

#[allow(dead_code)]
impl Divider {
	pub fn new(orientation: Orientation) -> Divider {
		Divider { orientation }
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

#[cfg(test)]
mod tests {
	use std::fs::read_to_string;

    use cursive::{direction::Orientation, views::{Button, LinearLayout}};
    use test::Bencher;

    use crate::{SettingsAndData, chan_data::{BoardsResponse, Post}};

    use super::{Divider, ScaleMode, ImageView};
	
	#[bench]
	fn bench_thread_list_creation(_: &mut Bencher) {
		let to_deserialize = read_to_string("assets/test/benchdata.json").unwrap();

		let settings = SettingsAndData {
			show_nsfw: false,
			scale_mode: ScaleMode::Linear,
		   render_mode: super::RenderMode::Color,
			
			boards: BoardsResponse{
			    boards: Vec::new(),
			    troll_flags: None,
			},
		};

		let mut threads_view = LinearLayout::new(Orientation::Vertical);
		for i in 0..100 {
			let now = std::time::Instant::now();
			let deser: Vec<Post> = get_threads_for_test(&to_deserialize);
			create_board_view(&settings, &mut threads_view, deser);
			let time_used = now.elapsed();
			println!("Completed iteration {}; took {:.3}s", i, time_used.as_secs_f64());
			println!("{:?}", threads_view.get_child(0).unwrap().downcast_ref::<LinearLayout>().unwrap().get_child(0).unwrap().downcast_ref::<ImageView>().unwrap().rendered);
		}
			// std::thread::sleep(Duration::from_secs(5));
	}

	fn create_board_view(settings: &SettingsAndData, threads_view: &mut LinearLayout, threads: Vec<Post>) {		
		while threads_view.get_child(0).is_some() { threads_view.remove_child(0); };
		let mut iter = threads.iter();
		

		if let Some(post) = iter.next() {
			threads_view.add_child(create_and_add_thread_panel(post, settings.scale_mode));
		}

		for post in iter {
			threads_view.add_child(Divider::horizontal());
			// if i > 5 {break} // TODO: Remove this
			threads_view.add_child(create_and_add_thread_panel(post, settings.scale_mode));
		}
}
	use serde::{Deserialize, Serialize};
	fn get_threads_for_test(test_str: &String) -> Vec<Post> {
		#[derive(Serialize, Deserialize)]
	
		struct CatalogPage {
			page: usize,
			threads: Vec<Post>,
		}
		serde_json::from_str::<Vec<CatalogPage>>(&test_str)
			.expect("failed to parse threads")
			.remove(0)
			.threads
	}

	use cursive::Vec2;

	/// Creates a LinearLayout for
	fn create_and_add_thread_panel(op: &Post, img_scale_method: ScaleMode) -> LinearLayout {
		println!("Creating thread panel");
		let mut thread_panel = LinearLayout::horizontal();
		if let Some(attachment) = &op.attachment {
			if !attachment.filedeleted {
				thread_panel.add_child(
					ImageView::new(
						format!("http://dernia/bench.jpg"), 
						Vec2::new(20, 10), 
						super::RenderMode::Grayscale, 
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

}