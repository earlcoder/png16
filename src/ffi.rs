#![allow(non_upper_case_globals)]
pub const IHDR: u32 = 0x49484452; //ImageHeader
pub const IDAT: u32 = 0x49444154; //Image Data
pub const tEXt: u32 = 0x74455874; //TextualData
pub const tIME: u32 = 0x74494d45; //ModifyDate
pub const PLTE: u32 = 0x504c5445; //Palette
pub const bKGD: u32 = 0x624b4744; //BackgroundColor
pub const cHRM: u32 = 0x6348524d; //PrimaryChromaticities
pub const dSIG: u32 = 0x64534947; //Digital Signature
pub const fRAc: u32 = 0x66524163; //Fractal Paramaters
pub const gAMA: u32 = 0x67414d41; //Gamma
pub const gIFg: u32 = 0x67494667; //GIFGraphicControlExtension
pub const gIFt: u32 = 0x67494674; //GIFPlainTextExtension
pub const gIFx: u32 = 0x67494678; //GIFApplicationExtension
pub const hIST: u32 = 0x68495354; //PaletteHistogram
pub const iCCP: u32 = 0x69434350; //ICC_Profile
pub const iTXt: u32 = 0x69545874; //InternationalText
pub const oFFs: u32 = 0x6f464673; //ImageOffset
pub const pCAL: u32 = 0x7043414c; //PixelCalibration
pub const pHYs: u32 = 0x70485973; //PhysicalPixel
pub const sBIT: u32 = 0x73424954; //SignificantBits
pub const sCAL: u32 = 0x7343414c; //SubjectScale
pub const sPLT: u32 = 0x73504c54; //SuggestedPalette
pub const sRGB: u32 = 0x73524742; //SRGBRendering
pub const sTER: u32 = 0x73544552; //StereoImage
pub const tRNS: u32 = 0x74524e53; //Transparency
pub const tXMP: u32 = 0x74584d50; //XMP
pub const vpAg: u32 = 0x76704167; //VirtualPage
pub const zTXt: u32 = 0x7a545874; //CompressedText

pub const TAIL: u32 = 0xAE426082;         //TrailingBits after IEND
pub const IEND: u64 = 0x0000000049454E44; //Image End
pub const PNG_SIG: u64 = 0x89504E470D0A1A0A; //Will never Change

pub const MAX_IDAT_SIZE: usize = 0x8000;
