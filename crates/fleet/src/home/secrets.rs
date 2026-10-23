//! What may not leave this machine for the home: an address that carries a credential.
//!
//! **Refused rather than scrubbed.** A copy with the credential cut out would read as differing
//! from this machine's for good, and the person would be asked to pick between their file and a
//! file nobody wrote. So a piece holding one is not added, and the reason names where.

/// Query keys that carry a credential, compared without case.
const CREDENTIAL_KEYS: &[&str] = &[
    "token", "access_token", "auth", "auth_token", "api_key", "apikey", "key", "secret", "client_secret", "password",
    "pass", "pwd", "sig", "signature", "private_token", "session",
];

fn ends_address(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')' | '>' | ']' | '}' | '<')
}

/// Why `text` may not leave this machine, naming the host and never the credential. `None` where
/// it may.
pub(crate) fn leak(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let after = &rest[at + 3..];
        let address: &str = after.split(ends_address).next().unwrap_or("");
        let authority = address.split(['/', '?', '#']).next().unwrap_or("");
        let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        if authority.contains('@') {
            return Some(format!("an address at {host} carries a user and password"));
        }
        if let Some((_, query)) = address.split_once('?') {
            let query = query.split('#').next().unwrap_or("");
            let named = query.split('&').filter_map(|pair| pair.split_once('=')).find(|(key, value)| {
                !value.is_empty() && CREDENTIAL_KEYS.iter().any(|known| key.eq_ignore_ascii_case(known))
            });
            if let Some((key, _)) = named {
                return Some(format!("an address at {host} carries `{key}` in its query"));
            }
        }
        rest = after;
    }
    None
}
