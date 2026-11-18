//! A call Fleet refused, as one value: the route, the status, and Fleet's own sentence and code where it sent a
//! `WireError`. Every caller reads an answer through `Refusal::read`, so a refusal is decoded once and a failure toast
//! can show what was sent and what came back (`toasts::tell_refused`).

use std::fmt;

use ipc::FleetBuildReport;

/// A call Fleet answered with a status outside 2xx.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub method: &'static str,
    pub route: String,
    pub status: u16,
    /// Fleet's sentence, where it answered with a `WireError`.
    pub said: Option<String>,
    /// Fleet's code, such as `request.too_large`, where it answered with a `WireError`.
    pub code: Option<String>,
    /// The body's size in bytes, where there was one.
    pub sent: Option<usize>,
    /// How many pictures rode in the body, where the caller knows.
    pub pictures: Option<usize>,
}

impl Refusal {
    /// Fleet's answer to `method route`, read for its `WireError`. An older Fleet answers a body over its limit with
    /// a bare 413 that is no `WireError`.
    pub fn read(method: &'static str, route: impl Into<String>, status: u16, body: &[u8]) -> Refusal {
        let wire = ipc::decode::<ipc::WireError>("WireError", body).ok();
        Refusal { method, route: route.into(), status, said: wire.as_ref().map(|error| error.message.clone()), code: wire.map(|error| error.code), sent: None, pictures: None }
    }

    pub fn sent(mut self, bytes: usize) -> Refusal {
        self.sent = Some(bytes);
        self
    }

    pub fn pictures(mut self, count: usize) -> Refusal {
        self.pictures = Some(count);
        self
    }

    /// Why, in a few words: what a headline ends on.
    pub fn short(&self) -> String {
        if self.status == 413 {
            "Fleet refused it as too large".into()
        } else if self.said.is_some() {
            "Fleet refused it".into()
        } else {
            format!("Fleet answered {}", self.status)
        }
    }

    /// The rows a failure toast shows and Copy debug info carries. `build` is Fleet's build as last reported, `report`
    /// what `GET /debug_report` gave.
    pub fn rows(&self, build: Option<&FleetBuildReport>, report: &Report) -> Vec<(&'static str, String)> {
        let (armada, harness) = match report {
            Report::Read(text) => install_of(text),
            _ => (None, None),
        };
        let unread = || match report {
            Report::Reading => "reading".to_string(),
            Report::Read(_) => "not in the report".to_string(),
            Report::Unread(_) => "not read".to_string(),
        };
        vec![
            ("Route", format!("{} {}", self.method, self.route)),
            ("Status", self.code.as_ref().map_or_else(|| self.status.to_string(), |code| format!("{} {code}", self.status))),
            ("Sent", self.sent_said()),
            ("Fleet", build.map_or_else(|| "build not reported".to_string(), build_said)),
            ("Armada", armada.unwrap_or_else(unread)),
            ("Harness", harness.unwrap_or_else(unread)),
            ("Fleet log", "Not shown; sent with the Job".to_string()),
        ]
    }

    fn sent_said(&self) -> String {
        let size = self.sent.map_or_else(|| "not known".to_string(), size_said);
        match self.pictures {
            Some(1) => format!("{size}, 1 picture"),
            Some(count) if count > 0 => format!("{size}, {count} pictures"),
            _ => size,
        }
    }
}

/// Fleet's sentence where it sent one, else the short why: the words the copied decoders gave, but for a bare 413.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.said {
            Some(said) => f.write_str(said),
            None => f.write_str(&self.short()),
        }
    }
}

/// Fleet's debug report, as far as it has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Report {
    Reading,
    Read(String),
    /// Why it could not be read.
    Unread(String),
}

/// Why a call came back without what it asked for: Fleet refused it, or it never reached Fleet or was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failed {
    Refused(Refusal),
    Unanswered(String),
}

impl Failed {
    /// Raise what a person sees for it: the failure toast for a refusal, a told sentence for the rest.
    pub fn tell(&self, headline: impl Into<String>, cx: &mut gpui::App) {
        let headline = headline.into();
        match self {
            Failed::Refused(refusal) => crate::toasts::tell_refused(headline, refusal.clone(), cx),
            Failed::Unanswered(why) => crate::toasts::tell(format!("{headline}: {why}"), cx),
        }
    }

