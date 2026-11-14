//! The Cloudflare tokens in the Keychain, in a throwaway keychain under `/tmp`. **Never the login
//! keychain**: the items are written to a keychain made for the case and deleted after it.

use adapter_traits::{CloudflareTokens, Secret};

use crate::cloudflare_tokens::keep_line;
use crate::KeychainCloudflareTokens;

const ACCOUNT: &str = "0123456789abcdef0123456789abcdef";

#[cfg(target_os = "macos")]
struct Throwaway {
    keychain: std::path::PathBuf,
}

#[cfg(target_os = "macos")]
impl Throwaway {
    fn made(name: &str) -> Throwaway {
        let keychain = std::path::PathBuf::from(format!("/tmp/armada-cloudflare-{name}-{}.keychain-db", std::process::id()));
        let _ = std::fs::remove_file(&keychain);
        let path = keychain.to_str().unwrap();
        for args in [["create-keychain", "-p", "x", path], ["unlock-keychain", "-p", "x", path]] {
            let status = std::process::Command::new("/usr/bin/security").args(args).status().expect("security runs");
            assert!(status.success(), "security {args:?}");
        }
        Throwaway { keychain }
    }
}

#[cfg(target_os = "macos")]
impl Drop for Throwaway {
    fn drop(&mut self) {
        let _ = std::process::Command::new("/usr/bin/security").args(["delete-keychain", self.keychain.to_str().unwrap()]).status();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_cloudflare_token_is_kept_read_back_replaced_and_forgotten() {
    let throwaway = Throwaway::made("kept");
    let tokens = KeychainCloudflareTokens::in_keychain(&throwaway.keychain);
    assert!(tokens.read(ACCOUNT).expect("read").is_none());
    tokens.keep(ACCOUNT, Secret::new("first_Token-1".to_string())).expect("kept");
    assert_eq!(tokens.read(ACCOUNT).expect("read").expect("held").expose(), "first_Token-1");
    assert!(tokens.read("ffffffffffffffffffffffffffffffff").expect("read").is_none(), "an item per account");
    tokens.keep(ACCOUNT, Secret::new("second_Token-2".to_string())).expect("replaced");
    assert_eq!(tokens.read(ACCOUNT).unwrap().unwrap().expose(), "second_Token-2");
    tokens.forget(ACCOUNT).expect("forgotten");
    assert!(tokens.read(ACCOUNT).unwrap().is_none());
    tokens.forget(ACCOUNT).expect("forgetting twice is not an error");
}

#[test]
fn the_token_goes_to_security_as_hex_and_never_as_itself() {
    let keychain = std::path::Path::new("/tmp/a.keychain-db");
    let line = keep_line(ACCOUNT, "tok en", Some(keychain));
    assert_eq!(line, format!("add-generic-password -U -s armada-cloudflare-token -a \"{ACCOUNT}\" -X 746f6b20656e \"/tmp/a.keychain-db\"\n"));
    assert!(!line.contains("tok en"));
}

#[test]
fn an_account_that_is_not_letters_and_digits_is_refused_before_the_keychain_is_asked() {
    // A keychain path nothing is at: a refusal must come before it is reached.
    let tokens = KeychainCloudflareTokens::in_keychain(&std::env::temp_dir().join("armada-cloudflare-none.keychain-db"));
    for account in ["", "a b", "a\"b", "../x", &"a".repeat(65)] {
        assert!(tokens.read(account).is_err(), "{account:?}");
        assert!(tokens.keep(account, Secret::new("t".to_string())).is_err(), "{account:?}");
        assert!(tokens.forget(account).is_err(), "{account:?}");
    }
}
