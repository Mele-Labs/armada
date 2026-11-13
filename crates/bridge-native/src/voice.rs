//! Talk's voices, heard on this Mac. Apple's recogniser and Whisper run Armada Voice, a helper app
//! that prints one JSON object a line; the caller names an engine and gets words back, never a path
//! or a process. A voice that cannot hear hands over to the next, so the person is asked only for
//! what macOS withholds. The Electron Bridge does the same in `apps/desktop/src/main/voice.ts`.
//!
//! macOS Dictation, the third voice, is started on the main thread (`crate::dictation`), so the
//! worker here only says when to: `Heard::Dictate`.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

/// Where Talk's words come from. The setting `thread.look` keeps is `voice`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Engine {
    Apple,
    Whisper,
    /// macOS's own, typing into whichever box has focus.
    Dictation,
}

impl Engine {
    /// The engine a setting names; anything unknown is Apple.
    pub fn named(name: &str) -> Engine {
        match name {
            "whisper" => Engine::Whisper,
            "dictation" => Engine::Dictation,
            _ => Engine::Apple,
        }
    }

    /// How long a released helper has to say its last word before it is killed. Apple's final
    /// transcript follows in well under a second; Whisper transcribes everything after the
    /// release, up to five minutes of speech.
    fn grace(self) -> Duration {
        match self {
            Engine::Apple => Duration::from_secs(6),
            Engine::Whisper => Duration::from_secs(120),
            Engine::Dictation => Duration::ZERO,
        }
    }
}

/// Why hearing ended early.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    HelperMissing,
    HelperFailed,
    MicrophoneDenied,
    MicrophoneUnavailable,
    SpeechDenied,
    OnDeviceUnavailable,
    RecognitionFailed,
    WhisperFailed,
}

impl Why {
    /// The helper's error code as it is read; whisper.cpp's own failures are all `WhisperFailed`.
    pub fn of(code: &str) -> Why {
        match code {
            "microphone_denied" => Why::MicrophoneDenied,
            "microphone_unavailable" => Why::MicrophoneUnavailable,
            "speech_denied" => Why::SpeechDenied,
            "on_device_unavailable" => Why::OnDeviceUnavailable,
            "recognition_failed" => Why::RecognitionFailed,
            "helper_missing" => Why::HelperMissing,
            code if code.starts_with("whisper_") => Why::WhisperFailed,
            _ => Why::HelperFailed,
        }
    }

    /// Another voice would not help with these, or only the person can settle them.
    fn final_one(self) -> bool {
        matches!(self, Why::MicrophoneDenied | Why::SpeechDenied | Why::MicrophoneUnavailable)
    }
}

/// One line Armada Voice prints.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HelperLine {
    Ready,
    Transcribing,
    Partial(String),
    Final(String),
    Error(String),
}

/// A line read, or `None` for anything that is not one: a log line, a half-written object.
pub fn read_helper_line(line: &str) -> Option<HelperLine> {
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        text: Option<String>,
        code: Option<String>,
    }
    let raw = ipc::decode::<Raw>("armada voice", line.as_bytes()).ok()?;
    match (raw.kind.as_str(), raw.text, raw.code) {
        ("ready", ..) => Some(HelperLine::Ready),
        ("transcribing", ..) => Some(HelperLine::Transcribing),
        ("partial", Some(text), _) => Some(HelperLine::Partial(text)),
        ("final", Some(text), _) => Some(HelperLine::Final(text)),
        ("error", _, Some(code)) => Some(HelperLine::Error(code)),
        _ => None,
    }
}

