//! What a crash leaves behind, and what the panel that tells him will read.
//!
//! A panic hook (`install`, the live app only) writes a report to `Armada/crashes/` before the process
//! goes: gpui can abort straight after a panic, so the hook does all its work before it returns, on the
//! panicking thread, and does not wait on anything another thread may hold. The same hook writes what he
//! typed and the stores have not yet written (`hold`), because an abort skips the quit flush.
//!
//! A report is seen once he has been told of it: `mark_seen` leaves an empty `<report>.seen` beside it, so
//! the report keeps the name its time was read from and the one it is attached by.
//!
//! A crash that is not a panic, a signal inside WebKit say, runs no hook. So each run leaves `run-<pid>` in
//! the same folder while it runs, and a quit he meant takes it away: a mark left by another pid is a run that
//! ended some other way, and macOS's own report from that pid is the crash. Seeing it takes the mark away.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, TryLockError};
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use ipc::{DispatchJob, DispatchKind, DispatchMessage, DispatchThen, ManifestId, SessionUpload, StartSession};

use crate::log::Note;

// ---- what is typed and not yet written -----------------------------------------------------------

/// The latest text of each draft file that has not been written yet, by file. A store puts its text here when
/// it changes and takes it out once the file holds it, so whatever is left is what an abort would lose.
static HELD: Mutex<Vec<(PathBuf, String)>> = Mutex::new(Vec::new());

/// The reports this run wrote. A panic off the main thread ends only its thread, so a run that then quits as he
/// meant marks them seen: the panel says Armada quit only when it did.
static WRITTEN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// A file's latest text, not yet written. Cheap: one lock and one string kept.
pub(crate) fn hold(path: &Path, text: String) {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match held.iter_mut().find(|(held, _)| held == path) {
        Some(entry) => entry.1 = text,
        None => held.push((path.to_path_buf(), text)),
    }
}

/// `text` is in the file. A newer text held for it stays held.
pub(crate) fn settle(path: &Path, text: &str) {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    held.retain(|(held, latest)| !(held == path && latest == text));
}

/// Write everything held. `wait` is whether to wait for the lock: the quit may, a panicking thread may not,
/// since it can be the one that holds it, so it writes nothing rather than hang.
pub(crate) fn flush(wait: bool) {
    let held = if wait {
        HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    } else {
        match HELD.try_lock() {
            Ok(held) => held,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        }
    };
    for (path, text) in held.iter() {
        if let Err(why) = write_atomic(path, text) {
            Note::warn("held drafts were not written").field("why", why.to_string()).emit();
        }
    }
}

/// A file made whole or not at all: a temp file beside it, written and synced, then renamed over it.
pub(crate) fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut name = path.file_name().map(|name| name.to_os_string()).unwrap_or_default();
    name.push(format!(".{}-{}.tmp", std::process::id(), COUNT.fetch_add(1, Ordering::Relaxed)));
    let temp = path.with_file_name(name);
    let written = std::fs::File::create(&temp).and_then(|mut file| file.write_all(text.as_bytes()).and_then(|()| file.sync_all())).and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

// ---- the hook ------------------------------------------------------------------------------------

/// Where reports are kept.
pub(crate) fn crashes_dir() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("crashes"))
}

/// Where macOS keeps its own crash reports.
pub(crate) fn macos_reports_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Logs/DiagnosticReports"))
}

/// Write a report on every panic, then what is held, then whatever hook was there before. Also write what is
/// held when the app quits, which the stores' own waits would otherwise miss. Once, at the live app's start.
pub(crate) fn install(cx: &mut gpui::App) {
    if let Some(dir) = crashes_dir() {
        if let Err(why) = begin_run(&dir, std::process::id()) {
            Note::warn("this run was not marked").field("why", why.to_string()).emit();
        }
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = crashes_dir() {
            // The panic's report speaks for this run, so macOS's of the same end is not offered again.
            end_run(&dir, std::process::id());
            let at = Utc::now();
            let thread = std::thread::current();
            let message = match info.payload().downcast_ref::<&str>() {
                Some(message) => (*message).to_string(),
                None => info.payload().downcast_ref::<String>().cloned().unwrap_or_default(),
            };
            let location = info.location().map(|place| format!("{}:{}:{}", place.file(), place.line(), place.column()));
            let build = build();
            let backtrace = std::backtrace::Backtrace::force_capture().to_string();
            let text = report(&Facts {
                at,
                local: at.with_timezone(&Local).fixed_offset(),
                build: &build,
                thread: thread.name().unwrap_or("unnamed"),
                message: &message,
                location: location.as_deref(),
                backtrace: &backtrace,
            });
            if let Ok(file) = write_report(&dir, at, &text) {
                if let Ok(mut written) = WRITTEN.try_lock() {
                    written.push(file);
                }
            }
        }
        flush(false);
        previous(info);
    }));
    cx.on_app_quit(|_| {
        flush(true);
        for file in WRITTEN.lock().map(|written| written.clone()).unwrap_or_default() {
            let _ = std::fs::write(seen_mark(&file), "");
        }
        if let Some(dir) = crashes_dir() {
            end_run(&dir, std::process::id());
        }
        std::future::ready(())
    })
    .detach();
}

