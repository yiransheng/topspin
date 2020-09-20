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
        let buf_full = buf.full();
        let nread = input_stream.read(buf.available())?;
        buf.advance(nread);
        if let Some(frame) = buf.read_frame()? {
            frame.write_to(&mut out, &mut err)?;
        }
        if nread == 0 && !buf_full {
            return Ok(());
        }
    }
}

#[derive(Debug)]
struct Buffer {
    inner: Box<[u8]>,
    read_cursor: usize,
    write_cursor: usize,
}

impl Buffer {
    const DEFAULT_SIZE: usize = 4096;

    fn new() -> Self {
        Self {
            inner: Box::new([0; Self::DEFAULT_SIZE]),
            read_cursor: 0,
            write_cursor: 0,
        }
    }

    fn with_capacity(cap: usize) -> Option<Self> {
        let capacity = match cap.checked_next_power_of_two() {
            Some(capacity) => capacity,
            _ => return None,
        };
        Some(Self {
            inner: vec![0; capacity].into_boxed_slice(),
            read_cursor: 0,
            write_cursor: 0,
        })
    }

    fn advance(&mut self, len: usize) {
        debug_assert!(self.write_cursor + len <= self.inner.len());
        self.write_cursor += len;
        if self.read_cursor > 0 && self.full() {
            // Buffer full and has space to cleanup (bytes already consumed)
            let unprocessed_data = self.read_cursor..self.write_cursor;
            (&mut self.inner).copy_within(unprocessed_data, 0);
            self.write_cursor -= self.read_cursor;
            self.read_cursor = 0;
        }
    }

    fn full(&self) -> bool {
        self.write_cursor == self.inner.len()
    }

    fn available(&mut self) -> &mut [u8] {
        &mut self.inner[self.write_cursor..]
    }

    fn read_frame<'a>(&'a mut self) -> io::Result<Option<Frame<'a>>> {
        let r = self.read_cursor;
        let w = self.write_cursor;
        let bytes = &self.inner[r..w];
        let (frame, len) = match Frame::parse(bytes).map_err::<io::Error, _>(Into::into)? {
            Some(x) => x,
            None => return Ok(None),
        };
        assert!(len <= 1024);
        if r + len == w {
            self.read_cursor = 0;
            self.write_cursor = 0;
        } else {
            self.read_cursor = r + len;
        }
        Ok(Some(frame))
    }
}

enum Frame<'a> {
    Stdout(&'a [u8]),
    Stderr(&'a [u8]),
}

#[derive(Debug, Copy, Clone)]
struct FrameError;

impl Into<io::Error> for FrameError {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::Other, "Invalid Frame Tag")
    }
}

impl<'a> Frame<'a> {
    const PREFIX_SIZE: usize = size_of::<u8>() + size_of::<u16>();

    // (Frame, bytes len consumed)
    fn parse<'b>(bytes: &'b [u8]) -> Result<Option<(Frame<'b>, usize)>, FrameError> {
        if bytes.len() < Self::PREFIX_SIZE {
            return Ok(None);
        }
        let tag: u8 = bytes[0];
        let len: u16 = u16::from_le_bytes((&bytes[1..Frame::PREFIX_SIZE]).try_into().unwrap());

        let data_start = Frame::PREFIX_SIZE;
        let data_end = data_start + (len as usize);
        if data_end > bytes.len() {
            return Ok(None);
        }
        let frame = match tag {
            STDOUT_TAG => Frame::Stdout(&bytes[data_start..data_end]),
            STDERR_TAG => Frame::Stderr(&bytes[data_start..data_end]),
            _ => return Err(FrameError),
        };

        Ok(Some((frame, data_end)))
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

    const FRAME_MAX: usize = 16;

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    struct FrameMeta {
        tag: u8,
        data_len: usize,
    }
    struct ChunkIter<'a, I> {
        source: &'a [u8],
        size_iter: I,
    }

    #[quickcheck]
    fn test_buffer(frames: Vec<FrameMeta>, chunk_sizes: Vec<usize>) -> bool {
        let passed = check_buffer(frames, chunk_sizes);
        passed
    }

    #[test]
    fn test_buffer_debug() {
        // Shunk test data produced by quickcheck from the last failed run.
        let passed = check_buffer(
            vec![
                FrameMeta {
                    tag: 2,
                    data_len: 11,
                },
                FrameMeta {
                    tag: 2,
                    data_len: 13,
                },
                FrameMeta {
                    tag: 2,
                    data_len: 2,
                },
                FrameMeta {
                    tag: 1,
                    data_len: 15,
                },
                FrameMeta {
                    tag: 1,
                    data_len: 10,
                },
            ],
            vec![40, 7, 19, 45, 84, 11, 6, 13, 0],
        );
        assert!(passed);
    }

    fn check_buffer(frames: Vec<FrameMeta>, chunk_sizes: Vec<usize>) -> bool {
        if frames.is_empty() || chunk_sizes.is_empty() {
            return true;
        }
        let mut all_bytes: Vec<u8> = vec![0; FRAME_MAX * 64];
        let mut bytes = &mut all_bytes[..];
        for mut frame in frames {
            let len = frame.read(bytes).unwrap();
            if len == 0 {
                break;
            }
            let (_, b) = bytes.split_at_mut(len);
            bytes = b;
        }
        let unwritten = bytes.len();
        let data_len = all_bytes.len() - unwritten;

        let chunks = ChunkIter {
            source: &all_bytes[..data_len],
            size_iter: chunk_sizes.into_iter(),
        };
        let mut buffer = Buffer::with_capacity(FRAME_MAX * 2).unwrap();
        for mut slice in chunks {
            // Drain the slice before moving on to the next chunk.
            while !slice.is_empty() {
                buffer.advance(0);
                if buffer.full() {
                    // Buffer should never be full (since it is sized 2*MAC_FRAME size)
                    return false;
                }
                let nread = slice.read(buffer.available()).unwrap();
                buffer.advance(nread);
                if buffer.read_frame().is_err() {
                    // Should never read illegal tag byte.
                    return false;
                }
            }
        }
        true
    }

    impl Arbitrary for FrameMeta {
        fn arbitrary<G: Gen>(g: &mut G) -> FrameMeta {
            FrameMeta {
                tag: if bool::arbitrary(g) {
                    STDOUT_TAG
                } else {
                    STDERR_TAG
                },
                data_len: usize::arbitrary(g) % (FRAME_MAX - Self::header_len()),
            }
        }
    }

    impl FrameMeta {
        fn len(&self) -> usize {
            self.data_len + Self::header_len()
        }
        fn header_len() -> usize {
            size_of::<u8>() + size_of::<u16>()
        }
    }

    impl Read for FrameMeta {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.tag == 0 {
                return Ok(0);
            }
            let h = Self::header_len();
            let nread = match buf.len() {
                0 => 0,
                n if n <= h => {
                    buf[0] = self.tag;
                    buf[1..n].copy_from_slice(&(self.data_len as u16).to_le_bytes()[..(n - 1)]);
                    n
                }
                _ => {
                    buf[0] = self.tag;
                    buf[1..h].copy_from_slice(&(self.data_len as u16).to_le_bytes()[..]);
                    std::cmp::min(self.len(), buf.len())
                }
            };
            Ok(nread)
        }
    }

    impl<'a, I> Iterator for ChunkIter<'a, I>
    where
        I: Iterator<Item = usize>,
    {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<Self::Item> {
            let mid = self.size_iter.next()?;
            let mid = std::cmp::max(mid % FRAME_MAX, 1);
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
}
