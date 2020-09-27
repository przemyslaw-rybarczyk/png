use crate::Result;
use std::io::Read;
use std::fs::File;

pub trait ByteReader {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>>;

    fn read_u8(&mut self) -> Result<u8> {
        let buf = self.read_buf(1)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let buf = self.read_buf(2)?;
        Ok(((buf[0] as u16) << 8) | ((buf[1] as u16) << 0))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let buf = self.read_buf(4)?;
        Ok(((buf[0] as u32) << 24) | ((buf[1] as u32) << 16) | ((buf[2] as u32) << 8) | ((buf[3] as u32) << 0))
    }
}

impl ByteReader for File {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>> {
        let mut buf = vec![0; len as usize].into_boxed_slice();
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}

pub struct BitReader<T> where T: ByteReader {
    reader: T,
    byte: u8,
    bits_left: u8,
}

impl<T> BitReader<T> where T: ByteReader {
    pub fn new(reader: T) -> BitReader<T> {
        BitReader { reader, byte: 0, bits_left: 0 }
    }

    pub fn end(self) -> T {
        self.reader
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        if self.bits_left == 0 {
            self.byte = self.reader.read_u8()?;
            self.bits_left = 8;
        }
        let bit = self.byte & 1 != 0;
        self.byte >>= 1;
        self.bits_left -= 1;
        Ok(bit)
    }

    pub fn read_bits(&mut self, len: u8) -> Result<u16> {
        if self.bits_left >= len {
            let bits = self.byte & ((1 << len) - 1);
            self.bits_left -= len;
            self.byte = u8::wrapping_shr(self.byte, len as u32);
            Ok(bits as u16)
        } else if self.bits_left + 8 >= len {
            let bits = self.byte as u16;
            let byte = self.reader.read_u8()?;
            let bits = (bits | ((byte as u16) << self.bits_left)) & ((1 << len) - 1);
            self.byte = u8::wrapping_shr(byte, (len - self.bits_left) as u32);
            self.bits_left = self.bits_left + 8 - len;
            Ok(bits)
        } else {
            let bits = self.byte as u16;
            let byte1 = self.reader.read_u8()?;
            let byte2 = self.reader.read_u8()?;
            let bits = (bits | ((byte1 as u16) << self.bits_left) | ((byte2 as u16) << (8 + self.bits_left))) & ((1 << len) - 1);
            self.byte = u8::wrapping_shr(byte2, (len - self.bits_left - 8) as u32);
            self.bits_left = self.bits_left + 16 - len;
            Ok(bits)
        }
    }
}

impl<T> ByteReader for BitReader<T> where T: ByteReader {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>> {
        self.bits_left = 0;
        self.reader.read_buf(len)
    }
}