fn words_in(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// The recogniser's running transcripts as words, each given once. It rewrites its guess as the
/// person talks, so a word is given when two transcripts in a row agree on it and on everything
/// before it, and the final transcript gives the rest. A word given and rewritten after stays as
/// given: the box has no way to take one back.
#[derive(Default)]
pub struct Settled {
    given: usize,
    last: Vec<String>,
}

impl Settled {
    pub fn partial(&mut self, text: &str) -> Vec<String> {
        let words = words_in(text);
        let agreed = words.iter().zip(&self.last).take_while(|(now, before)| now == before).count();
        self.last = words.clone();
        self.up_to(&words, agreed)
    }

    pub fn finished(&mut self, text: &str) -> Vec<String> {
        let words = words_in(text);
        let end = words.len();
        self.up_to(&words, end)
    }

    fn up_to(&mut self, words: &[String], end: usize) -> Vec<String> {
        if end <= self.given {
            return Vec::new();
        }
        let out = words[self.given..end].to_vec();
        self.given = end;
        out
    }
}

/// The voices tried for a chosen engine, in order, among those this Mac can start. Dictation is
/// last wherever it is there, and always can start.
pub fn voices_for(engine: Engine, helper: bool, whisper: bool) -> Vec<Engine> {
    let order: &[Engine] = match engine {
        Engine::Apple => &[Engine::Apple, Engine::Whisper, Engine::Dictation],
        Engine::Whisper => &[Engine::Whisper, Engine::Apple, Engine::Dictation],
        Engine::Dictation => &[Engine::Dictation],
    };
    order.iter().copied().filter(|one| *one == Engine::Dictation || (helper && (*one == Engine::Apple || whisper))).collect()
}

/// What follows a voice that ended: the end, with a reason where the person has to act, or the
/// next voice.
#[derive(PartialEq, Eq, Debug)]
pub enum Next {
    End(Option<Why>),
    Try,
}

pub fn next_after(why: Option<Why>, released: bool, given: bool, at: usize, voices: usize) -> Next {
    match why {
        Some(why) if why.final_one() => Next::End(Some(why)),
        None => Next::End(None),
        Some(_) if released || given || at + 1 >= voices => Next::End(None),
        Some(_) => Next::Try,
    }
}

/// What the box is told, in order: words as they settle, then one end.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Heard {
    Word(String),
    End(Option<Why>),
    /// No voice of the helper's could hear: macOS Dictation is started, and ended by the release.
    Dictate,
}

/// Where words come from: the helper, or on mock data a script that stands in for it.
pub struct Source {
    pub helper: Option<PathBuf>,
    pub whisper: Option<(String, String)>,
    pub script: Option<Script>,
}

/// What a mock helper says: words while it listens, words after the release, and a reason to end
/// on in place of either.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Script {
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub why: Option<String>,
}

struct Live {
    stdin: ChildStdin,
    child: Arc<Mutex<Child>>,
}

#[derive(Default)]
struct State {
    released: bool,
    live: Option<Live>,
}

/// A hearing under way: the words come over `heard`, and `release` is the stop.
pub struct Hearing {
    pub heard: async_channel::Receiver<Heard>,
    state: Arc<Mutex<State>>,
    engine: Engine,
    done: Arc<AtomicBool>,
}

impl Hearing {
    /// The voice finishes what it heard and ends.
    pub fn release(&self) {
        let Ok(mut state) = self.state.lock() else { return };
        state.released = true;
        if let Some(live) = state.live.take() {
            let Live { stdin, child } = live;
            drop(stdin);
            grace(child, self.engine, self.done.clone());
        }
    }
}

/// A released helper that has not ended by the end of its grace is killed.
fn grace(child: Arc<Mutex<Child>>, engine: Engine, done: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let step = Duration::from_millis(250);
        let mut waited = Duration::ZERO;
        while waited < engine.grace() {
            if done.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(step);
            waited += step;
        }
        if let Ok(mut child) = child.lock() {
            let _ = child.kill();
        }
    });
}

/// Starts hearing with `engine`, and with the next voice where it cannot.
pub fn hear(engine: Engine, source: Source) -> Hearing {
    let (send, heard) = async_channel::unbounded();
    let state = Arc::new(Mutex::new(State::default()));
    let done = Arc::new(AtomicBool::new(false));
    let hearing = Hearing { heard, state: state.clone(), engine, done: done.clone() };
    std::thread::spawn(move || {
        let end = match source.script {
            Some(script) => Finish::End(play(&script, &send, &state)),
            None => run(engine, &source, &send, &state, &done),
        };
        done.store(true, Ordering::Relaxed);
        let _ = send.send_blocking(match end {
            Finish::End(why) => Heard::End(why),
            Finish::Dictate => Heard::Dictate,
        });
    });
    hearing
}

/// How a worker's hearing finished.
enum Finish {
    End(Option<Why>),
    Dictate,
}

fn released(state: &Mutex<State>) -> bool {
    state.lock().map(|state| state.released).unwrap_or(true)
}

/// A mock helper: its words at once, its last ones when released, or its reason to stop.
fn play(script: &Script, send: &async_channel::Sender<Heard>, state: &Mutex<State>) -> Option<Why> {
    if let Some(why) = &script.why {
        return Some(Why::of(why));
    }
    for word in &script.words {
        let _ = send.send_blocking(Heard::Word(word.clone()));
    }
    while !released(state) {
        std::thread::sleep(Duration::from_millis(20));
    }
    for word in &script.after {
        let _ = send.send_blocking(Heard::Word(word.clone()));
    }
    None
}

