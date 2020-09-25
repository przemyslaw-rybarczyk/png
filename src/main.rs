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

pub fn format_error<T, E>(error: E) -> io::Result<T>
where E: Into<Box<dyn std::error::Error + Send + Sync>>, {
    Err(io::Error::new(io::ErrorKind::InvalidData, error))
}

const PNG_SIG : [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

fn main() -> io::Result<()> {
    let mut args = env::args();
    if args.len() != 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid number of arguments"));
    }
    let mut file = File::open(args.nth(1).unwrap())?;
    let sig = file.read_buf(8)?;
    println!("Signature: {:02X?}", sig);
    if *sig != PNG_SIG {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid PNG signature"));
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
        chunk.end();
        if *chunk_type == *b"IEND" {
            // TODO check for EOF
            break;
        }
    }

    Ok(())
}
