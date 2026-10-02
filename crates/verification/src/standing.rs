//! What a repository requires of every change, as it wrote it, for a Judge to
//! read beside the work and for the Drone doing it to read first. Read from the
//! repository and never from the Drone, so rule 2's blindness is unchanged.
//! `docs/contracts/agent-prompt.md` section 2, *Judge* and *Drone*.
//!
//! **One value, two wordings.** The Drone is told what its Judge is told, from
//! the same read and under the same bound, so the work is done to the standard
//! it is measured against. Only the sentence that introduces it differs: a
//! Judge is told it is a standard and not scope expansion, a Drone that it is
//! asked of this work.
//!
//! **And a Judge may look the rest up.** Since 2 Oct 2026 a Judge's call can
//! read the repository's checkout, and [`Standing::readable`] is what tells it
//! so: that it may read CLAUDE.md and the docs to learn what the repository
//! requires, and that work the repository requires is not scope expansion. The
//! named file stays as the repository's explicit statement, quoted in full, and
//! a repository that names none still gets a Judge that can read.

/// The most of a repository's standing rules a brief carries, in bytes, a
/// Judge's or a Drone's. `standing-rules-cap` in `crates/config/settings.toml`.
pub const STANDING_RULES: usize = 4 * 1024;

/// What a repository requires of every change, ready for a brief, and never
/// longer than [`STANDING_RULES`]. [`Standing::unstated`] renders nothing, so
/// a repository that names no file gets every brief as it was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    said: Option<Said>,
    /// Whether the Judge's call can read the checkout. Fleet's to set, beside
    /// the read it grants, and only a Judge is told: a Drone reads the
    /// repository already.
    readable: bool,
}

/// What a reading Judge is told before anything the repository named.
///
/// **Not to quote what it read**, because a refusal's quotation marks are held
/// to the brief (`crate::quoted`), and the files it opens are not in the brief:
/// a refusal quoting CLAUDE.md would be discarded as quoting what is not there.
const READABLE: &str = "\
You can read this repository's own checkout, read-only, for a few turns: its \
CLAUDE.md, its docs and its skills, to learn what the repository requires of a \
change. Look up what you need and no more. Work the repository requires of a \
change is not scope expansion, whatever the request says. The checkout is the \
repository as it stands and not the work under judgment, which is only what \
this brief shows. Name a file you read rather than quoting it: quotation marks \
are kept for words in this brief.\n\n";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Said {
    Read {
        path: String,
        text: String,
        /// The whole file's size, where it was over the bound and cut.
        cut_from: Option<usize>,
    },
    Unreadable {
        path: String,
    },
}

impl Standing {
    /// The repository names no file.
    pub fn unstated() -> Standing {
        Standing::default()
    }

    /// The same, for a Judge whose call can read the repository's checkout.
    pub fn readable(self) -> Standing {
        Standing {
            readable: true,
            ..self
        }
    }

    /// The file `path` names held `text`. Over [`STANDING_RULES`] it is cut at
    /// the last whole line that fits, and the brief says so.
    pub fn read(path: &str, text: &str) -> Standing {
        if text.trim().is_empty() {
            return Standing::unstated();
        }
        let cut_from = (text.len() > STANDING_RULES).then_some(text.len());
        let kept = match cut_from {
            None => text,
            Some(_) => {
                let mut end = STANDING_RULES;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                let fits = &text[..end];
                fits.rfind('\n').map_or(fits, |line| &fits[..line])
            }
        };
        Standing::said(Said::Read {
            path: path.to_string(),
            text: kept.trim_end().to_string(),
            cut_from,
        })
    }

    /// The repository names a file and it could not be read. Said rather than
    /// left out, so a kept brief shows why a Judge was not told.
    pub fn unreadable(path: &str) -> Standing {
        Standing::said(Said::Unreadable {
            path: path.to_string(),
        })
    }

    fn said(said: Said) -> Standing {
        Standing {
            said: Some(said),
            readable: false,
        }
    }

    /// The part of a brief this is, or an empty string.
    pub(crate) fn told(&self) -> String {
        let readable = match self.readable {
            true => READABLE,
            false => "",
        };
        format!("{readable}{}", self.named())
    }

    /// The file the repository named, as a Judge's brief carries it.
    fn named(&self) -> String {
        match &self.said {
            None => String::new(),
            Some(Said::Unreadable { path }) => format!(
                "This repository names {path} as what it requires of every change, and it \
                 could not be read, so what it requires is not shown here.\n\n"
            ),
            Some(Said::Read {
                path,
                text,
                cut_from,
            }) => {
                let mut told = String::from(
                    "What this repository requires of every change, in its own words. It \
                     applies to all work here whatever the request says, so work that does \
                     what it requires is not scope expansion. It is a standard the work is \
                     measured against rather than something under judgment:\n\n",
                );
                told.push_str(&quoted(path, text, *cut_from));
                told.push('\n');
                told
            }
        }
    }

    /// The block a Drone's opening brief carries, heading first, or `None`
    /// where the repository names no file — and the brief is then the one it
    /// was before the key existed. The same text as [`Standing::told`] under
    /// the same bound; only the sentence introducing it is the doer's.
    pub fn to_do(&self) -> Option<String> {
        let heading = "WHAT THIS REPOSITORY REQUIRES OF EVERY CHANGE\n\n";
        match &self.said {
            None => None,
            Some(Said::Unreadable { path }) => Some(format!(
                "{heading}This repository names {path} as what it requires of every change, \
                 and it could not be read, so what it requires is not shown here."
            )),
            Some(Said::Read {
                path,
                text,
                cut_from,
            }) => Some(format!(
                "{heading}What this repository requires of every change you make, in its own \
                 words. It applies to this work whatever the brief below asks, and the work \
                 is checked against it, so doing what it requires is part of the job rather \
                 than beyond it:\n\n{}",
                quoted(path, text, *cut_from).trim_end()
            )),
        }
    }
}

/// The file's kept text, indented, and the cut where there was one.
fn quoted(path: &str, text: &str, cut_from: Option<usize>) -> String {
    let mut quoted = String::new();
    for line in text.lines() {
        match line.is_empty() {
            true => quoted.push('\n'),
            false => quoted.push_str(&format!("  {line}\n")),
        }
    }
    if let Some(whole) = cut_from {
        quoted.push_str(&format!(
            "\n  (Cut here. {path} is {whole} bytes and a brief carries the \
             first {STANDING_RULES}; the rest is not shown.)\n"
        ));
    }
    quoted
}