fn run(engine: Engine, source: &Source, send: &async_channel::Sender<Heard>, state: &Arc<Mutex<State>>, done: &Arc<AtomicBool>) -> Finish {
    let voices = voices_for(engine, source.helper.is_some(), source.whisper.is_some());
    let mut given = false;
    for (at, voice) in voices.iter().enumerate() {
        let why = match (voice, source.helper.as_deref()) {
            (Engine::Dictation, _) => return Finish::Dictate,
            (_, Some(helper)) => hear_with(*voice, helper, source.whisper.as_ref(), send, state, done, &mut given),
            (_, None) => Some(Why::HelperMissing),
        };
        match next_after(why, released(state), given, at, voices.len()) {
            Next::End(why) => return Finish::End(why),
            Next::Try => {}
        }
    }
    Finish::End(None)
}

/// One voice, heard to its end.
fn hear_with(
    voice: Engine,
    helper: &Path,
    whisper: Option<&(String, String)>,
    send: &async_channel::Sender<Heard>,
    state: &Arc<Mutex<State>>,
    done: &Arc<AtomicBool>,
    given: &mut bool,
) -> Option<Why> {
    let mut command = Command::new(helper);
    match (voice, whisper) {
        (Engine::Whisper, Some((library, model))) => command.args(["whisper", library, model]),
        (Engine::Whisper, None) | (Engine::Dictation, _) => return Some(Why::HelperMissing),
        (Engine::Apple, _) => command.arg("listen"),
    };
    let Ok(mut child) = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return Some(Why::HelperMissing);
    };
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else { return Some(Why::HelperFailed) };
    let child = Arc::new(Mutex::new(child));
    {
        let Ok(mut held) = state.lock() else { return Some(Why::HelperFailed) };
        if held.released {
            drop(stdin);
            grace(child.clone(), voice, done.clone());
        } else {
            held.live = Some(Live { stdin, child: child.clone() });
        }
    }
    let mut settled = Settled::default();
    let mut ended: Option<Option<Why>> = None;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let words = match read_helper_line(&line) {
            Some(HelperLine::Partial(text)) => settled.partial(&text),
            Some(HelperLine::Final(text)) => {
                ended = Some(None);
                settled.finished(&text)
            }
            Some(HelperLine::Error(code)) => {
                ended = Some(Some(Why::of(&code)));
                Vec::new()
            }
            _ => Vec::new(),
        };
        for word in words {
            *given = true;
            let _ = send.send_blocking(Heard::Word(word));
        }
        if ended.is_some() {
            break;
        }
    }
    if let Ok(mut held) = state.lock() {
        held.live = None;
    }
    if let Ok(mut child) = child.lock() {
        if ended.is_some() {
            // Its work is done; it is left a moment to exit by itself, and killed if it does not.
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    // Released, a helper that ends without a final line heard nothing; unreleased, it died.
    ended.unwrap_or(if released(state) { None } else { Some(Why::HelperFailed) })
}

/// Armada Voice's executable: built beside the sources in a dev build, in `Contents/Helpers` in a
/// real one. Read at each press, so one built since start is found.
pub fn helper() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundled = exe.parent()?.parent()?.join("Helpers/Armada Voice.app/Contents/MacOS/armada-voice");
    if bundled.is_file() {
        return Some(bundled);
    }
    let built = exe.ancestors().find(|dir| dir.join("apps/desktop/native/voice").is_dir())?;
    let built = built.join("apps/desktop/native/voice/build/Armada Voice.app/Contents/MacOS/armada-voice");
    built.is_file().then_some(built)
}

// ---- Armada Voice's other commands: the voices a Mac can read with, what it may hear, and a sentence spoken ----

/// How good a voice is, best first: the order `ordered` lists them in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Quality {
    Premium,
    Enhanced,
    Default,
}

/// One voice the helper offers to read with.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub quality: Quality,
}

/// A line of `armada-voice voices`, or `None` for the closing `end` line and anything that is not a voice.
pub fn read_voice_line(line: &str) -> Option<Voice> {
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        id: Option<String>,
        name: Option<String>,
        language: Option<String>,
        quality: Option<String>,
    }
    let raw = ipc::decode::<Raw>("armada voice", line.as_bytes()).ok()?;
    if raw.kind != "voice" {
        return None;
    }
    let quality = match raw.quality.as_deref() {
        Some("premium") => Quality::Premium,
        Some("enhanced") => Quality::Enhanced,
        _ => Quality::Default,
    };
    Some(Voice { id: raw.id?, name: raw.name?, language: raw.language.unwrap_or_default(), quality })
}

