# List files

A list file is one where every line stands alone and a change is a line added:
component exports, an operation table, an icon registry, a module list. Two
branches that each add a line to one conflict on the last line, however
unrelated the lines are. On 14 Sep to 3 Oct that was `packages/components/src/index.ts`
35 times, `crates/ipc/operations.toml` 21 and `crates/fleet/src/tests/mod.rs` 11
(#1059). **Decided 2 Oct 2026: a list file keeps both sides of a merge, with no
lease.** `.claude/decisions/2026-10-02-a-plan-leases-its-numbers.md`.

## Declaring one

`.gitattributes` is the one place, a line a file: `<path> merge=union`. Nothing
else lists them. Git does the merge, so a branch needs nothing but the file.

| Declared here | Why it is safe |
|---|---|
| `packages/components/src/index.ts` | `export * from` lines only |
| `crates/ipc/operations.toml` | A header, then one `[operations.<name>]` table an entry |
| `crates/fleet/src/tests/mod.rs` | `mod` lines and their comments only |
| `packages/icons/icons.toml` | A header, then one table an entry |
| `apps/desktop/src/renderer/src/mock/scenario-rows.ts` | `export * from "./scenarios/<row>";` lines only, one a scenario. Each row, with its own imports, is a file in `scenarios/` |

`cargo xtask verify-foundations` (*every declared list file holds only entries*)
names any line of a declared file that is not an entry for its kind: an export,
an `@import`, a `mod`, a comment, a table. **Keeping both sides of code that is
not a list interleaves two edits unseen**, which is why the gate refuses it
rather than a reviewer.

## Adding a mock scenario's row

A file in `apps/desktop/src/renderer/src/mock/scenarios/` exporting one `Scenario`, built with `holding` from
`../holding`, and one line for it in `scenario-rows.ts`. **`scenario.ts` is not edited.** The export name
must be unique, or `export *` drops it without saying, and `scenario.test.ts` fails where a file or a
name is missing from the list. **Order:** `scenario.ts` sorts the rows by export name, because a union
merge orders lines by merge. The seventeen rows that were in `scenario.ts` carry `s010`–`s170` ahead of
their names so they list exactly as before, and the test pins them; a new row is named for what it is
and sorts by that name among them. Nothing the owner saw moved.

## Not declared, and what would make each safe

| File | Why not | Smallest split |
|---|---|---|
| `packages/components/src/index.css` | Ends with a `.armada-helm-dock` rule, and a later `@import` after it | Move the rule to its own stylesheet and import it, leaving imports only. A visual change, so it waits for a walk |
| `crates/ipc/src/lib.rs` | `pub use` re-exports are rustfmt-wrapped multi-line statements, not lines, and `#[cfg(test)] mod tests;` sits between the two lists | One `pub use m::*;` a module, which gives up the curated export surface, or `pub use` lines each kept to one line |
| `packages/protocol/src/pending.ts` | Deletions, below | None; it is edited by removing |

## Limits

- **Union keeps both sides of a deletion too.** Two branches deleting different
  entries put both back, so a list file is only for appends.
  `packages/protocol/src/pending.ts` conflicts when two branches each delete an
  entry, and is not declared for that reason.
- **Two edits to the same existing line keep both versions.** A repeated
  export, `mod` or TOML key fails to compile or parse. A repeated line inside a
  `notes = """` string does not, so read a conflict in a table body.
- **A branch cut before `.gitattributes` landed conflicts once.** Git reads the
  attributes from the branch being merged into, so the first merge of `main`
  into an old branch is the branch's own conflict. Measured 5 Oct 2026.
- **Only `git merge` reads them.** `armada land`'s candidate worktree and
  Fleet's `bring_up_to_date` both run plain `git merge`, so they do.
  `merge_by: forge` is GitHub's merge, which ignores `.gitattributes`.

## Checked by

`crates/armada/src/tests/list_files.rs`: two branches each append a line to the
declared file and merge one after another with no conflict, with an undeclared
control that conflicts. It runs real git, which the acceptance package may not
(*hermetic*, `docs/practices/acceptance-tests.md`).
