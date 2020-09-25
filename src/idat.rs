use crate::chunk::ChunkReader;
use crate::file::ByteReader;
use crate::Error;
use crate::Result;

pub struct IdatReader {
    chunk: ChunkReader,
}

impl IdatReader {
    pub fn new(chunk: ChunkReader) -> Result<IdatReader> {
        Ok(IdatReader { chunk })
    }

    pub fn end(self) -> Result<ChunkReader> {
        Ok(self.chunk)
    }
}

impl ByteReader for IdatReader {
    fn read_buf(&mut self, len: u32) -> Result<Box<[u8]>> {
        match self.chunk.read_buf(len) {
            Err(Error::EndOfChunk(data)) => {
                let len = len as usize;
                let mut buf = vec![0; len].into_boxed_slice();
                for i in 0 .. len {
                    buf[i] = data[i];
                }
                let mut bytes_read = data.len();
                while bytes_read < len {
                    let chunk_type = unsafe {
                        let old_chunk = std::ptr::read(&mut self.chunk);
                        let (new_chunk, _, chunk_type) = ChunkReader::new(old_chunk.end()?)?;
                        std::ptr::write(&mut self.chunk, new_chunk);
                        chunk_type
                    };
                    if *chunk_type != *b"IDAT" {
                        let mut data = vec![0; bytes_read].into_boxed_slice();
                        for i in 0 .. bytes_read {
                            data[i] = buf[i];
                        }
                        return Err(Error::EndOfChunk(data));
                    }
                    match self.chunk.read_buf((len - bytes_read) as u32) {
                        Ok(data) | Err(Error::EndOfChunk(data)) => {
                            for i in 0 .. len {
                                buf[bytes_read + i] = data[i];
                            }
                            bytes_read += data.len();
                        },
                        result => return result,
                    }
                }
                Ok(buf)
            },
            result => result,
        }
    }
}