/// Premium first, then Enhanced, then the rest, each by name.
pub fn ordered(mut voices: Vec<Voice>) -> Vec<Voice> {
    voices.sort_by(|a, b| a.quality.cmp(&b.quality).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then_with(|| a.id.cmp(&b.id)));
    voices
}

/// Every voice the helper lists, ordered. Blocks until the helper ends, so it runs off the main thread.
pub fn list_voices(helper: &Path) -> Result<Vec<Voice>, Why> {
    let out = Command::new(helper).arg("voices").stdin(Stdio::null()).stderr(Stdio::null()).output().map_err(|_| Why::HelperMissing)?;
    if !out.status.success() {
        return Err(Why::HelperFailed);
    }
    Ok(ordered(String::from_utf8_lossy(&out.stdout).lines().filter_map(read_voice_line).collect()))
}

/// The voices a walk lists, with no helper to ask.
pub fn mock_voices() -> Vec<Voice> {
    let voice = |name: &str, language: &str, quality| Voice { id: format!("com.apple.voice.{}", name.to_lowercase()), name: name.into(), language: language.into(), quality };
    ordered(vec![
        voice("Samantha", "en-US", Quality::Default),
        voice("Zoe", "en-US", Quality::Premium),
        voice("Daniel", "en-GB", Quality::Enhanced),
        voice("Ava", "en-US", Quality::Premium),
        voice("Alex", "en-US", Quality::Default),
        voice("Allison", "en-US", Quality::Enhanced),
        voice("Karen", "en-AU", Quality::Default),
    ])
}

/// What macOS lets the helper do, as `probe` names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Access {
    Granted,
    Denied,
    Restricted,
    Unasked,
    Unknown,
}

impl Access {
    fn named(word: &str) -> Access {
        match word {
            "granted" => Access::Granted,
            "denied" => Access::Denied,
            "restricted" => Access::Restricted,
            "unasked" => Access::Unasked,
            _ => Access::Unknown,
        }
    }
}

/// The line `armada-voice probe` prints, as the microphone's access. `None` for any other line.
pub fn read_probe_line(line: &str) -> Option<Access> {
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        microphone: Option<String>,
    }
    let raw = ipc::decode::<Raw>("armada voice", line.as_bytes()).ok()?;
    (raw.kind == "probe").then(|| Access::named(raw.microphone.as_deref().unwrap_or_default()))
}

/// Whether the microphone may be heard, asked of the helper. Blocks, so it runs off the main thread.
pub fn microphone(helper: &Path) -> Option<Access> {
    let out = Command::new(helper).arg("probe").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().find_map(read_probe_line)
}

/// What a sentence being spoken is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spoke {
    Speaking,
    Spoken,
    Stopped,
    /// The voice asked for is not on this Mac.
    Missing,
    Failed,
}

/// A line of `armada-voice speak`, or `None` for one that says nothing a person is told.
pub fn read_speak_line(line: &str) -> Option<Spoke> {
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        code: Option<String>,
    }
    let raw = ipc::decode::<Raw>("armada voice", line.as_bytes()).ok()?;
    match (raw.kind.as_str(), raw.code.as_deref()) {
        ("speaking", _) => Some(Spoke::Speaking),
        ("spoken", _) => Some(Spoke::Spoken),
        ("stopped", _) => Some(Spoke::Stopped),
        ("error", Some("voice_missing")) => Some(Spoke::Missing),
        ("error", _) => Some(Spoke::Failed),
        _ => None,
    }
}

