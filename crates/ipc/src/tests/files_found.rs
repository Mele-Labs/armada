//! `FilesFound` answers files or, with `?folders=true`, folders; the field that is not the answer is
//! left out, and an answer from a Fleet that did not have `folders` still reads.

use crate::{decode, encode, FilesFound};

#[test]
fn files_found_with_and_without_folders_round_trips() {
    let files = FilesFound { paths: vec!["src/lib.rs".into()], folders: Vec::new() };
    let json = encode(&files).unwrap();
    assert_eq!(json, r#"{"paths":["src/lib.rs"]}"#, "folders is left out where the answer is files");
    assert_eq!(decode::<FilesFound>("files", json.as_bytes()).unwrap(), files);

    let folders = FilesFound { paths: Vec::new(), folders: vec!["crates/fleet".into()] };
    let json = encode(&folders).unwrap();
    assert_eq!(json, r#"{"paths":[],"folders":["crates/fleet"]}"#);
    assert_eq!(decode::<FilesFound>("files", json.as_bytes()).unwrap(), folders);
}

#[test]
fn an_older_answer_without_folders_reads() {
    let older = br#"{"paths":["README.md"]}"#;
    let found = decode::<FilesFound>("files", older).unwrap();
    assert_eq!(found, FilesFound { paths: vec!["README.md".into()], ..FilesFound::default() });
}
