//! A piece as the YAML text its file holds. The parsers read a piece's keys by hand, so the
//! document a writer builds is shaped like the file and not like any wire type: the caller owns
//! that shape, and this is the one place YAML is produced for it.
//!
//! **What this makes is only text, and text is not yet a piece.** The caller reads it back with
//! the piece's own parser before keeping it, which is what holds the writer to the format.

/// `document` as the text of a YAML file.
///
/// **A struct whose fields are the file's keys, and an untagged enum for a key that varies.**
/// An externally tagged enum is written as a YAML tag (`!step_starts plan`), which no parser here
/// reads.
pub fn piece_text<T: serde::Serialize>(document: &T) -> Result<String, String> {
    serde_yaml_ng::to_string(document).map_err(|why| why.to_string())
}
