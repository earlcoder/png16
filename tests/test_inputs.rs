extern crate png16;
extern crate scan_dir;
use scan_dir::ScanDir;
use std::io::prelude::*;
#[test]
fn test() {

	let files: Vec<_> = ScanDir::files()
		.read("./test_images", |iter| {
			iter.filter(|&(_, ref name)| name.ends_with(".png"))
				.map(|(entry, _)| entry.path())
				.collect()
		})
		.unwrap();

	for f in files {
		let mut png = match png16::decode_16bit_png(f.to_str().unwrap()) {
			Ok(png) => png,
			Err(e) => {
				panic!("Error Decoding PNG: {:?}", e);
			},
		};
		match png16::encode_png(png, png16::DEPTH_16, "test.png") {
			Ok(_) => (),
			Err(e) => panic!("Error Encoding PNG: {:?}", e),
		};		
	}
}
