//! Requests' secret variables in the Keychain: [`KeychainSecrets`], one generic-password item per
//! variable under one service, its account `<repository key>/<environment>/<name>`, reached through
//! `/usr/bin/security`.
//!
//! **Through `security`, not the Security framework**, because an item trusts the program that
//! wrote it, and `security` does not change when Fleet is rebuilt
//! (`docs/spikes/034-can-fleet-keep-a-cookie-in-the-keychain.md`). **A value is written on
//! `security -i`'s standard input as hex**, so it is never in a process's arguments for `ps` to
//! read. No error here carries a value, nor `security`'s own output from a write, which can echo
//! the line it was given.
//!
//! **`security -i` reads 4095 bytes a line** and runs the rest as another command, so a value
//! longer than [`PART`] is kept in parts: the first in the variable's own item, each after it under
//! [`SECRET_PART_SERVICE`] at `<account>/<n>`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use adapter_traits::{RequestSecrets, Secret, SecretError, SecretVariable};

/// The Keychain's own tool.
const SECURITY: &str = "/usr/bin/security";
/// The service every item is kept under; the variable is its account.
pub const SECRET_SERVICE: &str = "armada-request-variable";
/// The service a long value's later parts are kept under, apart from any variable's account.
pub const SECRET_PART_SERVICE: &str = "armada-request-variable-part";
/// `security`'s exit when there is no such item.
const NO_SUCH_ITEM: i32 = 44;
/// The longest line `security -i` reads whole, its newline included.
const LINE_MOST: usize = 4095;
/// A part's bytes: twice this in hex leaves the line a thousand bytes for its account and keychain.
const PART: usize = 1500;

/// The secret variables, in a keychain.
pub struct KeychainSecrets {
    /// `None` is the person's default keychain. A test names a throwaway one.
    keychain: Option<PathBuf>,
}

/// The item's account. Not secret.
fn account_of(variable: &SecretVariable) -> String {
    format!("{}/{}/{}", variable.repository, variable.environment, variable.name)
}

/// Where part `index` of `account`'s value is kept: the first in the variable's own item.
fn part_of(account: &str, index: usize) -> (&'static str, String) {
    match index {
        0 => (SECRET_SERVICE, account.to_string()),
        _ => (SECRET_PART_SERVICE, format!("{account}/{index}")),
    }
}

