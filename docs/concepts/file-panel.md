# File panel

**What it is:** Any file Bridge opens, opened in the one side panel in a code editor where the person edits and saves it.

---

**Kind:** Concept.

A file opens in the side panel as an item of kind `file` (`apps/desktop/src/renderer/src/shared-panel/store.ts`).
It shows in a code editor; edits are kept, and ⌘S saves.
Why CodeMirror and not the text box the Manifest file view uses: `.claude/decisions/2026-10-19-codemirror-is-bridges-one-editor-dependency.md`.

## Where a file comes from

> **Rule.** The renderer names a panel item, never a path of its own. Main touches the path only when the source the item names lists it.
> Why: a renderer that could hand main any path could read or overwrite any file the person owns.

| Source | What main checks the path against | Opens |
|---|---|---|
| A Session's file | The Session's ledger file artifacts, `namesFile` in `apps/desktop/src/main/session-file.ts` | Yes |
| settings.json | The one path Fleet reported | Yes |
| A picked file | A file chosen in this run's ⌘O dialog | Yes |
| A Kit item | Nothing: main's Kit rows carry no path, so there is no set to belong to | Refused for now; Kit wiring follows the System sidebar PR |

An agent puts a file there with `show_file` (`crates/ipc/operations/show_file.toml`, described in `docs/concepts/session/showing-a-file.md`).
It opens when that Session is on screen; otherwise its thread row opens it.

Selected lines, or a region of a picture, can be noted and sent to a Job from the panel (`session/noting-what-you-read.md`).

## Saving

> **Rule.** Main writes only if the file's sha256 still matches what was read.
> Why: a file changed on disk since it was read is not the file the person edited.

| On disk | The panel |
|---|---|
| Unchanged since read | Writes, and clears the unsaved edit |
| Changed, and the person has not edited | Reloads it |
| Changed, and the person has edited | Writes nothing. It shows On disk beside Your edit, with Take disk and Keep mine |

Unsaved edits are kept per path across closing the panel, and cleared on save.

## What does not open for editing

Text that is not UTF-8, and a file over 8 MiB (`MOST_BYTES` in `apps/desktop/src/main/session-file.ts`), do not open for editing. A picture shows and cannot be edited.

## In their own editor

Header icons open the file in the person's own editor, or show it in Finder.
The editor is `editor.command` in settings.json, then `$VISUAL`, then `$EDITOR`, then the OS.

## Where it lives

| Part | File |
|---|---|
| Editor and panel | `packages/components/src/compositions/FilePanel/CodeEditor.tsx`, `FilePanel.tsx` |
| Main: read, save, watch, membership | `apps/desktop/src/main/file-panel.ts` |
| Main: dialog, editor, Finder | `apps/desktop/src/main/file-panel-channels.ts` |
