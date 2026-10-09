//! Fleet's `/events` WebSocket, read one text message at a time.
//!
//! **Server frames only.** Fleet sends unmasked text frames and never asks
//! anything of the client, so this reads text, continuation, ping/pong and
//! close, and ends the stream on anything else.

use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Messages<R> {
    io: R,
}

impl<R: AsyncRead + Unpin> Messages<R> {
    pub fn new(io: R) -> Self {
        Messages { io }
    }

    /// The next text message, or `None` when Fleet closed or sent something
    /// this does not read.
    pub async fn next(&mut self) -> Option<String> {
        let mut text = Vec::new();
        loop {
            let mut head = [0u8; 2];
            self.io.read_exact(&mut head).await.ok()?;
            let (last, opcode) = (head[0] & 0x80 != 0, head[0] & 0x0f);
            if head[1] & 0x80 != 0 {
                return None;
            }
            let length = match head[1] & 0x7f {
                126 => {
                    let mut two = [0u8; 2];
                    self.io.read_exact(&mut two).await.ok()?;
                    u16::from_be_bytes(two) as u64
                }
                127 => {
                    let mut eight = [0u8; 8];
                    self.io.read_exact(&mut eight).await.ok()?;
                    u64::from_be_bytes(eight)
                }
                short => short as u64,
            };
            if length > 16 * 1024 * 1024 {
                return None;
            }
            let mut payload = vec![0u8; length as usize];
            self.io.read_exact(&mut payload).await.ok()?;
            match opcode {
                0x1 | 0x0 => {
                    text.extend_from_slice(&payload);
                    if last {
                        return String::from_utf8(text).ok();
                    }
                }
                0x9 | 0xa => {}
                _ => return None,
            }
        }
    }
}
