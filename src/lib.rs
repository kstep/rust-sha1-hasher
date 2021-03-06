//! A minimal implementation of SHA1 for rust.
//!
//! Example:
//!
//! ```rust
//! extern crate sha1_hasher as sha1;
//! use std::io::Write;
//!
//! # fn main() {
//!
//! let mut m = sha1::Sha1::new();
//! m.write("Hello World!".as_bytes());
//! assert_eq!(&*m.hexdigest(), "2ef7bde608ce5404e97d5f042f95f89f1c232871");
//! # }
//! ```

#![feature(collections)]
#![feature(step_by)]

#![feature(test)]

#[cfg(test)] extern crate test;

use std::io::{Write, BufWriter};
use std::io::Result as IoResult;
use std::default::Default;
use std::mem::transmute;

/// Represents a Sha1 hash object in memory.
#[derive(Clone)]
pub struct Sha1 {
    state: [u32; 5],
    data: Vec<u8>,
    len: u64,
}

const DEFAULT_STATE : [u32; 5] =
    [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];

fn to_hex(input: &[u8]) -> String {
    let mut s = String::new();
    for b in input.iter() {
        s.push_str(&*format!("{:02x}", *b));
    }
    return s;
}

impl Default for Sha1 {
    #[inline]
    fn default() -> Sha1 {
        Sha1::new()
    }
}

impl Write for Sha1 {
    fn write(&mut self, bytes: &[u8]) -> IoResult<usize> {
        let mut d = self.data.clone();
        self.data.clear();

        d.push_all(bytes);

        for chunk in d[..].chunks(64) {
            if chunk.len() == 64 {
                self.len += 64;
                self.process_block(chunk);
            } else {
                self.data.push_all(chunk);
            }
        }

        Ok(bytes.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}


impl Sha1 {

    /// Creates an fresh sha1 hash object.
    pub fn new() -> Sha1 {
        Sha1 {
            state: DEFAULT_STATE,
            data: Vec::with_capacity(256),
            len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.state = DEFAULT_STATE;
        self.data.clear();
        self.len = 0;
    }

    pub fn finish(&self) -> Vec<u8> {
        let mut buf = [0u8; 20].to_vec();
        self.output(&mut *buf);
        buf
    }

    fn process_block(&mut self, block: &[u8]) {
        //assert_eq!(block.len(), 64);

        let mut words = [0u32; 80];
        let mut j = 0;
        for i in (0..block.len()).step_by(4) {
            words[j] = (block[i + 3] as u32) |
                       ((block[i + 2] as u32) << 8) |
                       ((block[i + 1] as u32) << 16) |
                       ((block[i] as u32) << 24);
            j += 1;
        }

        fn ff(b: u32, c: u32, d: u32) -> u32 { d ^ (b & (c ^ d)) }
        fn gg(b: u32, c: u32, d: u32) -> u32 { b ^ c ^ d }
        fn hh(b: u32, c: u32, d: u32) -> u32 { (b & c) | (d & (b | c)) }
        fn ii(b: u32, c: u32, d: u32) -> u32 { b ^ c ^ d }

        for i in (16..80) {
            let n = words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16];
            words[i] = n.rotate_left(1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];

        for i in (0..80) {
            let (f, k) = match i {
                0 ... 19 => (ff(b, c, d), 0x5a827999),
                20 ... 39 => (gg(b, c, d), 0x6ed9eba1),
                40 ... 59 => (hh(b, c, d), 0x8f1bbcdc),
                60 ... 79 => (ii(b, c, d), 0xca62c1d6),
                _ => (0, 0),
            };

            let tmp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(words[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }

    /// Retrieve digest result.  The output must be large enough to
    /// contain result (20 bytes).
    pub fn output(&self, out: &mut [u8]) {
        // these are unlikely to fail, since we're writing to memory
        #![allow(unused_must_use)]

        let mut m = Sha1 {
            state: self.state,
            data: Vec::new(),
            len: 0,
        };

        let mut w = Vec::<u8>::with_capacity(256);
        w.push_all(&*self.data);
        w.push_all(&[0x80u8]);
        let padding = 64 - ((self.data.len() + 9) % 64);
        for _ in (0..padding) {
            w.push(0u8);
        }
        w.push_all(unsafe { &transmute::<_, [u8; 8]>(((self.data.len() as u64 + self.len) * 8).to_be()) });
        for chunk in w[..].chunks(64) {
            m.process_block(chunk);
        }

        let mut w = BufWriter::new(out);
        for &n in m.state.iter() {
            w.write_all(unsafe { &transmute::<_, [u8; 4]>(n.to_be()) });
        }
    }

    pub fn hexdigest(&self) -> String {
        to_hex(&*self.finish())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use test::Bencher;
    use super::{Sha1, to_hex};

    #[test]
    fn test_simple() {
        let mut m = Sha1::new();

        let tests = [
            ("The quick brown fox jumps over the lazy dog",
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"),
            ("The quick brown fox jumps over the lazy cog",
            "de9f2c7fd25e1b3afad3e85a0bd17d9b100db4b3"),
            ("", "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
            ("testing\n", "9801739daae44ec5293d4e1f53d3f4d2d426d91c"),
            ("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            "025ecbd5d70f8fb3c5457cd96bab13fda305dc59"),
        ];

        for &(s, ref h) in tests.iter() {
            let data = s.as_bytes();

            m.reset();
            m.write(data).unwrap();
            let hh = m.hexdigest();

            assert_eq!(hh.len(), h.len());
            assert_eq!(&*hh, *h);
        }
    }

    #[test]
    fn test_dirty_run() {
        let mut m = Sha1::new();

        m.write(b"123").unwrap();
        let out1 = m.finish();

        m.write_all(b"123").unwrap();
        let out2 = m.finish();

        assert!(out1 != out2);
        assert_eq!(&*to_hex(&*out1), "40bd001563085fc35165329ea1ff5c5ecbdbbeef");
        assert_eq!(&*to_hex(&*out2), "601f1889667efaebb33b8c12572835da3f027f78");
    }

    #[bench]
    fn test_sha1_speed(b: &mut Bencher) {
        let mut m = Sha1::new();
        let s = "The quick brown fox jumps over the lazy dog.";
        let n = 1000u64;

        b.bytes = n * s.len() as u64;
        b.iter(|| {
            for _ in (0..n) {
                m.write_all(s.as_bytes()).unwrap();
            }
            //assert_eq!(m.hexdigest(), "7ca27655f67fceaa78ed2e645a81c7f1d6e249d2");
        });
    }
}
