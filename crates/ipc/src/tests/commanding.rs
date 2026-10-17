//! The allowlist rows and the Kit changes, as the wire spells them.
//!
//! Nothing generates the DTO types from this crate yet, so these cases are what
//! pin the spellings Bridge mirrors by hand.

use crate::{decode, encode, Instant};

/// An always-allow off the record, as Fleet will convert it. **Every field
/// crosses**, and `run` crosses whole because it is what a removal names.
fn allowed() -> core_model::AllowedCommand {
    core_model::AllowedCommand {
        run: String::from("cargo nextest run -p ipc"),
        allowed_at: core_model::Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z"),
        by: core_model::Actor::Human,
    }
}

#[test]
fn an_allowed_command_crosses_whole_from_the_record() {
    let record = allowed();
    let row = crate::AllowedCommandRow::from(&record);
    assert_eq!(row.allowed_at, Instant::from(&record.allowed_at));
    let json = encode(&row).expect("plain data");
    assert!(
        json.contains("\"run\":\"cargo nextest run -p ipc\""),
        "{json}"
    );
    assert!(json.contains("\"by\":\"human\""), "{json}");
    assert_eq!(
        decode::<crate::AllowedCommandRow>("an allowed command", json.as_bytes())
            .expect("it reads back"),
        row
    );
}

/// **A change and what was applied cross as one tagged shape**, and a Kit row
/// carries its source flat beside the command.
#[test]
fn a_kit_change_and_an_allowlist_row_cross_as_tagged_shapes() {
    let change = crate::RetroChange::AllowCommand {
        command: String::from("grep -a -c"),
    };
    assert_eq!(
        encode(&change).expect("plain data"),
        r#"{"kind":"allow_command","command":"grep -a -c"}"#
    );
    let row = crate::KitAllowedCommand {
        run: String::from("grep -a -c"),
        from: crate::KitAllowedSource::RetroItem {
            lesson_id: String::from("01JOB-3"),
        },
    };
    let json = encode(&row).expect("plain data");
    assert_eq!(
        json,
        r#"{"run":"grep -a -c","source":"retro_item","lesson_id":"01JOB-3"}"#
    );
    assert_eq!(
        decode::<crate::KitAllowedCommand>("a row", json.as_bytes()).expect("reads back"),
        row
    );
}

/// **A change the model wrote badly costs the change and not the item.**
#[test]
fn a_change_that_will_not_read_costs_the_change_and_not_the_item() {
    let item = |change: &str| {
        format!(
            r#"{{"items":[{{"who":"drone","lands_in":"kit","title":"t","what":"w","fix":"f","evidence":["refusal:1"]{change}}}]}}"#
        )
    };
    let read = |text: String| {
        decode::<crate::RetroWritten>("a retro", text.as_bytes())
            .expect("reads")
            .items
    };
    let good = read(item(
        r#","change":{"kind":"allow_command","refusal":"refusal:1","command":"grep -a"}"#,
    ));
    assert_eq!(
        good[0].change,
        Some(crate::RetroChangeAnswered::AllowCommand {
            refusal: String::from("refusal:1"),
            command: Some(String::from("grep -a")),
        })
    );
    for bad in [
        r#","change":{"kind":"set_model","model":"x"}"#,
        r#","change":"allow grep""#,
        r#","change":null"#,
        "",
    ] {
        let items = read(item(bad));
        assert_eq!(items.len(), 1, "the item stays: {bad}");
        assert_eq!(items[0].change, None, "{bad}");
    }
}
