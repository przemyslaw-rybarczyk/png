use crate::file::ByteReader;
use crate::Error;
use crate::Result;
use std::io::Seek;
use std::fs::File;

pub struct ChunkReader {
    file: File,
    length: u32,
    bytes_read: u32,
}

impl ChunkReader {
    pub fn new(mut file: File) -> Result<(ChunkReader, u32, Box<[u8]>)> {
        println!("");
        let length = file.read_u32()?;
        println!("Length: {}", length);
        if length > 0x7FFFFFFF {
            warn!("Length exceeds (2^31)-1");
        }
        let chunk_type = file.read_buf(4)?;
        println!("Chunk type: {:02X?} ({})", chunk_type, String::from_utf8_lossy(&chunk_type));
        Ok((ChunkReader { file, length, bytes_read: 0 }, length, chunk_type))
    }

    pub fn end(mut self) -> Result<File> {
        if self.bytes_read < self.length {
            self.file.seek(std::io::SeekFrom::Current((self.length - self.bytes_read) as i64))?;
            self.bytes_read = self.length;
        }
        self.file.read_u32()?;
        Ok(self.file)
    }
}

impl ByteReader for ChunkReader {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>> {
        if self.bytes_read + len > self.length {
            let buf = self.file.read_buf(self.length - self.bytes_read)?;
            return Err(Error::EndOfChunk(buf));
        }
        let buf = self.file.read_buf(len)?;
        self.bytes_read += len;
        Ok(buf)
    }
}
