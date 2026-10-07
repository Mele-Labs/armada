# List files

A list file is one where every line stands alone and a change is a line added:
an operation table, an icon registry, a module list. Two branches that each add
a line to one conflict on the last line, however unrelated the lines are. On
14 Sep to 3 Oct that was `packages/components/src/index.ts` 35 times,
the operation table 21 and `crates/fleet/src/tests/mod.rs` 11 (#1059).

**A list is a directory with one file an entry.** Two branches adding two
entries touch two files, and nothing merges. This replaces the `merge=union`
lines of 2 Oct 2026, which kept both sides of a merge in git and which a pull
request ignores: measured on a scratch repository, 6 Oct 2026, GitHub reports a
conflict where git's union merge would not have.
`.claude/decisions/2026-10-06-ci-and-pull-requests-replace-the-merge-line.md`.

## The lists

| List | An entry is | Read by |
|---|---|---|
| `crates/ipc/operations/` | `<operation>.toml` holding `[operations.<operation>]`; a dotted event name is the file name, `job.created.toml`. `_header.toml` says what the fields are | `crates/ipc/build.rs`, the protocol rules in `xtask`, and `fleet`'s Helm test |
| `packages/icons/icons/` | `<glyph>.toml` holding `[icons.<glyph>]` and every `[[icons.<glyph>.usage]]` of it. `_header.toml` says what the fields are | the icon and action rules in `xtask` |
| `packages/icons/conventions/` | `<rule>.toml` holding `[conventions.<rule>]` | the same |
| `apps/desktop/src/renderer/src/mock/scenarios/` | A file exporting one `Scenario`; `scenario.ts` reads the directory with `import.meta.glob` | `scenario.ts` |
| `crates/fleet/src/tests/` | A `<module>.rs`; `crates/fleet/build.rs` writes `mods.inc`, which git ignores, and `mod.rs` includes it | `crates/fleet/build.rs` |

**The key is the file's name.** `cargo xtask verify-foundations` (*every list
entry is one file named for its key*) fails a file that holds two tables or a
table its name does not give: that is the old shared file again under a new
path. It fails a `merge=union` line too, for what it no longer does.

A reader that wants the whole list reads the directory in name order and splits
on the table header, so a parser written for the one file works on the
directory. A line number a rule cites for a registry is a line of that
concatenation, not of a file: open the entry by its key.

## Adding an entry

Add the file. Nothing else lists it.

- **An operation, an icon, a convention:** a file named for the key, with the
  table in it.
- **A mock scenario's row:** a file in `mock/scenarios/` exporting one `Scenario`,
  built with `holding` from `../holding`. The export name must be unique, or the
  glob keeps one and drops the other, and `scenario.test.ts` fails where a name
  repeats. `scenario.ts` sorts the rows by export name. The seventeen rows that
  were in `scenario.ts` carry `s010`–`s170` ahead of their names so they list
  exactly as before, and the test pins them; a new row is named for what it is
  and sorts by that name among them.
- **A fleet test module:** a `.rs` file in `crates/fleet/src/tests/`. A module
  `mod.rs` declares itself, for the few that are `pub(crate)`, is left to it.

## Not a directory, and why

| File | Why not | What would make it one |
|---|---|---|
| `packages/components/src/index.ts` | A barrel. TypeScript has no glob export, so each component is a line someone writes, and a branch adding one conflicts with another doing the same. 35 conflicts in three weeks | Importing a component by its own path, `@armada/components/<Name>`, which gives up the one curated entry |
| `packages/components/src/index.css` | Ends with a `.armada-helm-dock` rule, and a later `@import` after it | Move the rule to its own stylesheet and import it, leaving imports only. A visual change, so it waits for a walk |
| `crates/ipc/src/lib.rs` | `pub use` re-exports are rustfmt-wrapped multi-line statements, not lines, and `#[cfg(test)] mod tests;` sits between the two lists | One `pub use m::*;` a module, which gives up the curated export surface |
| `packages/protocol/src/pending.ts` | Deletions: two branches deleting different entries conflict, and a directory does not change that | None; it is edited by removing |

## Limits

- **A directory removes the conflict on adding, not on editing.** Two branches
  editing the same entry still conflict, as they should.
- **`mods.inc` is written by a build.** A checkout has none until `crates/fleet`
  has been built once, and a rule that reads `mod.rs` for a declaration of
  every test file (`every test file under tests/ is declared`) trusts the
  include instead.

## Checked by

`cargo xtask verify-foundations`, the rule above, and the suites that read each
list: `crates/ipc`'s build, `xtask`'s protocol and icon rules, `scenario.test.ts`.
