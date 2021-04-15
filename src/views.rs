use wasmer_enumset::EnumSet;
use image::{DynamicImage, GenericImageView, Pixel, imageops::FilterType};
use cursive::{Vec2, View, direction::Orientation, theme::{Color, ColorStyle, ColorType, Effect, Style}, utils::markup::StyledString};

use crate::get_client;

pub mod traits;
pub struct ImageView {
	ascii_img: Vec<StyledString>,
	size: Vec2,
}

impl ImageView {
	pub fn new<'a>(url: impl AsRef<str>, dims: impl Into<Vec2>, scale_method: FilterType) -> ImageView {
		let dims = dims.into();
		let req = get_client()
			.get(url.as_ref())
			.build()
			.expect("Failed to build boards list request");
		let resp = get_client()
			.execute(req)
			.expect("Error requesting boards list");
		let bytes = resp.bytes().unwrap();
		
		let img = decode_image(bytes.as_ref());
		let ascii_img = Self::convert_img_to_ascii(&img, dims, scale_method);
		let size = Vec2::new(ascii_img[0].width(), ascii_img.len());
		ImageView {
			ascii_img,
			size,
		}
		// img.get_pixel(0, 0);
	}
	
	fn convert_img_to_ascii(img: &DynamicImage, dims: Vec2, scale_method: FilterType) -> Vec<StyledString> {
		let resized = img.resize(dims.x as u32, dims.y as u32 * 2,scale_method);
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

}

impl View for ImageView {
	fn draw(&self, printer: &cursive::Printer) {
		for y in 0..printer.output_size.y {
			if let Some(line) = self.ascii_img.get(y) {
				printer.print_styled((0, y), line.into());
			} else {
				// printer is bigger than our image; no further lines will be valid
				return;
			}
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