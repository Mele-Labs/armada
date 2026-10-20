# Does Bridge take a code editor as a dependency, when a surface builds from tokens and shadcn primitives alone?

**Decided 2026-10-19.** He asked that any file Bridge opens open in the side panel, where he edits and saves it. The Manifest file view edits in a plain mono text box (`packages/components/src/compositions/ManifestFile/ManifestFile.tsx`); he chose a code editor over it: line numbers, syntax colours, ⌘F and undo.

**Chosen: CodeMirror 6, and it is Bridge's one editor dependency.** `apps/CLAUDE.md` says a surface builds from tokens and shadcn primitives alone. This is the named exception, and he approved it.

**Cost he took:** a second way to edit text in Bridge. The Manifest file view keeps its text box; the file panel's editor is the only place CodeMirror is imported.

**What it needs that a primitive does not:**

- **A shadow root.** Bridge's CSP (`default-src 'self'`) refuses the `<style>` element CodeMirror injects. The editor renders inside a shadow root and adopts its stylesheets with `adoptedStyleSheets`, which the policy allows.
- **Syntax colours with no tokens of their own.** They borrow the `--tool-*` hues and neutral steps until syntax tokens exist in `packages/tokens`.

**Where it landed:** `packages/components/src/compositions/FilePanel/CodeEditor.tsx` and `FilePanel.tsx`. What the panel is and how it saves: `docs/concepts/file-panel.md`.