/// What built this: the crate's version, where it runs from and the protocol it speaks.
fn build() -> String {
    let exe = std::env::current_exe().map(|path| path.display().to_string()).unwrap_or_default();
    format!("bridge-native {} protocol {} at {exe}", env!("CARGO_PKG_VERSION"), ipc::ProtocolId::current())
}

/// What a report says.
pub(crate) struct Facts<'a> {
    pub at: DateTime<Utc>,
    pub local: DateTime<FixedOffset>,
    pub build: &'a str,
    pub thread: &'a str,
    pub message: &'a str,
    pub location: Option<&'a str>,
    pub backtrace: &'a str,
}

/// The report's text. Its first line starting `panic: ` is what a person is shown of it.
pub(crate) fn report(facts: &Facts) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "time: {}", facts.local.format("%Y-%m-%d %H:%M:%S %:z"));
    let _ = writeln!(text, "time utc: {}", facts.at.format("%Y-%m-%dT%H:%M:%SZ"));
    let _ = writeln!(text, "build: {}", facts.build);
    let _ = writeln!(text, "thread: {}", facts.thread);
    let mut lines = facts.message.lines();
    let _ = writeln!(text, "panic: {}", lines.next().filter(|line| !line.is_empty()).unwrap_or("no message"));
    for line in lines {
        let _ = writeln!(text, "{line}");
    }
    if let Some(place) = facts.location {
        let _ = writeln!(text, "at: {place}");
    }
    let _ = write!(text, "\nbacktrace:\n{}\n", facts.backtrace);
    text
}

/// The file name a crash at `at` gets, and the same with `n` when another thread crashed in the same second.
fn file_name(at: DateTime<Utc>, n: u32) -> String {
    let stamp = at.format("%Y-%m-%dT%H%M%SZ");
    if n < 2 { format!("crash-{stamp}.txt") } else { format!("crash-{stamp}-{n}.txt") }
}

/// The report, written and synced. A report from the same second is not overwritten.
pub(crate) fn write_report(dir: &Path, at: DateTime<Utc>, text: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    for n in 1..100 {
        let path = dir.join(file_name(at, n));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                file.sync_all()?;
                return Ok(path);
            }
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(why) => return Err(why),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "a hundred reports in one second"))
}

// ---- what the panel reads ------------------------------------------------------------------------

/// A report he has not been told of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Crash {
    pub file: PathBuf,
    pub at: DateTime<Utc>,
    /// The first line of the panic's message.
    pub panic: String,
    /// Where it panicked, as `file:line:column`; empty where the report does not say.
    pub location: String,
    /// Set where macOS's report is the crash and no panic wrote one: the mark of the run it ended. `panic`
    /// then holds the exception and `location` the frame it faulted in.
    pub run: Option<PathBuf>,
}

/// The time a report's name carries, and which of that second's it is, if the name is a report's.
fn time_of(name: &str) -> Option<(DateTime<Utc>, u32)> {
    let rest = name.strip_prefix("crash-")?.strip_suffix(".txt")?;
    let time = NaiveDateTime::parse_from_str(rest.get(..18)?, "%Y-%m-%dT%H%M%SZ").ok()?;
    let nth = match rest.get(18..)? {
        "" => 1,
        more => more.strip_prefix('-')?.parse().ok()?,
    };
    Some((Utc.from_utc_datetime(&time), nth))
}

fn seen_mark(file: &Path) -> PathBuf {
    let mut name = file.file_name().map(|name| name.to_os_string()).unwrap_or_default();
    name.push(".seen");
    file.with_file_name(name)
}

/// Where the report says it panicked.
pub(crate) fn location_line(text: &str) -> String {
    text.lines().find_map(|line| line.strip_prefix("at: ")).unwrap_or_default().to_string()
}

/// The first panic line of a report's text.
pub(crate) fn panic_line(text: &str) -> String {
    text.lines().find_map(|line| line.strip_prefix("panic: ")).unwrap_or_default().to_string()
}