    /// Raise the failure toast where Fleet refused it, and nothing else: for a press whose own line already
    /// says why, so a call that never reached Fleet stays on that line as before.
    pub fn tell_refusal(&self, headline: impl Into<String>, cx: &mut gpui::App) {
        if let Failed::Refused(refusal) = self {
            crate::toasts::tell_refused(headline, refusal.clone(), cx);
        }
    }
}

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failed::Refused(refusal) => refusal.fmt(f),
            Failed::Unanswered(why) => f.write_str(why),
        }
    }
}

impl From<Failed> for String {
    fn from(failed: Failed) -> String {
        failed.to_string()
    }
}

impl From<String> for Failed {
    fn from(why: String) -> Failed {
        Failed::Unanswered(why)
    }
}

impl From<Refusal> for Failed {
    fn from(refusal: Refusal) -> Failed {
        Failed::Refused(refusal)
    }
}

/// The one reading of an answer: its bytes on a 2xx, else the refusal of `method route`, with the body's size.
pub fn answered(method: &'static str, route: &str, sent: Option<usize>, status: u16, bytes: Vec<u8>) -> Result<Vec<u8>, Refusal> {
    if (200..300).contains(&status) {
        return Ok(bytes);
    }
    let refusal = Refusal::read(method, route, status, &bytes);
    Err(match sent {
        Some(size) => refusal.sent(size),
        None => refusal,
    })
}

/// A POST that blocks, so it runs on the background executor: the answer's bytes, or why not.
pub fn post(port: u16, route: &str, body: String) -> Result<Vec<u8>, Failed> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
    let size = body.len();
    let (status, bytes) = runtime.block_on(crate::transport::post(port, route, body))?;
    Ok(answered("POST", route, Some(size), status, bytes)?)
}

/// A size as a person reads it.
pub fn size_said(bytes: usize) -> String {
    const KB: f64 = 1024.;
    let size = bytes as f64;
    if size < KB {
        format!("{bytes} bytes")
    } else if size < KB * KB {
        format!("{:.0} KB", size / KB)
    } else {
        format!("{:.1} MB", size / (KB * KB))
    }
}

/// The build's commit, cut to nine characters, and how far it stands behind main.
fn build_said(build: &FleetBuildReport) -> String {
    let on = match build.on {
        ipc::BuildSource::Main => "main",
        ipc::BuildSource::Preview => "preview",
    };
    let mut said = match &build.commit {
        Some(commit) => format!("{on} {}", commit.chars().take(9).collect::<String>()),
        None => format!("{on}, commit not recorded"),
    };
    if let Some(position) = &build.position {
        said.push_str(&format!(", {} behind main", position.behind));
    }
    said
}

/// Armada's version and the harness with its version, read off the report's Install block (`ipc::debug_report`):
/// its first row is `armada`, its fourth the harness. Labels are padded by at least three spaces.
fn install_of(report: &str) -> (Option<String>, Option<String>) {
    let rows: Vec<(String, String)> = report
        .lines()
        .skip_while(|line| line.trim() != "Install")
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| line.trim().split_once("   ").map(|(label, value)| (label.trim().to_string(), value.trim().to_string())))
        .collect();
    let armada = rows.first().filter(|(label, _)| label == "armada").map(|(_, value)| value.clone());
    let harness = rows.get(3).map(|(label, value)| format!("{label} {value}"));
    (armada, harness)
}

/// What Copy debug info puts on the clipboard and Dispatch job tells the Job: the headline, Fleet's sentence, the rows,
/// and the report whole.
pub fn debug_text(headline: &str, refusal: &Refusal, build: Option<&FleetBuildReport>, report: &Report) -> String {
    let mut lines = vec![headline.to_string()];
    if let Some(said) = &refusal.said {
        lines.push(said.clone());
    }
    lines.push(String::new());
    let rows = refusal.rows(build, report);
    let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0) + 3;
    lines.extend(rows.into_iter().filter(|(label, _)| *label != "Fleet log").map(|(label, value)| format!("{label:<width$}{value}")));
    lines.push(String::new());
    lines.push(match report {
        Report::Read(text) => text.clone(),
        Report::Reading => "Fleet's debug report was still being read.".to_string(),
        Report::Unread(why) => format!("Fleet's debug report was not read: {why}"),
    });
    lines.join("\n")
}

