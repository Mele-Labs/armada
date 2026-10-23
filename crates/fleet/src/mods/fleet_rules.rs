//! What a Fleet mod's `fleet.toml` may say: which notices Fleet tells, quiets, batches or raises as
//! a call, and for which Job, repository or everywhere. Checked before Fleet acts on it.

use ipc::{Does, FleetFile, Notice, RuleScope};

/// The most `fleet.toml` may weigh.
pub(crate) const MOST_BYTES: u64 = 8 * 1024;

const MOST_RULES: usize = 64;

/// `text` read, or the problem with where it stopped.
pub(crate) fn read(text: &str) -> Result<FleetFile, String> {
    toml::from_str(text).map_err(|why| crate::mods::toml_problem("fleet.toml", text, &why))
}

/// Whether `word` is one a Fleet notice goes by. **Read off `Notice`'s own spellings**, so a notice
/// added there is accepted here with nothing else to change.
pub(crate) fn is_notice_word(word: &str) -> bool {
    let plain = !word.is_empty() && word.len() <= 40 && word.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    plain && ipc::decode::<Notice>("a notice", format!("\"{word}\"").as_bytes()).is_ok()
}

/// Every problem in `text`, empty where Fleet may act on it.
pub(crate) fn problems(text: &str) -> Vec<String> {
    let file = match read(text) {
        Ok(file) => file,
        Err(why) => return vec![why],
    };
    let mut found = Vec::new();
    if file.rule.len() > MOST_RULES {
        found.push(format!("fleet.toml: a mod holds at most {MOST_RULES} rules"));
    }
    let mut seen: Vec<(&str, &RuleScope)> = Vec::new();
    for (index, rule) in file.rule.iter().enumerate() {
        let number = index + 1;
        if !is_notice_word(&rule.notice) {
            found.push(format!("fleet.toml rule {number}: `{}` is not a notice Fleet sends", rule.notice));
        }
        if seen.contains(&(rule.notice.as_str(), &rule.scope)) {
            found.push(format!("fleet.toml rule {number}: `{}` is ruled twice for the same place", rule.notice));
        }
        seen.push((&rule.notice, &rule.scope));
    }
    found
}

/// How near a rule's place is to the Session: a Job's own beats its repository's, which beats
/// everywhere's.
fn nearness(scope: &RuleScope, session: &str, repository: &str) -> Option<u8> {
    match scope {
        RuleScope::Job(id) if id == session => Some(2),
        RuleScope::Repo(id) if id == repository => Some(1),
        RuleScope::Everywhere => Some(0),
        _ => None,
    }
}

/// What the rules in `files` say to do with `notice` for `session` in `repository`. **The nearest
/// place wins, and among rules of one place the later file does**, as layout mods stack. `None` is
/// no rule: the notice goes as it always has.
pub(crate) fn does_for(files: &[FleetFile], notice: &str, session: &str, repository: &str) -> Option<Does> {
    files
        .iter()
        .flat_map(|file| file.rule.iter())
        .filter(|rule| rule.notice == notice)
        .filter_map(|rule| nearness(&rule.scope, session, repository).map(|near| (near, rule.does)))
        .fold(None, |best: Option<(u8, Does)>, next| match best {
            Some(held) if held.0 > next.0 => Some(held),
            _ => Some(next),
        })
        .map(|(_, does)| does)
}
