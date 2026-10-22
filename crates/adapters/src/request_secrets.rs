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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use adapter_traits::{RequestSecrets, Secret, SecretError, SecretVariable};

/// The Keychain's own tool.
const SECURITY: &str = "/usr/bin/security";
/// The service every item is kept under; the variable is its account.
pub const SECRET_SERVICE: &str = "armada-request-variable";
/// `security`'s exit when there is no such item.
const NO_SUCH_ITEM: i32 = 44;

/// The secret variables, in a keychain.
pub struct KeychainSecrets {
    /// `None` is the person's default keychain. A test names a throwaway one.
    keychain: Option<PathBuf>,
}

/// The item's account. Not secret.
fn account_of(variable: &SecretVariable) -> String {
    format!("{}/{}/{}", variable.repository, variable.environment, variable.name)
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

impl KeychainSecrets {
    /// The person's default keychain.
    pub fn in_default_keychain() -> KeychainSecrets {
        KeychainSecrets { keychain: None }
    }

    /// The keychain at `keychain`, for a test that must never touch the person's own.
    pub fn in_keychain(keychain: &Path) -> KeychainSecrets {
        KeychainSecrets { keychain: Some(keychain.to_path_buf()) }
    }

    fn security(&self, verb: &str, account: &str, reading: bool) -> Command {
        let mut command = Command::new(SECURITY);
        command.args([verb, "-s", SECRET_SERVICE, "-a", account]);
        if reading {
            command.arg("-w");
        }
        if let Some(keychain) = &self.keychain {
            command.arg(keychain);
        }
        command.stdin(Stdio::null());
        command
    }

    fn found(&self, account: &str, reading: bool) -> Result<Option<Vec<u8>>, SecretError> {
        let out = self
            .security("find-generic-password", account, reading)
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
}

impl RequestSecrets for KeychainSecrets {
    fn read(&self, variable: &SecretVariable) -> Result<Option<Secret<String>>, SecretError> {
        let account = account_of(variable);
        let Some(bytes) = self.found(&account, true)? else { return Ok(None) };
        let mut value =
            String::from_utf8(bytes).map_err(|_| SecretError::Unkept(format!("the value kept for {account} is not text")))?;
        if value.ends_with('\n') {
            value.pop();
        }
        Ok(Some(Secret::new(value)))
    }

    fn is_kept(&self, variable: &SecretVariable) -> Result<bool, SecretError> {
        // Without `-w` the item's attributes are printed and its value is not.
        Ok(self.found(&account_of(variable), false)?.is_some())
    }

    fn keep(&self, variable: &SecretVariable, value: Secret<String>) -> Result<(), SecretError> {
        value_fits(value.expose())?;
        let account = account_of(variable);
        if account.contains(['\n', '\r']) {
            return Err(SecretError::Refused(String::from("a variable's name is one line")));
        }
        let mut line = format!(
            "add-generic-password -U -s {SECRET_SERVICE} -a {} -X {}",
            quoted(&account),
            hex(value.expose().as_bytes())
        );
        if let Some(keychain) = &self.keychain {
            line.push(' ');
            line.push_str(&quoted(&keychain.to_string_lossy()));
        }
        line.push('\n');
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        let wrote = child.stdin.take().map(|mut stdin| stdin.write_all(line.as_bytes()));
        drop(line);
        let status =
            child.wait().map_err(|why| SecretError::Unkept(format!("{SECURITY} did not finish: {why}")))?;
        // `security -i` can exit 0 on a line that failed, so what was written is read back.
        let read_back = match (wrote, status.success()) {
            (Some(Ok(())), true) => self.read(variable)?,
            _ => None,
        };
        match read_back.as_ref().map(|read| read.expose()) == Some(value.expose()) {
            true => Ok(()),
            false => Err(SecretError::Unkept(format!("the Keychain did not keep {account}"))),
        }
    }

    fn forget(&self, variable: &SecretVariable) -> Result<(), SecretError> {
        let account = account_of(variable);
        let out = self
            .security("delete-generic-password", &account, false)
            .output()
            .map_err(|why| SecretError::Unkept(format!("{SECURITY} would not start: {why}")))?;
        match out.status.code() {
            Some(0) | Some(NO_SUCH_ITEM) => Ok(()),
            _ => Err(SecretError::Unkept(format!(
                "the Keychain would not delete {account}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
        }
    }
}
