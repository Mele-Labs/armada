//! Emit the protocol ID, and the operations an agent may reach from `operations/`.
//!
//! The ID is a hash of the wire surface, so two branches that each change the
//! wire cannot conflict on a number: they change different files and the ID
//! follows. `docs/practices/protocol.md` says what the surface is and what the
//! hash does not see. `apps/desktop/codegen/protocol-id.mjs` computes the same
//! hash for Bridge, and a rule in `xtask` holds the two together.
//!
//! The agent's tool set is emitted here for a second reason. A list of tool
//! names written in `api` would be a second copy of the `agent_access` column,
//! and a second copy is how a tool gets added to the inventory and stays
//! unreachable.
//!
//! No dependencies, and no TOML parser. Adding either would put a build-time
//! dependency underneath the whole workspace.

use std::fs;
use std::path::{Path, PathBuf};

// `protocol_id` and its helper, in a file of their own so `xtask` can run the
// same code and compare it with the Node copy Bridge is built with.
include!("wire_hash.rs");

fn main() {
    // Read at run time, never `env!`: that bakes the path of whichever tree
    // compiled this script into the binary, and a target directory carried to
    // another worktree then reads the old tree's file.
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/ipc sits two levels below the repo root")
        .to_path_buf();

    // Each directory is named relative to the package, so the fingerprint a
    // target directory keeps names the tree it is used in. An absolute path
    // pinned it to the tree that built it.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=operations");
    println!("cargo:rerun-if-changed=../../packages/protocol/src");
    println!("cargo:rerun-if-changed=../api/src/routes");

    let id = protocol_id(&root);
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    fs::write(
        out.join("protocol_id.rs"),
        format!("const PROTOCOL_ID_HASH: &str = \"{id}\";\n"),
    )
    .expect("OUT_DIR is writable");

    // One file an operation, so two branches adding two operations touch two
    // files: a pull request is merged by the forge, which ignores `.gitattributes`.
    let inventory = manifest.join("operations");
    let named = read_operations(&inventory);
    fs::write(out.join("reachable.rs"), reachable(&named, &inventory))
        .expect("OUT_DIR is writable");
}

/// Every `operations/*.toml`, in name order, as the one text [`reachable`]
/// splits on `\n[operations.`. The directory's mtime changes when an entry is
/// added or removed; an edit inside one is each file's own, so they are named.
fn read_operations(dir: &Path) -> String {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| {
            panic!(
                "{} is the authority on which operations an agent may reach: {e}",
                dir.display()
            )
        })
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    let mut text = String::from("\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let body = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()));
        text.push_str(body.trim_end());
        text.push_str("\n\n");
    }
    text
}

/// Every row whose `agent_access` is `Yes`, as a table, every row reading
/// `Drafts only` as a second, and every row reading `Helm only` as a third.
///
/// **Three tables, never one with a flag.** `REACHABLE` is what the door offers
/// any agent; `DRAFTING` is offered to a Helm session alone for drafting a Job
/// (`#941`); `HELM_ONLY` is offered to a Helm session alone for everything else
/// a person may ask it to do (`#1150`) — so a reader that forgot a flag could
/// not hand any of the three to a caller it was not written for.
fn reachable(inventory: &str, source: &Path) -> String {
    let mut rows = String::new();
    let mut drafting = String::new();
    let mut helm_only = String::new();
    let mut found = 0usize;
    for block in inventory.split("\n[operations.") {
        let Some((header, body)) = block.split_once(']') else {
            continue;
        };
        let name = header.trim().trim_matches('"');
        // The text before the first table is the file's own header, and it
        // reaches here as a "name" with a paragraph in it.
        if name.is_empty() || name.contains(char::is_whitespace) {
            continue;
        }
        let into = match field(body, "agent_access").as_deref() {
            Some("Yes") => {
                found += 1;
                &mut rows
            }
            Some("Drafts only") => &mut drafting,
            Some("Helm only") => &mut helm_only,
            _ => continue,
        };
        let kind = field(body, "kind").unwrap_or_default();
        let description = field(body, "description").unwrap_or_default();
        into.push_str(&format!(
            "    Reachable {{ operation: {}, kind: {}, description: {} }},\n",
            quoted(name),
            quoted(&kind),
            quoted(&description),
        ));
    }
    // A parser that matched nothing would emit an empty tool set, and an empty
    // tool set is a door that opens onto nothing while every gate stays green.
    assert!(
        found > 0,
        "{} named no operation with `agent_access = \"Yes\"`, which has never been true — \
         either the file's shape changed or this parser no longer matches it",
        source.display()
    );
    format!(
        "pub const REACHABLE: &[Reachable] = &[\n{rows}];\n\
         pub const DRAFTING: &[Reachable] = &[\n{drafting}];\n\
         pub const HELM_ONLY: &[Reachable] = &[\n{helm_only}];\n"
    )
}

/// One `key = "value"` out of a block, ignoring the `\"\"\"` prose fields.
fn field(block: &str, key: &str) -> Option<String> {
    block
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|rest| rest.trim().trim_matches('"').to_string())
}

/// A Rust string literal. The inventory holds backticks and apostrophes and no
/// escapes, but a literal built by concatenation has to survive one arriving.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
