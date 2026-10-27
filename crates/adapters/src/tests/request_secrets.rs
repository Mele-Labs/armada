//! The Keychain's secret variables, in a throwaway keychain under `/tmp`. **Never the login
//! keychain**: every item is written to a keychain made for the case and deleted after it, so the
//! person's own keychain and its search list are never touched.

use adapter_traits::{RequestSecrets, Secret, SecretError, SecretVariable};

use crate::KeychainSecrets;

fn variable(environment: &str, name: &str) -> SecretVariable {
    SecretVariable { repository: "acme--shop".to_string(), environment: environment.to_string(), name: name.to_string() }
}

#[cfg(target_os = "macos")]
fn security(args: &[&str]) {
    let status = std::process::Command::new("/usr/bin/security").args(args).status().expect("security runs");
    assert!(status.success(), "security {args:?}");
}

/// A keychain made for one case, deleted when it goes.
#[cfg(target_os = "macos")]
struct Throwaway {
    keychain: std::path::PathBuf,
}

#[cfg(target_os = "macos")]
impl Throwaway {
    fn made(name: &str) -> Throwaway {
        let keychain = std::path::PathBuf::from(format!("/tmp/armada-secrets-{name}-{}.keychain-db", std::process::id()));
        let _ = std::fs::remove_file(&keychain);
        let path = keychain.to_str().unwrap();
        security(&["create-keychain", "-p", "x", path]);
        security(&["unlock-keychain", "-p", "x", path]);
        Throwaway { keychain }
    }

    fn secrets(&self) -> KeychainSecrets {
        KeychainSecrets::in_keychain(&self.keychain)
    }
}

#[cfg(target_os = "macos")]
impl Drop for Throwaway {
    fn drop(&mut self) {
        let _ = std::process::Command::new("/usr/bin/security")
            .args(["delete-keychain", self.keychain.to_str().unwrap()])
            .status();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_secret_is_kept_read_back_whole_and_forgotten() {
    let throwaway = Throwaway::made("kept");
    let secrets = throwaway.secrets();
    // An environment's name may hold a space and a quote; the value a space, a quote and `;`.
    let token = variable("my \"staging\"", "token");
    let value = "Bearer a b\"c; d=e";
    assert!(!secrets.is_kept(&token).expect("asked"));
    assert!(secrets.read(&token).expect("read").is_none());
    secrets.keep(&token, Secret::new(value.to_string())).expect("kept");
    assert!(secrets.is_kept(&token).expect("asked"));
    assert_eq!(secrets.read(&token).expect("read").expect("held").expose(), value);
    assert!(secrets.read(&variable("my \"staging\"", "other")).expect("read").is_none());
    assert!(secrets.read(&variable("production", "token")).expect("read").is_none(), "an item per environment");

    secrets.keep(&token, Secret::new("second-value".to_string())).expect("replaced");
    assert_eq!(secrets.read(&token).unwrap().unwrap().expose(), "second-value");

    secrets.forget(&token).expect("forgotten");
    assert!(!secrets.is_kept(&token).unwrap());
    secrets.forget(&token).expect("forgetting twice is not an error");
}

/// A browser's cookie: kB of `name=value; ` pairs holding `%`, `|` and JSON-ish text.
fn cookie(length: usize) -> String {
    let pair = "sess_id=a%7B%22u%22%3A42%7D|x; theme=dark; ";
    pair.repeat(length / pair.len() + 1)[..length].to_string()
}

#[test]
fn no_line_security_is_given_for_a_long_secret_passes_what_it_reads_in_one() {
    let keychain = std::path::Path::new("/tmp/armada-secrets-a-keychain-path-of-some-length.keychain-db");
    let lines = crate::request_secrets::keep_lines("acme--shop/qa-9/cookie", &cookie(4096), Some(keychain)).expect("lines");
    for line in &lines {
        // `security -i` reads 4095 bytes a line and runs what follows as another command.
        assert!(line.len() <= 4095, "a line of {} bytes", line.len());
        assert!(line.ends_with(&format!("{:?}\n", keychain.to_str().unwrap())), "every line names the keychain");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_cookie_of_kilobytes_is_kept_whole_replaced_by_a_shorter_one_and_forgotten() {
    let throwaway = Throwaway::made("cookie");
    let secrets = throwaway.secrets();
    let session = variable("qa-9", "cookie");
    let long = cookie(4096);
    assert!(long.contains(';') && long.contains('=') && long.contains('%') && long.contains('|'));
    secrets.keep(&session, Secret::new(long.clone())).expect("kept");
    assert_eq!(secrets.read(&session).expect("read").expect("held").expose(), &long);

    // The longer value's later parts must not be read as the end of the shorter one.
    let short = cookie(1600);
    secrets.keep(&session, Secret::new(short.clone())).expect("replaced");
    assert_eq!(secrets.read(&session).unwrap().unwrap().expose(), &short);

    secrets.keep(&session, Secret::new(long.clone())).expect("kept again");
    secrets.forget(&session).expect("forgotten");
    assert!(!secrets.is_kept(&session).unwrap());
    let part = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", crate::request_secrets::SECRET_PART_SERVICE, "-a", "acme--shop/qa-9/cookie/1"])
        .arg(&throwaway.keychain)
        .status()
        .unwrap();
    assert_eq!(part.code(), Some(44), "no part is left behind");
}

#[test]
fn a_failed_save_names_the_keychains_status_and_never_an_echoed_line() {
    let said = "security: unknown command \"6162633d\"\n\
                security: SecKeychainItemCreateFromContent (/tmp/a.keychain-db): User canceled the operation.\n\
                add-generic-password: returned -128\n";
    let status = crate::request_secrets::status_of(said);
    assert_eq!(status, "OSStatus -128, User canceled the operation.");
    assert_eq!(crate::request_secrets::status_of("security: unknown command \"6162633d\"\n"), "security named no status");
}

#[test]
fn a_value_that_does_not_fit_is_refused_before_the_keychain_is_asked_and_not_repeated() {
    let folder = std::env::temp_dir().join(format!("armada-secrets-refused-{}", std::process::id()));
    // A keychain path nothing is at: a refusal must come before it is reached.
    let secrets = KeychainSecrets::in_keychain(&folder.join("none.keychain-db"));
    for value in ["", "two\nlines", "caf\u{e9}"] {
        let refused = secrets.keep(&variable("staging", "token"), Secret::new(value.to_string())).unwrap_err();
        assert!(matches!(&refused, SecretError::Refused(why) if !why.contains("lines")), "{refused:?}");
    }
}
