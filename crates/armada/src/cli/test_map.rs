//! `armada test-map`'s form: `record --check <key> --out <file>`, the laps of one Check, written
//! for Fleet to read. `docs/concepts/test-map.md`, *How it stays current*.

use std::fmt;
use std::path::PathBuf;

use super::{Fault, Verb, TEST_MAP};

/// Which of `armada test-map`'s forms was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestMapAct {
    /// `record`: run the collector Armada picks for the Check in this repository, and write the
    /// laps and the commit they were read at to `out`.
    Record { check: String, out: PathBuf },
}

/// The forms, as `armada help` prints them.
pub(super) fn usage(out: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(
        out,
        "  armada {TEST_MAP} record --check <key> --out <file>    \
         write which files each test of one Check reaches to <file>; a coverage build of the \
         workspace, so CI runs it"
    )
}

pub(super) fn read(rest: &[String], faults: &mut Vec<Fault>) -> Option<Verb> {
    let form = rest.first().map(String::as_str);
    if form != Some("record") {
        faults.push(Fault::TestMapFormUnknown {
            given: rest.first().cloned(),
        });
        return None;
    }
    let (mut check, mut out) = (None, None);
    let mut args = rest[1..].iter();
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--check", Some(given)) => check = Some(given.clone()),
            ("--out", Some(given)) => out = Some(PathBuf::from(given)),
            ("--check" | "--out", None) => faults.push(Fault::TestMapNeeds {
                flag: arg.clone(),
                valueless: true,
            }),
            _ => faults.push(Fault::NoSuchFlag {
                given: arg.clone(),
                allowed: vec!["--check".to_string(), "--out".to_string()],
            }),
        }
    }
    for (flag, given) in [("--check", check.is_some()), ("--out", out.is_some())] {
        let said_already = faults
            .iter()
            .any(|fault| matches!(fault, Fault::TestMapNeeds { flag: said, .. } if said == flag));
        if !given && !said_already {
            faults.push(Fault::TestMapNeeds {
                flag: flag.to_string(),
                valueless: false,
            });
        }
    }
    Some(Verb::TestMap(TestMapAct::Record {
        check: check?,
        out: out?,
    }))
}
