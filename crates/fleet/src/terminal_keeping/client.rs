//! Fleet's end of a terminal keeper's socket: the three things a run needs of a terminal, a
//! reader, a writer and a resize, and a way to end it. `super::keeper` holds the other end.

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::wire;

/// How long a keeper has to answer an attach.
const READY_WITHIN: Duration = Duration::from_secs(5);

/// A connection's writing end, shared by the keys, the resize and the end, with what the keeper
/// last said about the foreground.
pub(crate) struct Link {
    write: Mutex<UnixStream>,
    foreground: AtomicBool,
}

impl Link {
    fn send(&self, kind: u8, payload: &[u8]) -> bool {
        let mut write = self.write.lock().expect("not poisoned");
        wire::write_frame(&mut *write, kind, payload).is_ok()
    }

    /// Tell the keeper to stop the shell and go. Best effort: a keeper that is gone says so by
    /// closing the connection.
    pub(crate) fn end(&self) {
        self.send(wire::END, &[]);
    }

    pub(crate) fn resize(&self, cols: u16, rows: u16) -> bool {
        let mut payload = cols.to_be_bytes().to_vec();
        payload.extend_from_slice(&rows.to_be_bytes());
        self.send(wire::RESIZE, &payload)
    }

    /// Close the connection, as a Fleet that died does.
    #[cfg(test)]
    pub(crate) fn close(&self) {
        let _ = self.write.lock().expect("not poisoned").shutdown(std::net::Shutdown::Both);
    }

    /// Whether a program other than the shell has the terminal's foreground.
    pub(crate) fn foreground_taken(&self) -> bool {
        self.foreground.load(Ordering::SeqCst)
    }
}

/// What the keeper printed, as the bytes arrived. Ends where the shell has exited and all it
/// printed is read, or where the connection closed first.
pub(crate) struct Printed {
    stream: UnixStream,
    link: Arc<Link>,
    pending: Vec<u8>,
    taken: usize,
    done: bool,
    exit: Option<mpsc::Sender<Option<i32>>>,
}

impl Read for Printed {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.taken < self.pending.len() {
                let count = buffer.len().min(self.pending.len() - self.taken);
                buffer[..count].copy_from_slice(&self.pending[self.taken..self.taken + count]);
                self.taken += count;
                return Ok(count);
            }
            if self.done {
                return Ok(0);
            }
            match wire::read_frame(&mut self.stream) {
                Ok(Some((wire::OUTPUT, bytes))) => {
                    self.pending = bytes;
                    self.taken = 0;
                }
                Ok(Some((wire::FOREGROUND, payload))) => {
                    self.link.foreground.store(payload.first() == Some(&1), Ordering::SeqCst);
                }
                Ok(Some((wire::EXIT, payload))) => {
                    self.done = true;
                    if let Some(exit) = self.exit.take() {
                        let _ = exit.send(exit_code(&payload));
                    }
                }
                Ok(Some(_)) => {}
                // The connection closed with no exit: the exit sender is dropped unsent.
                Ok(None) | Err(_) => self.done = true,
            }
        }
    }
}

fn exit_code(payload: &[u8]) -> Option<i32> {
    match payload {
        [1, a, b, c, d, ..] => Some(i32::from_be_bytes([*a, *b, *c, *d])),
        _ => None,
    }
}

/// The keys: each write is a frame, so two writers never split one another's.
pub(crate) struct Keys(Arc<Link>);

impl Write for Keys {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(wire::MOST) {
            if !self.0.send(wire::INPUT, chunk) {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "the terminal's keeper is gone"));
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A keeper attached to.
pub(crate) struct Attached {
    /// The shell's pid, which leads its own group.
    pub(crate) pid: u32,
    pub(crate) printed: Printed,
    pub(crate) keys: Keys,
    pub(crate) link: Arc<Link>,
    /// The shell's exit code where it left with one, sent once its output is read. A connection
    /// that closed first closes this unsent.
    pub(crate) exit: mpsc::Receiver<Option<i32>>,
}

/// A connection to the keeper at `socket`, or `None` where none answers. A socket nobody answers
/// on is a keeper that is gone, and is removed.
pub(crate) fn connect(socket: &Path) -> Option<UnixStream> {
    match UnixStream::connect(socket) {
        Ok(stream) => Some(stream),
        Err(why) if why.kind() == io::ErrorKind::NotFound => None,
        Err(_) => {
            let _ = std::fs::remove_file(socket);
            None
        }
    }
}

/// Attach on `stream`, asking for the last `tail` bytes of the spool before what comes after.
pub(crate) fn attach(stream: UnixStream, tail: u64) -> io::Result<Attached> {
    let mut reading = stream.try_clone()?;
    let link = Arc::new(Link { write: Mutex::new(stream), foreground: AtomicBool::new(false) });
    reading.set_read_timeout(Some(READY_WITHIN))?;
    if !link.send(wire::ATTACH, &tail.to_be_bytes()) {
        return Err(io::Error::new(io::ErrorKind::BrokenPipe, "the keeper would not take an attach"));
    }
    let ready = match wire::read_frame(&mut reading)? {
        Some((wire::READY, payload)) if payload.len() >= 5 => payload,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "the keeper did not answer ready")),
    };
    reading.set_read_timeout(None)?;
    link.foreground.store(ready[4] == 1, Ordering::SeqCst);
    let (exiting, exit) = mpsc::channel();
    Ok(Attached {
        pid: u32::from_be_bytes([ready[0], ready[1], ready[2], ready[3]]),
        printed: Printed { stream: reading, link: Arc::clone(&link), pending: Vec::new(), taken: 0, done: false, exit: Some(exiting) },
        keys: Keys(Arc::clone(&link)),
        link,
        exit,
    })
}

/// Tell the keeper at `socket`, where one answers, to stop its shell and go.
pub(crate) fn end_at(socket: &Path) {
    if let Ok(mut stream) = UnixStream::connect(socket) {
        let _ = wire::write_frame(&mut stream, wire::END, &[]);
    }
}
