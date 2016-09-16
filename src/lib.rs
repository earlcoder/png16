#![crate_name = "png16"]
#![crate_type = "rlib"]
#![allow(dead_code)]

extern crate byteorder;
extern crate inflate;
extern crate flate2;
extern crate crc;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use crc::crc32;
use flate2::{Compression, FlateReadExt};
use std::fs::File;
use std::io::{Result, Error, ErrorKind, SeekFrom, BufReader, BufWriter};
use std::io::prelude::*;
use std::num::Wrapping;
use std::str;
pub mod ffi;

pub const DEPTH_16: u8 = 0x10;
pub const DEPTH_8: u8 = 0x08;

#[derive(Debug)]
pub struct PNG_IHDR {
	pub total_bytes: u32,
	pub width: u32,
	pub height: u32,
	pub depth: u8,
	pub color_type: u8,
	pub compression: u8,
	pub filter: u8,
	pub interlace: u8,
	pub crc: u32,
}

pub struct PNG {
	pub ihdr: PNG_IHDR,
	pub header: Vec<u8>,
	pub rgb: Vec<u16>,
	pub alpha: Vec<u16>,
}

impl Default for PNG {
	fn default() -> PNG {
		PNG {
			ihdr: PNG_IHDR { ..Default::default() },
			header: Vec::new(),
			rgb: Vec::new(),
			alpha: Vec::new(),
		}
	}
}

impl Default for PNG_IHDR {
	fn default() -> PNG_IHDR {
		PNG_IHDR {
			total_bytes: 0,
			width: 0,
			height: 0,
			depth: 0,
			color_type: 0,
			compression: 0,
			filter: 0,
			interlace: 0,
			crc: 0,
		}
	}
}

pub fn decode_16bit_png(filepath: &str) -> Result<PNG> {
	let img = match File::open(filepath) {
		Ok(img) => img,
		Err(e) => return Err(e),
	};

	let mut png = PNG { ..Default::default() };

	let mut reader = BufReader::new(&img);
	let mut reader_ref = reader.get_mut();

	png.ihdr = match parse_ihdr(&mut reader_ref) {
		Ok(ihdr) => ihdr,
		Err(e) => return Err(e),
	};

	png.header = match get_header(&mut reader_ref) {
		Ok(header) => header,
		Err(e) => return Err(e),
	};

	let mut rgba = match get_rgba(&mut reader_ref, png.ihdr.width, png.ihdr.depth) {
		Ok(rgba) => rgba,
		Err(e) => return Err(e),
	};

	// Split RGBA Into Two Buffers
	match get_rgb_a(&mut rgba, &mut png.rgb, &mut png.alpha) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};

	Ok(png)
}

fn get_rgb_a(rgba: &mut Vec<u16>, rgb: &mut Vec<u16>, alpha: &mut Vec<u16>) -> Result<bool> {
	let mut i = 0;
	while i < rgba.len() {
		rgb.push(rgba[i]);
		rgb.push(rgba[i + 1]);
		rgb.push(rgba[i + 2]);
		alpha.push(rgba[i + 3]);
		i += 4;
	}
	Ok(true)
}


