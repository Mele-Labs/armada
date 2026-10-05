//! `armada need`'s forms: `<path> "<what>"`, `--took <path> "<value>"`,
//! `--release <path>` and `--status`.

use std::fmt;

use super::{Fault, Verb, NEED};

/// Which of `armada need`'s forms was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeedAct {
    /// `<path> "<what>"`: declare a need for this branch, and hear who is ahead.
    Declare { path: String, what: String },
    /// `--took <path> "<value>"`: what this branch took there.
    Took { path: String, value: String },
    /// `--release <path>`: give the need back.
    Release { path: String },
    /// `--status`: every need standing, by path and in order.
    Status,
}

/// The forms, as `armada help` prints them.
pub(super) fn usage(out: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (shape, what) in [
        (
            "<path> \"<what>\"          ",
            "declare what this branch needs there; says who is ahead",
        ),
        (
            "--took <path> \"<value>\"  ",
            "say what this branch took (V95, 23.5)",
        ),
        ("--release <path>         ", "give a need back"),
        (
            "--status                 ",
            "every need, by path and in order",
        ),
    ] {
        writeln!(out, "  armada {NEED} {shape}  {what}")?;
    }
    Ok(())
}

pub(super) fn read(rest: &[String], faults: &mut Vec<Fault>) -> Option<Verb> {
    let shapes = "`need <path> \"<what>\"`, `need --took <path> \"<value>\"`, \
                  `need --release <path>`, `need --status`"
        .to_string();
    let wrong = |faults: &mut Vec<Fault>| {
        faults.push(Fault::NeedForm {
            shapes: shapes.clone(),
        });
        None
    };
    let act = match rest.first().map(String::as_str) {
        Some("--status") if rest.len() == 1 => NeedAct::Status,
        Some("--release") if rest.len() == 2 => NeedAct::Release {
            path: rest[1].clone(),
        },
        Some("--took") if rest.len() == 3 => NeedAct::Took {
            path: rest[1].clone(),
            value: rest[2].clone(),
        },
        Some(path) if !path.starts_with('-') && rest.len() == 2 => NeedAct::Declare {
            path: path.to_string(),
            what: rest[1].clone(),
        },
        _ => return wrong(faults),
    };
    Some(Verb::Need(act))
}