/// `text` as one argument on `security -i`'s line, which splits on spaces and reads `"` and `\`.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// One line of printable ASCII, which a header can carry and `security -w` reads back as it was
/// written.
fn value_fits(value: &str) -> Result<(), SecretError> {
    match !value.is_empty() && value.bytes().all(|byte| (0x20..0x7f).contains(&byte)) {
        true => Ok(()),
        false => Err(SecretError::Refused(String::from("a secret's value is one line of printable ASCII"))),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The lines `security -i` is given to keep `value` under `account`, one a part, each ending in
/// `\n` and none longer than it reads whole.
pub(crate) fn keep_lines(account: &str, value: &str, keychain: Option<&Path>) -> Result<Vec<String>, SecretError> {
    let mut lines = Vec::new();
    for (index, part) in value.as_bytes().chunks(PART).enumerate() {
        let (service, held) = part_of(account, index);
        let mut line = format!("add-generic-password -U -s {service} -a {} -X {}", quoted(&held), hex(part));
        if let Some(keychain) = keychain {
            line.push(' ');
            line.push_str(&quoted(&keychain.to_string_lossy()));
        }
        line.push('\n');
        if line.len() > LINE_MOST {
            return Err(SecretError::Refused(String::from(
                "the variable's name and the keychain's path are too long for security to be given its value",
            )));
        }
        lines.push(line);
    }
    Ok(lines)
}

/// What `security -i` said of a failed write: the `returned <status>` it prints after a command and
/// the Security framework's message. No other line is read, since an unknown command is echoed whole.
pub(crate) fn status_of(said: &str) -> String {
    let mut status = None;
    let mut message = None;
    for line in said.lines() {
        if let Some(code) = line.strip_prefix("add-generic-password: returned ") {
            status = status.or(code.trim().parse::<i32>().ok().filter(|code| *code != 0));
        }
        if line.starts_with("security: Sec") {
            message = message.or(line.split_once("): ").map(|(_, words)| words.trim()));
        }
    }
    match (status, message) {
        (Some(status), Some(message)) => format!("OSStatus {status}, {message}"),
        (Some(status), None) => format!("OSStatus {status}"),
        (None, Some(message)) => message.to_string(),
        (None, None) => String::from("security named no status"),
    }
}

impl KeychainSecrets {
    /// The person's default keychain.
    pub fn in_default_keychain() -> KeychainSecrets {
        KeychainSecrets { keychain: None }
    }

    /// The keychain at `keychain`, for a test that must never touch the person's own.
    pub fn in_keychain(keychain: &Path) -> KeychainSecrets {
        KeychainSecrets { keychain: Some(keychain.to_path_buf()) }
    }

    fn security(&self, verb: &str, (service, account): (&str, &str), reading: bool) -> Command {
        let mut command = Command::new(SECURITY);
        command.args([verb, "-s", service, "-a", account]);
        if reading {
            command.arg("-w");
        }
        if let Some(keychain) = &self.keychain {
            command.arg(keychain);
        }
        command.stdin(Stdio::null());
        command
    }

    fn found(&self, (service, account): (&str, &str), reading: bool) -> Result<Option<Vec<u8>>, SecretError> {
        let out = self
            .security("find-generic-password", (service, account), reading)
            .output()
            .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        match out.status.code() {
            Some(0) => Ok(Some(out.stdout)),
            Some(NO_SUCH_ITEM) => Ok(None),
            _ => Err(SecretError::Unkept(format!(
                "the Keychain would not read {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }

    /// Delete `account`'s parts from `from` on, until one is not there.
    fn forget_parts(&self, account: &str, from: usize) -> Result<(), SecretError> {
        for index in from.max(1).. {
            let (service, held) = part_of(account, index);
            let out = self
                .security("delete-generic-password", (service, &held), false)
                .output()
                .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
            match out.status.code() {
                Some(0) => continue,
                Some(NO_SUCH_ITEM) => break,
                _ => {
                    return Err(SecretError::Unkept(format!(
                        "the Keychain would not delete {held}: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    )));
                }
            }
        }
        Ok(())
    }

    /// The lines given to `security -i`, and what it said, for [`status_of`] alone to read.
    fn interactive(&self, lines: String) -> Result<String, SecretError> {
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            // A write that fails is found when the value is read back.
            let _ = stdin.write_all(lines.as_bytes());
        }
        drop(lines);
        let out =
            child.wait_with_output().map_err(|why| SecretError::Unkept(format!("{SECURITY} did not finish: {why}")))?;
        Ok(format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
    }
}

/// `security -w`'s output without the newline it ends with.
fn without_newline(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    bytes
}

impl RequestSecrets for KeychainSecrets {
    fn read(&self, variable: &SecretVariable) -> Result<Option<Secret<String>>, SecretError> {
        let account = account_of(variable);
        let Some(first) = self.found((SECRET_SERVICE, &account), true)? else { return Ok(None) };
        let mut bytes = without_newline(first);
        for index in 1.. {
            let (service, held) = part_of(&account, index);
            let Some(part) = self.found((service, &held), true)? else { break };
            bytes.extend(without_newline(part));
        }
        let value =
            String::from_utf8(bytes).map_err(|_| SecretError::Unkept(format!("the value kept for {account} is not text")))?;
        Ok(Some(Secret::new(value)))
    }

    fn is_kept(&self, variable: &SecretVariable) -> Result<bool, SecretError> {
        // Without `-w` the item's attributes are printed and its value is not.
        Ok(self.found((SECRET_SERVICE, &account_of(variable)), false)?.is_some())
    }

    fn keep(&self, variable: &SecretVariable, value: Secret<String>) -> Result<(), SecretError> {
        value_fits(value.expose())?;
        let account = account_of(variable);
        if account.contains(['\n', '\r']) {
            return Err(SecretError::Refused(String::from("a variable's name is one line")));
        }
        let lines = keep_lines(&account, value.expose(), self.keychain.as_deref())?;
        let parts = lines.len();
        let said = self.interactive(lines.concat())?;
        // A longer value kept before leaves parts past this one's, which would be read as its end.
        let stale = self.forget_parts(&account, parts);
        // `security -i` can exit 0 on a line that failed, so what was written is read back.
        let read_back = match stale {
            Ok(()) => self.read(variable).ok().flatten(),
            Err(_) => None,
        };
        if read_back.as_ref().map(|read| read.expose()) == Some(value.expose()) {
            return Ok(());
        }
        // A value kept in part is never sent, so none is left.
        let _ = self.forget(variable);
        Err(SecretError::Unkept(format!("the Keychain did not keep {account}: {}", status_of(&said))))
    }

    fn forget(&self, variable: &SecretVariable) -> Result<(), SecretError> {
        let account = account_of(variable);
        let out = self
            .security("delete-generic-password", (SECRET_SERVICE, &account), false)
            .output()
            .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        match out.status.code() {
            Some(0) | Some(NO_SUCH_ITEM) => self.forget_parts(&account, 1),
            _ => Err(SecretError::Unkept(format!(
                "the Keychain would not delete {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }
}
