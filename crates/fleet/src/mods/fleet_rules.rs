//! What a Fleet mod's `fleet.toml` may say: which notices Fleet tells, quiets, batches or raises as
//! a call, and for which Job, repository or everywhere. Checked before Fleet acts on it.

use ipc::{FleetFile, FleetRule, Notice, RuleScope};

use crate::teaching::link;

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

/// Why `rule`'s `matches` and `when` cannot work, where they cannot. `taught_on` is the text of the
/// notice the rule was taught on, which a pattern must find words in.
pub(crate) fn rule_fault(rule: &FleetRule, taught_on: Option<&str>) -> Option<String> {
    if let Some(pattern) = &rule.matches {
        if let Some(why) = link::match_fault(pattern, taught_on) {
            return Some(why);
        }
    }
    let mut tested: Vec<ipc::Condition> = Vec::new();
    for instead in &rule.when {
        if tested.contains(&instead.condition) {
            return Some("a condition is tested twice, and the second could never be reached".to_string());
        }
        tested.push(instead.condition);
    }
    None
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
    let mut seen: Vec<(&str, &RuleScope, Option<&str>)> = Vec::new();
    for (index, rule) in file.rule.iter().enumerate() {
        let number = index + 1;
        if !is_notice_word(&rule.notice) {
            found.push(format!("fleet.toml rule {number}: `{}` is not a notice Fleet sends", rule.notice));
        }
        if seen.contains(&(rule.notice.as_str(), &rule.scope, rule.matches.as_deref())) {
            found.push(format!("fleet.toml rule {number}: `{}` is ruled twice for the same place", rule.notice));
        }
        seen.push((&rule.notice, &rule.scope, rule.matches.as_deref()));
        if let Some(why) = rule_fault(rule, None) {
            found.push(format!("fleet.toml rule {number}: {why}"));
        }
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

/// The rule in `files` that covers `notice`, whose words are `text`, for `session` in `repository`.
/// **A rule covers a notice only if its `matches` finds words in the text. The nearest place of the
/// rules that cover it wins, and among rules of one place the later file does**, as layout mods
/// stack. `None` is no rule: the notice goes as it always has.
pub(crate) fn rule_for<'a>(files: &'a [FleetFile], notice: &str, text: &str, session: &str, repository: &str) -> Option<&'a FleetRule> {
    files
        .iter()
        .flat_map(|file| file.rule.iter())
        .filter(|rule| rule.notice == notice)
        .filter(|rule| rule.matches.as_deref().is_none_or(|pattern| link::finds(pattern, text)))
        .filter_map(|rule| nearness(&rule.scope, session, repository).map(|near| (near, rule)))
        .fold(None, |best: Option<(u8, &FleetRule)>, next| match best {
            Some(held) if held.0 > next.0 => Some(held),
            _ => Some(next),
        })
        .map(|(_, rule)| rule)
}
