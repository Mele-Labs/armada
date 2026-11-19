//! Local network: Armada Classic's own check of what a Job browser's page reaches on this Mac or the owner's network
//! (`capture/local-network.ts`), as far as WebKit lets it be done.
//!
//! **WebKit gives Bridge no hook on each request**, where Electron lets a session decide each one. What is done instead:
//!
//! * A `WKContentRuleList` refuses loads to loopback and private address literals, `localhost`, `*.localhost` and `*.local`
//!   from a page whose own host is public, and is rebuilt when a choice changes, leaving the sites that were allowed out.
//! * A document-start script (`local_network.js`) sees fetch, XMLHttpRequest, WebSocket and EventSource reach such a host,
//!   and the ask is raised from what it says (`needs_check`).
//!
//! **What this misses next to Classic**: a public name that resolves to a private address. Classic looks a name up and keeps
//! the answer a minute; a content rule list sees only the text of an address, and the script only the text it was given, so such
//! a name is taken as public and goes through. It also misses an address spelled as one number (`2130706433`) or in octal,
//! and a page on the local network reaching the loopback is not told apart from one reaching its own space: Classic checks that
//! (a more private target than the page's own), and a rule list can only tell a public page from a private one.

use serde_json::{json, Value};

use super::choices::{Choice, Permission, Settings};

/// How private an address is, the most private first (`RANK`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Space {
    Loopback,
    Local,
    Public,
}

/// The space of a host name or address as its text says: no name is looked up. A port is not part of it.
pub fn space_of(host: &str) -> Space {
    let name = host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
    if name == "localhost" || name.ends_with(".localhost") {
        return Space::Loopback;
    }
    if name.ends_with(".local") {
        return Space::Local;
    }
    if let Ok(v4) = name.parse::<std::net::Ipv4Addr>() {
        return space_of_v4(v4);
    }
    if let Ok(v6) = name.parse::<std::net::Ipv6Addr>() {
        if let Some(v4) = v6.to_ipv4_mapped() {
            return space_of_v4(v4);
        }
        let first = v6.segments()[0];
        return if v6.is_loopback() || v6.is_unspecified() {
            Space::Loopback
        } else if first & 0xfe00 == 0xfc00 || first & 0xffc0 == 0xfe80 {
            Space::Local
        } else {
            Space::Public
        };
    }
    Space::Public
}

fn space_of_v4(address: std::net::Ipv4Addr) -> Space {
    let [a, b, ..] = address.octets();
    if a == 127 || a == 0 {
        Space::Loopback
    } else if a == 10 || (a == 100 && (64..128).contains(&b)) || (a == 169 && b == 254) || (a == 172 && (16..32).contains(&b)) || (a == 192 && b == 168) {
        Space::Local
    } else {
        Space::Public
    }
}

/// The name a `host:port` names, brackets kept on an IPv6 address.
pub fn hostname(host: &str) -> &str {
    if let Some(end) = host.strip_prefix('[').and_then(|_| host.find(']')) {
        return &host[..=end];
    }
    host.split(':').next().unwrap_or(host)
}

/// Whether a page's reach is one to ask about: **only a request from a more public address to a more private one**, the rule
/// Chromium's own check has, and a page on the loopback is never checked (`reachTest`).
pub fn needs_check(page: Space, target: Space) -> bool {
    page != Space::Loopback && target < page
}

/// What follows `scheme://` in an address on this Mac or the owner's network, as content rule `url-filter`s: none of them can use
/// `|`, which a rule list's patterns do not have, so each range of a number is its own.
const PRIVATE: [&str; 20] = [
    r"127\.[0-9]+\.[0-9]+\.[0-9]+[:/]",
    r"0\.[0-9]+\.[0-9]+\.[0-9]+[:/]",
    r"\[::1\][:/]",
    r"\[::\][:/]",
    r"localhost[:/]",
    r"[^/:]*\.localhost[:/]",
    r"[^/:]*\.local[:/]",
    r"10\.[0-9]+\.[0-9]+\.[0-9]+[:/]",
    r"100\.6[4-9]\.[0-9]+\.[0-9]+[:/]",
    r"100\.[7-9][0-9]\.[0-9]+\.[0-9]+[:/]",
    r"100\.1[01][0-9]\.[0-9]+\.[0-9]+[:/]",
    r"100\.12[0-7]\.[0-9]+\.[0-9]+[:/]",
    r"169\.254\.[0-9]+\.[0-9]+[:/]",
    r"172\.1[6-9]\.[0-9]+\.[0-9]+[:/]",
    r"172\.2[0-9]\.[0-9]+\.[0-9]+[:/]",
    r"172\.3[01]\.[0-9]+\.[0-9]+[:/]",
    r"192\.168\.[0-9]+\.[0-9]+[:/]",
    r"\[f[cd][0-9a-f][0-9a-f]:",
    r"\[fe[89ab][0-9a-f]:",
    // IPv4-mapped loopback, as a browser spells it.
    r"\[::ffff:7f[0-9a-f][0-9a-f]:",
];