/// `GET /debug_report`, blocking: run it on the background executor.
pub fn read_report(port: u16) -> Report {
    let read = || -> Result<String, String> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = runtime.block_on(crate::transport::get(port, "/debug_report"))?;
        let bytes = answered("GET", "/debug_report", None, status, bytes).map_err(|refusal| refusal.to_string())?;
        ipc::decode::<ipc::DebugReport>("DebugReport", &bytes).map(|report| report.text).map_err(|why| why.to_string())
    };
    read().map_or_else(Report::Unread, Report::Read)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const TOO_LARGE: &[u8] = br#"{"code":"request.too_large","message":"POST /sessions/dispatch carried 14.2 MB; Fleet takes at most 10 MB.","run_id":"r","fields":{},"chain":[]}"#;

    #[test]
    fn a_bare_413_is_a_refusal_with_its_route_status_and_a_sentence_of_our_own() {
        let refusal = answered("POST", "/sessions/dispatch", Some(14_900_000), 413, b"Failed to buffer the request body: length limit exceeded".to_vec()).expect_err("a refusal");
        assert_eq!((refusal.method, refusal.route.as_str(), refusal.status), ("POST", "/sessions/dispatch", 413));
        assert_eq!((refusal.said.as_deref(), refusal.code.as_deref()), (None, None));
        assert_eq!(refusal.to_string(), "Fleet refused it as too large");
        assert_eq!(refusal.sent, Some(14_900_000));
    }

    #[test]
    fn a_413_with_a_wire_error_keeps_fleets_sentence_and_code() {
        let refusal = answered("POST", "/sessions/dispatch", Some(14_900_000), 413, TOO_LARGE.to_vec()).expect_err("a refusal");
        assert_eq!((refusal.route.as_str(), refusal.status), ("/sessions/dispatch", 413));
        assert_eq!(refusal.code.as_deref(), Some("request.too_large"));
        assert_eq!(refusal.to_string(), "POST /sessions/dispatch carried 14.2 MB; Fleet takes at most 10 MB.");
        assert_eq!(refusal.short(), "Fleet refused it as too large");
    }

    #[test]
    fn a_refusal_that_is_no_wire_error_says_the_status_as_before() {
        assert_eq!(Refusal::read("GET", "/x", 502, b"<html>").to_string(), "Fleet answered 502");
    }

    #[test]
    fn a_refusals_rows_read_the_build_and_the_reports_install_block() {
        let build = ipc::decode::<FleetBuildReport>("FleetBuildReport", br#"{"on":"main","commit":"9f3c2a1b7d0e4c5a","position":{"ahead":0,"behind":4}}"#).expect("a report");
        let report = Report::Read("Armada debug report\n\nInstall\n  armada            0.4.2\n  runs from         ~/bin/armada\n  armada resolves   yes\n  agent             2.1.3\n\ntaken now".into());
        let refusal = Refusal::read("POST", "/sessions/dispatch", 413, TOO_LARGE).sent(14_900_000).pictures(3);
        let rows = refusal.rows(Some(&build), &report);
        let value = |label: &str| rows.iter().find(|(one, _)| *one == label).map(|(_, value)| value.as_str()).unwrap_or_default().to_string();
        assert_eq!(value("Route"), "POST /sessions/dispatch");
        assert_eq!(value("Status"), "413 request.too_large");
        assert_eq!(value("Sent"), "14.2 MB, 3 pictures");
        assert_eq!(value("Fleet"), "main 9f3c2a1b7, 4 behind main");
        assert_eq!(value("Armada"), "0.4.2");
        assert_eq!(value("Harness"), "agent 2.1.3");
        assert_eq!(refusal.rows(None, &Report::Reading).iter().find(|(one, _)| *one == "Armada").map(|(_, value)| value.as_str()), Some("reading"));
    }
}
