#[macro_use]
macro_rules! warn {
    ( $f:expr $( , $x:expr )* ) => {
        eprintln!(concat!("! Warning: ", $f), $($x)*);
    };
}

mod chunk;
mod file;
mod filter;
mod idat;
mod ihdr;
mod zlib;

use crate::chunk::ChunkReader;
use crate::file::ByteReader;
use crate::idat::IdatReader;
use crate::ihdr::PartialColorMode;
use crate::ihdr::ColorMode;
use crate::ihdr::InterlaceMethod;
use std::env;
use std::fs::File;
use std::io;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Sdl(String),
    SdlWindow(sdl2::video::WindowBuildError),
    EndOfChunk(Box<[u8]>),
    Format(&'static str),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<sdl2::video::WindowBuildError> for Error {
    fn from(err: sdl2::video::WindowBuildError) -> Self {
        Error::SdlWindow(err)
    }
}

impl From<sdl2::IntegerOrSdlError> for Error {
    fn from(err: sdl2::IntegerOrSdlError) -> Self {
        match err {
            sdl2::IntegerOrSdlError::IntegerOverflows(s, _) => Error::Sdl(s.to_string()),
            sdl2::IntegerOrSdlError::SdlError(s) => Error::Sdl(s),
        }
    }
}

impl From<sdl2::render::TextureValueError> for Error {
    fn from(err: sdl2::render::TextureValueError) -> Self {
        match err {
            sdl2::render::TextureValueError::WidthOverflows(_) => Error::Sdl("Texture width overflow".to_string()),
            sdl2::render::TextureValueError::HeightOverflows(_) => Error::Sdl("Texture height overflow".to_string()),
            sdl2::render::TextureValueError::WidthMustBeMultipleOfTwoForFormat(_, _) =>
                Error::Sdl("Texture width must be a multiple of two for format".to_string()),
            sdl2::render::TextureValueError::SdlError(s) => Error::Sdl(s),
        }
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
    let filename = args.nth(1).unwrap();
    let mut file = File::open(&filename)?;

    let sdl_context = sdl2::init().map_err(Error::Sdl)?;
    let video_subsystem = sdl_context.video().map_err(Error::Sdl)?;

    let sig = file.read_buf(8)?;
    println!("Signature: {:02X?}", sig);
    if *sig != PNG_SIG {
        return Err(Error::Format("Invalid PNG signature"));
    }

    // TODO make interface nicer (don't shadow file)
    let (mut file, width, height, partial_color_mode, interlace_method) = ihdr::load_ihdr(file)?;
    let mut canvas = video_subsystem.window(&filename, width, height).build()?.into_canvas().build()?;
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(sdl2::pixels::PixelFormatEnum::ARGB8888, width, height)?;

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
                chunk = zlib::read_zlib(IdatReader::new(chunk)?, &mut buf)?.end()?;
                texture.with_lock(None,
                    |pixels, pitch| filter::unfilter_uninterlace(&mut buf, pixels, pitch, width, height, &color_mode, interlace_method))
                    .map_err(Error::Sdl)??;
                canvas.copy(&texture, None, None).map_err(Error::Sdl)?;
                canvas.present();
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

    let mut event_pump = sdl_context.event_pump().map_err(Error::Sdl)?;
    'wait: loop {
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit {..} => break 'wait,
                _ => (),
            }
        }
        canvas.copy(&texture, None, None).map_err(Error::Sdl)?;
        canvas.present();
    }

    Ok(())
}
