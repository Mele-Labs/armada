//! What a Manifest's `tracker:` block means for Jira: the keys it reads, the
//! addresses they make, and the `jq` filters that reduce `acli`'s JSON to lines.
//! `docs/concepts/tracker.md`, *Jira*.
//!
//! **No parse**, `crate::issue_tracker`'s way: `acli` has no `--jq`, so its JSON
//! is piped through `jq` and what comes back is `@tsv`. Every filter reads both
//! `.fields.x` and a flat `.x`, because the output shape is unverified and the
//! command that checks it is in the docs.

use std::collections::BTreeMap;

use adapter_traits::QueryGuide;

pub(crate) const PROVIDER: &str = "jira";

const KEYS: &[&str] = &["site", "project", "jql"];

/// The fields every search reads, by the name a row shows and the id Jira keys them by.
const FIELDS: &[(&str, &str)] = &[
    ("Status", "status"),
    ("Priority", "priority"),
    ("Issue type", "issuetype"),
    ("Labels", "labels"),
];

/// What a search asks `acli` for: the listing's, who reported it, and [`FIELDS`]. **Only what
/// `acli`'s search takes**: it refuses `updated`, `resolution`, `components`, `fixVersions`,
/// `sprint` and every custom field, so a search is done by its status alone.
const SEARCH_FIELDS: &str = "key,summary,labels,assignee,description,reporter,status,priority,issuetype";

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
        if block.contains_key("fields") {
            return Err(format!(
                "the jira tracker takes no `fields`: `acli`'s search returns only {SEARCH_FIELDS}, \
                 so no other field can be listed"
            ));
        }
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
        let mut clauses: Vec<String> = self.projects_clause().into_iter().collect();
        clauses.push("statusCategory != Done".to_string());
        clauses.extend(self.jql.iter().map(|jql| format!("({jql})")));
        format!("{} ORDER BY updated DESC", clauses.join(" AND "))
    }

    fn projects_clause(&self) -> Option<String> {
        match self.projects.as_slice() {
            [] => None,
            [one] => Some(format!("project = \"{one}\"")),
            many => {
                let keys: Vec<String> = many.iter().map(|one| format!("\"{one}\"")).collect();
                Some(format!("project in ({})", keys.join(", ")))
            }
        }
    }

    /// What the block selects, open or closed: its projects and its `jql`.
    fn selection(&self) -> Vec<String> {
        let mut clauses: Vec<String> = self.projects_clause().into_iter().collect();
        clauses.extend(self.jql.iter().map(|jql| format!("({jql})")));
        clauses
    }

    /// The issues `query` matches within what the block selects, newest edit first, or why the
    /// query cannot be run.
    pub(crate) fn search_for(&self, query: &str) -> Result<String, String> {
        if query.to_ascii_lowercase().contains("order by") {
            return Err("a search takes no ORDER BY: it is listed newest edit first".to_string());
        }
        let mut clauses = self.selection();
        let query = query.trim();
        if !query.is_empty() {
            clauses.push(format!("({query})"));
        }
        Ok(format!("{} ORDER BY updated DESC", clauses.join(" AND ")))
    }

    /// Every field a search reads, by the name a row shows and the id it is read by.
    pub(crate) fn named_fields(&self) -> Vec<(String, String)> {
        FIELDS.iter().map(|(name, id)| (name.to_string(), id.to_string())).collect()
    }

    /// The `--fields` a search asks `acli` for.
    pub(crate) fn search_fields(&self) -> String {
        SEARCH_FIELDS.to_string()
    }

    /// How a model writes JQL for this block.
    pub(crate) fn guide(&self) -> QueryGuide {
        let guide = format!("{JQL_GUIDE} The query is ANDed with what this repository selects, `{}`.", self.selection().join(" AND "));
        QueryGuide {
            language: "JQL".to_string(),
            guide,
            open: "statusCategory != Done".to_string(),
            assigned_to_me: "assignee = currentUser() AND statusCategory != Done".to_string(),
            fields: self.named_fields().into_iter().map(|(name, _)| name).collect(),
        }
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

    /// Every issue key a text links on this site, each once and in the order written. A link at the
    /// end of a sentence or inside a markdown link keeps no trailing punctuation.
    pub(crate) fn keys_in(&self, text: &str) -> Vec<String> {
        let needle = format!("{}/browse/", self.host);
        let mut keys: Vec<String> = Vec::new();
        for (at, _) in text.to_ascii_lowercase().match_indices(&needle) {
            let rest = &text[at + needle.len()..];
            let key = rest.split(|c: char| c.is_whitespace() || matches!(c, '/' | '?' | '#')).next().unwrap_or_default();
            let key = key.trim_end_matches(['.', ',', ';', ':', '!', ')', ']', '>', '"', '\'', '`']);
            if is_key(key) && !keys.iter().any(|had| had == key) {
                keys.push(key.to_string());
            }
        }
        keys
    }
}