pub fn encode_png(mut png: PNG, depth: u8, result: &str) -> Result<bool> {
	let filterd_rgba = match filter_rgba(&mut png, depth) {
		Ok(filterd_rgba) => filterd_rgba,
		Err(e) => return Err(e),
	};

	let mut deflated = vec![];
	match filterd_rgba.zlib_encode(Compression::Default).read_to_end(&mut deflated) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};

	let out = match File::create(result) {
		Ok(out) => out,
		Err(e) => return Err(e),
	};

	let mut writer = BufWriter::new(&out);
	let mut writer_mut = writer.get_mut();

	match writer_mut.write_u64::<BigEndian>(ffi::PNG_SIG) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u32::<BigEndian>(png.ihdr.total_bytes) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u32::<BigEndian>(ffi::IHDR) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};

	let mut ihdr = Vec::<u8>::new();

	// CRC32 Requires Chunk Tag
	ihdr.push(0x49);
	ihdr.push(0x48);
	ihdr.push(0x44);
	ihdr.push(0x52);

	for i in (0..4).rev() {
		ihdr.push((png.ihdr.width >> i * 8) as u8);
	}
	for i in (0..4).rev() {
		ihdr.push((png.ihdr.height >> i * 8) as u8);
	}

	ihdr.push(depth);
	ihdr.push(png.ihdr.color_type);
	ihdr.push(png.ihdr.compression);
	ihdr.push(png.ihdr.filter);
	ihdr.push(png.ihdr.interlace);

	let ihdr_crc32 = crc32::checksum_ieee(ihdr.as_slice());

	match writer_mut.write_u32::<BigEndian>(png.ihdr.width) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u32::<BigEndian>(png.ihdr.height) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u8(depth) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u8(png.ihdr.color_type) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u8(png.ihdr.compression) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u8(png.ihdr.filter) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u8(png.ihdr.interlace) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u32::<BigEndian>(ihdr_crc32) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write(&png.header) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};

	for c in deflated.chunks(ffi::MAX_IDAT_SIZE) {
		match writer_mut.write_u32::<BigEndian>(c.len() as u32) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		let mut crc_check = Vec::<u8>::new();

		crc_check.push(0x49);
		crc_check.push(0x44);
		crc_check.push(0x41);
		crc_check.push(0x54);

		match crc_check.write(c) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		let crc = crc32::checksum_ieee(crc_check.as_slice());

		match writer_mut.write(&crc_check) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		match writer_mut.write_u32::<BigEndian>(crc) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
	}

	match writer_mut.write_u64::<BigEndian>(ffi::IEND) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};
	match writer_mut.write_u32::<BigEndian>(ffi::TAIL) {
		Ok(_) => (),
		Err(e) => return Err(e),
	};

	Ok(true)
}

fn parse_ihdr(reader: &mut SeekableReader) -> Result<PNG_IHDR> {
	let mut header = PNG_IHDR { ..Default::default() };

	let png_sig = match reader.read_u64::<BigEndian>() {
		Err(e) => {
			return Err(e);
		},
		Ok(png_sig) => {
			if png_sig != ffi::PNG_SIG {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid PNG Signature"));
			}
			png_sig
		},
	};

	header.total_bytes = match reader.read_u32::<BigEndian>() {
		Err(e) => {
			return Err(e);
		},
		Ok(total_bytes) => {
			if total_bytes != 13 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Byte Count"));
			}
			total_bytes
		},
	};

	match reader.read_u32::<BigEndian>() {
		Err(e) => {
			return Err(e);
		},
		Ok(ihdr) => {
			if ihdr != ffi::IHDR {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid IHDR"));
			}
			()
		},
	};

	// Limit Image Size To 65535x65535.
	header.width = match reader.read_u32::<BigEndian>() {
		Err(e) => {
			return Err(e);
		},
		Ok(width) => {
			if width <= 0 || width >= (1 << 16) - 1 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Width"));
			}
			width
		},
	};

	header.height = match reader.read_u32::<BigEndian>() {
		Err(e) => {
			return Err(e);
		},
		Ok(height) => {
			if height <= 0 || height >= (1 << 16) - 1 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Height"));
			}
			height
		},
	};
	// Currently Only 16bit decoding supported, 8bit coming soon(TM)
	header.depth = match reader.read_u8() {
		Err(e) => {
			return Err(e);
		},
		Ok(depth) => {
			if depth != 8 && depth != 16 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Bit Depth"));
			}
			depth
		},
	};
	// TrueColor With Alpha Only, Truecolor and Grayscale coming soon(TM)
	header.color_type = match reader.read_u8() {
		Err(e) => {
			return Err(e);
		},
		Ok(color_type) => {
			if color_type != 6 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Color Type"));
			}
			color_type
		},
	};
	// PNG Only Supports Compression Type 0
	header.compression = match reader.read_u8() {
		Err(e) => {
			return Err(e);
		},
		Ok(compression) => {
			if compression != 0 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Compression Type"));
			}
			compression
		},
	};
	// PNG Only Supports Filter Type 0 [None, Sub, Up, Avg Paeth]
	header.filter = match reader.read_u8() {
		Err(e) => {
			return Err(e);
		},
		Ok(filter) => {
			if filter != 0 {
				return Err(Error::new(ErrorKind::InvalidData, "Invalid Filter Mode"));
			}
			filter
		},
	};

	header.interlace = match reader.read_u8() {
		Err(e) => {
			return Err(e);
		},
		Ok(interlace) => {
			if interlace != 0 {
				return Err(Error::new(ErrorKind::InvalidData, "Interlace Not Supported"));
			}
			interlace
		},
	};
	// TODO: Run CRC32 Check
	header.crc = match reader.read_u32::<BigEndian>() {
		Ok(crc) => crc,
		Err(e) => return Err(e),
	};

	Ok(header)
}