/// The newest report in `dir` not marked seen.
pub(crate) fn newest_unseen(dir: &Path) -> Option<Crash> {
    let entries = std::fs::read_dir(dir).ok()?;
    let ((at, _), file) = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| Some((time_of(entry.file_name().to_str()?)?, entry.path())))
        .filter(|(_, file)| !seen_mark(file).exists())
        .max()?;
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    Some(Crash { file, at, panic: panic_line(&text), location: location_line(&text), run: None })
}

/// The newest report not yet seen.
pub(crate) fn last_unseen() -> Option<Crash> {
    newest_unseen(&crashes_dir()?)
}

/// He has been told of this one. macOS's report is left as it is, and the mark of the run it ended goes.
pub(crate) fn mark_seen(crash: &Crash) -> std::io::Result<()> {
    match &crash.run {
        Some(run) => std::fs::remove_file(run).or_else(|why| if why.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(why) }),
        None => std::fs::write(seen_mark(&crash.file), ""),
    }
}

// ---- a run that did not quit -----------------------------------------------------------------------

/// The bundle the native app is built as, which macOS names in the first line of each report.
const BUNDLE: &str = "dev.armada.bridge.native";

fn run_mark(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("run-{pid}"))
}

/// This run has begun. The mark's time is when.
pub(crate) fn begin_run(dir: &Path, pid: u32) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(run_mark(dir, pid), "")
}

/// This run ended as he meant, or a panic report speaks for it.
pub(crate) fn end_run(dir: &Path, pid: u32) {
    let _ = std::fs::remove_file(run_mark(dir, pid));
}

/// A run that ended without a quit he meant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub pid: u32,
    pub began: SystemTime,
    pub mark: PathBuf,
}

/// Every run mark but `me`'s. One native app runs at a time (`instance.rs`), so each is a run that ended.
pub(crate) fn ended_runs(dir: &Path, me: u32) -> Vec<Run> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.strip_prefix("run-")?.parse().ok()?;
            let began = entry.metadata().ok()?.modified().ok()?;
            (pid != me).then(|| Run { pid, began, mark: entry.path() })
        })
        .collect()
}

/// The marks of ended runs, but the one a crash is offered by: a run macOS wrote no report of was killed or
/// stopped, and an older crash than the newest is not offered.
pub(crate) fn forget_runs(runs: &[Run], offered: Option<&Crash>) {
    for run in runs.iter().filter(|run| offered.and_then(|crash| crash.run.as_ref()) != Some(&run.mark)) {
        let _ = std::fs::remove_file(&run.mark);
    }
}

#[derive(serde::Deserialize)]
struct IpsHead {
    #[serde(rename = "bundleID", default)]
    bundle: String,
}

#[derive(serde::Deserialize)]
struct IpsBody {
    pid: u32,
    #[serde(default)]
    exception: IpsException,
    #[serde(rename = "faultingThread", default)]
    faulting: usize,
    #[serde(default)]
    threads: Vec<IpsThread>,
    #[serde(rename = "usedImages", default)]
    images: Vec<IpsImage>,
}

#[derive(serde::Deserialize, Default)]
struct IpsException {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    signal: String,
    #[serde(default)]
    subtype: String,
}

#[derive(serde::Deserialize)]
struct IpsThread {
    #[serde(default)]
    frames: Vec<IpsFrame>,
}

#[derive(serde::Deserialize)]
struct IpsFrame {
    #[serde(rename = "imageIndex", default)]
    image: usize,
    #[serde(rename = "imageOffset", default)]
    offset: u64,
    symbol: Option<String>,
}

#[derive(serde::Deserialize)]
struct IpsImage {
    name: Option<String>,
}

/// What macOS's report says of a crash of this app.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacosCrash {
    pub pid: u32,
    /// As `EXC_BAD_ACCESS (SIGSEGV), KERN_INVALID_ADDRESS at 0x10`.
    pub exception: String,
    /// The faulting thread's top frame, as `symbol in image`.
    pub frame: String,
}

/// A report's text read: a first line of JSON naming the app, then the report as JSON. None where it is
/// another app's or does not read.
pub(crate) fn read_ips(text: &str) -> Option<MacosCrash> {
    let (head, body) = text.split_once('\n')?;
    let head: IpsHead = ipc::decode("the first line of macOS's crash report", head.as_bytes()).ok()?;
    if head.bundle != BUNDLE {
        return None;
    }
    let body: IpsBody = ipc::decode("macOS's crash report", body.as_bytes()).ok()?;
    let said = &body.exception;
    let mut exception = if said.kind.is_empty() { "no exception named".to_string() } else { said.kind.clone() };
    if !said.signal.is_empty() {
        let _ = write!(exception, " ({})", said.signal);
    }
    if !said.subtype.is_empty() {
        let _ = write!(exception, ", {}", said.subtype);
    }
    let frame = body
        .threads
        .get(body.faulting)
        .and_then(|thread| thread.frames.first())
        .map(|top| {
            let image = body.images.get(top.image).and_then(|image| image.name.clone()).unwrap_or_else(|| "an unnamed image".into());
            match &top.symbol {
                Some(symbol) => format!("{symbol} in {image}"),
                None => format!("{image} +0x{:x}", top.offset),
            }
        })
        .unwrap_or_default();
    Some(MacosCrash { pid: body.pid, exception, frame })
}