/// What a load is, that a page's own scripts and markup make. A main-frame document is left out: an address typed in the page is
/// never refused, so the owner can always go there. A frame inside the page is its own rule (`load-context`).
const KINDS: [&str; 9] = ["image", "style-sheet", "script", "font", "raw", "svg-document", "media", "ping", "websocket"];

/// A host as a pattern for the page's own address, or none where it holds a character an address cannot.
fn site_pattern(host: &str) -> Option<String> {
    let host = host.to_ascii_lowercase();
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']')) {
        return None;
    }
    let escaped: String = host.chars().flat_map(|c| if matches!(c, '.' | '[' | ']') { vec!['\\', c] } else { vec![c] }).collect();
    Some(format!("^[a-z]+://{escaped}/"))
}

fn block(target: &str, kinds: &[&str], context: Option<&str>, tops: (&str, &[String])) -> Value {
    let mut trigger = json!({ "url-filter": format!("^[a-z]+://{target}"), "resource-type": kinds });
    if let Some(context) = context {
        trigger["load-context"] = json!([context]);
    }
    trigger[tops.0] = json!(tops.1);
    json!({ "trigger": trigger, "action": { "type": "block" } })
}

/// The rule list for a page of `repository`, as the JSON `WKContentRuleListStore` compiles; none where nothing is to be refused.
///
/// With Local network at Ask or Block by default, every private target is refused unless the page is on a private host or its own
/// choice is Allow, which is the one thing a page can be given. With it at Allow by default, only the pages whose own choice is not
/// Allow are refused, and a Mac whose every site is allowed has no list at all.
pub fn rules_json(settings: &Settings, repository: &str) -> Option<String> {
    let own = |choice: Choice| -> Vec<String> { settings.hosts_with(repository, Permission::LocalNetwork, choice).iter().filter_map(|host| site_pattern(host)).collect() };
    let rules: Vec<Value> = if settings.default_of(Permission::LocalNetwork) == Choice::Allow {
        let refused: Vec<String> = [Choice::Ask, Choice::Block].into_iter().flat_map(own).collect();
        if refused.is_empty() {
            return None;
        }
        let tops = ("if-top-url", refused.as_slice());
        PRIVATE.iter().flat_map(|target| [block(target, &KINDS, None, tops), block(target, &["document"], Some("child-frame"), tops)]).collect()
    } else {
        let mut except: Vec<String> = PRIVATE.iter().map(|target| format!("^[a-z]+://{target}")).collect();
        except.extend(own(Choice::Allow));
        let tops = ("unless-top-url", except.as_slice());
        PRIVATE.iter().flat_map(|target| [block(target, &KINDS, None, tops), block(target, &["document"], Some("child-frame"), tops)]).collect()
    };
    Some(Value::Array(rules).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_in_the_space_its_text_says() {
        for loopback in ["127.0.0.1", "127.9.9.9", "0.0.0.0", "localhost", "app.localhost", "[::1]", "::1", "::", "LOCALHOST", "::ffff:127.0.0.1"] {
            assert_eq!(space_of(loopback), Space::Loopback, "{loopback}");
        }
        for local in ["10.1.2.3", "100.64.0.1", "100.127.255.1", "169.254.1.1", "172.16.0.1", "172.31.255.255", "192.168.68.98", "printer.local", "fd12::1", "fe80::1", "[fe80::1]"] {
            assert_eq!(space_of(local), Space::Local, "{local}");
        }
        for public in ["8.8.8.8", "example.com", "100.63.0.1", "100.128.0.1", "172.15.0.1", "172.32.0.1", "192.169.0.1", "2001:db8::1", "notlocalhost", "local"] {
            assert_eq!(space_of(public), Space::Public, "{public}");
        }
    }

    #[test]
    fn the_name_of_a_host_and_port_is_the_name_alone() {
        assert_eq!(hostname("localhost:3000"), "localhost");
        assert_eq!(hostname("example.com"), "example.com");
        assert_eq!(hostname("[::1]:8080"), "[::1]");
        assert_eq!(hostname("[fe80::1]"), "[fe80::1]");
        assert_eq!(space_of(hostname("[::1]:8080")), Space::Loopback);
    }

    #[test]
    fn only_a_request_to_a_more_private_space_than_the_pages_own_is_checked() {
        assert!(needs_check(Space::Public, Space::Loopback));
        assert!(needs_check(Space::Public, Space::Local));
        assert!(needs_check(Space::Local, Space::Loopback));
        assert!(!needs_check(Space::Local, Space::Local));
        assert!(!needs_check(Space::Local, Space::Public));
        assert!(!needs_check(Space::Public, Space::Public));
        assert!(!needs_check(Space::Loopback, Space::Loopback), "a page on the loopback is never checked");
        assert!(!needs_check(Space::Loopback, Space::Public));
    }

    fn list(choices: &str) -> Option<Vec<Value>> {
        let settings = Settings::parse(choices);
        rules_json(&settings, "armada").map(|text| match ipc::decode::<Value>("rules", text.as_bytes()).expect("JSON") {
            Value::Array(rules) => rules,
            other => panic!("not a list: {other}"),
        })
    }

    #[test]
    fn with_nothing_chosen_every_private_target_is_refused_unless_the_page_is_private() {
        let rules = list("{}").expect("a list");
        assert_eq!(rules.len(), PRIVATE.len() * 2, "each target, as a load and as a frame");
        let first = &rules[0];
        assert_eq!(first["action"]["type"], "block");
        assert_eq!(first["trigger"]["url-filter"], r"^[a-z]+://127\.[0-9]+\.[0-9]+\.[0-9]+[:/]");
        assert!(!first["trigger"]["resource-type"].as_array().expect("kinds").iter().any(|kind| kind == "document"), "an address typed in the page is never refused");
        let frames = &rules[1];
        assert_eq!(frames["trigger"]["resource-type"][0], "document");
        assert_eq!(frames["trigger"]["load-context"][0], "child-frame");
        let unless = first["trigger"]["unless-top-url"].as_array().expect("tops");
        assert_eq!(unless.len(), PRIVATE.len(), "a private page's own loads are left alone");
        assert!(first["trigger"].get("if-top-url").is_none());
    }

    #[test]
    fn a_site_that_was_allowed_is_left_out_of_the_list_and_one_that_was_blocked_is_not() {
        let rules = list(r#"{"armada":{"sites":{"192.168.68.98:5481":{"local_network":"allow"},"[::1]:8080":{"local_network":"allow"},"bad.com":{"local_network":"block"}}}}"#).expect("a list");
        let unless: Vec<&str> = rules[0]["trigger"]["unless-top-url"].as_array().expect("tops").iter().filter_map(Value::as_str).collect();
        assert!(unless.contains(&r"^[a-z]+://192\.168\.68\.98:5481/"), "{unless:?}");
        assert!(unless.contains(&r"^[a-z]+://\[::1\]:8080/"), "{unless:?}");
        assert!(!unless.iter().any(|one| one.contains("bad")), "a Block stays refused");
    }

    #[test]
    fn where_every_site_is_allowed_by_default_only_the_sites_that_are_not_are_refused_and_none_means_no_list() {
        assert!(list(r#"{"defaults":{"local_network":"allow"}}"#).is_none(), "nothing to refuse");
        let rules = list(r#"{"defaults":{"local_network":"allow"},"armada":{"sites":{"bad.com":{"local_network":"block"},"ok.com":{"local_network":"allow"}}}}"#).expect("a list");
        let only: Vec<&str> = rules[0]["trigger"]["if-top-url"].as_array().expect("tops").iter().filter_map(Value::as_str).collect();
        assert_eq!(only, [r"^[a-z]+://bad\.com/"]);
        assert!(rules[0]["trigger"].get("unless-top-url").is_none());
    }

    #[test]
    fn a_host_with_a_character_no_address_has_is_not_made_a_pattern() {
        assert_eq!(site_pattern("a.com").as_deref(), Some(r"^[a-z]+://a\.com/"));
        assert_eq!(site_pattern("a.com/(x|y)"), None);
        assert_eq!(site_pattern(""), None);
    }

    #[test]
    fn no_target_pattern_uses_an_alternation_or_a_counted_repeat_which_a_rule_list_does_not_have() {
        for pattern in PRIVATE {
            assert!(!pattern.contains('|') && !pattern.contains('{'), "{pattern}");
        }
    }
}
