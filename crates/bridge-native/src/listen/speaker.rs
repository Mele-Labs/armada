//! Armada Voice's `speak` and `voices`, run off the main thread, and the mock that stands in for them in a walk
//! and a check, which never starts the helper and so never makes a sound.
//!
//! `speak [<voice id>]` reads text lines on stdin and prints `{"kind":"speaking"|"spoken","index":n}`. A
//! `{"stop":true}` line gives `{"kind":"stopped"}`, and an unknown voice `{"kind":"error","code":"voice_missing"}`.
//! `voices` prints `{"kind":"voice","id","name","language","quality"}` lines, then `{"kind":"end"}`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

use super::Why;

/// What a start of a line told the card, in order: it began, then it ended, was stopped, or could not be said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Speaking,
    Spoken,
    Stopped,
    Failed(Why),
}

/// A voice this Mac has, as `voices` lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    /// `default`, `enhanced` or `premium`.
    #[serde(default)]
    pub quality: String,
}

/// One line the helper prints, or `None` for anything that is not one: a log line, a half-written object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading {
    Speaking,
    Spoken,
    Stopped,
    Error(String),
    Voice(VoiceInfo),
    End,
}

pub fn read_line(line: &str) -> Option<Reading> {
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        code: Option<String>,
        id: Option<String>,
        name: Option<String>,
        quality: Option<String>,
    }
    let raw = ipc::decode::<Raw>("armada voice", line.as_bytes()).ok()?;
    match (raw.kind.as_str(), raw.code, raw.id, raw.name) {
        ("speaking", ..) => Some(Reading::Speaking),
        ("spoken", ..) => Some(Reading::Spoken),
        ("stopped", ..) => Some(Reading::Stopped),
        ("error", Some(code), ..) => Some(Reading::Error(code)),
        ("voice", _, Some(id), Some(name)) => Some(Reading::Voice(VoiceInfo { id, name, quality: raw.quality.unwrap_or_default() })),
        ("end", ..) => Some(Reading::End),
        _ => None,
    }
}

/// The voice's name as a card says it: the name, then its quality where it is better than the default.
/// The default voice has no name the helper gives, so it reads as what it is. A voice the Mac does not list
/// has no name to say.
pub fn voice_label(voices: &[VoiceInfo], id: &str) -> Option<String> {
    if id.is_empty() {
        return Some("Default voice".to_string());
    }
    let voice = voices.iter().find(|voice| voice.id == id || voice.name == id)?;
    Some(match voice.quality.as_str() {
        "enhanced" => format!("{}, Enhanced", voice.name),
        "premium" => format!("{}, Premium", voice.name),
        _ => voice.name.clone(),
    })
}

/// A line being spoken: its events come over `events`, and `stop` ends it.
pub struct Speech {
    pub events: async_channel::Receiver<Event>,
    stop: Arc<Stop>,
}

#[derive(Default)]
struct Stop {
    asked: AtomicBool,
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Option<Child>>,
}

impl Speech {
    /// Stop speaking. The helper says `stopped` and the speech ends; one that does not is killed after a moment.
    pub fn stop(&self) {
        self.stop.asked.store(true, Ordering::Relaxed);
        if let Ok(mut stdin) = self.stop.stdin.lock() {
            if let Some(stdin) = stdin.as_mut() {
                let _ = stdin.write_all(b"{\"stop\":true}\n");
                let _ = stdin.flush();
            }
        }
        reap(self.stop.clone(), Duration::from_secs(2));
    }
}

/// A helper still alive after `grace` is killed.
fn reap(stop: Arc<Stop>, grace: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(grace);
        if let Ok(mut child) = stop.child.lock() {
            if let Some(child) = child.as_mut() {
                if matches!(child.try_wait(), Ok(None)) {
                    let _ = child.kill();
                }
            }
        }
    });
}

/// What the mock helper does in place of the real one.
#[derive(Clone, Debug, Default)]
pub struct Mock {
    /// Armada Voice is not there.
    pub missing: bool,
    /// The next line starts this far in, as if it had been speaking a while.
    pub ahead: Duration,
    /// The voices it lists.
    pub voices: Vec<VoiceInfo>,
}

/// Where speech comes from.
pub enum Speaker {
    /// The helper, found each time a line starts, so one built since start is found.
    Helper,
    /// A timer that answers `speaking` and `spoken` in the line's estimated time.
    Mock(Mock),
}

impl Speaker {
    /// Start a line. Everything runs off the main thread: a missing helper arrives as an event.
    pub fn speak(&mut self, words: &str, voice: &str, total: Duration) -> (Speech, Duration) {
        let (send, events) = async_channel::unbounded();
        let stop = Arc::new(Stop::default());
        let speech = Speech { events, stop: stop.clone() };
        let mut ahead = Duration::ZERO;
        match self {
            Speaker::Helper => {
                let (words, voice) = (words.replace(['\n', '\r'], " "), voice.to_string());
                std::thread::spawn(move || run_helper(crate::voice::helper(), &words, &voice, &send, &stop));
            }
            Speaker::Mock(mock) => {
                ahead = std::mem::take(&mut mock.ahead);
                if mock.missing {
                    let _ = send.send_blocking(Event::Failed(Why::HelperMissing));
                } else {
                    let left = total.saturating_sub(ahead);
                    std::thread::spawn(move || run_mock(left, &send, &stop));
                }
            }
        }
        (speech, ahead)
    }

}

