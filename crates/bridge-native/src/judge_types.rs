//! Judge types as Bridge asks Fleet for them: the ones a repository and project see, and saving and deleting
//! Kit's. The Judge's box, the Judge tab and Settings › Judges all read the same list, in Fleet's order.

use std::collections::HashMap;

use gpui::{App, AppContext as _, Task};
use ipc::{DeleteJudgeType, JudgeReading, JudgeType, SaveJudgeType};

use crate::link::percent_encoded;
use crate::settings::wire::{body, request, Ask, Refused};

/// The type an ask reads as when it names none, and the one the box has chosen when it opens.
pub const DEFAULT: &str = "adversarial";

/// What `GET /judge_types/:repository` answers, less what Bridge does not use.
pub type Listed = Vec<JudgeType>;

fn route(repository: &str) -> String {
    format!("/judge_types/{}", percent_encoded(repository))
}

/// `GET /judge_types/:repository?project=`: the types the project sees in that repository.
pub fn list(port: u16, repository: &str, project: Option<&str>, cx: &App) -> Task<Result<Listed, Refused>> {
    let query = project.map(|project| format!("?project={}", percent_encoded(project))).unwrap_or_default();
    let read = request::<ipc::JudgeTypes>(port, Ask::Get(format!("{}{query}", route(repository))), "JudgeTypes", cx);
    cx.background_spawn(async move { read.await.map(|read| read.types) })
}

/// `POST /judge_types/:repository`: Kit's or the repository's copy of a type, by its name.
pub fn save(port: u16, repository: &str, save: SaveJudgeType, cx: &App) -> Task<Result<(), Refused>> {
    let read = request::<serde::de::IgnoredAny>(port, Ask::Post(route(repository), body(&save)), "SaveJudgeType", cx);
    cx.background_spawn(async move { read.await.map(|_| ()) })
}

/// `POST /judge_types/:repository/delete`: Kit's copy of a type, which brings Armada's back where it replaced one.
pub fn delete(port: u16, repository: &str, one: DeleteJudgeType, cx: &App) -> Task<Result<(), Refused>> {
    let read = request::<serde::de::IgnoredAny>(port, Ask::Post(format!("{}/delete", route(repository)), body(&one)), "DeleteJudgeType", cx);
    cx.background_spawn(async move { read.await.map(|_| ()) })
}

/// The key a list is held under: the repository and the project it was read for.
pub fn key(repository: &str, project: Option<&str>) -> String {
    format!("{repository}\u{1f}{}", project.unwrap_or_default())
}

/// A type's id said as a name, for a reading kept before names were kept with it.
fn titled(id: &str) -> String {
    let said = id.replace('-', " ");
    let mut chars = said.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The name of the type a reading was read as: the one it had then. A reading kept before judge types read as Adversarial.
pub fn name_of(reading: &JudgeReading) -> String {
    reading.judge_name.clone().or_else(|| reading.judge.as_deref().map(titled)).unwrap_or_else(|| titled(DEFAULT))
}

/// What a box holds of the lists it has read: each by its key, and the read going for it.
#[derive(Default)]
pub struct Held {
    pub read: HashMap<String, Result<Listed, String>>,
    pub reading: HashMap<String, Task<()>>,
}

/// The first sentence of a type's prompt, which a list row says of it.
pub fn first_sentence(prompt: &str) -> String {
    let text = prompt.trim();
    let end = text.char_indices().find(|(at, ch)| matches!(ch, '.' | '\n') && (*ch == '\n' || text[at + 1..].chars().next().is_none_or(char::is_whitespace))).map_or(text.len(), |(at, _)| at);
    text[..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_names_the_type_it_read_as() {
        let wire = |extra: &str| format!(r#"{{"id":"r","artifact":{{"kind":"job","id":"j","title":"t"}},"prompt":"","state":"done","asked_at":"2026-11-09T13:55:00Z","read":false,"notes":[]{extra}}}"#);
        let read = |extra: &str| ipc::decode::<JudgeReading>("reading", wire(extra).as_bytes()).expect("a reading");
        assert_eq!(name_of(&read("")), "Adversarial");
        assert_eq!(name_of(&read(r#","judge":"system-design""#)), "System design");
        assert_eq!(name_of(&read(r#","judge":"code","judge_name":"Code (mine)""#)), "Code (mine)");
    }

    #[test]
    fn a_description_is_the_first_sentence() {
        assert_eq!(first_sentence("Read the diff for bugs. Then the tests."), "Read the diff for bugs");
        assert_eq!(first_sentence("One line only"), "One line only");
        assert_eq!(first_sentence("First line\nsecond"), "First line");
    }
}
