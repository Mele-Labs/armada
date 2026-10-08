//! The ID is one string on the wire.

use crate::{decode, encode, Cursor, JobList, ProtocolId, Resync, StreamMessage};

fn resync() -> Resync {
    Resync {
        protocol_id: ProtocolId::current(),
        cursor: Cursor::at(0),
        jobs: JobList {
            jobs: Vec::new(),
            unreadable: Vec::new(),
        },
    }
}

#[test]
fn the_id_crosses_as_one_string_field() {
    let json = encode(&resync()).expect("plain data");
    assert!(
        json.contains(&format!(r#""protocol_id":"{}""#, ProtocolId::current())),
        "{json}"
    );
    assert_eq!(
        decode::<Resync>("resync", json.as_bytes()).expect("it round-trips"),
        resync()
    );
}

#[test]
fn a_message_from_before_the_id_does_not_read() {
    let body = br#"{"message":"resync","protocol_version":{"major":23,"minor":71},"cursor":0,
        "jobs":{"jobs":[],"unreadable":[]}}"#;
    assert!(decode::<StreamMessage>("stream message", body).is_err());
}
