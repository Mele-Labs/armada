//! What an agent is told about a file attached to a message: its path, and for a big one its size and
//! how to read it, so a pasted response is opened in parts and never all at once. A picture, a small
//! file and a file that cannot be read are told as they always were.

use crate::session_host::uploads::attached_line;
use crate::tests::tmp::TempDir;

/// `bytes` long, written under `name` in a directory of its own.
fn kept(name: &str, bytes: usize) -> (TempDir, String) {
    let dir = TempDir::new();
    let path = dir.path().join(name);
    std::fs::write(&path, vec![b'x'; bytes]).expect("written");
    let path = path.to_string_lossy().into_owned();
    (dir, path)
}

#[test]
fn a_small_file_a_missing_one_and_a_picture_are_told_by_path_alone() {
    let (_small, small) = kept("note.txt", 1_000);
    assert_eq!(attached_line(&small), format!("Attached file: {small}"));

    let gone = "/nowhere/at/all.txt";
    assert_eq!(attached_line(gone), format!("Attached file: {gone}"));

    // A picture is as large as it is and is not read as text, so its size says nothing of its cost.
    let (_picture, picture) = kept("shot.png", 400_000);
    assert_eq!(attached_line(&picture), format!("Attached file: {picture}"));
}

#[test]
fn a_file_at_the_limit_is_told_by_path_alone_and_one_byte_over_is_not() {
    let (_at, at) = kept("at.txt", 32_000);
    assert_eq!(attached_line(&at), format!("Attached file: {at}"));

    let (_over, over) = kept("over.txt", 32_001);
    assert!(attached_line(&over).len() > format!("Attached file: {over}").len());
}

#[test]
fn a_big_file_is_told_with_its_size_and_to_be_read_in_parts() {
    let (_dir, path) = kept("response.txt", 40_000);
    let line = attached_line(&path);
    assert!(line.starts_with(&format!("Attached file: {path}")), "{line}");
    assert!(line.contains("39 KB"), "{line}");
    assert!(line.contains("about 10k tokens"), "{line}");
    assert!(line.contains("in parts"), "{line}");
    assert!(line.contains("grep or jq"), "{line}");
    assert_eq!(line.lines().count(), 1, "one line, so a path read off it is the first word after the colon");
}

#[test]
fn a_file_of_a_megabyte_or_more_is_told_in_megabytes() {
    let (_dir, path) = kept("huge.json", 3 * 1024 * 1024);
    let line = attached_line(&path);
    assert!(line.contains("3.0 MB"), "{line}");
    assert!(line.contains("about 786k tokens"), "{line}");
}
