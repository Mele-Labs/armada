//! The ticket a plan's group works, where the agent left it blank. **Fleet reads it off the group's
//! own words**: a group whose title or tasks name exactly one ticket the Job holds works that one, so
//! a ticket can follow its group to the Job started for it without resting on the agent passing `issue`.
//!
//! **Never over what the agent said.** A group that names an `issue` keeps it, and a group that names
//! none of the Job's tickets, or more than one, is left blank: Fleet cannot tell which is meant.

use ipc::PlanGroupInput;

/// Give each group with no `issue` the one ticket of `held` its title or tasks name, if they name one.
/// `held` is the targets of the Job's `issue` rows.
pub(crate) fn name_the_tickets(groups: &mut [PlanGroupInput], held: &[String]) {
    for group in groups.iter_mut().filter(|group| group.issue.is_none()) {
        let words = std::iter::once(group.title.as_str()).chain(group.tasks.iter().map(|task| task.title.as_str()));
        let mut named: Vec<&String> = Vec::new();
        for text in words {
            for ticket in held.iter().filter(|ticket| names(text, ticket)) {
                if !named.contains(&ticket) {
                    named.push(ticket);
                }
            }
        }
        if let [only] = named.as_slice() {
            group.issue = Some((*only).clone());
        }
    }
}

/// Whether `text` writes `ticket`: `#42` for the forge's number, else the key as it is, in either case.
/// **Whole**: `armada-cwj` is not in `armada-cwj2` or `armada-cwj-1`, nor `#4` in `#42`, though a
/// full stop closing a sentence ends one.
fn names(text: &str, ticket: &str) -> bool {
    let needle = match ticket.bytes().all(|byte| byte.is_ascii_digit()) {
        true => format!("#{ticket}"),
        false => ticket.to_ascii_lowercase(),
    };
    if needle.len() <= 1 {
        return false;
    }
    let text = text.to_ascii_lowercase();
    let part_of_a_key = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    let mut from = 0;
    while let Some(at) = text[from..].find(&needle) {
        let (start, end) = (from + at, from + at + needle.len());
        let before = text[..start].chars().next_back();
        let mut after = text[end..].chars();
        let next = after.next();
        let closes = next.is_none_or(|c| !part_of_a_key(c) && (c != '.' || after.next().is_none_or(|d| !d.is_alphanumeric())));
        if closes && before.is_none_or(|c| !part_of_a_key(c)) {
            return true;
        }
        from = end;
    }
    false
}
