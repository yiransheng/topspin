use std::convert::TryInto;
use std::io::{self, Read, StderrLock, StdoutLock, Write};
use std::mem::size_of;
use std::net::{TcpStream};

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

    fn advance(&mut self, len: usize) {
        debug_assert!(self.write_cursor + len < self.inner.len());
        self.write_cursor += len;
    }

    fn write_buffer(&mut self) -> &mut [u8] {
        &mut self.inner[self.write_cursor..]
    }

    fn read_frame<'a>(&'a mut self) -> Option<Frame<'a>> {
        let r = self.read_cursor;
        let w = self.write_cursor;
        let bytes = &self.inner[r..w];
        let (frame, len) = Frame::parse(bytes)?;
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
        let len: u64 = u64::from_be_bytes((&bytes[1..Frame::PREFIX_SIZE]).try_into().unwrap());

        let data_start = Frame::PREFIX_SIZE;
        let data_end = data_start + (len as usize);
        if data_end > bytes.len() {
            return None;
        }
        let frame = match [tag] {
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