fn is_armada_ips(name: &str) -> bool {
    name.starts_with("Armada") && name.ends_with(".ips")
}

/// The newest of macOS's reports in `reports` of one of `runs`: from its pid, and written after it began.
pub(crate) fn newest_macos(reports: &Path, runs: &[Run]) -> Option<Crash> {
    let earliest = runs.iter().map(|run| run.began).min()?;
    std::fs::read_dir(reports)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_str().is_some_and(is_armada_ips))
        .filter_map(|entry| Some((entry.path(), entry.metadata().ok()?.modified().ok()?)))
        .filter(|(_, written)| *written >= earliest)
        .filter_map(|(file, written)| {
            let said = read_ips(&std::fs::read_to_string(&file).ok()?)?;
            let run = runs.iter().find(|run| run.pid == said.pid && written >= run.began)?;
            Some(Crash { file, at: written.into(), panic: said.exception, location: said.frame, run: Some(run.mark.clone()) })
        })
        .max_by_key(|crash| crash.at)
}

/// How long after a report macOS may have written its own and still be taken for the same crash.
const REPORT_WINDOW: Duration = Duration::from_secs(5 * 60);

/// Of macOS's reports (path and modified time), the newest written within the window after `at`.
pub(crate) fn pick_report(found: &[(PathBuf, SystemTime)], at: SystemTime) -> Option<PathBuf> {
    found
        .iter()
        .filter(|(_, modified)| modified.duration_since(at).is_ok_and(|after| after <= REPORT_WINDOW))
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path.clone())
}

/// The report macOS wrote for this crash: `Armada*.ips` in `dir`.
pub(crate) fn macos_report(dir: &Path, crash: &Crash) -> Option<PathBuf> {
    let found: Vec<(PathBuf, SystemTime)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_str().is_some_and(is_armada_ips))
        .filter_map(|entry| Some((entry.path(), entry.metadata().ok()?.modified().ok()?)))
        .collect();
    pick_report(&found, crash.at.into())
}

/// A draft left in a box, as the panel lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeptDraft {
    pub key: String,
    /// What the panel names it by: Dispatch sheet, Composer or Note.
    pub kind: &'static str,
    /// The kind and the Job it was for, where there is one.
    pub label: String,
    pub first_line: String,
}

/// The first line that says something.
fn first_line(text: &str) -> String {
    text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_string()
}

/// The drafts with words, labelled by the place that kept them. `drafts` are the dispatch sheet's, the
/// Operator's, the annotation's and the walks' (`drafts.rs`), `by_job` the Job views' (`job_view/composer.rs`),
/// and `title_of` a Job's title by its id.
pub(crate) fn kept_in(drafts: &HashMap<String, String>, by_job: &HashMap<String, crate::job_view::composer::Kept>, title_of: &dyn Fn(&str) -> Option<String>) -> Vec<KeptDraft> {
    let mut listed: Vec<KeptDraft> = drafts
        .iter()
        .filter_map(|(key, words)| {
            let (kind, label, words) = match key.as_str() {
                "dispatch:request" => ("Dispatch sheet", "Dispatch sheet", words.clone()),
                // The Operator's box keeps its words and chips together as one text.
                "operator:compose" => ("Composer", "Operator composer", ipc::decode::<crate::job_view::composer::Kept>("the Operator's draft", words.as_bytes()).map(|kept| kept.text).unwrap_or_else(|_| words.clone())),
                "annotate:note" => ("Note", "Note", words.clone()),
                key if key.starts_with("walk-note:") => ("Note", "Note", words.clone()),
                _ => return None,
            };
            let first_line = first_line(&words);
            (!first_line.is_empty()).then(|| KeptDraft { key: key.clone(), kind, label: label.to_string(), first_line })
        })
        .collect();
    listed.extend(by_job.iter().filter_map(|(key, kept)| {
        let first_line = first_line(&kept.text);
        if first_line.is_empty() {
            return None;
        }
        let named = |job: &str| title_of(job).unwrap_or_else(|| job.to_string());
        let (kind, label) = match key.split(':').collect::<Vec<_>>().as_slice() {
            [job] => ("Composer", format!("Composer \u{b7} {}", named(job))),
            ["teach", job] => ("Note", format!("Teach \u{b7} {}", named(job))),
            ["waiting", job, _] => ("Note", format!("Answer \u{b7} {}", named(job))),
            [_, job, ..] if title_of(job).is_some() => ("Note", format!("Note \u{b7} {}", named(job))),
            _ => ("Note", "Note".to_string()),
        };
        Some(KeptDraft { key: key.clone(), kind, label, first_line })
    }));
    listed.sort_by(|a, b| (&a.label, &a.key).cmp(&(&b.label, &b.key)));
    listed
}

