//! What a repository requires of every change, as it wrote it, for a Judge to
//! read beside the work. Read from the repository and never from the Drone, so
//! rule 2's blindness is unchanged. `docs/contracts/agent-prompt.md` section 2,
//! *Judge*.
//!
//! **Only the Judge is handed it.** The Drone's opening brief carried the same
//! file from 1 Oct 2026 and stopped on 2 Oct: a Drone reads the repository
//! itself, so the block only repeated it (owner, 2 Oct 2026).
//!
//! **A Judge may look the rest up**, since 2 Oct 2026: [`Standing::readable`]
//! tells it it can read the checkout. The named file stays the repository's
//! explicit statement, and one naming none still gets a Judge that can read.

use crate::wording::{Piece, Wording};

/// The most of a repository's standing rules a Judge's brief carries, in
/// bytes, as it ships. `limits.standingRulesKib` in settings.json moves it.
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
/// a refusal quoting one would be discarded as quoting what is not there.
///
/// **No "turn" in it**, because the convergence look rides the same text and
/// is held to never mention one: a turn count there would be judging the Drone.
pub const JUDGE_READABLE: Piece = Piece {
    id: "judgeReadable",
    shipped: "\
You can read this repository's own checkout, read-only and briefly: the \
instructions it keeps for agents, its docs and its skills, to learn what it \
requires of a change. Look up what you need and no more. Work the repository requires of a \
change is not scope expansion, whatever the request says. The checkout is the \
repository as it stands and not the work under judgment, which is only what \
this brief shows. Name a file you read rather than quoting it: quotation marks \
are kept for words in this brief.",
};

/// What a Judge is told where the file the repository names would not read.
pub const JUDGE_STANDING_UNREADABLE: Piece = Piece {
    id: "judgeStandingUnreadable",
    shipped: "This repository names {path} as what it requires of every change, and it \
              could not be read, so what it requires is not shown here.",
};

/// What a Judge is told before the repository's own rules.
pub const JUDGE_STANDING: Piece = Piece {
    id: "judgeStanding",
    shipped: "What this repository requires of every change, in its own words. It \
              applies to all work here whatever the request says, so work that does \
              what it requires is not scope expansion. It is a standard the work is \
              measured against rather than something under judgment:",
};

/// What a Judge is told where the rules were cut to fit.
pub const JUDGE_STANDING_CUT: Piece = Piece {
    id: "judgeStandingCut",
    shipped: "(Cut here. {path} is {whole} bytes and a brief carries the \
              first {cap}; the rest is not shown.)",
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Said {
    Read {
        path: String,
        text: String,
        /// The whole file's size, where it was over the bound and cut.
        cut_from: Option<usize>,
        /// The bound it was cut at.
        cap: usize,
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
        Standing::read_within(path, text, STANDING_RULES)
    }

    /// [`read`](Standing::read), cut at `cap` bytes rather than what ships.
    pub fn read_within(path: &str, text: &str, cap: usize) -> Standing {
        if text.trim().is_empty() {
            return Standing::unstated();
        }
        let cut_from = (text.len() > cap).then_some(text.len());
        let kept = match cut_from {
            None => text,
            Some(_) => {
                let mut end = cap;
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
            cap,
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
    pub(crate) fn told(&self, wording: &Wording) -> String {
        let readable = match self.readable {
            true => format!("{}\n\n", wording.get(JUDGE_READABLE)),
            false => String::new(),
        };
        format!("{readable}{}", self.named(wording))
    }

    /// The file the repository named, as a Judge's brief carries it.
    fn named(&self, wording: &Wording) -> String {
        match &self.said {
            None => String::new(),
            Some(Said::Unreadable { path }) => format!(
                "{}\n\n",
                wording.fill(JUDGE_STANDING_UNREADABLE, &[("path", path)])
            ),
            Some(Said::Read {
                path,
                text,
                cut_from,
                cap,
            }) => {
                let mut told = format!("{}\n\n", wording.get(JUDGE_STANDING));
                told.push_str(&quoted(path, text, *cut_from, *cap, wording));
                told.push('\n');
                told
            }
        }
    }
}

/// The file's kept text, indented, and the cut where there was one.
fn quoted(
    path: &str,
    text: &str,
    cut_from: Option<usize>,
    cap: usize,
    wording: &Wording,
) -> String {
    let mut quoted = String::new();
    for line in text.lines() {
        match line.is_empty() {
            true => quoted.push('\n'),
            false => quoted.push_str(&format!("  {line}\n")),
        }
    }
    if let Some(whole) = cut_from {
        let cut = wording.fill(
            JUDGE_STANDING_CUT,
            &[
                ("path", path),
                ("whole", &whole.to_string()),
                ("cap", &cap.to_string()),
            ],
        );
        quoted.push_str(&format!("\n  {cut}\n"));
    }
    quoted
}
