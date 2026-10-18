# Hard rules

**Part of:** [Design System — UI & Voice](../design-system.md).

1. **No Tailwind arbitrary values.** Never `bg-[#3b82f6]`, `p-[13px]`,
   `text-[15px]`. Every value comes from the token set below.
   Lint-enforced in the app — arbitrary values fail the build.
2. **Only shadcn/ui primitives:** button, input, textarea, select, checkbox,
   radio, switch, badge, card, table, dialog, sheet, tabs, toast, tooltip,
   dropdown-menu, popover, separator, scroll-area, skeleton, alert,
   **command**. Compose from these; do not invent new base components.
   `textarea` is sanctioned because a Job's brief is prose a person writes at
   length, and a single-line input for it is a control that fights its content.
   **A text box someone types into keeps what was typed** across leaving its
   panel and coming back, keyed by what it is about; `keep={false}` is only for a
   field that edits a stored value.
   `command` (cmdk) backs the command palette. A `kbd` element and an `a`
   element are the two non-shadcn primitives — `kbd` is specified under Keyboard
   and command palette, and `a` is what a fact that names something outside
   Armada is drawn as. It takes `--accent`, which this document already gives
   links, and underlines on hover only. **No surface may navigate.** Every
   anchor cancels its own default and hands the address to the process that
   owns the shell; a window that loaded a forge would be a window with no rail
   and no way back, which is the frozen surface Bridge exists to escape.
   **The capture window is not a surface, which is why it may load an
   address.** It draws no Armada surface inside the page it shows — it is
   Bridge's chrome around another repository's running app, pinned to that
   Run's own loopback origin, with no rail to lose and no history to go back
   through. What it may load, what it refuses and what it never claims is
   `../practices/capture-window.md`.
3. **Status colors are never chosen.** They map to the Job state machine
   one to one. Never assign a status color by aesthetic judgment. **Below
   Job level, hue exists only where `tokens/status.css` declares it**,
   and every value there aliases its Job counterpart so the mapping is
   declared rather than inferred. Read the file rather than a list; this
   rule used to enumerate the cases and went stale twice. Anything the
   file does not declare stays neutral. See Below Job level under Tokens.
   **Outside status and accent, these hue families exist.** `--helm` marks
   Helm's own chrome and nothing else. `--you` marks the owner's own
   messages in a Session thread and nothing else. The **tool families** in
   `tokens/tools.css` say what a Drone's call does, and they are a
   separate file because they alias no status and must never carry one —
   see Tool families under Tokens. The **node kinds** in
   `tokens/kinds.css` say what a node on the approval canvas is, on that
   canvas alone, for the same reason in a file of their own — see Node
   kinds under Tokens. Depth's light and glass never take a
   status hue.
4. **Dark is primary.** Design dark first. Light exists but is secondary. Light is `packages/tokens/src/light.css`; every other theme is described in `docs/concepts/themes.md`.
5. **Icons: lucide-react only**, used sparingly. A dashboard dense with
   icons reads as noise.
6. **React Flow (`@xyflow/react`) is the one sanctioned graph surface.** It
   draws a [Studio](../../concepts/studio.md)'s whiteboard, a Job's workflow
   canvas and the sketch beside a dispatch prompt, and nothing else takes
   a second library — a surface that needs to draw takes the extracted
   canvas rather than a drawing tool. It supplies
   placement, pan, zoom, fit and selection, and nothing that is seen:
   every node inside it is built from the primitives above and the
   tokens below, its controls are `button`, and every value its own
   stylesheet would paint is set to a token. No second graph or canvas
   library, and no node drawn from React Flow's defaults.
   **Every canvas carries one rail down its leading edge**, and the acts
   on a selected node hover over that node rather than sitting in a panel
   — the owner's own drawing, 28 Sep 2026. The rail is groups: what a
   surface places, how a person looks, and whatever one canvas alone has.
   [Iconography](../iconography.md), *The canvas rail*, binds every glyph on
   it. The rail is a column of the frame and not a layer over the graph,
   so nothing a fit draws can end up under it.
   **A Job's Workflow tab is the canvas alone**: the stacked run it once
   toggled to went on 8 Oct 2026, the owner's word, and every act it offered
   is on the canvas.
   **Freehand is the sketch pad's alone, and it is still no second
   library.** A line a person draws by hand is an SVG path in React
   Flow's own viewport, so it pans and zooms with the boxes; it takes no
   drawing tool and adds nothing to `GraphCanvas`, which every other
   graph draws on. A surface that wants a pen mounts its own through the
   canvas's `children`, or it does without one.
   **The pad is as tall as where it opens.** Beside a prompt it is
   `--h-sketch-pad`, because it shares its card; opened over a Studio it
   fills its sheet, with that height as its floor — the owner's call of
   1 Oct 2026. Filling is layout, never a second fixed height.
7. **No aggregate count beside what it counts.** A number saying how many
   rows, Checks or items a list holds, drawn next to that list, repeats what
   the reader can already see and teaches the eye to skip numbers. Draw the
   items and leave the count off. A count goes only where the items are not
   drawn: a folded list, or a tab naming a backlog, on *Destinations*' rule
   for the figure beside a name. The owner, 29 Sep 2026.
