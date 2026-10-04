//! The pool's shape on this machine: which slots it has, and which a person
//! closed. Written at `.armada/slots/pool` beside the slots' own records, so
//! it outlives Fleet and is never committed; `setup.worktrees` stays the size
//! a fresh machine starts at.
//!
//! **Slots are numbers, not a count**, so removing slot 3 of 5 leaves 1, 2, 4
//! and 5 where they are, held or not. Adding takes the lowest number unused.

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::PathBuf;

use adapter_traits::SLOT_ROOT as SLOTS;

use super::{git, Pool, SlotState};

/// What `.armada/slots/pool` says. `bays` is `None` until a person adds or
/// removes one, and the Manifest's size stands.
#[derive(Debug, Default)]
pub(super) struct Shape {
    pub(super) bays: Option<Vec<usize>>,
    pub(super) closed: BTreeSet<usize>,
}

impl Shape {
    fn read(text: &str) -> Shape {
        let numbers = |name: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(name)?.strip_prefix(' '))
                .map(|said| {
                    said.split_whitespace()
                        .filter_map(|n| n.parse().ok())
                        .collect::<Vec<usize>>()
                })
        };
        Shape {
            bays: numbers("bays").map(|mut bays| {
                bays.sort_unstable();
                bays.dedup();
                bays
            }),
            closed: numbers("closed").unwrap_or_default().into_iter().collect(),
        }
    }

    fn written(&self) -> String {
        let said = |numbers: &mut dyn Iterator<Item = &usize>| {
            numbers.map(usize::to_string).collect::<Vec<_>>().join(" ")
        };
        let mut text = String::new();
        if let Some(bays) = &self.bays {
            text.push_str(&format!("bays {}\n", said(&mut bays.iter())));
        }
        if !self.closed.is_empty() {
            text.push_str(&format!("closed {}\n", said(&mut self.closed.iter())));
        }
        text
    }
}

/// Why a change to the pool's shape changed nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum Unshaped {
    NoSuchSlot(usize),
    /// Its holder, named for a person.
    Held(String),
    Stranded(String),
    Busy,
    NotACheckout,
    Dirty(Vec<String>),
    LastSlot,
    Vcs(String),
}

impl Unshaped {
    /// One sentence, for `armada worktree`.
    pub fn said(&self) -> String {
        match self {
            Unshaped::NoSuchSlot(n) => format!("there is no slot-{n}"),
            Unshaped::Held(holder) => format!("{holder} holds it"),
            Unshaped::Stranded(why) => format!("it is stranded, holding {why}"),
            Unshaped::Busy => String::from("a lease or a release is under way on it"),
            Unshaped::NotACheckout => {
                String::from("it is not a checkout, so it is a person's to remove")
            }
            Unshaped::Dirty(files) => format!(
                "{} uncommitted, first {}",
                files.len(),
                files.first().map(String::as_str).unwrap_or_default()
            ),
            Unshaped::LastSlot => String::from("the pool keeps one slot"),
            Unshaped::Vcs(why) => why.clone(),
        }
    }
}

impl Pool {
    /// Every slot's number, in order: this machine's own where a person
    /// changed it, else `1..=setup.worktrees`.
    pub fn bays(&self) -> Vec<usize> {
        self.shape()
            .bays
            .unwrap_or_else(|| (1..=self.count).collect())
    }

    /// Whether a person closed slot `number`.
    pub fn closed(&self, number: usize) -> bool {
        self.shape().closed.contains(&number)
    }

    /// Whether a lease may take slot `number`: one of the pool's, and open.
    /// Asked again under the slot's lock, since a person may close or remove
    /// it while a lease is choosing.
    pub(super) fn leasable(&self, number: usize) -> bool {
        let shape = self.shape();
        !shape.closed.contains(&number)
            && shape
                .bays
                .map_or(number >= 1 && number <= self.count, |bays| {
                    bays.contains(&number)
                })
    }

    /// One more slot, numbered lowest-unused, and not made: the next lease
    /// makes it.
    pub fn add(&self) -> Result<usize, Unshaped> {
        self.reshaped(|pool, shape| {
            let bays = shape.bays.get_or_insert_with(|| pool.bays());
            let number = (1..).find(|n| !bays.contains(n)).unwrap_or(1);
            bays.push(number);
            bays.sort_unstable();
            Ok(number)
        })
    }

