//! A Job's terminal outliving Fleet. The shell runs under `armada terminal-keep`, which opens the
//! pseudo-terminal itself, spools what the shell prints and serves a unix socket; Fleet holds a
//! connection, not the terminal, so a restart loses neither the shell nor its screen.
//!
//! The same shape as a hosted session's keeper (`crate::session_host::keeper`), with bytes where
//! that one has lines: [`wire`] holds the frames, [`keeper`] the process, [`client`] Fleet's end.
//! **Only a Job's terminals are kept.** A run with a command is Fleet's child as it was.
//!
//! **A record per terminal**, `<stem>.term` beside its socket, names the Job, the terminal and the
//! run id, so the next Fleet finds the keeper (`crate::session_running` reattaches at boot).
//! Written whole by rename, as the servers' `held` record is, and removed whenever the shell ends.

pub mod client;
pub mod keeper;
pub mod wire;

use std::io;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha1::{Digest, Sha1};

use crate::detach::Detached;
use crate::session_running::{terminal_id, terminal_number};

pub use keeper::run;

/// How much of the spool a Fleet that attaches is replayed: what a run holds on its screen.
pub const TAIL: u64 = 256 * 1024;

/// How long a keeper has to bind its socket.
const WITHIN: Duration = Duration::from_secs(10);

/// What a keeper is started with.
#[derive(Clone, Debug)]
pub struct Keep {
    pub socket: PathBuf,
    pub spool: PathBuf,
    pub dir: PathBuf,
    pub cols: u16,
    pub rows: u16,
    /// How long it runs the shell with no Fleet attached.
    pub alone_for: Duration,
}

/// How a keeper is started.
#[derive(Clone, Debug)]
pub enum Starting {
    /// The `armada` binary, detached and without Fleet's mark, so `crate::orphans` leaves it.
    Binary { exe: PathBuf, home: PathBuf },
    /// A thread of this process, for a test: no binary is built for it, and the thread outlives
    /// whatever Fleet the test drops.
    #[cfg(any(test, feature = "test-support"))]
    InProcess,
}

impl Starting {
    /// This binary, which is the `armada` that serves, with `home` as the user's.
    pub fn this_binary(home: &Path) -> Starting {
        Starting::Binary {
            exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("armada")),
            home: home.to_path_buf(),
        }
    }

    /// Start the keeper `keep` describes and answer a connection to it once its socket is up.
    /// `log` takes what the keeper says on its error output.
    pub fn start(&self, keep: &Keep, log: &Path) -> Result<UnixStream, String> {
        match self {
            Starting::Binary { exe, home } => {
                if let Some(folder) = keep.socket.parent() {
                    crate::session_host::keepers_made(folder, home, Path::new(crate::session_host::PRIVATE_ROOT))?;
                }
                // Emptied, then held for append, so the keeper's error and its last words are one file.
                let logged = std::fs::write(log, "")
                    .and_then(|()| std::fs::OpenOptions::new().append(true).open(log))
                    .map_err(|why| format!("the terminal's log could not be made: {why}"))?;
                let mut keeper = Detached::program(exe)
                    .arg(crate::terminal_keeping::VERB)
                    .arg(&keep.socket)
                    .arg(&keep.spool)
                    .arg(keep.alone_for.as_secs().to_string())
                    .arg(keep.cols.to_string())
                    .arg(keep.rows.to_string())
                    .arg(&keep.dir)
                    .outliving_fleet()
                    .writing_output_to(&logged)
                    .map_err(|why| why.to_string())?
                    .spawn()
                    .map_err(|why| format!("the terminal would not start: {why}"))?;
                // A keeper binds within milliseconds. Polled, and bounded, so one that died says so.
                let waited = std::time::Instant::now();
                loop {
                    if let Ok(stream) = UnixStream::connect(&keep.socket) {
                        return Ok(stream);
                    }
                    if let Ok(Some(status)) = keeper.try_wait() {
                        return Err(format!("the terminal exited ({status}) before it came up{}", crate::session_host::kept::tail_of(log)));
                    }
                    if waited.elapsed() > WITHIN {
                        return Err(format!("the terminal did not come up within {} seconds", WITHIN.as_secs()));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            #[cfg(any(test, feature = "test-support"))]
            Starting::InProcess => {
                if let Some(folder) = keep.socket.parent() {
                    std::fs::create_dir_all(folder).map_err(|why| why.to_string())?;
                }
                let bound = keeper::bind(keep).map_err(|why| why.to_string())?;
                std::thread::spawn(move || bound.serve());
                UnixStream::connect(&keep.socket).map_err(|why| why.to_string())
            }
        }
    }
}

/// The verb `armada` runs a keeper under.
pub const VERB: &str = "terminal-keep";

/// The files of one terminal's keeper, all beside its socket.
#[derive(Clone, Debug)]
pub struct Files {
    pub record: PathBuf,
    pub socket: PathBuf,
    pub spool: PathBuf,
}

impl Files {
    /// Where Job `job`'s terminal `number` keeps its files under `folder`. **A short name, hashed
    /// from the Job's id**, so a socket path fits wherever a session keeper's does.
    pub fn of(folder: &Path, job: &str, number: u32) -> Files {
        let hash = Sha1::digest(job.as_bytes());
        let stem = format!("{:02x}{:02x}{:02x}{:02x}-t{number}", hash[0], hash[1], hash[2], hash[3]);
        Files {
            record: folder.join(format!("{stem}.term")),
            socket: folder.join(format!("{stem}.sock")),
            spool: folder.join(format!("{stem}.spool")),
        }
    }

    pub fn log(&self) -> PathBuf {
        self.spool.with_extension("log")
    }

    /// Everything of the keeper's that Fleet put or may remove. **Fleet takes these away before it
    /// says a shell ended**, so the next keeper under this number does not find the last one's
    /// socket; the keeper takes only what is still its own.
    pub fn removed(&self) {
        for path in [&self.record, &self.socket, &self.spool, &self.log()] {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// What a record says.
#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    pub job: String,
    pub number: u32,
    pub run_id: String,
    pub dir: String,
    pub socket: PathBuf,
    pub spool: PathBuf,
}

impl Record {
    /// Write the record whole or not at all.
    pub fn written(&self, to: &Path) -> io::Result<()> {
        let text = format!(
            "job {}\nterminal {}\nrun {}\ndir {}\nsocket {}\nspool {}\n",
            self.job,
            terminal_id(self.number),
            self.run_id,
            self.dir,
            self.socket.display(),
            self.spool.display()
        );
        let next = to.with_extension("term.next");
        std::fs::write(&next, text)?;
        std::fs::rename(next, to)
    }

    /// The record at `path`, or `None` where it is unreadable or lacks a field.
    pub fn read(path: &Path) -> Option<Record> {
        let text = std::fs::read_to_string(path).ok()?;
        let field = |key: &str| text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix(' ').map(str::to_string));
        Some(Record {
            job: field("job")?,
            number: terminal_number(&field("terminal")?)?,
            run_id: field("run")?,
            dir: field("dir")?,
            socket: PathBuf::from(field("socket")?),
            spool: PathBuf::from(field("spool")?),
        })
    }

    pub fn files(&self, record: &Path) -> Files {
        Files { record: record.to_path_buf(), socket: self.socket.clone(), spool: self.spool.clone() }
    }
}

/// Every record in `folder`.
pub fn records_in(folder: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = listing
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "term"))
        .collect();
    found.sort();
    found
}