// ********************************************************
// Copy all bytes after IHDR before IDAT
// Header buffer will be copied to encoding side unchanged
// ********************************************************
fn get_header(img: &mut SeekableReader) -> Result<Vec<u8>> {
	let mut header = Vec::<u8>::new();
	loop {
		let chunk_tag = match img.read_u64::<BigEndian>() {
			Ok(chunk_tag) => chunk_tag,
			Err(e) => return Err(e),
		};

		if (chunk_tag as u32) == ffi::IDAT {
			match img.seek(SeekFrom::Current(-8)) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
			break;
		}

		for i in (0..8).rev() {
			header.push((chunk_tag >> 8 * i) as u8);
		}

		match img.take(chunk_tag >> 32).read_to_end(&mut header) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};

		match img.take(4).read_to_end(&mut header) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
	}
	Ok(header)
}

fn get_rgba(img: &mut SeekableReader, width: u32, depth: u8) -> Result<Vec<u16>> {
	let mut data_chunk = vec![];
	// Collect All IDAT Bytes
	loop {
		let idat_header = match img.read_u64::<BigEndian>() {
			Ok(idat_header) => idat_header,
			Err(e) => return Err(e),
		};
		if idat_header as u32 != ffi::IDAT {
			match img.seek(SeekFrom::Current(-8)) {
				Ok(_) => (),
				Err(e) => return Err(e),
			}
			break;
		}
		match img.take(idat_header >> 32).read_to_end(&mut data_chunk) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};

		let idat_crc = match img.read_u32::<BigEndian>() {
			Ok(idat_crc) => idat_crc,
			Err(e) => return Err(e),
		};
	}
	// Inflate Compressed IDAT Bytes
	let mut inflated = vec![];
	match data_chunk.zlib_decode().read_to_end(&mut inflated) {
		Ok(_) => {},
		Err(e) => return Err(e),
	};

	let rgba = match get_unfilterd_idat(&mut inflated, width, depth) {
		Ok(rgba) => rgba,
		Err(e) => return Err(e),
	};

	Ok(rgba)
}

// ************************************************************************************
// x=the byte being filtered;
// a=the byte corresponding to x in the pixel immediately before the pixel containing x
// b=the byte corresponding to x in the previous scanline;
// c=the byte corresponding to b in the pixel immediately before the pixel containing b
// ************************************************************************************
fn get_unfilterd_idat(inflated: &mut Vec<u8>, width: u32, depth: u8) -> Result<Vec<u16>> {
	let mut decode = vec![];
	for c in inflated.chunks(((width * depth as u32) / 2 + 1) as usize) {
		match c[0] {
			0x00 => none_defilter(c, &mut decode),
			0x01 => sub_defilter(c, depth, &mut decode),
			0x02 => up_defilter(c, width, depth, &mut decode),
			0x03 => avg_defilter(c, width, depth, &mut decode),
			0x04 => paeth_defilter(c, width, depth, &mut decode),
			_ => return Err(Error::new(ErrorKind::InvalidData, "Invalid Filter Type")),
		}
	}

	// Conver to u16
	let mut rgba = Vec::<u16>::new();
	let mut count = 0;
	while count < decode.len() {
		rgba.push(((decode[count] as u16) << 8) + decode[count + 1] as u16);
		count += 2;
	}
	Ok(rgba)
}

// ****************************************************
// 	Recon(x) = Filt(x)
// ****************************************************
fn none_defilter(chunk: &[u8], decode: &mut Vec<u8>) {
	for i in 1..chunk.len() {
		decode.push(chunk[i])
	}
}

// ***************************************************************
// 	Recon(x) = Filt(x) + Recon(a)
// ***************************************************************
fn sub_defilter(chunk: &[u8], depth: u8, decode: &mut Vec<u8>) {
	let mut de_filterd = vec![];
	for i in 1..chunk.len() {
		if i <= (depth / 2) as usize {
			de_filterd.push(chunk[i]);
		} else {
			let recon_x = Wrapping(chunk[i]) + Wrapping(de_filterd[i - 9]);
			de_filterd.push(recon_x.0);
		}
		decode.push(de_filterd[i - 1]);
	}
}

