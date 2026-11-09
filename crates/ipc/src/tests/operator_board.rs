//! The Operator's board on the wire: the suggestions no Job holds ride beside the arrivals, and a board
//! written before they did still reads.

use crate::operator_board::{AnswerOperatorSuggestion, OperatorBoard, OperatorBoardOption, OperatorBoardSuggestion, OperatorBoardSwitch, OperatorPaused};

#[test]
fn a_suggestion_is_an_id_its_text_two_options_and_when_it_was_raised() {
    let board = OperatorBoard {
        on: true,
        suggestions: vec![OperatorBoardSuggestion {
            id: "suggestion:7".into(),
            text: "The Operator would file your note".into(),
            options: vec![OperatorBoardOption { label: "Do it".into() }, OperatorBoardOption { label: "Skip".into() }],
            at: "2026-11-05T09:00:00.000Z".into(),
        }],
        ..OperatorBoard::default()
    };
    let wire = serde_json::to_value(&board).expect("written");
    assert_eq!(
        wire["suggestions"],
        serde_json::json!([{ "id": "suggestion:7", "text": "The Operator would file your note", "options": [{ "label": "Do it" }, { "label": "Skip" }], "at": "2026-11-05T09:00:00.000Z" }])
    );
    assert_eq!(serde_json::from_value::<OperatorBoard>(wire).expect("read"), board);
}

#[test]
fn a_board_without_suggestions_reads_with_none_and_writes_an_empty_list() {
    let board: OperatorBoard = serde_json::from_str(r#"{"on":false}"#).expect("read");
    assert!(board.suggestions.is_empty());
    assert_eq!(serde_json::to_value(&board).expect("written")["suggestions"], serde_json::json!([]));
}

#[test]
fn a_press_is_an_id_and_a_choice_and_nothing_else() {
    let press: AnswerOperatorSuggestion = serde_json::from_str(r#"{"id":"suggestion:7","choice":1}"#).expect("read");
    assert_eq!(press, AnswerOperatorSuggestion { id: "suggestion:7".into(), choice: 1 });
}

#[test]
fn a_board_names_a_pause_only_while_it_holds() {
    let calm = OperatorBoard { on: true, ..OperatorBoard::default() };
    assert!(serde_json::to_value(&calm).expect("written").get("paused").is_none());

    let paused = OperatorBoard { on: true, paused: Some(OperatorPaused { week: 83, at: 80 }), ..OperatorBoard::default() };
    let wire = serde_json::to_value(&paused).expect("written");
    assert_eq!(wire["paused"], serde_json::json!({ "week": 83, "at": 80 }));
    assert_eq!(serde_json::from_value::<OperatorBoard>(wire).expect("read"), paused);
    assert_eq!(serde_json::from_str::<OperatorBoard>(r#"{"on":true}"#).expect("read").paused, None, "a board written before it reads");
}

#[test]
fn a_board_names_the_moves_of_the_switch_only_while_there_are_some() {
    let calm = OperatorBoard { on: true, ..OperatorBoard::default() };
    assert!(serde_json::to_value(&calm).expect("written").get("switched").is_none());

    let moved = OperatorBoard { on: false, switched: vec![OperatorBoardSwitch { on: false, at: "2026-11-09T10:00:00.000Z".into() }], ..OperatorBoard::default() };
    let wire = serde_json::to_value(&moved).expect("written");
    assert_eq!(wire["switched"], serde_json::json!([{ "on": false, "at": "2026-11-09T10:00:00.000Z" }]));
    assert_eq!(serde_json::from_value::<OperatorBoard>(wire).expect("read"), moved);
    assert!(serde_json::from_str::<OperatorBoard>(r#"{"on":true}"#).expect("read").switched.is_empty(), "a board written before it reads");
}
