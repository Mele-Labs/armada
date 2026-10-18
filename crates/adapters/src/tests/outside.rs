//! Which calls leave the repository, and that what they were given never goes with them.

use adapter_traits::{AgentHarness, Reach, ToolUse};

use crate::harness::HeadlessAgent;
use crate::outside::{command_reach, tool_reach};

fn seen(service: &str, name: &str, line: &str) -> Option<Reach> {
    Some(Reach { service: service.to_string(), name: name.to_string(), line: line.to_string() })
}

/// A `gh` command is the forge's, and its line is `gh` and the subcommand words: never an argument or a flag.
#[test]
fn a_gh_command_is_named_by_its_subcommand_and_nothing_after() {
    for (typed, line) in [
        ("gh pr view 2540 --json title,body", "gh pr view"),
        ("gh run list --workflow ci --limit 5", "gh run list"),
        ("gh pr checkout my-branch", "gh pr checkout"),
        ("gh pr create --title 'Fix' --body 'a secret in it'", "gh pr create"),
        ("  gh   pr   status", "gh pr status"),
        ("gh api repos/Mele-Labs/armada/pulls --jq .", "gh api"),
        ("gh api user", "gh api"),
        ("gh status", "gh status"),
        ("gh -R Mele-Labs/armada pr view", "gh"),
        ("gh", "gh"),
        ("gh auth login --with-token abcdefghijklmnopqrstuvwxyz0123", "gh auth login"),
        ("gh workflow run Release.yml", "gh workflow run"),
    ] {
        assert_eq!(command_reach(typed), seen("forge", "GitHub", line), "{typed}");
    }
}

/// `gh issue` is the issue tracker's, whatever it does to one.
#[test]
fn a_gh_issue_command_is_the_issue_trackers() {
    assert_eq!(command_reach("gh issue create --title x --body y"), seen("tracker", "the issue tracker", "gh issue create"));
    assert_eq!(command_reach("gh issue list"), seen("tracker", "the issue tracker", "gh issue list"));
    assert_eq!(command_reach("gh issue"), seen("tracker", "the issue tracker", "gh issue"));
}

/// Only a command whose first word is `gh` is one: a word that merely contains it, or comes later, is not.
#[test]
fn a_command_that_does_not_begin_with_gh_is_not_a_call() {
    for typed in ["cargo test", "ghc Main.hs", "echo gh pr view", "git status && gh pr view", "cd x; gh pr list", "", "   ", "GH pr view"] {
        assert_eq!(command_reach(typed), None, "{typed}");
    }
}

/// A server's tool is a call to the server, named by its tool and not its arguments.
#[test]
fn a_servers_tool_is_a_call_to_that_server_by_the_tools_name() {
    for (tool, service, name, line) in [
        ("mcp__claude_ai_Figma__get_design_context", "figma", "Figma", "get_design_context"),
        ("mcp__claude_ai_Google_Drive__search_files", "google_drive", "Google drive", "search_files"),
        ("mcp__plugin_playwright_playwright__browser_click", "playwright", "Playwright", "browser_click"),
        ("mcp__context7__query-docs", "context7", "Context7", "query-docs"),
        ("mcp__github__get_me", "forge", "GitHub", "get_me"),
        ("mcp__plugin_dev_github__create_issue", "forge", "GitHub", "create_issue"),
        ("mcp__claude_ai_Todoist__find-tasks", "todoist", "Todoist", "find-tasks"),
        ("mcp__sentry__a__b", "sentry", "Sentry", "a__b"),
    ] {
        assert_eq!(tool_reach(tool, "some detail"), seen(service, name, line), "{tool}");
    }
}

/// Linear, Jira and the rest are one service, however the harness prefixes their names.
#[test]
fn an_issue_trackers_server_is_the_one_tracker_service() {
    for tool in [
        "mcp__linear__list_issues",
        "mcp__plugin_productivity_linear__list_issues",
        "mcp__claude_ai_Linear__list_issues",
        "mcp__jira__list_issues",
        "mcp__plugin_productivity_atlassian__list_issues",
        "mcp__plugin_productivity_asana__list_issues",
        "mcp__plugin_productivity_clickup__list_issues",
        "mcp__plugin_productivity_monday__list_issues",
    ] {
        assert_eq!(tool_reach(tool, ""), seen("tracker", "the issue tracker", "list_issues"), "{tool}");
    }
}

/// Armada's own servers and the code-graph helper never leave the machine, so they are not calls.
#[test]
fn a_server_that_stays_on_this_machine_is_not_a_call() {
    for tool in [
        "mcp__armada__who_owns",
        "mcp__armada-fleet__show_window",
        "mcp__plugin_armada_armada-fleet__waiting_for",
        "mcp__armada_extra__x",
        "mcp__gitnexus__query",
        "mcp__plugin_gitnexus_gitnexus__impact",
    ] {
        assert_eq!(tool_reach(tool, ""), None, "{tool}");
    }
}

