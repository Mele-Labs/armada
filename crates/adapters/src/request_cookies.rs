//! The owner's kept request headers in the Keychain: [`KeychainCookies`], one generic-password item
//! per host under one service, reached through `/usr/bin/security`. The item holds the header's
//! value, so a host holds one header, and keeping another replaces it.
//!
//! **Through `security`, not the Security framework**, because an item trusts the program that
//! wrote it, and `security` does not change when Fleet is rebuilt
//! (`docs/spikes/034-can-fleet-keep-a-cookie-in-the-keychain.md`). **A value is written on
//! `security -i`'s standard input as hex**, so it is never in a process's arguments for `ps` to
//! read. No error here carries a value, nor `security`'s own output from a write, which can echo
//! the line it was given.
//!
//! **The hosts and the header names are not secret**, and are kept in a file of their own, a host
//! and its header a line (`api.example.test Authorization`), so they are listed without asking the
//! Keychain for anything. The Keychain is where a value is; the file is which hosts have one and
//! under what name. A line with no header is a `Cookie`, which is what the file held before it
//! named any.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use adapter_traits::{CookieError, KeptHeader, KeptHost, RequestCookies, Secret, COOKIE_HEADER};

/// The Keychain's own tool.
const SECURITY: &str = "/usr/bin/security";
/// The service every item is kept under; the host is its account.
pub const COOKIE_SERVICE: &str = "armada-request-cookie";
/// `security`'s exit when there is no such item.
const NO_SUCH_ITEM: i32 = 44;

/// One writer of the hosts file at a time.
static WRITING: Mutex<()> = Mutex::new(());

/// The kept headers, in a keychain, and the hosts holding one in `hosts_file`.
pub struct KeychainCookies {
    /// `None` is the person's default keychain. A test names a throwaway one.
    keychain: Option<PathBuf>,
    hosts_file: PathBuf,
}

impl KeychainCookies {
    /// The person's default keychain, with the hosts listed in `hosts_file`.
    pub fn in_default_keychain(hosts_file: &Path) -> KeychainCookies {
        KeychainCookies { keychain: None, hosts_file: hosts_file.to_path_buf() }
    }

    /// The keychain at `keychain`, for a test that must never touch the person's own.
    pub fn in_keychain(keychain: &Path, hosts_file: &Path) -> KeychainCookies {
        KeychainCookies { keychain: Some(keychain.to_path_buf()), hosts_file: hosts_file.to_path_buf() }
    }

    fn security(&self, verb: &str, host: &str, reading: bool) -> Command {
        let mut command = Command::new(SECURITY);
        command.args([verb, "-s", COOKIE_SERVICE, "-a", host]);
        if reading {
            command.arg("-w");
        }
        if let Some(keychain) = &self.keychain {
            command.arg(keychain);
        }
        command.stdin(Stdio::null());
        command
    }

