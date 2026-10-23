//! Running what a tracker mod names: a program with a bound on its time and its output, and the
//! placeholders filled into each value as that value's own text needs.
//!
//! **No shell.** A placeholder becomes part of one argument, a URL or a body, and is never split
//! or read again, so a query holding `;` or `$(...)` is a query and nothing else.

use std::io::{ErrorKind, Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::Secret;

use super::file::{CREDENTIAL, QUERY, REFERENCE};

/// The longest one program, or one HTTP call, may take.
pub(crate) const WITHIN: Duration = Duration::from_secs(30);

/// The most one answer may weigh, a program's output or an HTTP body.
pub(crate) const MOST_OUTPUT: usize = 4 * 1024 * 1024;

/// How much of a program's error output is kept.
const MOST_STDERR: u64 = 16 * 1024;

/// The longest a tracker's own words are kept in a refusal.
const SAID_AT_MOST: usize = 300;

/// What a program said, run to its end.
pub(crate) struct Ran {
    pub(crate) succeeded: bool,
    pub(crate) code: Option<i32>,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: String,
}

/// `argv` run with `stdin`, its output read to at most [`MOST_OUTPUT`], stopped after [`WITHIN`].
/// The refusal names the program and never its arguments, which may hold the credential.
pub(crate) fn run(argv: &[String], stdin: Option<&[u8]>) -> Result<Ran, String> {
    let program = argv.first().map(String::as_str).unwrap_or_default();
    let mut child = Command::new(program)
        .args(&argv[1..])
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| match why.kind() {
            ErrorKind::NotFound => format!("`{program}` is not installed, or not on the PATH Fleet runs with"),
            _ => format!("`{program}` could not be started: {why}"),
        })?;
    // Fed from its own thread, so a program that answers while it is still being fed never
    // waits on a writer that waits on it.
    let feeding = stdin.zip(child.stdin.take()).map(|(bytes, mut pipe)| {
        let bytes = bytes.to_vec();
        std::thread::spawn(move || pipe.write_all(&bytes))
    });
    let over = Arc::new(AtomicBool::new(false));
    let stdout = child.stdout.take().map(|pipe| {
        let over = Arc::clone(&over);
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(MOST_OUTPUT as u64 + 1).read_to_end(&mut bytes);
            over.store(bytes.len() > MOST_OUTPUT, Ordering::SeqCst);
            bytes
        })
    });
    let stderr = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = (&mut pipe).take(MOST_STDERR).read_to_end(&mut bytes);
            // The rest is read and dropped, so a program that says a lot is never blocked on it.
            let _ = std::io::copy(&mut pipe, &mut std::io::sink());
            String::from_utf8_lossy(&bytes).into_owned()
        })
    });
    let started = Instant::now();
    let status = loop {
        if over.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("`{program}` said more than {MOST_OUTPUT} bytes"));
        }
        if started.elapsed() > WITHIN {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("`{program}` did not finish within {} seconds", WITHIN.as_secs()));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(why) => return Err(format!("`{program}` could not be waited on: {why}")),
        }
    };
    if let Some(feeding) = feeding {
        let _ = feeding.join();
    }
    let stdout = stdout.and_then(|it| it.join().ok()).unwrap_or_default();
    if stdout.len() > MOST_OUTPUT {
        return Err(format!("`{program}` said more than {MOST_OUTPUT} bytes"));
    }
    Ok(Ran {
        succeeded: status.success(),
        code: status.code(),
        stdout,
        stderr: stderr.and_then(|it| it.join().ok()).unwrap_or_default(),
    })
}

/// What one call fills its placeholders with.
#[derive(Default)]
pub(crate) struct Fills<'a> {
    pub(crate) query: Option<&'a str>,
    pub(crate) reference: Option<&'a str>,
    pub(crate) credential: Option<&'a Secret<String>>,
}

/// How a value is written where it lands.
#[derive(Clone, Copy)]
pub(crate) enum Escape {
    /// An argument or a header: as it is.
    Plain,
    /// Percent-encoded, for a URL.
    Url,
    /// Inside a JSON string, for a body.
    Json,
}

impl Fills<'_> {
    /// `template` with each placeholder replaced. A word that is not a placeholder is left as
    /// written, braces and all; the file's check has refused any this call cannot fill.
    pub(crate) fn fill(&self, template: &str, escape: Escape) -> String {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(at) = rest.find('{') {
            out.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            let word: String = after.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').collect();
            let value = after[word.len()..].starts_with('}').then(|| self.value(&word)).flatten();
            match value {
                Some(value) => {
                    out.push_str(&escaped(value, escape));
                    rest = &after[word.len() + 1..];
                }
                None => {
                    out.push('{');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    fn value(&self, word: &str) -> Option<&str> {
        match word {
            QUERY => self.query,
            REFERENCE => self.reference,
            CREDENTIAL => self.credential.map(|it| it.expose().as_str()),
            _ => None,
        }
    }

    /// `said` with the credential taken out, in every form a call could have written it, and cut
    /// to a length a refusal carries. **Every refusal a call makes passes through here.**
    pub(crate) fn scrubbed(&self, said: &str) -> String {
        let mut said = said.trim().to_string();
        if let Some(secret) = self.credential.map(|it| it.expose()).filter(|it| !it.is_empty()) {
            for form in [Escape::Plain, Escape::Url, Escape::Json] {
                said = said.replace(&escaped(secret, form), "[credential]");
            }
        }
        match said.char_indices().nth(SAID_AT_MOST) {
            Some((at, _)) => format!("{}…", &said[..at]),
            None => said,
        }
    }
}

fn escaped(value: &str, escape: Escape) -> String {
    match escape {
        Escape::Plain => value.to_string(),
        Escape::Url => value
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (byte as char).to_string(),
                other => format!("%{other:02X}"),
            })
            .collect(),
        // `ipc`'s encoder writes the string with its quotes; inside a body's own quotes it goes without.
        Escape::Json => ipc::encode(&value)
            .ok()
            .and_then(|quoted| quoted.strip_prefix('"').and_then(|it| it.strip_suffix('"')).map(str::to_string))
            .unwrap_or_default(),
    }
}

/// What a program that failed said: its error output, or its exit where it said nothing.
pub(crate) fn failed_with(ran: &Ran) -> String {
    let said = ran.stderr.trim();
    match (said.is_empty(), ran.code) {
        (false, _) => said.to_string(),
        (true, Some(code)) => format!("it exited with {code} and said nothing"),
        (true, None) => "it was stopped by a signal".to_string(),
    }
}