/// Anything that is not a server's tool or a web call is not one: a built-in, a malformed name, a bare prefix.
#[test]
fn a_tool_that_is_neither_a_server_nor_the_web_is_not_a_call() {
    for tool in ["Read", "Bash", "Task", "Grep", "mcp__figma", "mcp____x", "mcp__figma__", "mcp__", "figma__get", "webfetch"] {
        assert_eq!(tool_reach(tool, "https://example.com"), None, "{tool}");
    }
}

/// A page fetched is its host, and nothing a person could have put after it.
#[test]
fn a_page_fetched_is_its_host_and_no_more() {
    for (address, host) in [
        ("https://doc.rust-lang.org/std/index.html", "doc.rust-lang.org"),
        ("https://Docs.RS/serde?token=abc123&q=secret#frag", "docs.rs"),
        ("http://user:pw@example.com:8080/a/b?c=d", "example.com"),
        ("https://example.com?access_token=zzz", "example.com"),
        ("https://example.com#section", "example.com"),
        ("example.com/path", "example.com"),
        ("https://[::1]:3000/x", "[::1]"),
        ("  https://api.github.example/v3/repos  ", "api.github.example"),
    ] {
        assert_eq!(tool_reach("WebFetch", address), seen("web", "the web", host), "{address}");
    }
}

/// A fetch with no address to read is left out rather than entered with a guess.
#[test]
fn a_fetch_with_no_host_is_not_entered() {
    for address in ["", "   ", "https://", "https:///path", "https://?q=1", "not a url"] {
        assert_eq!(tool_reach("WebFetch", address), None, "{address:?}");
    }
}

/// A search is the word `search`: its query is never carried.
#[test]
fn a_search_is_the_word_search_and_never_the_query() {
    assert_eq!(tool_reach("WebSearch", "how to hide api_key=abc123 in logs"), seen("web", "the web", "search"));
    assert_eq!(tool_reach("WebSearch", ""), seen("web", "the web", "search"));
}

/// Read off a stream line, the harness answers `Reached` for each of these, once, and nothing it answers
/// holds the query, the arguments or the body the call carried.
#[test]
fn the_harness_answers_reached_for_a_call_and_carries_nothing_it_was_given() {
    let harness = HeadlessAgent::at("/usr/local/bin/agent");
    let events = harness.read(
        r#"{"type":"assistant","message":{"content":[
             {"type":"tool_use","id":"a","name":"Bash","input":{"command":"gh pr view 12 --json body --jq .body"}},
             {"type":"tool_use","id":"b","name":"WebFetch",
              "input":{"url":"https://docs.rs/serde/latest?token=hunter2","prompt":"summarise secrets"}},
             {"type":"tool_use","id":"c","name":"WebSearch","input":{"query":"password hunter2 leak"}},
             {"type":"tool_use","id":"d","name":"mcp__claude_ai_Figma__get_design_context",
              "input":{"fileKey":"hunter2","nodeId":"1:2"}},
             {"type":"tool_use","id":"e","name":"mcp__armada-fleet__list_sessions","input":{}},
             {"type":"tool_use","id":"f","name":"Bash","input":{"command":"cargo test --workspace"}},
             {"type":"tool_use","id":"g","name":"Task","input":{"description":"look it up on the web"}}]}}"#,
    );
    let used: Vec<Option<ToolUse>> = events.iter().map(|event| harness.used(event)).collect();
    assert_eq!(
        used,
        vec![
            Some(ToolUse::Reached(Reach { service: "forge".into(), name: "GitHub".into(), line: "gh pr view".into() })),
            Some(ToolUse::Reached(Reach { service: "web".into(), name: "the web".into(), line: "docs.rs".into() })),
            Some(ToolUse::Reached(Reach { service: "web".into(), name: "the web".into(), line: "search".into() })),
            Some(ToolUse::Reached(Reach { service: "figma".into(), name: "Figma".into(), line: "get_design_context".into() })),
            None,
            Some(ToolUse::Ran("cargo test --workspace".to_string())),
            None,
        ]
    );
    assert!(events.iter().all(|event| harness.edited(event).is_none()), "no call here edits a file");
}

/// A fetch whose address is longer than a row shows is read whole, so its host is still found.
#[test]
fn a_long_address_still_names_its_host() {
    let harness = HeadlessAgent::at("/usr/local/bin/agent");
    let long = format!("https://example.org/{}", "a/".repeat(300));
    let line = format!(
        r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"a","name":"WebFetch","input":{{"url":"{long}"}}}}]}}}}"#
    );
    let used: Vec<Option<ToolUse>> = harness.read(&line).iter().map(|event| harness.used(event)).collect();
    assert_eq!(used, vec![Some(ToolUse::Reached(Reach { service: "web".into(), name: "the web".into(), line: "example.org".into() }))]);
}
