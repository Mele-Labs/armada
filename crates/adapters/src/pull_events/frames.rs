//! A pull request's event stream as frames: bytes in, what each frame says out. **Pure**, so
//! recorded streams in the tests stand for the network.
//!
//! A frame is `id:`, `event:` and `data:` lines closed by a blank line. A line starting with `:` is
//! a heartbeat and says nothing. `data` is one CloudEvent, whose own `data` holds the pull
//! request's `api_url`, or on the last frame the stream's `outcome`.

use serde::Deserialize;

/// The last frame's type: after it the stream says nothing more.
const END: &str = "github.stream.end";
/// What the stream says about itself rather than about a pull request.
const CONTROL: &str = "github.stream.";

/// What a frame says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// Something about a pull request moved.
    Event { kind: String, api_url: Option<String> },
    /// The stream is over, and how.
    End(Outcome),
}

/// What the last frame said the stream came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    SubscriptionEnded,
    Reconnect,
    /// Said in words this does not know, or in none: read as a reconnect.
    Unknown(String),
}

/// Frames read as bytes arrive, in whatever pieces they come in.
#[derive(Debug, Default)]
pub(crate) struct Frames {
    /// Bytes of a line not yet ended.
    line: Vec<u8>,
    kind: Option<String>,
    data: Vec<String>,
}

#[derive(Deserialize)]
struct Cloud {
    outcome: Option<String>,
    data: Option<Inside>,
}

#[derive(Deserialize)]
struct Inside {
    api_url: Option<String>,
    outcome: Option<String>,
}

impl Frames {
    /// What `bytes` completed.
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<Frame> {
        let mut frames = Vec::new();
        for &byte in bytes {
            if byte != b'\n' {
                self.line.push(byte);
                continue;
            }
            let line = std::mem::take(&mut self.line);
            let line = String::from_utf8_lossy(&line);
            if let Some(frame) = self.take(line.trim_end_matches('\r')) {
                frames.push(frame);
            }
        }
        frames
    }

    fn take(&mut self, line: &str) -> Option<Frame> {
        if line.is_empty() {
            return self.closed();
        }
        // A heartbeat, or a comment.
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.kind = Some(value.to_string()),
            "data" => self.data.push(value.to_string()),
            _ => {}
        }
        None
    }

    fn closed(&mut self) -> Option<Frame> {
        let kind = self.kind.take();
        let data = std::mem::take(&mut self.data);
        if kind.is_none() && data.is_empty() {
            return None;
        }
        let cloud = ipc::decode::<Cloud>("an event stream frame", data.join("\n").as_bytes()).ok();
        let kind = kind.unwrap_or_default();
        if kind == END {
            let said = cloud
                .and_then(|it| it.data.and_then(|inside| inside.outcome).or(it.outcome))
                .unwrap_or_default();
            return Some(Frame::End(match said.as_str() {
                "subscription_ended" => Outcome::SubscriptionEnded,
                "reconnect" => Outcome::Reconnect,
                _ => Outcome::Unknown(said),
            }));
        }
        if kind.starts_with(CONTROL) {
            return None;
        }
        // **Any other frame is a change**, including one whose JSON would not read: the event says
        // only that the pull request is worth reading again.
        let api_url = cloud.and_then(|it| it.data).and_then(|inside| inside.api_url);
        Some(Frame::Event { kind, api_url })
    }
}
