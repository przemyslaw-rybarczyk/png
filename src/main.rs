#[macro_use]
macro_rules! warn {
    ( $f:expr $( , $x:expr )* ) => {
        eprintln!(concat!("! Warning: ", $f), $($x)*);
    };
}

mod chunk;
mod file;
mod ihdr;

use crate::chunk::ChunkReader;
use crate::file::ByteReader;
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

    let (width, height, partial_color_mode, interlace_method) = ihdr::load_ihdr(&mut file)?;

    loop {
        let (chunk, length, chunk_type) = ChunkReader::new(&mut file)?;
        match &*chunk_type {
            b"IHDR" => {
                warn!("Multiple IHDR chunks");
            },
            b"IDAT" => {
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
        chunk.end()?;
        if *chunk_type == *b"IEND" {
            // TODO check for EOF
            break;
        }
    }

    Ok(())
}
