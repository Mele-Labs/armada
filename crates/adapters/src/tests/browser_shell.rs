//! Which headless shell a Job is told about, asserted on a temp folder. Nothing is installed.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use adapter_traits::Environment;

use crate::browser_shell::{environment, install_if_missing, newest_in, Install};

fn root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("browser-shell-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp folder");
    dir
}

fn shell(root: &Path, number: u64, arch: &str) -> PathBuf {
    let executable = root.join(format!("chromium_headless_shell-{number}/chrome-headless-shell-{arch}/chrome-headless-shell"));
    std::fs::create_dir_all(executable.parent().expect("a folder")).expect("folders");
    std::fs::write(&executable, "").expect("a file");
    executable
}

fn value(environment: &Environment, name: &str) -> Option<String> {
    environment.vars().iter().find(|(held, _)| held == name).map(|(_, value)| value.clone())
}

#[test]
fn a_job_is_told_the_shell_when_one_exists() {
    let dir = root("one");
    let one = shell(&dir, 1234, "mac-arm64");
    let told = environment(Environment::nothing(), newest_in(&dir).as_deref()).expect("legal names");
    assert_eq!(value(&told, "PLAYWRIGHT_MCP_BROWSER").as_deref(), Some("chromium"));
    assert_eq!(value(&told, "PLAYWRIGHT_MCP_HEADLESS").as_deref(), Some("true"));
    assert_eq!(value(&told, "PLAYWRIGHT_MCP_EXECUTABLE_PATH"), Some(one.to_string_lossy().into_owned()));
}

#[test]
fn the_newest_of_two_is_the_one_named() {
    let dir = root("two");
    shell(&dir, 999, "mac-arm64");
    let newer = shell(&dir, 1234, "mac-x64");
    assert_eq!(newest_in(&dir), Some(newer));
}

#[test]
fn a_folder_without_the_executable_is_passed_over() {
    let dir = root("hollow");
    let older = shell(&dir, 999, "mac-arm64");
    std::fs::create_dir_all(dir.join("chromium_headless_shell-1500")).expect("a folder");
    std::fs::create_dir_all(dir.join("chromium-2000")).expect("a folder");
    assert_eq!(newest_in(&dir), Some(older));
}

#[test]
fn a_job_is_told_nothing_where_none_is_found() {
    let dir = root("none");
    assert_eq!(newest_in(&dir), None);
    assert_eq!(newest_in(&dir.join("absent")), None);
    let told = environment(Environment::nothing(), newest_in(&dir).as_deref()).expect("legal names");
    assert_eq!(told, Environment::nothing());
}

#[test]
fn the_install_is_asked_for_only_when_none_is_found() {
    let dir = root("install");
    let asked = Cell::new(0);
    let installing = || {
        asked.set(asked.get() + 1);
        shell(&dir, 1234, "mac-arm64");
        true
    };
    assert_eq!(install_if_missing(&dir, installing), Install::Installed);
    assert_eq!(asked.get(), 1);
    assert_eq!(install_if_missing(&dir, || panic!("a shell is there")), Install::NotNeeded);
}

#[test]
fn an_install_that_leaves_no_shell_is_a_failure() {
    let dir = root("failed");
    assert_eq!(install_if_missing(&dir, || true), Install::Failed);
    assert_eq!(install_if_missing(&dir, || false), Install::Failed);
}
