//! A part's question shares its key with a takes part's own `question`, so **it is written once and reads back on
//! both**: a takes part written twice over would not read back, and drop out of his Artifacts and his list.

use crate::{decode, encode, ArtifactPart, Instant, PartContent, PartState, VoteForm};

fn part(content: PartContent, question: Option<&str>) -> ArtifactPart {
    ArtifactPart {
        id: "layout".into(),
        content,
        asks: true,
        state: PartState::Open,
        revision: 1,
        drawn: "d-1".into(),
        added: Instant::carried("2026-11-20T09:00:00Z"),
        answer: None,
        question: question.map(str::to_string),
    }
}

#[test]
fn a_takes_parts_question_is_written_once_and_reads_back_on_the_part_and_the_takes() {
    let asked = Some("Which layout reads better?");
    let takes = part(PartContent::Takes { question: asked.map(str::to_string), form: VoteForm::Pick, takes: Vec::new(), parts: Vec::new() }, asked);
    let written = encode(&takes).expect("encodes");
    assert_eq!(written.matches(r#""question""#).count(), 1, "one key: {written}");
    assert_eq!(decode::<ArtifactPart>("a part", written.as_bytes()).expect("reads back"), takes);

    let words = part(PartContent::Words { text: "Evict by age.".into() }, Some("Is eviction by age right?"));
    let written = encode(&words).expect("encodes");
    assert_eq!(decode::<ArtifactPart>("a part", written.as_bytes()).expect("reads back"), words);
}

#[test]
fn takes_kept_before_a_part_carried_its_question_read_it_from_the_takes() {
    let kept = r#"{"id":"layout","kind":"takes","question":"Which layout?","form":"pick","takes":[],"asks":true,"state":"open","revision":1,"drawn":"d-1","added":"2026-11-20T09:00:00Z"}"#;
    let read = decode::<ArtifactPart>("a part", kept.as_bytes()).expect("reads");
    assert_eq!(read.question.as_deref(), Some("Which layout?"));
    assert!(matches!(&read.content, PartContent::Takes { question: Some(own), .. } if own == "Which layout?"));
}
