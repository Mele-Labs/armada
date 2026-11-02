//! What a tab mod's `tab.js` may be: one ES module, whose default export is a React component,
//! that Bridge runs as part of itself. Checked before Bridge loads it.
//!
//! **This is a check for mistakes, not a sandbox.** A tab is code Bridge loads with the whole Fleet
//! API in reach, as Armada's own tabs are, so nothing here limits what it may do. What is refused
//! is what could not run: a module Bridge cannot load, since it is loaded from text and no path
//! resolves an `import` from it. An instrument mod's `instrument.js` is the same kind of module, held to the same check.

/// The most `tab.js` or `instrument.js` may weigh.
pub(crate) const MOST_BYTES: u64 = 256 * 1024;

/// Every problem with `text`, each naming its line where it has one.
pub(crate) fn problems(text: &str) -> Vec<String> {
    problems_in(text, super::TAB, "tab")
}

/// [`problems`] for `file`, the module of what its mod calls `thing`.
pub(crate) fn problems_in(text: &str, file: &str, thing: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if text.trim().is_empty() {
        return vec![format!("{file} is empty")];
    }
    let mut exports = false;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim_start();
        if line.starts_with("export default") {
            exports = true;
        }
        if is_import(line) {
            problems.push(format!(
                "{file} line {number}: an `import` cannot be resolved, the {thing} is handed `host` instead"
            ));
        } else if line.starts_with("export") && line.contains(" from ") {
            problems.push(format!("{file} line {number}: an `export ... from` cannot be resolved"));
        }
    }
    if !exports {
        problems.push(format!("{file} has no `export default`, which must be the {thing}'s component"));
    }
    problems
}

/// A static `import`, which a module loaded from text cannot resolve. `import(` and `import.meta` are not one.
fn is_import(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("import") else { return false };
    rest.starts_with(|c: char| c.is_whitespace() || c == '{' || c == '*' || c == '"' || c == '\'')
}