/// A sentence being spoken: what the helper says of it comes over `said`, and `stop` ends it.
pub struct Speech {
    pub said: async_channel::Receiver<Spoke>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl Speech {
    /// Asks the helper to stop; it answers `Stopped`.
    pub fn stop(&self) {
        use std::io::Write;
        if let Ok(mut held) = self.stdin.lock() {
            if let Some(stdin) = held.as_mut() {
                let _ = stdin.write_all(b"{\"stop\":true}\n");
                let _ = stdin.flush();
            }
        }
    }
}

/// Speaks `text` in `voice` (empty is the Mac's own) through the helper, on a thread of its own.
pub fn speak(helper: PathBuf, voice: &str, text: &str) -> Speech {
    use std::io::Write;
    let (send, said) = async_channel::unbounded();
    let stdin: Arc<Mutex<Option<ChildStdin>>> = Arc::new(Mutex::new(None));
    let held = stdin.clone();
    let (voice, text) = (voice.to_string(), text.to_string());
    std::thread::spawn(move || {
        let mut command = Command::new(&helper);
        command.arg("speak");
        if !voice.is_empty() {
            command.arg(&voice);
        }
        let Ok(mut child) = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
            let _ = send.send_blocking(Spoke::Failed);
            return;
        };
        let (Some(mut input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = send.send_blocking(Spoke::Failed);
            return;
        };
        // The line goes in, and the pipe stays open so a stop can follow it; the helper ends once it has spoken.
        let _ = writeln!(input, "{}", text.replace('\n', " "));
        let _ = input.flush();
        if let Ok(mut held) = held.lock() {
            *held = Some(input);
        }
        let mut told = false;
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let Some(spoke) = read_speak_line(&line) else { continue };
            let over = !matches!(spoke, Spoke::Speaking);
            told = over;
            let _ = send.send_blocking(spoke);
            if over {
                break;
            }
        }
        if let Ok(mut held) = held.lock() {
            held.take();
        }
        let _ = child.kill();
        let _ = child.wait();
        if !told {
            let _ = send.send_blocking(Spoke::Failed);
        }
    });
    Speech { said, stdin }
}

/// The Settings pane a refused permission is fixed in.
pub fn settings_address(microphone: bool) -> &'static str {
    if microphone {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
    } else {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_SpeechRecognition"
    }
}

