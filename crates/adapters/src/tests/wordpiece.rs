//! The tokenizer the language model reads through, on a vocabulary of a few lines.

use crate::language_model::wordpiece::WordPiece;

const VOCAB: &str = "[PAD]\n[UNK]\n[CLS]\n[SEP]\nwant\nme\nto\nland\nit\n?\nun\n##land\n##ed\n,\n";

#[test]
fn words_are_lower_cased_and_punctuation_is_a_piece_of_its_own() {
    let pieces = WordPiece::from_vocab(VOCAB).expect("the marks are there");
    assert_eq!(pieces.pieces("Want me to LAND it?"), vec![4, 5, 6, 7, 8, 9]);
    assert_eq!((pieces.start, pieces.end), (2, 3));
}

#[test]
fn a_word_is_split_longest_first_and_unknown_whole_when_any_part_is() {
    let pieces = WordPiece::from_vocab(VOCAB).expect("the marks are there");
    assert_eq!(pieces.pieces("unlanded, unlandx"), vec![10, 11, 12, 13, 1]);
}

#[test]
fn a_vocabulary_without_its_marks_is_refused() {
    assert!(WordPiece::from_vocab("want\nme\n").is_none());
}
