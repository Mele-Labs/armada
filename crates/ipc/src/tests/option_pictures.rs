//! A question's option may be a picture, and an answer may pick parts across the pictures. Both are absent by
//! default, so an option of words and an answer of words are the bytes they always were.

use serde_json::json;

use crate::{AnswerWaiting, AnswerWaitingParts, WaitingInput, WaitingOption};

#[test]
fn an_option_of_words_is_the_bytes_it_always_was() {
    let option = json!({"label": "Left", "description": "The quiet one"});
    let read: WaitingOption = serde_json::from_value(option.clone()).unwrap();
    assert_eq!(read.picture, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), option);
}

#[test]
fn an_option_carries_a_picture_with_its_parts_boxed_in_the_stills_pixels() {
    let input = json!({
        "id": "pick-header",
        "text": "Which header?",
        "options": [
            {"label": "Wide", "picture": {
                "image": "/files/wide.png", "width": 1200, "height": 800,
                "parts": [
                    {"name": "nav", "label": "Navigation", "x": 0, "y": 0, "w": 1200, "h": 90},
                    {"name": "hero", "label": "Hero", "x": 0, "y": 90, "w": 1200, "h": 400}
                ]
            }},
            {"label": "Words only"}
        ]
    });
    let read: WaitingInput = serde_json::from_value(input.clone()).unwrap();
    let picture = read.options[0].picture.as_ref().expect("the first option is a picture");
    assert_eq!((picture.width, picture.height, picture.parts.len()), (1200, 800, 2));
    assert_eq!(picture.parts[1].name, "hero");
    assert_eq!(read.options[1].picture, None);
    assert_eq!(serde_json::to_value(&read).unwrap(), input);
}

#[test]
fn a_picture_pressed_whole_has_no_parts_on_the_wire() {
    let option = json!({"label": "Tall", "picture": {"image": "https://example.test/tall.png", "width": 600, "height": 900}});
    let read: WaitingOption = serde_json::from_value(option.clone()).unwrap();
    assert!(read.picture.as_ref().expect("a picture").parts.is_empty());
    assert_eq!(serde_json::to_value(&read).unwrap(), option);
}

#[test]
fn an_answer_picks_parts_by_option_index_and_part_name_in_its_own_body() {
    let answer = json!({
        "session_id": "s1", "item_id": "pick-header",
        "parts_picked": [{"option": 0, "part": "nav"}, {"option": 2, "part": "hero"}]
    });
    let read: AnswerWaitingParts = serde_json::from_value(answer.clone()).unwrap();
    assert_eq!((read.parts_picked[0].option, read.parts_picked[1].part.as_str()), (0, "hero"));
    assert_eq!(serde_json::to_value(&read).unwrap(), answer);
}

#[test]
fn an_answer_of_a_whole_option_or_words_carries_no_parts() {
    let plain = json!({"session_id": "s1", "item_id": "pick-header", "choice": 1});
    let read: AnswerWaiting = serde_json::from_value(plain.clone()).unwrap();
    assert_eq!(serde_json::to_value(&read).unwrap(), plain);
}