fn run_mock(left: Duration, send: &async_channel::Sender<Event>, stop: &Stop) {
    let _ = send.send_blocking(Event::Speaking);
    let step = Duration::from_millis(20);
    let mut waited = Duration::ZERO;
    while waited < left {
        if stop.asked.load(Ordering::Relaxed) {
            let _ = send.send_blocking(Event::Stopped);
            return;
        }
        std::thread::sleep(step);
        waited += step;
    }
    let _ = send.send_blocking(Event::Spoken);
}

fn run_helper(helper: Option<PathBuf>, words: &str, voice: &str, send: &async_channel::Sender<Event>, stop: &Arc<Stop>) {
    let Some(helper) = helper else {
        let _ = send.send_blocking(Event::Failed(Why::HelperMissing));
        return;
    };
    let mut command = Command::new(helper);
    command.arg("speak");
    if !voice.is_empty() {
        command.arg(voice);
    }
    let Ok(mut child) = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        let _ = send.send_blocking(Event::Failed(Why::HelperMissing));
        return;
    };
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = send.send_blocking(Event::Failed(Why::HelperFailed));
        return;
    };
    let _ = writeln!(stdin, "{words}");
    let _ = stdin.flush();
    if let Ok(mut held) = stop.child.lock() {
        *held = Some(child);
    }
    if let Ok(mut held) = stop.stdin.lock() {
        // A stop pressed before the line was written is sent as soon as the helper can take it.
        if stop.asked.load(Ordering::Relaxed) {
            let _ = stdin.write_all(b"{\"stop\":true}\n");
            let _ = stdin.flush();
        }
        *held = Some(stdin);
    }
    let mut ended = false;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let event = match read_line(&line) {
            Some(Reading::Speaking) => Event::Speaking,
            Some(Reading::Spoken) => Event::Spoken,
            Some(Reading::Stopped) => Event::Stopped,
            Some(Reading::Error(code)) => Event::Failed(if code == "voice_missing" { Why::VoiceMissing } else { Why::HelperFailed }),
            _ => continue,
        };
        let _ = send.send_blocking(event);
        if event != Event::Speaking {
            ended = true;
            break;
        }
    }
    if !ended {
        let _ = send.send_blocking(if stop.asked.load(Ordering::Relaxed) { Event::Stopped } else { Event::Failed(Why::HelperFailed) });
    }
    // The helper ends when its input does.
    if let Ok(mut held) = stop.stdin.lock() {
        *held = None;
    }
    reap(stop.clone(), Duration::from_secs(3));
}

/// `armada-voice voices`, read to its end. Blocks: run it off the main thread.
pub fn list_voices(helper: &Path) -> Vec<VoiceInfo> {
    let Ok(mut child) = Command::new(helper).arg("voices").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return Vec::new();
    };
    let mut voices = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            match read_line(&line) {
                Some(Reading::Voice(voice)) => voices.push(voice),
                Some(Reading::End) => break,
                _ => {}
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    voices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_line_is_read_and_anything_else_is_not() {
        assert_eq!(read_line(r#"{"kind":"speaking","index":0}"#), Some(Reading::Speaking));
        assert_eq!(read_line(r#"{"kind":"spoken","index":0}"#), Some(Reading::Spoken));
        assert_eq!(read_line(r#"{"kind":"stopped"}"#), Some(Reading::Stopped));
        assert_eq!(read_line(r#"{"kind":"error","code":"voice_missing"}"#), Some(Reading::Error("voice_missing".into())));
        assert_eq!(
            read_line(r#"{"kind":"voice","id":"com.apple.voice.premium.en-US.Zoe","name":"Zoe","language":"en-US","quality":"premium"}"#),
            Some(Reading::Voice(VoiceInfo { id: "com.apple.voice.premium.en-US.Zoe".into(), name: "Zoe".into(), quality: "premium".into() }))
        );
        assert_eq!(read_line(r#"{"kind":"end"}"#), Some(Reading::End));
        assert_eq!(read_line("speaking"), None);
        assert_eq!(read_line(r#"{"kind":"error"}"#), None);
    }

    #[test]
    fn a_voice_is_named_by_its_id_with_its_quality_and_the_default_reads_as_the_default() {
        let voices = vec![
            VoiceInfo { id: "z".into(), name: "Zoe".into(), quality: "premium".into() },
            VoiceInfo { id: "a".into(), name: "Alex".into(), quality: "default".into() },
        ];
        assert_eq!(voice_label(&voices, "z").as_deref(), Some("Zoe, Premium"));
        assert_eq!(voice_label(&voices, "Alex").as_deref(), Some("Alex"));
        assert_eq!(voice_label(&voices, "").as_deref(), Some("Default voice"));
        assert_eq!(voice_label(&voices, "gone"), None);
    }

    #[test]
    fn the_mock_speaks_on_a_timer_and_a_missing_helper_arrives_as_an_event() {
        let mut speaker = Speaker::Mock(Mock::default());
        let (speech, _) = speaker.speak("one two", "", Duration::from_millis(60));
        assert_eq!(speech.events.recv_blocking().ok(), Some(Event::Speaking));
        assert_eq!(speech.events.recv_blocking().ok(), Some(Event::Spoken));
        let mut missing = Speaker::Mock(Mock { missing: true, ..Mock::default() });
        let (speech, _) = missing.speak("one two", "", Duration::from_secs(5));
        assert_eq!(speech.events.recv_blocking().ok(), Some(Event::Failed(Why::HelperMissing)));
        let mut speaker = Speaker::Mock(Mock::default());
        let (speech, _) = speaker.speak("one two", "", Duration::from_secs(30));
        assert_eq!(speech.events.recv_blocking().ok(), Some(Event::Speaking));
        speech.stop();
        assert_eq!(speech.events.recv_blocking().ok(), Some(Event::Stopped));
    }
}
