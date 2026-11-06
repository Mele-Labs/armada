//! The frames a terminal keeper and Fleet speak over its socket. Bytes, not lines: a terminal's
//! output and a person's keys are anything, so every message is a kind, a length and that many bytes.
//!
//! `[kind: u8][length: u32, big endian][payload]`.
//!
//! | From Fleet | Payload |
//! |---|---|
//! | [`ATTACH`] | the most bytes of the spool to replay first, a `u64` |
//! | [`INPUT`] | the keys |
//! | [`RESIZE`] | columns then rows, each a `u16` |
//! | [`END`] | nothing: stop the shell and go |
//!
//! | From the keeper | Payload |
//! |---|---|
//! | [`READY`] | the shell's pid as a `u32`, then `1` where a program other than the shell has the foreground |
//! | [`OUTPUT`] | what the shell printed |
//! | [`FOREGROUND`] | `1` or `0`, sent when it changes |
//! | [`EXIT`] | `1` and an `i32` where the shell left with a code, else `0`: sent once, after all output |

use std::io::{self, Read, Write};

pub const ATTACH: u8 = b'A';
pub const INPUT: u8 = b'I';
pub const RESIZE: u8 = b'R';
pub const END: u8 = b'E';

pub const READY: u8 = b'Y';
pub const OUTPUT: u8 = b'O';
pub const FOREGROUND: u8 = b'F';
pub const EXIT: u8 = b'X';

/// The most a frame carries. A larger one is split by whoever sends it and refused by whoever reads.
pub const MOST: usize = 64 * 1024;

/// A frame, ready to write whole.
pub fn frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut whole = Vec::with_capacity(5 + payload.len());
    whole.push(kind);
    whole.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    whole.extend_from_slice(payload);
    whole
}

/// The frames `bytes` makes, each at most [`MOST`].
pub fn frames(kind: u8, bytes: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    bytes.chunks(MOST).map(move |chunk| frame(kind, chunk))
}

/// Write one frame in a single call, so two writers holding the same lock never interleave.
pub fn write_frame(out: &mut impl Write, kind: u8, payload: &[u8]) -> io::Result<()> {
    out.write_all(&frame(kind, payload))?;
    out.flush()
}

/// The next frame, or `None` where the other end closed between two.
pub fn read_frame(from: &mut impl Read) -> io::Result<Option<(u8, Vec<u8>)>> {
    let mut head = [0u8; 5];
    match from.read_exact(&mut head) {
        Ok(()) => {}
        Err(why) if why.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(why) => return Err(why),
    }
    let length = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if length > MOST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "a frame longer than the most"));
    }
    let mut payload = vec![0u8; length];
    from.read_exact(&mut payload)?;
    Ok(Some((head[0], payload)))
}
