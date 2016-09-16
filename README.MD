## PNG16

###### PNG16 Currently decodes 16bit RGBA PNGs. Encoding options 16bit RGBA PNG, 8bit RGBA PNG.
### ImageMagick Conversion Command:
```cmd
convert -alpha on -colorspace sRGB -depth 16 INPUTFILE.EXT -depth 16 -colorspace sRGB -alpha on png64:output.png 
```
#### If Dealing with images containing ICC:
```cmd
convert -profile INPUTFILE.EXT -alpha on -colorspace sRGB -depth 16 INPUTFILE.EXT -depth 16 -colorspace sRGB -alpha on -profile INPUTFILE.EXT png64:output.png
```

**Functions**
```rust
fn decode_16bit_png(filepath: &str) -> Result<PNG> 
fn encode_png(mut png: PNG, depth: u8, result: &str) -> Result<bool>
```

**PNG16 Structs/Const**
```rust
pub const DEPTH_16: u8 = 0x10;
pub const DEPTH_8: u8 = 0x08;

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
```

**Usage**
```rust
extern crate png16;
...
let mut png = match png16::decode_16bit_png("16bit.png") {
		Ok(png) => png,
		Err(e) => {
			panic!("Error Decoding PNG: {:?}", e);
		},
	};
	
match png16::encode_png(png, png16::DEPTH_16, "result.png") {
		Ok(_) => (),
		Err(e) => panic!("Error Encoding PNG: {:?}", e),
	};
	
```