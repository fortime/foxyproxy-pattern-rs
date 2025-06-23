use std::io::{self, Read, Write};

use base64::{engine::general_purpose::STANDARD, read::DecoderReader, write::EncoderWriter};

use crate::Encoding;

struct FromHexReader<R> {
    r: R,
    buf: [u8; 4096],
    buf_offset: usize,
    buf_filled: usize,
}

impl<R> FromHexReader<R> {
    pub fn new(r: R) -> Self {
        Self {
            r,
            buf: [0; 4096],
            buf_offset: 0,
            buf_filled: 0,
        }
    }
}

impl<R> Read for FromHexReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut second = false;
        loop {
            if self.buf_filled < self.buf.len() {
                let len = self.r.read(&mut self.buf[self.buf_filled..])?;
                if len == 0 && second {
                    break Err(io::Error::other("incomplete hex string"));
                }
                self.buf_filled += len;
            }
            let mut count = 0;
            while self.buf_offset + 1 < self.buf_filled && count < self.buf.len() {
                buf[count] = hex_to_u8(self.buf[self.buf_offset], self.buf[self.buf_offset + 1])?;
                self.buf_offset += 2;
                count += 1;
            }
            if self.buf_offset + 1 >= self.buf_filled {
                self.buf_filled = if self.buf_offset == self.buf_filled {
                    0
                } else {
                    self.buf[0] = self.buf[self.buf_filled - 1];
                    1
                };
                self.buf_offset = 0;
            }
            if count == 0 && self.buf_filled > 0 {
                // there should be more incoming data.
                second = true;
                continue;
            }
            break Ok(count);
        }
    }
}

struct SkipSpaceReader<R> {
    r: R,
}

impl<R> SkipSpaceReader<R> {
    pub fn new(r: R) -> Self {
        Self { r }
    }
}

impl<R> Read for SkipSpaceReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let len = self.r.read(buf)?;
            if len == 0 {
                break Ok(0);
            }
            let mut fast = 0;
            let mut slow = 0;
            while fast < len {
                if !buf[fast].is_ascii_whitespace() {
                    buf[slow] = buf[fast];
                    slow += 1;
                }
                fast += 1;
            }
            if slow != 0 {
                break Ok(slow);
            }
        }
    }
}

struct ToHexWriter<W> {
    w: W,
    buf: [u8; 4096],
    buf_offset: usize,
    buf_filled: usize,
}

impl<W> ToHexWriter<W> {
    pub fn new(w: W) -> Self {
        Self {
            w,
            buf: [0; 4096],
            buf_offset: 0,
            buf_filled: 0,
        }
    }
}

impl<W> Write for ToHexWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut read = 0;
        while read < buf.len() && self.buf_filled < self.buf.len() {
            self.buf[self.buf_filled] = u8_to_hex_char(buf[read] >> 4);
            self.buf[self.buf_filled + 1] = u8_to_hex_char(buf[read]);
            self.buf_filled += 2;
            read += 1;
        }
        let len = self.w.write(&self.buf[self.buf_offset..self.buf_filled])?;
        self.buf_offset += len;
        if self.buf_offset == self.buf_filled {
            self.buf_offset = 0;
            self.buf_filled = 0;
        }
        Ok(read)
    }

    fn flush(&mut self) -> io::Result<()> {
        while self.buf_offset < self.buf_filled {
            self.w.flush()?;
            self.buf_offset += self.w.write(&self.buf[self.buf_offset..self.buf_filled])?;
        }

        self.buf_offset = 0;
        self.buf_filled = 0;

        self.w.flush()
    }
}

fn hex_to_u8(high: u8, low: u8) -> io::Result<u8> {
    Ok((hex_char_to_u8(high)? << 4) + hex_char_to_u8(low)?)
}

fn hex_char_to_u8(c: u8) -> io::Result<u8> {
    if c.is_ascii_digit() {
        Ok(c - b'0')
    } else if (b'a'..=b'f').contains(&c) {
        Ok(c - b'a' + 10)
    } else if (b'A'..=b'F').contains(&c) {
        Ok(c - b'A' + 10)
    } else {
        Err(io::Error::other(format!(
            "unexpected char from hex string: {}",
            c
        )))
    }
}

fn u8_to_hex_char(b: u8) -> u8 {
    let b = b & 0x0f;
    if b < 10 {
        b + b'0'
    } else {
        b - 10 + b'a'
    }
}

pub fn decode<R>(r: R, encoding: Encoding) -> Box<dyn Read>
where
    R: Read + 'static,
{
    match encoding {
        Encoding::Raw => Box::new(r) as Box<_>,
        Encoding::Base64 => {
            Box::new(DecoderReader::new(SkipSpaceReader::new(r), &STANDARD)) as Box<_>
        }
        Encoding::Hex => Box::new(FromHexReader::new(r)) as Box<_>,
    }
}

pub fn encode<W>(w: W, encoding: Encoding) -> Box<dyn Write>
where
    W: Write + 'static,
{
    match encoding {
        Encoding::Raw => Box::new(w) as Box<_>,
        Encoding::Base64 => Box::new(EncoderWriter::new(w, &STANDARD)) as Box<_>,
        Encoding::Hex => Box::new(ToHexWriter::new(w)) as Box<_>,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{bail, Result};

    use super::hex_to_u8;

    fn decode_hex_string(s: &str) -> Result<String> {
        let bs = s.as_bytes();
        let mut i = 0;
        let mut res = vec![];
        while i < bs.len() {
            if i + 1 == bs.len() {
                bail!("invalid hex string");
            }
            res.push(hex_to_u8(bs[i], bs[i + 1])?);
            i += 2;
        }
        Ok(String::from_utf8(res)?)
    }

    #[test]
    fn test_hex_to_u8() {
        let raw = "Hello world!";
        assert_eq!(
            decode_hex_string("48656C6C6F20776F726C6421").expect("failed to decode hex string"),
            raw
        );
        assert_eq!(
            decode_hex_string("48656c6c6f20776f726c6421").expect("failed to decode hex string"),
            raw
        );
    }
}
