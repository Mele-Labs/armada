//! What a Manifest's `tracker:` block means for Jira: the keys it reads, the
//! addresses they make, and the `jq` filters that reduce `acli`'s JSON to lines.
//! `docs/concepts/tracker.md`, *Jira*.
//!
//! **No parse**, `crate::issue_tracker`'s way: `acli` has no `--jq`, so its JSON
//! is piped through `jq` and what comes back is `@tsv`. Every filter reads both
//! `.fields.x` and a flat `.x`, because the output shape is unverified and the
//! command that checks it is in the docs.

use std::collections::BTreeMap;

pub(crate) const PROVIDER: &str = "jira";

const KEYS: &[&str] = &["site", "project", "jql"];

/// The keys of one `tracker:` block, read and checked. **The backlog is whatever the search
/// selects**: the projects named, the `jql`, or both together. An issue is named by its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JiraSettings {
    host: String,
    projects: Vec<String>,
    jql: Option<String>,
}

impl JiraSettings {
    /// The block, or the sentence that says what is wrong with it.
    pub(crate) fn read(block: &BTreeMap<String, String>) -> Result<JiraSettings, String> {
        if let Some(unknown) = block.keys().find(|key| !KEYS.contains(&key.as_str())) {
            return Err(format!(
                "tracker key `{unknown}` is not one the jira tracker reads: {}",
                KEYS.join(", ")
            ));
        }
        let site = block.get("site").ok_or("the jira tracker needs `site`")?;
        let projects: Vec<String> = block
            .get("project")
            .map(|named| {
                named
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|one| !one.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(bad) = projects.iter().find(|one| !is_project(one)) {
            return Err(format!(
                "`project: {bad}` is not a Jira project key: capitals, digits and underscores"
            ));
        }
        let host = host_of(site)
            .ok_or_else(|| format!("`site: {site}` is not a site name or a host"))?;
        let jql = block.get("jql").cloned().filter(|jql| !jql.trim().is_empty());
        if let Some(jql) = &jql {
            if jql.to_ascii_lowercase().contains("order by") {
                return Err("`jql` is an extra filter and takes no ORDER BY".to_string());
            }
        }
        if projects.is_empty() && jql.is_none() {
            return Err("the jira tracker needs `project` or `jql` to say which issues it lists".to_string());
        }
        Ok(JiraSettings { host, projects, jql })
    }

    /// Open issues the block selects, newest edit first.
    pub(crate) fn search(&self) -> String {
        let mut clauses: Vec<String> = Vec::new();
        match self.projects.as_slice() {
            [] => {}
            [one] => clauses.push(format!("project = \"{one}\"")),
            many => {
                let keys: Vec<String> = many.iter().map(|one| format!("\"{one}\"")).collect();
                clauses.push(format!("project in ({})", keys.join(", ")));
            }
        }
        clauses.push("statusCategory != Done".to_string());
        if let Some(jql) = &self.jql {
            clauses.push(format!("({jql})"));
        }
        format!("{} ORDER BY updated DESC", clauses.join(" AND "))
    }

    pub(crate) fn browse(&self, key: &str) -> String {
        format!("https://{}/browse/{key}", self.host)
    }

    /// The issue key a text links on this site, with or without the scheme.
    /// **Any project's**: the link names its own.
    pub(crate) fn key_in(&self, text: &str) -> Option<String> {
        let needle = format!("{}/browse/", self.host);
        let at = text.to_ascii_lowercase().find(&needle)? + needle.len();
        let key = text[at..].split(['/', '?', '#', ' ', '\n']).next()?;
        is_key(key).then(|| key.to_string())
    }
}

/// `acme` is `acme.atlassian.net`; anything with a dot is a host as written.
fn host_of(site: &str) -> Option<String> {
    let site = site.to_ascii_lowercase();
    let plain = !site.is_empty()
        && !site.starts_with(['-', '.'])
        && site.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    match (plain, site.contains('.')) {
        (false, _) => None,
        (true, true) => Some(site),
        (true, false) => Some(format!("{site}.atlassian.net")),
    }
}

fn is_project(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_uppercase())
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// `ABC-123`. **Checked before a key reaches a command line**, so nothing that
/// is not a key is ever an argument.
pub(crate) fn is_key(text: &str) -> bool {
    match text.rsplit_once('-') {
        Some((project, number)) => {
            is_project(project) && !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Jira labels hold no whitespace, so Armada's `armada: in progress` is
/// `armada-in-progress`: each run of spaces and colons becomes one hyphen.
pub(crate) fn as_label(name: &str) -> String {
    name.split(|c: char| c.is_whitespace() || c == ':')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Shared definitions of every filter: a field read from either shape, an
/// Atlassian Document Format body flattened to text, a person by name.
const PRELUDE: &str = r#"
def f($n): (.fields[$n]? // .[$n]?);
def adf: if type == "string" then .
  elif type == "object" then
    (if .type == "text" then (.text // "")
     elif .type == "hardBreak" then "\n"
     else (((.content // []) | map(adf) | join(""))
           + (if ((.type // "") | IN("paragraph","heading","listItem","codeBlock","blockquote","tableRow")) then "\n" else "" end))
     end)
  elif type == "array" then (map(adf) | join(""))
  else "" end;
def who: if type == "object" then (.displayName // .emailAddress // .name // .accountId // "")
  elif type == "string" then . else "" end;
def names: [.[]? | if type == "object" then (.name // empty) else . end];
def rfc: sub("(?<h>[+-][0-9]{2})(?<m>[0-9]{2})$"; "\(.h):\(.m)");
def issues: if type == "array" then .[] else ((.issues // .workItems // .values // [.])[]) end;
"#;

/// A search result, one issue a line: key, summary, labels (US apart),
/// assignee, last edit, and the start of the description.
pub(crate) fn listing_filter(excerpt: usize) -> String {
    format!(
        r#"{PRELUDE}
issues | [(.key // ""), (f("summary") // "" | tostring),
  (f("labels") | names | join("\u001f")), (f("assignee") | who),
  (f("updated") // "" | tostring | rfc),
  (f("description") | adf | gsub("\\s+"; " ") | gsub("^ | $"; "") | .[0:{excerpt}])] | @tsv"#
    )
}

/// One issue: key, summary, status category (or name), and whether it is resolved.
pub(crate) fn reading_filter() -> String {
    format!(
        r#"{PRELUDE}
issues | [(.key // ""), (f("summary") // "" | tostring),
  (f("status") | if type == "object" then (.statusCategory.key // .name // "") elif type == "string" then . else "" end | ascii_downcase),
  (if f("resolution") == null then "" else "resolved" end)] | @tsv"#
    )
}

/// An issue's labels, one a line.
pub(crate) fn labels_filter() -> String {
    format!("{PRELUDE}\nissues | (f(\"labels\") | names | .[])")
}

/// The text a Drone is briefed from: summary, a blank line, the description.
pub(crate) fn text_filter() -> String {
    format!("{PRELUDE}\nissues | ((f(\"summary\") // \"\") + \"\\n\\n\" + (f(\"description\") | adf))")
}

/// When an issue was last edited.
pub(crate) fn edited_filter() -> String {
    format!("{PRELUDE}\nissues | (f(\"updated\") // \"\" | tostring | rfc)")
}
