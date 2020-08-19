use std::io::{Read, Result};

#[derive(Debug)]
pub struct CachedReader<R: Read> {
    reader: R,
    buffer: Vec<u8>,
    offset: usize,
}

impl<R: Read> CachedReader<R> {
    pub fn new(inner: R) -> CachedReader<R> {
        CachedReader {
            reader: inner,
            buffer: vec![],
            offset: 0,
        }
    }

    pub fn cache(&self) -> &[u8] {
        &self.buffer
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: Read> Read for CachedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() > (self.buffer.len() - self.offset) {
            let mut chunk = [0; 128];
            let n = self.reader.read(&mut chunk)?;
            self.buffer.extend_from_slice(&chunk[..n]);
        }

        let len = buf.len().min(self.buffer.len() - self.offset);
        buf[..len].clone_from_slice(&self.buffer[self.offset..self.offset + len]);
        self.offset += len;
        Ok(len)
    }
}
