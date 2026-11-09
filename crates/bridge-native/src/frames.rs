//! Fleet's `/events` WebSocket, read one text message at a time, and what a message says.
//!
//! **Server frames only.** Fleet sends unmasked text frames and never asks anything of the
//! client, so this reads text, continuation, ping/pong and close, and ends the stream on
//! anything else (`crates/pocket/src/fleet_events.rs`).

use tokio::io::{AsyncRead, AsyncReadExt};

use ipc::StreamMessage;

pub struct Messages<R> {
    io: R,
}

impl<R: AsyncRead + Unpin> Messages<R> {
    pub fn new(io: R) -> Self {
        Messages { io }
    }

    /// The next text message, or `None` when Fleet closed or sent something this does not read.
    /// **Not cancel-safe**: a caller that races it against a timer polls the same future again.
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

/// One message read as the wire type. A message this build cannot read is the caller's to act on.
pub fn decode(text: &str) -> Result<StreamMessage, String> {
    ipc::decode("stream message", text.as_bytes()).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(opcode: u8, last: bool, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![if last { 0x80 } else { 0 } | opcode];
        match payload.len() {
            len if len < 126 => out.push(len as u8),
            len if len <= u16::MAX as usize => {
                out.push(126);
                out.extend_from_slice(&(len as u16).to_be_bytes());
            }
            len => {
                out.push(127);
                out.extend_from_slice(&(len as u64).to_be_bytes());
            }
        }
        out.extend_from_slice(payload);
        out
    }

    #[tokio::test]
    async fn text_frames_of_every_length_form_are_read() {
        let medium = "m".repeat(300);
        let long = "l".repeat(70_000);
        let mut wire = frame(0x1, true, b"short");
        wire.extend(frame(0x1, true, medium.as_bytes()));
        wire.extend(frame(0x1, true, long.as_bytes()));
        let mut messages = Messages::new(wire.as_slice());
        assert_eq!(messages.next().await.as_deref(), Some("short"));
        assert_eq!(messages.next().await, Some(medium));
        assert_eq!(messages.next().await, Some(long));
        assert_eq!(messages.next().await, None);
    }

    #[tokio::test]
    async fn continuations_join_and_pings_are_skipped() {
        let mut wire = frame(0x1, false, b"hel");
        wire.extend(frame(0x9, true, b"ping"));
        wire.extend(frame(0x0, true, b"lo"));
        let mut messages = Messages::new(wire.as_slice());
        assert_eq!(messages.next().await.as_deref(), Some("hello"));
    }

    #[tokio::test]
    async fn close_masked_and_binary_frames_end_the_stream() {
        for wire in [frame(0x8, true, b""), frame(0x2, true, b"x"), vec![0x81, 0x81, 0, 0, 0, 0, b'x']] {
            assert_eq!(Messages::new(wire.as_slice()).next().await, None);
        }
    }

    #[test]
    fn the_three_stream_messages_decode() {
        let resync = decode(r#"{"message":"resync","protocol_id":"62579657051166d8","cursor":7}"#);
        assert!(matches!(resync, Ok(StreamMessage::Resync(read)) if read.cursor.position() == 7));
        let missed = decode(r#"{"message":"missed","dropped":3}"#);
        assert!(matches!(missed, Ok(StreamMessage::Missed(read)) if read.dropped == 3));
        let event = decode(
            r#"{"message":"event","cursor":8,"event":{"kind":"repositories.changed","repositories":[]}}"#,
        );
        let Ok(StreamMessage::Event(event)) = event else { panic!("not an event") };
        assert_eq!((event.cursor.position(), event.event.kind().as_str()), (8, "repositories.changed"));
        assert!(decode(r#"{"message":"nonsense"}"#).is_err());
        assert!(decode("not json").is_err());
    }
}
