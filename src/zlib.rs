use crate::file::ByteReader;
use crate::idat::IdatReader;
use crate::Error;
use crate::Result;

struct BitReader<T> where T: ByteReader {
    reader: T,
    byte: u8,
    bits_left: u8,
}

impl<T> BitReader<T> where T: ByteReader {
    fn new(reader: T) -> BitReader<T> {
        BitReader { reader, byte: 0, bits_left: 0 }
    }

    fn end(self) -> T {
        self.reader
    }

    fn read_bit(&mut self) -> Result<bool> {
        if self.bits_left == 0 {
            self.byte = self.reader.read_u8()?;
            self.bits_left = 8;
        }
        let bit = self.byte & 1 != 0;
        self.byte >>= 1;
        self.bits_left -= 1;
        Ok(bit)
    }
}

impl<T> ByteReader for BitReader<T> where T: ByteReader {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>> {
        self.bits_left = 0;
        self.reader.read_buf(len)
    }
}

enum BlockType {
    Uncompressed(u16),
    FixedHuffman,
    DynamicHuffman,
    EndOfFile,
}

pub struct ZlibReader {
    idat: BitReader<IdatReader>,
    block_final: bool,
    block_type: BlockType,
}

fn next_block(idat: &mut BitReader<IdatReader>) -> Result<(bool, BlockType)> {
    let bfinal = idat.read_bit()?;
    let btype0 = idat.read_bit()?;
    let btype1 = idat.read_bit()?;
    println!("Final block: {}", bfinal as u32);
    println!("Block type: {}{}", btype1 as u32, btype0 as u32);
    let btype = match (btype1, btype0) {
        (false, false) => {
            let len = idat.read_u16()?;
            println!("Block length: {}", len);
            let nlen = idat.read_u16()?;
            println!("One's complement of block's length: {}", nlen);
            if !len != nlen {
                warn!("One's complement of block length is incorrect");
            }
            BlockType::Uncompressed(len)
        },
        (false, true) => BlockType::FixedHuffman,
        (true, false) => BlockType::DynamicHuffman, // TODO read codes
        (true, true) => return Err(Error::Format("Invalid block type")),
    };
    Ok((bfinal, btype))
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
        let mut idat = BitReader::new(idat);
        let (block_final, block_type) = next_block(&mut idat)?;
        Ok(ZlibReader { idat, block_final, block_type })
    }

    pub fn end(self) -> Result<IdatReader> {
        Ok(self.idat.end())
    }
}