/// What the stores kept in their files, read now.
pub(crate) fn kept_now(title_of: &dyn Fn(&str) -> Option<String>) -> Vec<KeptDraft> {
    let drafts = crate::drafts::file().map(|path| crate::drafts::read_file(&path)).unwrap_or_default();
    let by_job = crate::job_view::composer::read_drafts_file();
    kept_in(&drafts, &by_job, title_of)
}

// ---- Start a Job to fix it -----------------------------------------------------------------------

/// The repository the fix is for, among those Fleet serves, by the name the picker gives it.
pub(crate) fn armada_repository(served: &[crate::dispatch::live::Served]) -> Option<ManifestId> {
    served.iter().find(|one| one.name.as_ref().eq_ignore_ascii_case("armada")).map(|one| one.id.clone())
}

fn upload(name: &str, bytes: &[u8]) -> SessionUpload {
    SessionUpload { name: name.to_string(), media_type: "text/plain".to_string(), data: base64::engine::general_purpose::STANDARD.encode(bytes) }
}

/// The dispatch for "Start a Job to fix it": the words, with the report and macOS's own attached.
/// `files` are each file's name and bytes.
pub(crate) fn fix_dispatch(crash: &Crash, files: Vec<(String, Vec<u8>)>, repository: ManifestId) -> DispatchJob {
    let at = crash.at.with_timezone(&Local).format("%Y-%m-%d %H:%M");
    let text = match crash.run {
        None => format!("Armada quit at {at} on this panic. Find the cause and fix it.\n\n{}", crash.panic),
        Some(_) => format!("Armada quit at {at} on this crash, which macOS reported. Find the cause and fix it.\n\n{}\n{}", crash.panic, crash.location),
    };
    let start = StartSession {
        manifest_id: repository,
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some(text.clone()),
        held: false,
        after: None,
        project_rows: Vec::new(),
        on_your_words: None,
        project: None,
        workflow: None,
    };
    let attachments = files.iter().map(|(name, bytes)| upload(name, bytes)).collect();
    let then = vec![DispatchThen::Message(DispatchMessage { text, attachments, mentions: Vec::new(), sketches: Vec::new() })];
    DispatchJob { start, kind: Some(DispatchKind::Bug), then }
}

