use crate::chunk::ChunkReader;
use crate::file::ByteReader;
use crate::Error;
use crate::Result;
use std::fs::File;

pub struct Palette {
}

pub enum ColorMode {
    Grayscale1,
    Grayscale2,
    Grayscale4,
    Grayscale8,
    Grayscale16,
    RGB8,
    RGB16,
    Palette1(Palette),
    Palette2(Palette),
    Palette4(Palette),
    Palette8(Palette),
    GrayscaleAlpha8,
    GrayscaleAlpha16,
    RGBA8,
    RGBA16,
}

pub enum PartialColorMode {
    Full(ColorMode),
    Partial(fn(Palette) -> ColorMode),
}

pub fn get_color_mode(bit_depth: u8, color_type: u8) -> Result<PartialColorMode> {
    use ColorMode::*;
    use PartialColorMode::*;
    match (color_type, bit_depth) {
        (0, 1) => Ok(Full(Grayscale1)),
        (0, 2) => Ok(Full(Grayscale2)),
        (0, 4) => Ok(Full(Grayscale4)),
        (0, 8) => Ok(Full(Grayscale8)),
        (0, 16) => Ok(Full(Grayscale16)),
        (2, 8) => Ok(Full(RGB8)),
        (2, 16) => Ok(Full(RGB16)),
        (3, 1) => Ok(Partial(Palette1)),
        (3, 2) => Ok(Partial(Palette2)),
        (3, 4) => Ok(Partial(Palette4)),
        (3, 8) => Ok(Partial(Palette8)),
        (4, 8) => Ok(Full(GrayscaleAlpha8)),
        (4, 16) => Ok(Full(GrayscaleAlpha16)),
        (6, 8) => Ok(Full(RGBA8)),
        (6, 16) => Ok(Full(RGBA16)),
        _ => Err(Error::Format("Invalid bit depth and color mode combination")),
    }
}

pub enum InterlaceMethod {
    NoInterlace,
    Adam7,
}

pub fn get_interlace_method(interlace_method: u8) -> Result<InterlaceMethod> {
    match interlace_method {
        0 => Ok(InterlaceMethod::NoInterlace),
        1 => Ok(InterlaceMethod::Adam7),
        _ => Err(Error::Format("Invalid interlace method")),
    }
}

pub fn load_ihdr(file: File) -> Result<(File, u32, u32, PartialColorMode, InterlaceMethod)> {
    let (mut chunk, _, chunk_type) = ChunkReader::new(file)?;
    if *chunk_type != *b"IHDR" {
        return Err(Error::Format("First chunk is not IHDR"));
    }
    let width = chunk.read_u32()?;
    println!("Width: {}", width);
    if width == 0 {
        return Err(Error::Format("Width is zero"));
    }
    if width > 0x7FFFFFFF {
        warn!("Width exceeds (2^32)-1");
    }
    let height = chunk.read_u32()?;
    println!("Height: {}", height);
    if height == 0 {
        return Err(Error::Format("Height is zero"));
    }
    if height > 0x7FFFFFFF {
        warn!("Height exceeds (2^32)-1");
    }
    let bit_depth = chunk.read_u8()?;
    println!("Bit depth: {}", bit_depth);
    let color_type = chunk.read_u8()?;
    println!("Color type: {}", color_type);
    let partial_color_mode = get_color_mode(bit_depth, color_type)?;
    let compression_method = chunk.read_u8()?;
    println!("Compression method: {}", compression_method);
    if compression_method != 0 {
        return Err(Error::Format("Unrecognized compression method"));
    }
    let filter_method = chunk.read_u8()?;
    if filter_method != 0 {
        return Err(Error::Format("Unrecognized filter method"));
    }
    let interlace_method = chunk.read_u8()?;
    println!("Interlace method: {}", interlace_method);
    let interlace_method = get_interlace_method(interlace_method)?;
    let file = chunk.end()?;
    Ok((file, width, height, partial_color_mode, interlace_method))
}