// ***************************************************************
// 	Recon(x) = Filt(x) + Recon(b)
// ***************************************************************
fn up_defilter(chunk: &[u8], width: u32, depth: u8, decode: &mut Vec<u8>) {
	let mut offset: usize = 0x00;
	let first_line: bool = if decode.len() == 0 { true } else { false };
	if decode.len() != 0 {
		offset = decode.len() - ((width * depth as u32) / 2) as usize;
	}

	for i in 1..chunk.len() {
		if first_line {
			decode.push(chunk[i]);
		} else {
			let recon_x = Wrapping(chunk[i]) + Wrapping(decode[offset + (i - 1)]);
			decode.push(recon_x.0);
		}

	}

}

// ***************************************************************
// Recon(x) = Filt(x) + floor((Recon(a) + Recon(b)) / 2)
// ***************************************************************
fn avg_defilter(chunk: &[u8], width: u32, depth: u8, decode: &mut Vec<u8>) {
	let mut de_filterd = vec![];
	let mut offset: usize = 0x00;
	let first_line: bool = if decode.len() == 0 { true } else { false };
	if decode.len() != 0 {
		offset = decode.len() - ((width * depth as u32) / 2) as usize;
	}

	let pix_size: usize = (depth / 2) as usize;
	for i in 1..chunk.len() {
		if first_line {
			if i <= pix_size {
				de_filterd.push(chunk[i]);
			} else {
				let recon_x = Wrapping(chunk[i]) + Wrapping(de_filterd[i - 9] >> 1);
				de_filterd.push(recon_x.0);
			}

		} else {
			if i <= pix_size {
				let recon_x = Wrapping(chunk[i]) + Wrapping(decode[offset + (i - 1)] >> 1);
				de_filterd.push(recon_x.0);
			} else {
				let floor_ab = decode[offset + (i - 1)] as i16 + de_filterd[i - 9] as i16;
				let recon_x = chunk[i] as i16 + (floor_ab >> 1);
				de_filterd.push(recon_x as u8);
			}

		}
		decode.push(de_filterd[i - 1] as u8);

	}
}

// ******************************************************************
// Recon(x) = Filt(x) + PaethPredictor(Recon(a), Recon(b), Recon(c))
// ******************************************************************
fn paeth_defilter(chunk: &[u8], width: u32, depth: u8, decode: &mut Vec<u8>) {
	let mut de_filterd = vec![];
	let (mut a, mut b, mut c, mut p, mut pa, mut pb, mut pc, mut pr): (i64, i64, i64, i64, i64, i64, i64, i64);

	let offset = decode.len() - ((width * depth as u32) / 2) as usize;
	for i in 1..chunk.len() {
		if i <= (depth / 2) as usize {
			a = 0x00;
			b = decode[offset + (i - 1)] as i64;
			c = 0x00;

			// PaethPredictor ()
			p = a + b - c;
			pa = (p - a).abs();
			pb = (p - b).abs();
			pc = (p - c).abs();

			if pa <= pb && pa <= pc {
				pr = (chunk[i] as i64 + a as i64) as i64;
			} else if pb <= pc {
				pr = (chunk[i] as i64 + b as i64) as i64;
			} else {
				pr = (chunk[i] as i64 + c as i64) as i64;
			}
			de_filterd.push(pr as u8);

		} else {
			a = de_filterd[i - 9] as i64;
			b = decode[offset + (i - 1)] as i64;
			c = decode[offset + (i - 9)] as i64;
			p = a + b - c;
			pa = (p - a).abs();
			pb = (p - b).abs();
			pc = (p - c).abs();

			if pa <= pb && pa <= pc {
				pr = (chunk[i] as i64 + a as i64) as i64;
			} else if pb <= pc {
				pr = (chunk[i] as i64 + b as i64) as i64;
			} else {
				pr = (chunk[i] as i64 + c as i64) as i64;
			}

			de_filterd.push(pr as u8);
		}

		decode.push(de_filterd[i - 1]);
	}

}

