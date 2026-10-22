//! The fine-tuned adapter committed beside the pinned model. What it reads is measured outside the
//! repository, where the base weights are; here, that it parses and carries what Fleet asks of it.

use adapter_traits::TextReading;
use sha2::{Digest, Sha256};

use crate::language_model::encoding::Tuned;
use crate::language_model::{shape, TUNED};

#[test]
fn the_committed_adapter_has_a_head_for_every_reading() {
    let tuned = Tuned::from_bytes(TUNED).expect("the committed adapter parses");
    assert!(!tuned.adapters.is_empty());
    for reading in [TextReading::ReplyAsks, TextReading::TellRefused, TextReading::SentenceAsks, TextReading::SentenceFollowsUp, TextReading::ItemClear] {
        assert!(tuned.heads.contains_key(shape(reading).0), "{reading:?}");
    }
}

/// Only the adapter is committed, never the model: it stays well under a megabyte.
#[test]
fn the_committed_adapter_is_small() {
    assert!(TUNED.len() < 1_000_000, "{} bytes", TUNED.len());
}

/// SHA-256 over every tensor of the adapter but the heads named in `leave`, as `name`, a zero byte and the
/// little-endian bytes of its values, in name order.
fn digest_without(tuned: &Tuned, leave: &[&str]) -> String {
    let mut named: Vec<(String, &candle_core::Tensor)> = Vec::new();
    for (at, layer) in tuned.adapters.iter().enumerate() {
        for (part, tensor) in [("query_a", &layer.query_a), ("query_b", &layer.query_b), ("value_a", &layer.value_a), ("value_b", &layer.value_b)] {
            named.push((format!("layer.{at}.{part}"), tensor));
        }
    }
    for (head, (weight, bias)) in &tuned.heads {
        if !leave.contains(&head.as_str()) {
            named.push((format!("head.{head}.weight"), weight));
            named.push((format!("head.{head}.bias"), bias));
        }
    }
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = Sha256::new();
    for (name, tensor) in named {
        hash.update(name.as_bytes());
        hash.update([0u8]);
        for value in tensor.flatten_all().and_then(|flat| flat.to_vec1::<f32>()).expect("an f32 tensor") {
            hash.update(value.to_le_bytes());
        }
    }
    hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A head added on the frozen adapter leaves what was there as it was: the same encoder layers and the
/// same four heads, to the bit, so `asks`, `refused`, `sentence_asks` and `sentence_follows_up` read what
/// they read before `item_clear` was trained. The digest is of the file as it stood with those four.
#[test]
fn the_earlier_heads_and_the_adapter_are_the_bits_they_were() {
    let tuned = Tuned::from_bytes(TUNED).expect("the committed adapter parses");
    assert_eq!(digest_without(&tuned, &["item_clear"]), "701d806879f35063f38b50886dc66764533b19b0c5275f40f03c7415815b4e30");
}
