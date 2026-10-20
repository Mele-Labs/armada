//! The Keychain cookie store, in a throwaway keychain under `/tmp`. **Never the login keychain**:
//! every item is written to a keychain made for the case and deleted after it, so the person's own
//! keychain and its search list are never touched.

use adapter_traits::{CookieError, KeptHost, RequestCookies, Secret};

use crate::KeychainCookies;

#[cfg(target_os = "macos")]
fn security(args: &[&str]) {
    let status = std::process::Command::new("/usr/bin/security").args(args).status().expect("security runs");
    assert!(status.success(), "security {args:?}");
}

/// A keychain made for one case, deleted when it goes.
#[cfg(target_os = "macos")]
struct Throwaway {
    keychain: std::path::PathBuf,
    folder: std::path::PathBuf,
}

#[cfg(target_os = "macos")]
impl Throwaway {
    fn made(name: &str) -> Throwaway {
        let folder = std::env::temp_dir().join(format!("armada-cookies-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let keychain = std::path::PathBuf::from(format!("/tmp/armada-cookies-{name}-{}.keychain-db", std::process::id()));
        let _ = std::fs::remove_file(&keychain);
        let path = keychain.to_str().unwrap();
        security(&["create-keychain", "-p", "x", path]);
        security(&["unlock-keychain", "-p", "x", path]);
        Throwaway { keychain, folder }
    }

    fn cookies(&self) -> KeychainCookies {
        KeychainCookies::in_keychain(&self.keychain, &self.folder.join("request-hosts"))
    }
}

#[cfg(target_os = "macos")]
impl Drop for Throwaway {
    fn drop(&mut self) {
        let _ = std::process::Command::new("/usr/bin/security")
            .args(["delete-keychain", self.keychain.to_str().unwrap()])
            .status();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

fn kept(host: &str, header: &str) -> KeptHost {
    KeptHost { host: host.to_string(), header: header.to_string() }
}

#[cfg(target_os = "macos")]
#[test]
fn a_header_kept_for_a_host_is_read_back_whole_and_its_host_and_header_listed() {
    let throwaway = Throwaway::made("kept");
    let cookies = throwaway.cookies();
    // A space, a quote, `;` and `=`: the spike's own value.
    let cookie = "session=a b\"c; theme=dark";
    cookies.keep("API.example.test", "Cookie", Secret::new(cookie.to_string())).expect("kept");
    let read = cookies.kept("api.example.test").expect("read").expect("held");
    assert_eq!((read.header.as_str(), read.value.expose().as_str()), ("Cookie", cookie));
    assert_eq!(cookies.hosts().unwrap(), vec![kept("api.example.test", "Cookie")]);
    assert!(cookies.kept("other.example.test").expect("read").is_none());

    // A host holds one header, so keeping another replaces the first.
    cookies.keep("api.example.test", "Authorization", Secret::new("Bearer second".to_string())).expect("replaced");
    let read = cookies.kept("api.example.test").unwrap().unwrap();
    assert_eq!((read.header.as_str(), read.value.expose().as_str()), ("Authorization", "Bearer second"));
    assert_eq!(cookies.hosts().unwrap(), vec![kept("api.example.test", "Authorization")]);

    cookies.forget("api.example.test").expect("forgotten");
    assert!(cookies.kept("api.example.test").unwrap().is_none());
    assert!(cookies.hosts().unwrap().is_empty());
    cookies.forget("api.example.test").expect("forgetting twice is not an error");
}

#[cfg(target_os = "macos")]
#[test]
fn the_hosts_file_holds_the_header_name_and_no_value() {
    let throwaway = Throwaway::made("listed");
    let cookies = throwaway.cookies();
    cookies.keep("api.example.test", "Authorization", Secret::new("Bearer s3cr3t-value".to_string())).expect("kept");
    let listed = std::fs::read_to_string(throwaway.folder.join("request-hosts")).unwrap();
    assert_eq!(listed, "api.example.test Authorization\n");
}

#[cfg(target_os = "macos")]
#[test]
fn a_host_the_file_lists_without_a_header_is_a_cookie() {
    let throwaway = Throwaway::made("legacy");
    let cookies = throwaway.cookies();
    cookies.keep("api.example.test", "Cookie", Secret::new("a=b".to_string())).expect("kept");
    std::fs::write(throwaway.folder.join("request-hosts"), "api.example.test\n").unwrap();
    assert_eq!(cookies.hosts().unwrap(), vec![kept("api.example.test", "Cookie")]);
    assert_eq!(cookies.kept("api.example.test").unwrap().unwrap().header, "Cookie");
}

#[test]
fn a_host_header_or_value_that_does_not_fit_is_refused_before_the_keychain_is_asked() {
    let folder = std::env::temp_dir().join(format!("armada-cookies-refused-{}", std::process::id()));
    // A keychain path nothing is at: a refusal must come before it is reached.
    let cookies = KeychainCookies::in_keychain(&folder.join("none.keychain-db"), &folder.join("request-hosts"));
    for host in ["https://api.example.test", "api.example.test:8443", "api.example.test/v1", ""] {
        let refused = cookies.keep(host, "Cookie", Secret::new("a=b".to_string())).unwrap_err();
        assert!(matches!(refused, CookieError::Refused(_)), "{host}: {refused:?}");
    }
    for header in ["", "Not A Header", "Authorization:", "X\nY"] {
        let refused = cookies.keep("api.example.test", header, Secret::new("a=b".to_string())).unwrap_err();
        assert!(matches!(refused, CookieError::Refused(_)), "{header:?}: {refused:?}");
    }
    let refused = cookies.keep("api.example.test", "Cookie", Secret::new("a=b\nc=d".to_string())).unwrap_err();
    assert!(matches!(&refused, CookieError::Refused(why) if !why.contains("c=d")), "{refused:?}");
    assert!(!folder.join("request-hosts").exists());
}
