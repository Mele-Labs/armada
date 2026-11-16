//! The Cloudflare token the person pasted for an account, one generic-password item each under
//! [`CLOUDFLARE_TOKEN_SERVICE`], its account the Cloudflare account's id, reached through
//! `/usr/bin/security` for the reasons `request_secrets` gives. A token is written on `security -i`'s
//! standard input, so it is never in a process's arguments.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use adapter_traits::{CloudflareTokens, Secret};

const SECURITY: &str = "/usr/bin/security";
/// The service every token is kept under.
pub const CLOUDFLARE_TOKEN_SERVICE: &str = "armada-cloudflare-token";
/// `security`'s exit when there is no such item.
const NO_SUCH_ITEM: i32 = 44;

/// The tokens, in the person's default keychain.
pub struct KeychainCloudflareTokens {
    /// `None` is the person's default keychain. A test names a throwaway one.
    keychain: Option<PathBuf>,
}

/// One argument on `security -i`'s line, which splits on spaces and reads `"` and `\`.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// An account is a Cloudflare account id, 32 hex digits.
fn plain(account: &str) -> Result<(), String> {
    match !account.is_empty() && account.len() <= 64 && account.bytes().all(|b| b.is_ascii_alphanumeric()) {
        true => Ok(()),
        false => Err(String::from("A Cloudflare account id is letters and digits.")),
    }
}

/// The line `security -i` is given to keep `token` for `account`, ending in `\n`.
pub(crate) fn keep_line(account: &str, token: &str, keychain: Option<&Path>) -> String {
    let mut line = format!("add-generic-password -U -s {CLOUDFLARE_TOKEN_SERVICE} -a {} -X {}", quoted(account), hex(token.as_bytes()));
    if let Some(keychain) = keychain {
        line.push(' ');
        line.push_str(&quoted(&keychain.to_string_lossy()));
    }
    line.push('\n');
    line
}

impl KeychainCloudflareTokens {
    /// The person's default keychain.
    pub fn new() -> KeychainCloudflareTokens {
        KeychainCloudflareTokens { keychain: None }
    }

    /// The keychain at `keychain`, for a test that must never touch the person's own.
    pub fn in_keychain(keychain: &Path) -> KeychainCloudflareTokens {
        KeychainCloudflareTokens { keychain: Some(keychain.to_path_buf()) }
    }

    fn security(&self, verb: &str, account: &str, reading: bool) -> Command {
        let mut command = Command::new(SECURITY);
        command.args([verb, "-s", CLOUDFLARE_TOKEN_SERVICE, "-a", account]);
        if reading {
            command.arg("-w");
        }
        if let Some(keychain) = &self.keychain {
            command.arg(keychain);
        }
        command.stdin(Stdio::null());
        command
    }
}

impl Default for KeychainCloudflareTokens {
    fn default() -> Self {
        KeychainCloudflareTokens::new()
    }
}

impl CloudflareTokens for KeychainCloudflareTokens {
    fn keep(&self, account: &str, token: Secret<String>) -> Result<(), String> {
        plain(account)?;
        let line = keep_line(account, token.expose(), self.keychain.as_deref());
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            // A write that fails is found when the token is read back.
            let _ = stdin.write_all(line.as_bytes());
        }
        drop(line);
        child.wait().map_err(|why| format!("{SECURITY} did not finish: {why}"))?;
        // `security -i` can exit 0 on a line that failed, so what was written is read back.
        match self.read(account)? {
            Some(read) if read.expose() == token.expose() => Ok(()),
            _ => Err(format!("The Keychain did not keep the Cloudflare token for account {account}.")),
        }
    }

    fn read(&self, account: &str) -> Result<Option<Secret<String>>, String> {
        plain(account)?;
        let out = self
            .security("find-generic-password", account, true)
            .output()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        match out.status.code() {
            Some(0) => {
                let text = String::from_utf8(out.stdout).map_err(|_| String::from("The token kept in the Keychain is not text."))?;
                Ok(Some(Secret::new(text.trim_end_matches('\n').to_string())))
            }
            Some(NO_SUCH_ITEM) => Ok(None),
            _ => Err(format!(
                "The Keychain would not read the Cloudflare token for account {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        }
    }

    fn forget(&self, account: &str) -> Result<(), String> {
        plain(account)?;
        let out = self
            .security("delete-generic-password", account, false)
            .output()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        match out.status.code() {
            Some(0) | Some(NO_SUCH_ITEM) => Ok(()),
            _ => Err(format!(
                "The Keychain would not delete the Cloudflare token for account {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        }
    }
}
