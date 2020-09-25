use crate::Result;
use std::io::Read;
use std::fs::File;

pub trait ByteReader {
    fn read_buf(&mut self, len: usize) -> Result<Box<[u8]>>;

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
    fn read_buf(&mut self, len: usize) -> Result<Box<[u8]>> {
        let mut buf = vec![0; len].into_boxed_slice();
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}