fn filter_rgba(png: &mut PNG, depth: u8) -> Result<Vec<u8>> {
	let mut b_chunk = vec![];
	let mut main = Vec::<u8>::new();

	let mut rgba = Vec::<u8>::new();
	if depth == DEPTH_16 {
		for j in 0..(png.ihdr.width * png.ihdr.height) as usize {
			for i in 0..3 {
				rgba.push((png.rgb[i + (3 * j)] >> 8) as u8);
				rgba.push(png.rgb[i + (3 * j)] as u8);
			}

			rgba.push((png.alpha[j] >> 8) as u8);
			rgba.push(png.alpha[j] as u8);
		}
	} else {
		for j in 0..(png.ihdr.width * png.ihdr.height) as usize {
			for i in 0..3 {
				rgba.push(((png.rgb[i + (3 * j)] >> 8) as f32).round() as u8);

			}
			rgba.push(((png.alpha[j] >> 8) as f32).round() as u8);

		}
	}
	// Scanline Total Bytes
	for c in rgba.chunks(((png.ihdr.width * depth as u32) / 2) as usize) {
		// Test Every Filter + Compression For Smallest Size
		let none = match apply_none_filter(&c) {
			Ok(none) => none,
			Err(e) => return Err(e),
		};
		let sub = match apply_sub_filter(&c, depth) {
			Ok(sub) => sub,
			Err(e) => return Err(e),
		};
		let up = match apply_up_filter(&c, &b_chunk) {
			Ok(up) => up,
			Err(e) => return Err(e),
		};
		let avg = match apply_avg_filter(&c, &b_chunk, depth) {
			Ok(avg) => avg,
			Err(e) => return Err(e),
		};
		let paeth = match apply_paeth_filter(&c, &b_chunk, depth) {
			Ok(paeth) => paeth,
			Err(e) => return Err(e),
		};

		let (mut none_deflated, mut sub_deflated, mut up_deflated, mut avg_deflated, mut paeth_deflated) = (vec![], vec![], vec![], vec![], vec![]);

		match none.zlib_encode(Compression::Fast).read_to_end(&mut none_deflated) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		match sub.zlib_encode(Compression::Fast).read_to_end(&mut sub_deflated) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		match up.zlib_encode(Compression::Fast).read_to_end(&mut up_deflated) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		match avg.zlib_encode(Compression::Fast).read_to_end(&mut avg_deflated) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};
		match paeth.zlib_encode(Compression::Fast).read_to_end(&mut paeth_deflated) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};

		if paeth_deflated.len() <= avg_deflated.len() && paeth_deflated.len() <= up_deflated.len() && paeth_deflated.len() <= sub_deflated.len() && paeth_deflated.len() <= none_deflated.len() {
			match main.write(&paeth) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
		} else if avg_deflated.len() <= up_deflated.len() && avg_deflated.len() <= sub_deflated.len() && avg_deflated.len() <= none_deflated.len() {
			match main.write(&avg) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
		} else if up_deflated.len() <= sub_deflated.len() && up_deflated.len() <= none_deflated.len() {
			match main.write(&up) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
		} else if sub_deflated.len() <= none_deflated.len() {
			match main.write(&sub) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
		} else {
			match main.write(&none) {
				Ok(_) => (),
				Err(e) => return Err(e),
			};
		}

		b_chunk.clear();
		match b_chunk.write(&c) {
			Ok(_) => (),
			Err(e) => return Err(e),
		};

	}
	Ok(main)
}

// ***************************************************************
// Filt(x) = Orig(x)
// ***************************************************************
fn apply_none_filter(chunk: &[u8]) -> Result<Vec<u8>> {
	let mut filterd = vec![];
	filterd.push(0x00);
	for &b in chunk {
		filterd.push(b);
	}
	Ok(filterd)
}

// *********************************************************************
// Filt(x) = Orig(x) - Orig(a)
// *********************************************************************
fn apply_sub_filter(chunk: &[u8], depth: u8) -> Result<Vec<u8>> {
	let mut filterd = vec![];
	filterd.push(0x01);
	for i in 0..chunk.len() {
		if i < (depth >> 1) as usize {
			filterd.push(chunk[i] as u8);
		} else {
			let filt_x = Wrapping(chunk[i]) - Wrapping(chunk[i - (depth >> 1) as usize]);
			filterd.push(filt_x.0);
		}
	}
	Ok(filterd)
}

