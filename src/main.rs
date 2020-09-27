#[macro_use]
macro_rules! warn {
    ( $f:expr $( , $x:expr )* ) => {
        eprintln!(concat!("! Warning: ", $f), $($x)*);
    };
}

mod chunk;
mod file;
mod idat;
mod ihdr;
mod zlib;

use crate::chunk::ChunkReader;
use crate::file::ByteReader;
use crate::idat::IdatReader;
use crate::ihdr::PartialColorMode;
use crate::ihdr::ColorMode;
use crate::ihdr::InterlaceMethod;
use crate::zlib::read_zlib;
use std::env;
use std::fs::File;
use std::io;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    EndOfChunk(Box<[u8]>),
    Format(&'static str),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IO(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

const PNG_SIG : [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

fn decompressed_data_length(width: u32, height: u32, color_mode: &ColorMode, interlace_method: InterlaceMethod) -> usize {
    match interlace_method {
        InterlaceMethod::NoInterlace => ((width as usize * color_mode.bits_per_pixel() + 7) / 8 + 1) * height as usize,
        InterlaceMethod::Adam7 => std::todo!(),
    }
}

fn main() -> Result<()> {
    let mut args = env::args();
    if args.len() != 2 {
        return Err(Error::Format("Invalid number of arguments"));
    }
    let mut file = File::open(args.nth(1).unwrap())?;
    let sig = file.read_buf(8)?;
    println!("Signature: {:02X?}", sig);
    if *sig != PNG_SIG {
        return Err(Error::Format("Invalid PNG signature"));
    }

    // TODO make interface nicer (don't shadow file)
    let (mut file, width, height, partial_color_mode, interlace_method) = ihdr::load_ihdr(file)?;
    let color_mode = match partial_color_mode {
        PartialColorMode::Full(mode) => mode,
        PartialColorMode::Partial(_) => std::todo!(),
    };

    loop {
        let (mut chunk, length, chunk_type) = ChunkReader::new(file)?;
        match &*chunk_type {
            b"IHDR" => {
                warn!("Multiple IHDR chunks");
            },
            b"IDAT" => {
                let mut buf = vec![0; decompressed_data_length(width, height, &color_mode, interlace_method)].into_boxed_slice();
                chunk = read_zlib(IdatReader::new(chunk)?, &mut buf)?.end()?;
            },
            b"IEND" => {
                if length != 0 {
                    warn!("IEND chunk has nonzero length");
                }
            },
            _ => {
                // TODO warn on invalid chunk types
                if chunk_type[0] & 0x20 == 0 {
                    warn!("Unrecognized critical chunk");
                }
            },
        }
        file = chunk.end()?;
        if *chunk_type == *b"IEND" {
            // TODO check for EOF
            break;
        }
    }

    Ok(())
}