    /// Slot `number`, gone: refused unless free or not made. A made one's
    /// checkout is given back with `git worktree remove`, which refuses one
    /// holding anything uncommitted.
    pub fn remove(&self, number: usize) -> Result<(), Unshaped> {
        self.reshaped(|pool, shape| {
            let bays = shape.bays.get_or_insert_with(|| pool.bays());
            if !bays.contains(&number) {
                return Err(Unshaped::NoSuchSlot(number));
            }
            if bays.len() == 1 {
                return Err(Unshaped::LastSlot);
            }
            let lock = pool
                .locked(number)
                .map_err(Unshaped::Vcs)?
                .ok_or(Unshaped::Busy)?;
            let path = pool.path_of(number);
            match pool.state_of(number) {
                SlotState::Unmade => {}
                SlotState::Free | SlotState::Abandoned { .. } => {
                    match git::dirty(&path) {
                        Ok(files) if !files.is_empty() => return Err(Unshaped::Dirty(files)),
                        Err(why) => return Err(Unshaped::Vcs(why)),
                        Ok(_) => {}
                    }
                    git::git(&pool.root, &["worktree", "remove", &path.to_string_lossy()])
                        .map_err(Unshaped::Vcs)?;
                }
                SlotState::Busy => return Err(Unshaped::Busy),
                SlotState::NotACheckout => return Err(Unshaped::NotACheckout),
                SlotState::Held { holder, .. } => return Err(Unshaped::Held(holder.said())),
                SlotState::Stranded { why, .. } => return Err(Unshaped::Stranded(why)),
            }
            bays.retain(|n| *n != number);
            shape.closed.remove(&number);
            // Written under the slot's lock, so a lease that takes it next
            // finds it gone; the record is the lock, and goes after.
            pool.write_shape(shape)?;
            let _ = std::fs::remove_file(pool.record_path(number));
            drop(lock);
            Ok(())
        })
    }

    /// Never leased until reopened. Its holder keeps it until the lease ends.
    pub fn close(&self, number: usize) -> Result<(), Unshaped> {
        self.reshaped(|pool, shape| {
            if !pool.bays().contains(&number) {
                return Err(Unshaped::NoSuchSlot(number));
            }
            shape.closed.insert(number);
            Ok(())
        })
    }

    pub fn open(&self, number: usize) -> Result<(), Unshaped> {
        self.reshaped(|pool, shape| {
            if !pool.bays().contains(&number) {
                return Err(Unshaped::NoSuchSlot(number));
            }
            shape.closed.remove(&number);
            Ok(())
        })
    }

    pub(super) fn shape(&self) -> Shape {
        std::fs::read_to_string(self.shape_path())
            .map(|text| Shape::read(&text))
            .unwrap_or_default()
    }

    /// Change the shape under the pool's own lock, written only where
    /// `change` said yes.
    fn reshaped<T>(
        &self,
        change: impl FnOnce(&Pool, &mut Shape) -> Result<T, Unshaped>,
    ) -> Result<T, Unshaped> {
        let slots = self.root.join(SLOTS);
        std::fs::create_dir_all(&slots).map_err(|why| Unshaped::Vcs(why.to_string()))?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .append(true)
            .open(slots.join("pool.lock"))
            .map_err(|why| Unshaped::Vcs(why.to_string()))?;
        lock.lock().map_err(|why| Unshaped::Vcs(why.to_string()))?;
        let mut shape = self.shape();
        let answer = change(self, &mut shape)?;
        self.write_shape(&shape)?;
        Ok(answer)
    }

    /// Whole or not at all: a reader never sees half a file.
    fn write_shape(&self, shape: &Shape) -> Result<(), Unshaped> {
        let path = self.shape_path();
        let mut next = path.clone().into_os_string();
        next.push(".next");
        std::fs::write(&next, shape.written())
            .and_then(|()| std::fs::rename(&next, &path))
            .map_err(|why| Unshaped::Vcs(format!("{}: {why}", path.display())))
    }

    fn shape_path(&self) -> PathBuf {
        self.root.join(SLOTS).join("pool")
    }
}