/// A word joined to a draft with one space between, and none where the draft is empty or ends in
/// one.
pub fn joined(text: &str, word: &str) -> String {
    if text.is_empty() || text.ends_with(char::is_whitespace) {
        format!("{text}{word}")
    } else {
        format!("{text} {word}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_read_and_anything_else_is_not() {
        assert_eq!(read_helper_line(r#"{"kind":"ready"}"#), Some(HelperLine::Ready));
        assert_eq!(read_helper_line(r#"{"kind":"partial","text":"check the"}"#), Some(HelperLine::Partial("check the".into())));
        assert_eq!(read_helper_line(r#"{"kind":"final","text":"check the queue"}"#), Some(HelperLine::Final("check the queue".into())));
        assert_eq!(read_helper_line(r#"{"kind":"error","code":"speech_denied"}"#), Some(HelperLine::Error("speech_denied".into())));
        assert_eq!(read_helper_line(r#"{"kind":"partial"}"#), None);
        assert_eq!(read_helper_line("listening..."), None);
        assert_eq!(read_helper_line(r#"{"kind":"final","te"#), None);
    }

    #[test]
    fn a_code_is_a_reason_and_whispers_own_are_one() {
        assert_eq!(Why::of("microphone_denied"), Why::MicrophoneDenied);
        assert_eq!(Why::of("whisper_model_missing"), Why::WhisperFailed);
        assert_eq!(Why::of("anything else"), Why::HelperFailed);
    }

    #[test]
    fn a_word_is_given_when_two_transcripts_agree_on_it_and_the_final_gives_the_rest() {
        let mut settled = Settled::default();
        assert!(settled.partial("check").is_empty());
        assert_eq!(settled.partial("check the"), ["check"]);
        // The recogniser rewrites its guess: the word given stays, and only what follows it counts.
        assert!(settled.partial("czech the queue").is_empty());
        assert_eq!(settled.partial("czech the queue"), ["the", "queue"]);
        assert_eq!(settled.finished("check the queue now"), ["now"]);
    }

    #[test]
    fn a_voice_is_tried_only_where_it_can_start() {
        assert_eq!(voices_for(Engine::Whisper, true, true), [Engine::Whisper, Engine::Apple, Engine::Dictation]);
        assert_eq!(voices_for(Engine::Whisper, true, false), [Engine::Apple, Engine::Dictation]);
        assert_eq!(voices_for(Engine::Apple, true, true), [Engine::Apple, Engine::Whisper, Engine::Dictation]);
        assert_eq!(voices_for(Engine::Apple, false, true), [Engine::Dictation]);
        assert_eq!(voices_for(Engine::Dictation, true, true), [Engine::Dictation]);
    }

    #[test]
    fn a_refused_permission_ends_it_and_another_failure_hands_over_until_words_came() {
        assert_eq!(next_after(Some(Why::MicrophoneDenied), false, false, 0, 2), Next::End(Some(Why::MicrophoneDenied)));
        assert_eq!(next_after(None, false, false, 0, 2), Next::End(None));
        assert_eq!(next_after(Some(Why::HelperFailed), false, false, 0, 2), Next::Try);
        assert_eq!(next_after(Some(Why::HelperFailed), false, true, 0, 2), Next::End(None));
        assert_eq!(next_after(Some(Why::HelperFailed), true, false, 0, 2), Next::End(None));
        assert_eq!(next_after(Some(Why::HelperFailed), false, false, 1, 2), Next::End(None));
    }

    #[test]
    fn a_word_joins_the_draft_with_one_space() {
        assert_eq!(joined("", "check"), "check");
        assert_eq!(joined("check", "the"), "check the");
        assert_eq!(joined("check ", "the"), "check the");
        assert_eq!(joined("a\n", "b"), "a\nb");
    }

    #[test]
    fn a_script_gives_its_words_then_waits_for_the_release() {
        let script = Script { words: vec!["check".into(), "the".into()], after: vec!["queue".into()], why: None };
        let hearing = hear(Engine::Apple, Source { helper: None, whisper: None, script: Some(script) });
        assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::Word("check".into())));
        assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::Word("the".into())));
        hearing.release();
        assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::Word("queue".into())));
        assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::End(None)));
    }

    #[test]
    fn a_script_that_is_refused_ends_with_the_reason() {
        let script = Script { why: Some("microphone_denied".into()), ..Script::default() };
        let hearing = hear(Engine::Apple, Source { helper: None, whisper: None, script: Some(script) });
        assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::End(Some(Why::MicrophoneDenied))));
    }

    #[test]
    fn no_helper_hands_over_to_dictation_and_dictation_asked_for_is_the_only_voice() {
        for engine in [Engine::Apple, Engine::Dictation] {
            let hearing = hear(engine, Source { helper: None, whisper: None, script: None });
            assert_eq!(hearing.heard.recv_blocking().ok(), Some(Heard::Dictate));
        }
    }

    #[test]
    fn a_voice_line_is_read_and_the_end_and_noise_are_not() {
        let line = r#"{"kind":"voice","id":"com.apple.voice.premium.en-US.Zoe","name":"Zoe","language":"en-US","quality":"premium"}"#;
        assert_eq!(read_voice_line(line), Some(Voice { id: "com.apple.voice.premium.en-US.Zoe".into(), name: "Zoe".into(), language: "en-US".into(), quality: Quality::Premium }));
        assert_eq!(read_voice_line(r#"{"kind":"end"}"#), None);
        assert_eq!(read_voice_line("loading voices"), None);
        assert_eq!(read_voice_line(r#"{"kind":"voice","name":"Nobody"}"#), None);
    }

    #[test]
    fn voices_go_premium_then_enhanced_then_the_rest_by_name() {
        let names: Vec<(String, Quality)> = mock_voices().into_iter().map(|one| (one.name, one.quality)).collect();
        let want = [("Ava", Quality::Premium), ("Zoe", Quality::Premium), ("Allison", Quality::Enhanced), ("Daniel", Quality::Enhanced), ("Alex", Quality::Default), ("Karen", Quality::Default), ("Samantha", Quality::Default)];
        assert_eq!(names, want.map(|(name, quality)| (name.to_string(), quality)));
    }

    #[test]
    fn a_probe_line_names_the_microphone_and_a_speak_line_says_what_the_voice_is_doing() {
        assert_eq!(read_probe_line(r#"{"kind":"probe","pid":1,"microphone":"denied","speech":"granted","onDevice":true}"#), Some(Access::Denied));
        assert_eq!(read_probe_line(r#"{"kind":"probe","microphone":"unasked"}"#), Some(Access::Unasked));
        assert_eq!(read_probe_line(r#"{"kind":"ready"}"#), None);
        assert_eq!(read_speak_line(r#"{"kind":"speaking","index":0}"#), Some(Spoke::Speaking));
        assert_eq!(read_speak_line(r#"{"kind":"spoken","index":0}"#), Some(Spoke::Spoken));
        assert_eq!(read_speak_line(r#"{"kind":"stopped"}"#), Some(Spoke::Stopped));
        assert_eq!(read_speak_line(r#"{"kind":"error","code":"voice_missing"}"#), Some(Spoke::Missing));
        assert_eq!(read_speak_line(r#"{"kind":"error","code":"respawn_failed"}"#), Some(Spoke::Failed));
        assert_eq!(read_speak_line("hello"), None);
    }
}
