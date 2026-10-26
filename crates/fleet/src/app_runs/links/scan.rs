//! What a folder's own config files say about ports: where it expects a sibling, and where it
//! listens itself. **Read, never run, and no parser**: a port is a number after a name or a host,
//! and a few kinds of text carry one (`vite.config.*`, `.env*`, `package.json`, `next.config.*`,
//! a webpack dev server's proxy). A file it cannot read says nothing.

use std::path::Path;

/// The files looked at in a process's folder, besides every `.env*`.
const CONFIGS: &[&str] = &[
    "vite.config.js",
    "vite.config.ts",
    "vite.config.mjs",
    "vite.config.mts",
    "vite.config.cjs",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "webpack.config.js",
    "webpack.config.ts",
    "webpack.config.cjs",
    "package.json",
    "setupProxy.js",
    "src/setupProxy.js",
];

/// The most of a file that is read: a config file is small, and a bundle is not one.
const MOST_BYTES: u64 = 256 * 1024;

/// A file in a folder and what it holds.
pub(super) struct Config {
    /// Its name under the folder, `src/setupProxy.js` for a nested one.
    pub(super) name: String,
    pub(super) text: String,
}

/// The config files of `folder`, in a fixed order.
pub(super) fn configs_in(folder: &Path) -> Vec<Config> {
    let mut names: Vec<String> = CONFIGS.iter().map(|name| name.to_string()).collect();
    if let Ok(listing) = std::fs::read_dir(folder) {
        let mut envs: Vec<String> = listing
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".env"))
            .collect();
        envs.sort();
        names.extend(envs);
    }
    names
        .into_iter()
        .filter_map(|name| {
            let path = folder.join(&name);
            std::fs::metadata(&path).ok().filter(|meta| meta.is_file() && meta.len() <= MOST_BYTES)?;
            Some(Config { text: std::fs::read_to_string(&path).ok()?, name })
        })
        .collect()
}

fn usable(port: u32) -> Option<u16> {
    u16::try_from(port).ok().filter(|port| *port >= 1024)
}

/// The digits that start `text`, as a port a server could be leased, and none when more digits follow.
fn number_at(text: &str) -> Option<u16> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 5 {
        return None;
    }
    usable(digits.parse().ok()?)
}

/// Every port written after `localhost:`, `127.0.0.1:` or `0.0.0.0:`: **where this file expects
/// something to listen**, in the order they are written.
pub(super) fn expected_in(text: &str) -> Vec<u16> {
    let mut out: Vec<(usize, u16)> = Vec::new();
    for host in ["localhost:", "127.0.0.1:", "0.0.0.0:"] {
        for (at, _) in text.match_indices(host) {
            if let Some(port) = number_at(&text[at + host.len()..]) {
                out.push((at, port));
            }
        }
    }
    out.sort();
    out.into_iter().map(|(_, port)| port).collect()
}

/// Every port this text says the program itself listens on: `PORT=8731`, `port: 8731`,
/// `--port 8731`, `--port=8731` and `-p 8731`.
pub(super) fn listens_in(text: &str) -> Vec<u16> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    for (at, _) in lower.match_indices("port") {
        let before = lower[..at].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let rest = lower[at + 4..].trim_start_matches(['"', '\'', ' ', '\t', '=', ':']);
        if rest.len() < lower.len() - at - 4 {
            out.extend(number_at(rest));
        }
    }
    for (at, _) in text.match_indices(" -p ") {
        out.extend(number_at(text[at + 4..].trim_start()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proxy_target_and_an_env_url_name_the_port_a_sibling_is_expected_on() {
        let vite = "export default { server: { proxy: { '/api': 'http://127.0.0.1:8731' } } }";
        assert_eq!(expected_in(vite), [8731]);
        assert_eq!(expected_in("VITE_API=http://localhost:8731/v1\nOTHER=ws://0.0.0.0:9100"), [8731, 9100]);
        assert!(expected_in("host: example.com:8731 and http://localhost/ and localhost:80").is_empty());
    }

    #[test]
    fn what_a_program_says_it_listens_on_is_read_from_env_flags_and_config() {
        assert_eq!(listens_in("PORT=8731\n"), [8731]);
        assert_eq!(listens_in("server: { port: 5173 }"), [5173]);
        assert_eq!(listens_in("uvicorn app:app --port 8731 --host x"), [8731]);
        assert_eq!(listens_in("vite --port=5173"), [5173]);
        assert_eq!(listens_in("serve -p 4000 ."), [4000]);
        assert!(listens_in("the report 8731 and a portal 9000").is_empty());
    }
}
