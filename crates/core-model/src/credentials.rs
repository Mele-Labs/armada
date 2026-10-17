//! What a credential is called.

/// A name whose value is a credential. Matched case-insensitively as a
/// substring, so `AWS_SECRET_ACCESS_KEY` is caught by `secret`.
///
/// `fleet::redaction` scrubs a filed report with it — a fact about what a
/// credential is called belongs to the domain, not to reports alone.
///
/// **Deliberately narrow: `auth` is not here.** It catches `author`, and a
/// report whose commit author was redacted is missing the fact somebody filed
/// it to explain; `authorization` is the spelling that means the header. No
/// vendor prefixes either — that list is both a better detector and a list of
/// credential shapes this repository's own guard refuses to commit.
pub const CREDENTIAL_NAMES: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "passphrase",
    "credential",
    "apikey",
    "api_key",
    "access_key",
    "private_key",
    "authorization",
];
