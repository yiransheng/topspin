use std::convert::TryInto;
use std::io::{self, Read, StderrLock, StdoutLock, Write};
use std::mem::size_of;
use std::net::TcpStream;

use crate::constants::{STDERR_TAG, STDOUT_TAG};

pub fn run_log_client(alias: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:9527")?;
    stream.write(alias.trim().as_bytes())?;
    stream.write(b"\r\n")?;
    stream_logs(stream)
}

fn stream_logs<R: Read>(mut input_stream: R) -> io::Result<()> {
    let out = io::stdout();
    let err = io::stderr();
    let mut out = out.lock();
    let mut err = err.lock();

    let mut buf = Buffer::new();
    loop {
        let len = input_stream.read(buf.write_buffer())?;
        if len == 0 {
            return Ok(());
        }
        buf.advance(len);
        if let Some(frame) = buf.read_frame() {
            frame.write_to(&mut out, &mut err)?;
        }
    }
}

struct Buffer {
    inner: [u8; 4096],
    read_cursor: usize,
    write_cursor: usize,
}

impl Buffer {
    fn new() -> Self {
        Self {
            inner: [0; 4096],
            read_cursor: 0,
            write_cursor: 0,
        }
    }

    #[inline(always)]
    fn advance(&mut self, len: usize) {
        debug_assert!(self.write_cursor + len <= self.inner.len());
        self.write_cursor += len;
        if self.read_cursor > 0 && self.write_cursor == self.inner.len() {
            // Buffer full and has space to cleanup (bytes already consumed)
            let len = self.write_cursor - self.read_cursor;
            let mut new_inner = [0; 4096];
            (&mut new_inner[0..len])
                .clone_from_slice(&self.inner[self.read_cursor..self.write_cursor]);
            self.inner = new_inner;
            self.read_cursor = 0;
            self.write_cursor = len;
        }
    }

    fn write_buffer(&mut self) -> &mut [u8] {
        &mut self.inner[self.write_cursor..]
    }

    fn read_frame<'a>(&'a mut self) -> Option<Frame<'a>> {
        let r = self.read_cursor;
        let w = self.write_cursor;
        let bytes = &self.inner[r..w];
        let (frame, len) = Frame::parse(bytes)?;
        assert!(len <= 1024);
        if r + len == w {
            self.read_cursor = 0;
            self.write_cursor = 0;
        } else {
            self.read_cursor = r + len;
        }
        Some(frame)
    }
}

enum Frame<'a> {
    Stdout(&'a [u8]),
    Stderr(&'a [u8]),
}

impl<'a> Frame<'a> {
    const PREFIX_SIZE: usize = size_of::<u8>() + size_of::<u64>();

    // (Frame, bytes len consumed)
    fn parse<'b>(bytes: &'b [u8]) -> Option<(Frame<'b>, usize)> {
        if bytes.len() < Self::PREFIX_SIZE {
            return None;
        }
        let tag: u8 = bytes[0];
        let len: u64 = u64::from_le_bytes((&bytes[1..Frame::PREFIX_SIZE]).try_into().unwrap());

        let data_start = Frame::PREFIX_SIZE;
        let data_end = data_start + (len as usize);
        if data_end > bytes.len() {
            return None;
        }
        let frame = match tag {
            STDOUT_TAG => Frame::Stdout(&bytes[data_start..data_end]),
            STDERR_TAG => Frame::Stderr(&bytes[data_start..data_end]),
            _ => panic!("Invalid tag"),
        };

        Some((frame, data_end))
    }

    fn write_to(self, out: &mut StdoutLock, err: &mut StderrLock) -> io::Result<()> {
        match self {
            Frame::Stdout(bytes) => out.write_all(bytes),
            Frame::Stderr(bytes) => err.write_all(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    struct FrameMeta {
        tag: u8,
        len: usize,
    }

    impl Arbitrary for FrameMeta {
        fn arbitrary<G: Gen>(g: &mut G) -> FrameMeta {
            FrameMeta {
                tag: if bool::arbitrary(g) {
                    STDOUT_TAG
                } else {
                    STDERR_TAG
                },
                len: usize::arbitrary(g) % 1024,
            }
        }
    }
    impl FrameMeta {
        fn len(&self) -> usize {
            if self.tag > 0 {
                self.len + 1
            } else {
                self.len
            }
        }
    }

    impl Read for FrameMeta {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let nread = std::cmp::min(self.len(), buf.len());
            if self.tag > 0 && nread > 0 {
                buf[0] = self.tag;
                self.tag = 0;
                self.len -= nread - 1;
            } else {
                self.len -= nread;
            }
            Ok(nread)
        }
    }

    struct ChunkIter<'a, I> {
        source: &'a [u8],
        size_iter: I,
    }

    impl<'a, I> Iterator for ChunkIter<'a, I>
    where
        I: Iterator<Item = usize>,
    {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<Self::Item> {
            let mid = self.size_iter.next()?;
            let mid = mid % 1024 + 1;
            if mid > self.source.len() {
                self.source = &[];
                None
            } else {
                let (item, remaining) = self.source.split_at(mid);
                self.source = remaining;
                Some(item)
            }
        }
    }

    #[quickcheck]
    fn test_buffer(sizes: Vec<usize>, frames: Vec<FrameMeta>) -> bool {
        let mut all_bytes: Vec<u8> = vec![0; 1024 * 32];
        let mut bytes = &mut all_bytes[..];
        for mut frame in frames {
            let len = frame.read(bytes).unwrap();
            if len == 0 {
                break;
            }
            let (_, b) = bytes.split_at_mut(len);
            bytes = b;
        }

        let chunks = ChunkIter {
            source: &all_bytes[..],
            size_iter: sizes.into_iter(),
        };
        let mut buffer = Buffer::new();
        for mut slice in chunks {
            let nread = slice.read(buffer.write_buffer()).unwrap();
            if nread == 0 {
                return false;
            }
            buffer.advance(nread);
            buffer.read_frame();
        }
        true
    }
}
