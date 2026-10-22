//! The laps a CI run records travel as one file, and read back as written.
//! `docs/concepts/test-map.md`, *How it stays current*.

use core_model::{EdgeGrain, EdgeSource};

use crate::{decode, encode, RecordedLap, RecordedLaps};

fn recorded() -> RecordedLaps {
    RecordedLaps {
        commit: "4f2a9c1".to_string(),
        check: "test".to_string(),
        source: EdgeSource::Measured.into(),
        laps: vec![
            RecordedLap {
                member: "store".to_string(),
                test: "reads_a_row".to_string(),
                grain: EdgeGrain::Test.into(),
                files: vec!["crates/store/src/row.rs".to_string()],
            },
            RecordedLap {
                member: String::new(),
                test: "tests/store.test.ts".to_string(),
                grain: EdgeGrain::File.into(),
                files: Vec::new(),
            },
        ],
    }
}

#[test]
fn recorded_laps_read_back_as_they_were_written() {
    let written = encode(&recorded()).expect("written");
    let read: RecordedLaps = decode("recorded laps", written.as_bytes()).expect("read");
    assert_eq!(read, recorded());
}

#[test]
fn a_source_the_domain_does_not_have_is_refused_not_defaulted() {
    let said = decode::<RecordedLaps>(
        "recorded laps",
        br#"{"commit":"4f2a9c1","check":"test","source":"guessed","laps":[]}"#,
    )
    .expect_err("refused");
    assert!(said.to_string().contains("guessed"), "{said}");
}