    fn listed(&self) -> Result<Vec<KeptHost>, CookieError> {
        match std::fs::read_to_string(&self.hosts_file) {
            Ok(text) => {
                let mut hosts: Vec<KeptHost> = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(|line| {
                        let (host, header) = line.split_once(char::is_whitespace).unwrap_or((line, COOKIE_HEADER));
                        KeptHost { host: host.to_string(), header: header.trim().to_string() }
                    })
                    .collect();
                hosts.sort_by(|a, b| a.host.cmp(&b.host));
                hosts.dedup_by(|a, b| a.host == b.host);
                Ok(hosts)
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(why) => Err(CookieError::Unkept(format!("{}: {why}", self.hosts_file.display()))),
        }
    }

    fn list(&self, change: impl FnOnce(&mut Vec<KeptHost>)) -> Result<(), CookieError> {
        let _one_at_a_time = WRITING.lock().unwrap_or_else(|held| held.into_inner());
        let mut hosts = self.listed()?;
        change(&mut hosts);
        hosts.sort_by(|a, b| a.host.cmp(&b.host));
        hosts.dedup_by(|a, b| a.host == b.host);
        let unkept = |why: std::io::Error| CookieError::Unkept(format!("{}: {why}", self.hosts_file.display()));
        if let Some(folder) = self.hosts_file.parent() {
            std::fs::create_dir_all(folder).map_err(unkept)?;
        }
        let mut staged = self.hosts_file.as_os_str().to_owned();
        staged.push(".writing");
        let staged = PathBuf::from(staged);
        let text: String = hosts.iter().map(|one| format!("{} {}\n", one.host, one.header)).collect();
        std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, &self.hosts_file)).map_err(unkept)
    }

    /// The value in the Keychain for `host`, whatever header the file says it is under.
    fn value_of(&self, host: &str) -> Result<Option<Secret<String>>, CookieError> {
        let out = self
            .security("find-generic-password", host, true)
            .output()
            .map_err(|why| CookieError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        match out.status.code() {
            Some(0) => {
                let mut value = String::from_utf8(out.stdout)
                    .map_err(|_| CookieError::Unkept(format!("the header kept for {host} is not text")))?;
                if value.ends_with('\n') {
                    value.pop();
                }
                Ok(Some(Secret::new(value)))
            }
            Some(NO_SUCH_ITEM) => Ok(None),
            _ => Err(CookieError::Unkept(format!(
                "the Keychain would not read the header kept for {host}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }
}

/// A host the Keychain may be asked about: letters, digits, `.` and `-`, lower case.
pub fn host_named(host: &str) -> Result<String, CookieError> {
    let host = host.trim().to_ascii_lowercase();
    let fits = !host.is_empty()
        && host.len() <= 253
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !host.starts_with(['.', '-']);
    match fits {
        true => Ok(host),
        false => Err(CookieError::Refused(format!(
            "{host:?} is not a host: it is letters, digits, `.` and `-`, with no scheme, port or path"
        ))),
    }
}

/// A header name: an HTTP token, which is what a request line may carry, kept as typed.
pub fn header_named(header: &str) -> Result<String, CookieError> {
    let header = header.trim();
    let token = |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
    match !header.is_empty() && header.len() <= 128 && header.chars().all(token) {
        true => Ok(header.to_string()),
        false => Err(CookieError::Refused(
            "a header name is letters, digits and `-`, with no space or colon".to_string(),
        )),
    }
}

/// One line of printable ASCII, which a header can carry and `security -w` reads back as it was
/// written.
fn value_fits(value: &str) -> Result<(), CookieError> {
    match !value.trim().is_empty() && value.bytes().all(|byte| (0x20..0x7f).contains(&byte)) {
        true => Ok(()),
        false => Err(CookieError::Refused(
            "a header's value is one line of printable ASCII, as a browser sends it".to_string(),
        )),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl RequestCookies for KeychainCookies {
    fn kept(&self, host: &str) -> Result<Option<KeptHeader>, CookieError> {
        let host = host_named(host)?;
        let Some(value) = self.value_of(&host)? else { return Ok(None) };
        // An item the file does not list is a `Cookie`, the only header there was.
        let header = self
            .listed()?
            .into_iter()
            .find(|one| one.host == host)
            .map_or_else(|| COOKIE_HEADER.to_string(), |one| one.header);
        Ok(Some(KeptHeader { header, value }))
    }

    fn keep(&self, host: &str, header: &str, value: Secret<String>) -> Result<(), CookieError> {
        let host = host_named(host)?;
        let header = header_named(header)?;
        value_fits(value.expose())?;
        let mut line = format!(
            "add-generic-password -U -s {COOKIE_SERVICE} -a {host} -X {}",
            hex(value.expose().as_bytes())
        );
        if let Some(keychain) = &self.keychain {
            let named = keychain.to_string_lossy();
            if named.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
                return Err(CookieError::Unkept(format!(
                    "{named} cannot be named on `security -i`'s line"
                )));
            }
            line.push(' ');
            line.push_str(&named);
        }
        line.push('\n');
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| CookieError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        let wrote = child.stdin.take().map(|mut stdin| stdin.write_all(line.as_bytes()));
        drop(line);
        let status = child
            .wait()
            .map_err(|why| CookieError::Unkept(format!("{SECURITY} did not finish: {why}")))?;
        // `security -i` can exit 0 on a line that failed, so what was written is read back.
        let read_back = match (wrote, status.success()) {
            (Some(Ok(())), true) => self.value_of(&host)?,
            _ => None,
        };
        if read_back.as_ref().map(|read| read.expose()) != Some(value.expose()) {
            return Err(CookieError::Unkept(format!("the Keychain did not keep the header for {host}")));
        }
        self.list(|hosts| {
            hosts.retain(|one| one.host != host);
            hosts.push(KeptHost { host, header });
        })
    }

    fn forget(&self, host: &str) -> Result<(), CookieError> {
        let host = host_named(host)?;
        let out = self
            .security("delete-generic-password", &host, false)
            .output()
            .map_err(|why| CookieError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        match out.status.code() {
            Some(0) | Some(NO_SUCH_ITEM) => self.list(|hosts| hosts.retain(|one| one.host != host)),
            _ => Err(CookieError::Unkept(format!(
                "the Keychain would not delete the header kept for {host}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }

    fn hosts(&self) -> Result<Vec<KeptHost>, CookieError> {
        self.listed()
    }
}
