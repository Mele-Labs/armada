//! What drives a run, wherever its shell lives: Fleet's own pseudo-terminal, or a keeper's socket
//! (`crate::terminal_keeping`). **The one place that yields a reader, a writer and a resize for a
//! run**, so everything above it, the screen, typing, reading, watches and stop, is the same for both.

use std::io::{Read, Write};
use std::sync::Arc;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

use super::{other, shell, Keeping, Launch};
use crate::terminal_keeping::{client, Files, Keep, Record};

/// What a run is resized and asked about through, wherever its shell lives.
pub(super) trait Pty: Send {
    fn resize(&self, size: PtySize) -> bool;
    /// Whether a program other than the shell, which leads `pgid`, has the foreground.
    fn foreground_taken(&self, pgid: i32) -> bool;
}

impl Pty for Box<dyn MasterPty + Send> {
    fn resize(&self, size: PtySize) -> bool {
        MasterPty::resize(&**self, size).is_ok()
    }

    fn foreground_taken(&self, pgid: i32) -> bool {
        self.process_group_leader().is_some_and(|leader| leader != pgid)
    }
}

impl Pty for Arc<client::Link> {
    fn resize(&self, size: PtySize) -> bool {
        client::Link::resize(self, size.cols, size.rows)
    }

    fn foreground_taken(&self, _: i32) -> bool {
        client::Link::foreground_taken(self)
    }
}

/// What a run is typed into and resized through. Gone once the run has ended.
pub(super) struct Terminal {
    pub(super) pty: Box<dyn Pty>,
    pub(super) keys: Box<dyn Write + Send>,
}

/// A run's shell as a keeper holds it: the connection that ends it, and the files that find it.
pub(super) struct Kept {
    pub(super) link: Arc<client::Link>,
    pub(super) files: Files,
}

/// What a run is driven through, wherever its shell lives: the reading end, the terminal, and the
/// wait for the shell to leave. **The one place that yields a reader, a writer and a resize.**
pub(super) struct Driven {
    pub(super) pgid: i32,
    pub(super) reader: Box<dyn Read + Send>,
    pub(super) terminal: Terminal,
    pub(super) waiting: Box<dyn FnOnce() -> Waited + Send>,
    pub(super) kept: Option<Kept>,
}

/// How a wait for the shell ended.
pub(super) enum Waited {
    /// The shell left, with its code unless a signal ended it.
    Exited(Option<i32>),
    /// The connection to its keeper closed first: the shell may be running still.
    Lost,
}

/// A command or shell that is Fleet's own child, on a pseudo-terminal of Fleet's.
pub(super) fn local_driven(launch: &Launch<'_>) -> std::io::Result<Driven> {
    let pair = native_pty_system().openpty(launch.size).map_err(other)?;
    let mut shell = CommandBuilder::new(shell());
    match launch.command {
        Some(command) => shell.args(["-lc", command]),
        None => shell.arg("-l"),
    }
    shell.cwd(launch.dir);
    shell.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(shell).map_err(other)?;
    // With our slave end open the reader would never be told the terminal had closed.
    drop(pair.slave);
    let pid = child.process_id().ok_or_else(|| other("the command has no process id"))?;
    let reader = pair.master.try_clone_reader().map_err(other)?;
    let keys = pair.master.take_writer().map_err(other)?;
    Ok(Driven {
        // The command is its own session, so its group is its pid.
        pgid: pid as i32,
        reader,
        terminal: Terminal { pty: Box::new(pair.master), keys },
        waiting: Box::new(move || match child.wait() {
            Ok(status) if status.signal().is_none() => Waited::Exited(Some(status.exit_code() as i32)),
            _ => Waited::Exited(None),
        }),
        kept: None,
    })
}

/// A Job's shell under a new keeper, written down so the next Fleet finds it.
pub(super) fn kept_driven(id: &str, launch: &Launch<'_>, keeping: &Keeping) -> std::io::Result<Driven> {
    let session = launch.session.ok_or_else(|| other("a Job's shell names its Job"))?;
    let files = &keeping.files;
    // What an earlier shell under this number left, gone before the new keeper binds.
    files.removed();
    let keep = Keep {
        socket: files.socket.clone(),
        spool: files.spool.clone(),
        dir: launch.dir.into(),
        cols: launch.size.cols,
        rows: launch.size.rows,
        alone_for: crate::terminal_keeping::keeper::ALONE_FOR,
    };
    let stream = keeping.starting.start(&keep, &files.log()).map_err(other)?;
    let record = Record {
        job: session.to_string(),
        number: keeping.number,
        run_id: id.to_string(),
        dir: launch.dir.to_string(),
        socket: files.socket.clone(),
        spool: files.spool.clone(),
    };
    let attached = record.written(&files.record).and_then(|()| client::attach(stream, crate::terminal_keeping::TAIL)).inspect_err(|_| {
        client::end_at(&files.socket);
        files.removed();
    })?;
    Ok(driven_by(attached, files.clone()))
}

/// A run over a keeper already attached to.
pub(super) fn driven_by(attached: client::Attached, files: Files) -> Driven {
    let client::Attached { pid, printed, keys, link, exit } = attached;
    Driven {
        pgid: pid as i32,
        reader: Box::new(printed),
        terminal: Terminal { pty: Box::new(Arc::clone(&link)), keys: Box::new(keys) },
        waiting: Box::new(move || exit.recv().map_or(Waited::Lost, Waited::Exited)),
        kept: Some(Kept { link, files }),
    }
}