/// The same, reading the report and macOS's from disk. Off the main thread: it reads files.
pub(crate) fn fix_it(crash: &Crash, macos: Option<&Path>, repository: ManifestId) -> std::io::Result<DispatchJob> {
    let mut files = Vec::new();
    for path in std::iter::once(crash.file.as_path()).chain(macos) {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("report").to_string();
        files.push((name, std::fs::read(path)?));
    }
    Ok(fix_dispatch(crash, files, repository))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("armada-crash-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn at(text: &str) -> DateTime<Utc> {
        Utc.from_utc_datetime(&NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H%M%SZ").unwrap())
    }

    fn facts<'a>(message: &'a str, backtrace: &'a str) -> Facts<'a> {
        let time = at("2026-11-14T110102Z");
        Facts { at: time, local: time.with_timezone(&FixedOffset::east_opt(3600).unwrap()), build: "bridge-native 0.0.0 protocol abc", thread: "main", message, location: Some("src/x.rs:3:9"), backtrace }
    }

    #[test]
    fn a_report_names_the_time_the_build_the_thread_the_panic_and_where() {
        let text = report(&facts("already borrowed\nsecond line", "0: frame"));
        assert_eq!(
            text,
            "time: 2026-11-14 12:01:02 +01:00\ntime utc: 2026-11-14T11:01:02Z\nbuild: bridge-native 0.0.0 protocol abc\nthread: main\npanic: already borrowed\nsecond line\nat: src/x.rs:3:9\n\nbacktrace:\n0: frame\n"
        );
        assert_eq!(panic_line(&text), "already borrowed");
        assert_eq!(location_line(&text), "src/x.rs:3:9");
        assert_eq!(panic_line(&report(&facts("", ""))), "no message");
    }

    #[test]
    fn two_reports_in_one_second_are_both_kept_and_the_newest_unseen_one_is_found() {
        let dir = dir("reports");
        let first = write_report(&dir, at("2026-11-14T110102Z"), "panic: one\n").unwrap();
        let second = write_report(&dir, at("2026-11-14T110102Z"), "panic: two\n").unwrap();
        let later = write_report(&dir, at("2026-11-14T120000Z"), "time: x\npanic: three\n").unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "panic: one\n", "the second did not overwrite the first");
        std::fs::write(dir.join("notes.txt"), "not a report").unwrap();

        let newest = newest_unseen(&dir).expect("a report stands");
        assert_eq!((newest.file.as_path(), newest.at, newest.panic.as_str()), (later.as_path(), at("2026-11-14T120000Z"), "three"));
        assert_eq!(newest.location, "", "a report with no place names none");

        mark_seen(&newest).unwrap();
        let next = newest_unseen(&dir).expect("the others stand");
        assert_eq!(next.file, second, "a seen report is passed over; the later of the same second wins");
        mark_seen(&next).unwrap();
        mark_seen(&newest_unseen(&dir).unwrap()).unwrap();
        assert_eq!(newest_unseen(&dir), None);
        assert!(later.exists(), "marking seen leaves the report where it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_report_macos_wrote_is_the_newest_within_minutes_after_the_crash() {
        let crash = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let after = |seconds: u64| crash + Duration::from_secs(seconds);
        let found = vec![
            (PathBuf::from("before.ips"), crash - Duration::from_secs(30)),
            (PathBuf::from("near.ips"), after(4)),
            (PathBuf::from("newer.ips"), after(40)),
            (PathBuf::from("late.ips"), after(3600)),
        ];
        assert_eq!(pick_report(&found, crash), Some(PathBuf::from("newer.ips")));
        assert_eq!(pick_report(&found[..1], crash), None, "one written before the crash is another crash");
        assert_eq!(pick_report(&found[3..], crash), None, "one an hour later is another crash");
        assert_eq!(pick_report(&[], crash), None);
    }

    #[test]
    fn only_armada_reports_are_looked_at() {
        let reports = dir("macos");
        let crash = Crash { file: reports.join("crash-2026-11-14T110102Z.txt"), at: Utc::now(), panic: String::new(), location: String::new(), run: None };
        std::fs::write(reports.join("git-2026-11-14-160242.ips"), "x").unwrap();
        assert_eq!(macos_report(&reports, &crash), None);
        std::fs::write(reports.join("Armada-2026-11-14-111520.ips"), "x").unwrap();
        assert_eq!(macos_report(&reports, &crash), Some(reports.join("Armada-2026-11-14-111520.ips")));
        let _ = std::fs::remove_dir_all(&reports);
    }

    /// A report as macOS writes one: a first line naming the app, then the report, cut to what is read.
    fn ips(bundle: &str, pid: u32) -> String {
        let head = format!(r#"{{"app_name":"Armada","bug_type":"309","bundleID":"{bundle}","name":"Armada"}}"#);
        let body = format!(
            r#"{{"pid":{pid},"procName":"Armada","exception":{{"codes":"0x1, 0x10","type":"EXC_BAD_ACCESS","signal":"SIGSEGV","subtype":"KERN_INVALID_ADDRESS at 0x0000000000000010"}},"faultingThread":1,
"threads":[{{"frames":[{{"imageOffset":4,"imageIndex":0}}]}},{{"triggered":true,"frames":[{{"imageOffset":7292348,"symbol":"WTF::Detail::CallableWrapper::call(std::__1::error_code)","imageIndex":1}},{{"imageOffset":9,"imageIndex":0}}]}}],
"usedImages":[{{"name":"Armada"}},{{"name":"WebKit"}}]}}"#
        );
        format!("{head}\n{body}")
    }

    #[test]
    fn macos_report_names_the_exception_and_the_frame_it_faulted_in() {
        let said = read_ips(&ips("dev.armada.bridge.native", 73126)).expect("the report reads");
        assert_eq!(said.pid, 73126);
        assert_eq!(said.exception, "EXC_BAD_ACCESS (SIGSEGV), KERN_INVALID_ADDRESS at 0x0000000000000010");
        assert_eq!(said.frame, "WTF::Detail::CallableWrapper::call(std::__1::error_code) in WebKit", "the faulting thread's top frame, not the first thread's");
        assert_eq!(read_ips(&ips("dev.armada.bridge.electron", 73126)), None, "another bundle's report is not read");
        assert_eq!(read_ips("not a report"), None);
    }

    #[test]
    fn macos_report_of_a_run_that_did_not_quit_is_offered_once() {
        let crashes = dir("runs");
        let reports = dir("runs-macos");
        let written = |name: &str, text: String, at: SystemTime| {
            let path = reports.join(name);
            std::fs::write(&path, text).unwrap();
            std::fs::File::options().write(true).open(&path).unwrap().set_modified(at).unwrap();
            path
        };
        let began = SystemTime::now() - Duration::from_secs(600);
        begin_run(&crashes, 100).unwrap();
        std::fs::File::options().write(true).open(crashes.join("run-100")).unwrap().set_modified(began).unwrap();
        begin_run(&crashes, 300).unwrap();
        begin_run(&crashes, 7).unwrap();
        end_run(&crashes, 300);

        let runs = ended_runs(&crashes, 7);
        assert_eq!(runs.iter().map(|run| run.pid).collect::<Vec<_>>(), [100], "a run that quit as he meant left no mark, and this run's own is passed over");
        assert_eq!(newest_macos(&reports, &runs), None, "no report yet");

        written("Armada-2026-11-17-150000.ips", ips(BUNDLE, 100), began - Duration::from_secs(60));
        written("Armada-2026-11-17-161000.ips", ips(BUNDLE, 200), began + Duration::from_secs(60));
        written("Armada-2026-11-17-161100.ips", ips("dev.armada.bridge.electron", 100), began + Duration::from_secs(120));
        written("git-2026-11-17-161200.ips", ips(BUNDLE, 100), began + Duration::from_secs(180));
        assert_eq!(newest_macos(&reports, &runs), None, "one before the run began, another pid's, another bundle's and another app's are not its crash");

        let ours = written("Armada-2026-11-17-162203.ips", ips(BUNDLE, 100), began + Duration::from_secs(300));
        let crash = newest_macos(&reports, &runs).expect("the run's crash");
        assert_eq!((crash.file.as_path(), crash.panic.as_str(), crash.run.as_deref()), (ours.as_path(), "EXC_BAD_ACCESS (SIGSEGV), KERN_INVALID_ADDRESS at 0x0000000000000010", Some(crashes.join("run-100").as_path())));
        assert_eq!(crash.location, "WTF::Detail::CallableWrapper::call(std::__1::error_code) in WebKit");

        mark_seen(&crash).unwrap();
        assert!(ended_runs(&crashes, 7).is_empty(), "seen, the run's mark is gone, so it shows once");
        assert!(ours.exists() && !reports.join("Armada-2026-11-17-162203.ips.seen").exists(), "macOS's report is left as it is");
        assert!(crashes.join("run-7").exists(), "this run's mark stands until it quits");
        let _ = std::fs::remove_dir_all(&crashes);
        let _ = std::fs::remove_dir_all(&reports);
    }

    #[test]
    fn ended_runs_with_no_report_are_forgotten_and_the_offered_one_is_kept() {
        let crashes = dir("forget");
        for pid in [1, 2, 3] {
            begin_run(&crashes, pid).unwrap();
        }
        let runs = ended_runs(&crashes, 3);
        let offered = Crash { file: PathBuf::from("Armada.ips"), at: Utc::now(), panic: String::new(), location: String::new(), run: Some(crashes.join("run-2")) };
        forget_runs(&runs, Some(&offered));
        assert_eq!(ended_runs(&crashes, 3).iter().map(|run| run.pid).collect::<Vec<_>>(), [2]);
        forget_runs(&ended_runs(&crashes, 3), None);
        assert!(ended_runs(&crashes, 3).is_empty());
        let _ = std::fs::remove_dir_all(&crashes);
    }

    #[test]
    fn the_panel_lists_each_draft_with_words_by_the_place_that_kept_it() {
        use crate::job_view::composer::Kept;
        let drafts: HashMap<String, String> = [
            ("dispatch:request", "Fix the toast\nand more"),
            ("operator:compose", r#"{"text":"\nAsk about the queue","tags":[]}"#),
            ("walk-note:dispatch", "The gap is wrong"),
            ("annotate:note", ""),
            ("dispatch:session-tuning", "{\"model\":\"opus\"}"),
        ]
        .into_iter()
        .map(|(key, words)| (key.to_string(), words.to_string()))
        .collect();
        let by_job: HashMap<String, Kept> = [("job-1", "  half a thought"), ("teach:job-1", "Remember this"), ("job-2", "")]
            .into_iter()
            .map(|(key, text)| (key.to_string(), Kept { text: text.to_string(), tags: Vec::new() }))
            .collect();
        let title = |job: &str| (job == "job-1").then(|| "Fix the toasts".to_string());

        let kinds: Vec<&str> = kept_in(&drafts, &by_job, &title).into_iter().map(|one| one.kind).collect();
        assert_eq!(kinds, ["Composer", "Dispatch sheet", "Note", "Composer", "Note"], "what the panel's column says");
        let listed: Vec<(String, String)> = kept_in(&drafts, &by_job, &title).into_iter().map(|one| (one.label, one.first_line)).collect();
        let pair = |label: &str, line: &str| (label.to_string(), line.to_string());
        assert_eq!(
            listed,
            vec![
                pair("Composer \u{b7} Fix the toasts", "half a thought"),
                pair("Dispatch sheet", "Fix the toast"),
                pair("Note", "The gap is wrong"),
                pair("Operator composer", "Ask about the queue"),
                pair("Teach \u{b7} Fix the toasts", "Remember this"),
            ]
        );
    }

    #[test]
    fn start_a_job_to_fix_it_sends_the_words_and_both_reports_as_text() {
        let crash = Crash { file: PathBuf::from("crash-2026-11-14T110102Z.txt"), at: at("2026-11-14T110102Z"), panic: "already borrowed".into(), location: "src/x.rs:3:9".into(), run: None };
        let job = fix_dispatch(&crash, vec![("crash-2026-11-14T110102Z.txt".into(), b"panic: x".to_vec()), ("Armada-2026-11-14-111520.ips".into(), b"{}".to_vec())], ManifestId::carried("armada-1"));
        assert_eq!(job.start.manifest_id, ManifestId::carried("armada-1"));
        assert_eq!(job.kind, Some(DispatchKind::Bug));
        let Some(DispatchThen::Message(message)) = job.then.first() else { panic!("one message follows the start") };
        assert!(message.text.starts_with("Armada quit at "), "{}", message.text);
        assert!(message.text.contains("on this panic. Find the cause and fix it."), "{}", message.text);
        assert!(message.text.ends_with("already borrowed"), "{}", message.text);
        assert_eq!(job.start.request.as_deref(), Some(message.text.as_str()), "the brief and the first message are the same words");
        let names: Vec<(&str, &str)> = message.attachments.iter().map(|one| (one.name.as_str(), one.media_type.as_str())).collect();
        assert_eq!(names, vec![("crash-2026-11-14T110102Z.txt", "text/plain"), ("Armada-2026-11-14-111520.ips", "text/plain")]);
        assert_eq!(message.attachments[0].data, base64::engine::general_purpose::STANDARD.encode(b"panic: x"));
    }

    #[test]
    fn start_a_job_to_fix_a_crash_macos_reported_names_the_exception_and_the_frame() {
        let crash = Crash {
            file: PathBuf::from("Armada-2026-11-17-162203.ips"),
            at: at("2026-11-17T212203Z"),
            panic: "EXC_BAD_ACCESS (SIGSEGV)".into(),
            location: "call() in WebKit".into(),
            run: Some(PathBuf::from("run-100")),
        };
        let job = fix_dispatch(&crash, vec![("Armada-2026-11-17-162203.ips".into(), b"{}".to_vec())], ManifestId::carried("armada-1"));
        let Some(DispatchThen::Message(message)) = job.then.first() else { panic!("one message follows the start") };
        assert!(message.text.contains("on this crash, which macOS reported. Find the cause and fix it."), "{}", message.text);
        assert!(message.text.ends_with("EXC_BAD_ACCESS (SIGSEGV)\ncall() in WebKit"), "{}", message.text);
        assert_eq!(message.attachments.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Armada-2026-11-17-162203.ips"]);
    }

    #[test]
    fn what_is_held_is_written_whole_by_the_flush_and_a_settled_text_is_not() {
        let dir = dir("held");
        let kept = dir.join("native-drafts.json");
        let by_job = dir.join("native-drafts");
        hold(&kept, "old".into());
        hold(&kept, "{\"armada:kept:dispatch:request\":\"half a thought\"}".into());
        hold(&by_job, "{\"job-1\":{\"text\":\"a reply\"}}".into());
        flush(false);
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "{\"armada:kept:dispatch:request\":\"half a thought\"}", "the latest text, not an earlier one");
        assert_eq!(std::fs::read_to_string(&by_job).unwrap(), "{\"job-1\":{\"text\":\"a reply\"}}");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "no temp file is left");

        settle(&by_job, "{\"job-1\":{\"text\":\"a reply\"}}");
        settle(&kept, "old");
        std::fs::remove_file(&kept).unwrap();
        std::fs::remove_file(&by_job).unwrap();
        flush(false);
        assert!(kept.exists(), "a newer text is still held after an older one settled");
        assert!(!by_job.exists(), "a settled text is not written again");
        settle(&kept, "{\"armada:kept:dispatch:request\":\"half a thought\"}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