const JQL_GUIDE: &str = "Jira Query Language, as a filter. Write no ORDER BY: results come newest \
edit first, at most 100. A clause is `field operator value`; clauses join with AND and OR, group with \
parentheses, and NOT negates one. Operators: `=`, `!=`, `~` (contains text), `in (a, b)`, `not in (a, b)`, \
`is EMPTY`, `is not EMPTY`, `>`, `>=`, `<`, `<=`. Fields: `status`, `statusCategory` (`\"To Do\"`, \
`\"In Progress\"`, `Done`), `assignee`, `reporter`, `priority`, `issuetype`, `labels`, `component`, \
`fixVersion`, `sprint`, `created`, `updated`, `resolved`, `resolution`, `text` (anywhere in it). \
Functions: `currentUser()` (whoever is signed in), `openSprints()`, `closedSprints()`, `startOfWeek()`, \
`startOfMonth()`, `now()`; a relative date is `updated >= -7d`. Quote a value with spaces: \
`status = \"In Review\"`. Leave out statusCategory to list open and done alike. Examples: \
`assignee = currentUser() AND statusCategory != Done`, `sprint in openSprints() AND priority in (High, Highest)`, \
`labels = web AND updated >= -14d`, `statusCategory = Done AND resolved >= startOfMonth()`. It cannot \
compare two fields, and never names a project: the repository's selection does.";

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
def shown1: if type == "object" then ((.name // .value // .displayName // .key // "") | tostring)
  elif type == "string" then . elif . == null then "" else tostring end;
def shown: (if type == "array" then map(shown1) elif . == null then [] else [shown1] end)
  | map(select(. != "")) | join("\u001f");
def state: if type == "object" then (.statusCategory.key // .name // "") elif type == "string" then . else "" end
  | ascii_downcase;
"#;

/// A search result, one issue a line: key, summary, labels (US apart),
/// assignee, last edit, and the start of the description.
pub(crate) fn listing_filter(excerpt: usize) -> String {
    format!("{PRELUDE}\nissues | [{}] | @tsv", listing_columns(excerpt))
}

fn listing_columns(excerpt: usize) -> String {
    format!(
        r#"(.key // ""), (f("summary") // "" | tostring),
  (f("labels") | names | join("\u001f")), (f("assignee") | who),
  (f("updated") // "" | tostring | rfc),
  (f("description") | adf | gsub("\\s+"; " ") | gsub("^ | $"; "") | .[0:{excerpt}])"#
    )
}

/// A search result, one issue a line: the listing's columns, then reporter, status, resolution,
/// and each of the block's fields with its values `US` apart.
pub(crate) fn search_filter(excerpt: usize, settings: &JiraSettings) -> String {
    let fields: Vec<String> =
        settings.named_fields().iter().map(|(_, id)| format!("(f(\"{id}\") | shown)")).collect();
    format!(
        "{PRELUDE}\nissues | [{}, (f(\"reporter\") | who), (f(\"status\") | state), \
         (if f(\"resolution\") == null then \"\" else \"resolved\" end), {}] | @tsv",
        listing_columns(excerpt),
        fields.join(", ")
    )
}

/// One issue: key, summary, status category (or name), and whether it is resolved.
pub(crate) fn reading_filter() -> String {
    format!(
        r#"{PRELUDE}
issues | [(.key // ""), (f("summary") // "" | tostring),
  (f("status") | state),
  (if f("resolution") == null then "" else "resolved" end)] | @tsv"#
    )
}

/// The text a Drone is briefed from: summary, a blank line, the description.
pub(crate) fn text_filter() -> String {
    format!("{PRELUDE}\nissues | ((f(\"summary\") // \"\") + \"\\n\\n\" + (f(\"description\") | adf))")
}

/// When an issue was last edited.
pub(crate) fn edited_filter() -> String {
    format!("{PRELUDE}\nissues | (f(\"updated\") // \"\" | tostring | rfc)")
}
