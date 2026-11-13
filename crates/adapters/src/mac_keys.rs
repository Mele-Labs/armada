//! The private keys the Phone Gateway signs with toward the owner's other Macs, one generic-password
//! item each under [`MAC_KEY_SERVICE`], its account the other Mac's id, reached through
//! `/usr/bin/security` for the reasons `request_secrets` gives. A key is written on `security -i`'s
//! standard input, so it is never in a process's arguments.

use std::io::Write;
use std::process::{Command, Stdio};

use adapter_traits::Secret;

const SECURITY: &str = "/usr/bin/security";
/// The service every key is kept under.
pub const MAC_KEY_SERVICE: &str = "armada-mac-key";
/// `security`'s exit when there is no such item.
const NO_SUCH_ITEM: i32 = 44;

/// The keys, in the person's default keychain.
pub struct KeychainMacKeys;

/// One argument on `security -i`'s line, which splits on spaces and reads `"` and `\`.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// An account is a Mac's id, which its Gateway minted as hex.
fn plain(account: &str) -> Result<(), String> {
    match !account.is_empty() && account.len() <= 64 && account.bytes().all(|b| b.is_ascii_alphanumeric()) {
        true => Ok(()),
        false => Err(String::from("A Mac's id is letters and digits.")),
    }
}

impl KeychainMacKeys {
    fn security(verb: &str, account: &str, reading: bool) -> Command {
        let mut command = Command::new(SECURITY);
        command.args([verb, "-s", MAC_KEY_SERVICE, "-a", account]);
        if reading {
            command.arg("-w");
        }
        command.stdin(Stdio::null());
        command
    }

    pub fn keep(&self, account: &str, key: Secret<String>) -> Result<(), String> {
        plain(account)?;
        let line = format!(
            "add-generic-password -U -s {MAC_KEY_SERVICE} -a {} -X {}\n",
            quoted(account),
            hex(key.expose().as_bytes())
        );
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            // A write that fails is found when the key is read back.
            let _ = stdin.write_all(line.as_bytes());
        }
        drop(line);
        child.wait().map_err(|why| format!("{SECURITY} did not finish: {why}"))?;
        // `security -i` can exit 0 on a line that failed, so what was written is read back.
        match self.read(account)? {
            Some(read) if read.expose() == key.expose() => Ok(()),
            _ => Err(format!("The Keychain did not keep this Mac's key for Mac {account}.")),
        }
    }

    pub fn read(&self, account: &str) -> Result<Option<Secret<String>>, String> {
        plain(account)?;
        let out = Self::security("find-generic-password", account, true)
            .output()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        match out.status.code() {
            Some(0) => {
                let text = String::from_utf8(out.stdout).map_err(|_| String::from("The key kept in the Keychain is not text."))?;
                Ok(Some(Secret::new(text.trim_end_matches('\n').to_string())))
            }
            Some(NO_SUCH_ITEM) => Ok(None),
            _ => Err(format!(
                "The Keychain would not read this Mac's key for Mac {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        }
    }

    pub fn forget(&self, account: &str) -> Result<(), String> {
        plain(account)?;
        let out = Self::security("delete-generic-password", account, false)
            .output()
            .map_err(|why| format!("{SECURITY} would not start: {why}"))?;
        match out.status.code() {
            Some(0) | Some(NO_SUCH_ITEM) => Ok(()),
            _ => Err(format!(
                "The Keychain would not delete this Mac's key for Mac {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        }
    }
}
