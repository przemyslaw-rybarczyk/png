use crate::file::ByteReader;
use crate::idat::IdatReader;
use crate::Error;
use crate::Result;

pub struct ZlibReader {
    idat: IdatReader,
}

impl ZlibReader {
    pub fn new(mut idat: IdatReader) -> Result<ZlibReader> {
        let cmf = idat.read_u8()?;
        println!("Compression method: {}", cmf & 0xF);
        if cmf & 0xF != 0x8 {
            return Err(Error::Format("Unrecognized compression method"));
        }
        println!("Compression window size: {}", 1 << ((cmf >> 4) as u32 + 8));
        if cmf >> 4 > 7 {
            warn!("Compression window size above 32K");
        }
        let flags = idat.read_u8()?;
        println!("Check bits: {:02X}", flags & 0x1F);
        if (((cmf as u16) << 8) + flags as u16) % 31 != 0 {
            warn!("Check bits are incorrect");
        }
        println!("Preset dictionary: {}", (flags & 0x20) >> 5);
        if flags & 0x20 != 0 {
            return Err(Error::Format("Preset dictionary set"));
        }
        println!("Compression level: {}", flags >> 6);
        Ok(ZlibReader { idat })
    }

    pub fn end(self) -> Result<IdatReader> {
        Ok(self.idat)
    }
}
