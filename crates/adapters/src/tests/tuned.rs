//! The fine-tuned adapter committed beside the pinned model. What it reads is measured outside the
//! repository, where the base weights are; here, that it parses and carries what Fleet asks of it.

use adapter_traits::TextReading;

use crate::language_model::encoding::Tuned;
use crate::language_model::{shape, TUNED};

#[test]
fn the_committed_adapter_has_a_head_for_every_reading() {
    let tuned = Tuned::from_bytes(TUNED).expect("the committed adapter parses");
    assert!(!tuned.adapters.is_empty());
    for reading in [TextReading::ReplyAsks, TextReading::TellRefused] {
        assert!(tuned.heads.contains_key(shape(reading).0), "{reading:?}");
    }
}

/// Only the adapter is committed, never the model: it stays well under a megabyte.
#[test]
fn the_committed_adapter_is_small() {
    assert!(TUNED.len() < 1_000_000, "{} bytes", TUNED.len());
}