// *********************************************************************
// Filt(x) = Orig(x) - Orig(b)
// *********************************************************************
fn apply_up_filter(a_chunk: &[u8], b_chunk: &[u8]) -> Result<Vec<u8>> {
	let mut filterd = vec![];
	filterd.push(0x02);
	for i in 0..a_chunk.len() as usize {
		if a_chunk.len() == b_chunk.len() {
			filterd.push((a_chunk[i] as i16 - b_chunk[i] as i16) as u8);
		} else {
			filterd.push(a_chunk[i]);
		}
	}
	Ok(filterd)
}

// *********************************************************************************
// Filt(x) = Orig(x) - floor((Orig(a) + Orig(b)) / 2)
// *********************************************************************************
fn apply_avg_filter(a_chunk: &[u8], b_chunk: &[u8], depth: u8) -> Result<Vec<u8>> {
	let mut filterd = vec![];
	filterd.push(0x03);
	for i in 0..a_chunk.len() as usize {
		if a_chunk.len() == b_chunk.len() {
			if i < (depth >> 1) as usize {
				filterd.push((a_chunk[i] as f32 - (b_chunk[i] as f32 / 2.0_f32).floor()) as u8);
			} else {
				filterd.push((a_chunk[i] as f32 - ((a_chunk[i - (depth >> 1) as usize] as f32 + b_chunk[i] as f32 / 2.0_f32).floor())) as u8);
			}
		} else {
			if i < (depth >> 1) as usize {
				filterd.push(a_chunk[i]);
			} else {
				let result = {
					Wrapping(a_chunk[i]) - (Wrapping(a_chunk[i - (depth >> 1) as usize] >> 1))
				};
				filterd.push(result.0);
			}
		}
	}
	Ok(filterd)
}

// ***********************************************************************************
// Filt(x) = Orig(x) - PaethPredictor(Orig(a), Orig(b), Orig(c))
// ***********************************************************************************
fn apply_paeth_filter(a_chunk: &[u8], b_chunk: &[u8], depth: u8) -> Result<Vec<u8>> {
	let mut filterd = vec![];
	filterd.push(0x04);
	let (mut a, mut b, mut c, mut p, mut pa, mut pb, mut pc, mut pr): (i64, i64, i64, i64, i64, i64, i64, i64);


	for i in 0..a_chunk.len() as usize {
		if a_chunk.len() != b_chunk.len() {
			if i < (depth >> 1) as usize {
				filterd.push(a_chunk[i]);
			} else {
				let val = Wrapping(a_chunk[i]) - Wrapping(a_chunk[i - (depth >> 1) as usize]);
				filterd.push(val.0);
			}

		} else {
			if i < (depth >> 1) as usize {
				a = 0x00;
				b = b_chunk[i] as i64;
				c = 0x00;
				p = a + b - c;
				pa = (p - a).abs();
				pb = (p - b).abs();
				pc = (p - c).abs();

				if pa <= pb && pa <= pc {
					pr = (a_chunk[i] as i64 - a as i64) as i64;
				} else if pb <= pc {
					pr = (a_chunk[i] as i64 - b as i64) as i64;
				} else {
					pr = (a_chunk[i] as i64 - c as i64) as i64;
				}
				filterd.push(pr as u8);
			} else {
				a = a_chunk[i - (depth >> 1) as usize] as i64;
				b = b_chunk[i] as i64;
				c = b_chunk[i - (depth >> 1) as usize] as i64;
				p = a + b - c;
				pa = (p - a).abs();
				pb = (p - b).abs();
				pc = (p - c).abs();

				if pa <= pb && pa <= pc {
					pr = (a_chunk[i] as i64 - a as i64) as i64;
				} else if pb <= pc {
					pr = (a_chunk[i] as i64 - b as i64) as i64;
				} else {
					pr = (a_chunk[i] as i64 - c as i64) as i64;
				}
				filterd.push(pr as u8);
			}
		}
	}
	Ok(filterd)
}

pub trait SeekableReader: Seek + Read {}
impl<T: Seek + Read> SeekableReader for T {}

pub trait SeekableWriter: Seek + Write {}
impl<T: Seek + Write> SeekableWriter for T {}
