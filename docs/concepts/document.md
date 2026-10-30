# Document

**What it is:** The HTML page a Job designs to hand a person what it found, read in a floating sheet over the Job, noted in place and revised by the Job's reply.

---

**Kind:** Concept.

**Part of:** [Job](session.md).

The reader is `DocumentLayer` in `apps/desktop/src/renderer/src/session-view/DocumentLayer.tsx`; the walk is `job-document`.
A design, an investigation, a report or a set of findings is a document. The Job writes it with its own tools, using `plugins/armada/skills/writing-a-document/SKILL.md`.

## Status

| Part | State |
|---|---|
| The reading sheet, notes, Changes, Share's menu | Built in the mock; walk `job-document` approved 25 Oct 2026 |
| Fleet keeping revisions, the `document` artifact, the sketch notice | Built |
| The Delivery row, the Artifacts rows and thread headings reading the real artifact | Built |
| Save as web page and Save as PDF | Built |
| Each revision asking his approval, `answer_document`, the `document` item and the `document_approved` end | Built in Fleet; Bridge's wiring owed |

## What a document is

**Rule.** A document is a free HTML page the Job designs, and not markdown.
Why: the page takes the shape its content needs. The cost is that a note anchors by quoted words and a revision compares as text, not as typed blocks.

**Rule.** A document stays with the Job and can be shared out wherever the person wants.

**Rule.** Notes are the feedback on a document's words, and his Approve or Send back answers a revision whole.
This rule said notes were the only feedback until the owner approved asking for an approval on 30 Oct 2026, as a sketch asks: [Asking for his approval](#asking-for-his-approval).

## Where it is written

**Rule.** A Job writes its page to `.armada/documents/<name>.html` in its worktree.
Why: `.armada/*` is gitignored, so the page is never in the diff.

**Rule.** Fleet hears every Write or Edit to such a file and keeps a copy of each different write as the next revision, in its own store.
A rewrite to the same path is the next revision, and an identical write is none.
The copies are `r1.html`, `r2.html` and on, under the machine's `attachments/documents/<job id>/<page>/`, outside every worktree, so a released slot keeps them.
A revision is kept once the tool has answered, so a failed write keeps nothing.

**Rule.** Fleet puts one artifact of form `document` on the Job. Its title is the page's `<title>`, or its first `<h1>` where there is none, and it names the paths of the revisions.

**Rule.** Bridge reads each artifact of form `document` as a page: `revisions` is a JSON list of absolute paths, oldest first, and a list that is not one reads as none.
Main reads a file for the reader only where it is a path in a `document` row's `revisions`.
A Job with more than one page opens the one pressed, and a sent note's heading opens the page whose title it names.

**Rule.** Fleet reads each `<armada-sketch>` scene with the validator `show_sketch` uses, and tells the Job in a Fleet notice of one that will not draw. The write is not refused.
The notice is `document_sketch`, once per revision: `In the page <path>, sketch <n> of <total> ("<title>") will not draw: <problems>.`

## Asking for his approval

**Rule.** Every revision Fleet keeps is put to the person: its artifact's `detail` gains `asked` `"true"`, `revision`, the newest revision's number, and `kept_at`, when it was kept.
It is read off the page Fleet kept, never off the Job's tool call.
A page kept before this has no `asked` and asks nothing.

**Rule.** Until he answers, the page waits in his Needs you as `document:<path>` and the Job's row calls him ([Waiting on the person](session/waiting-on-the-person.md)).

**Rule.** He approves the revision he read, or sends it back with a note, with `answer_document`; an answer to a revision kept since is refused.
The Job is told as his message: `Approved document: <title>, revision <n>` and to go on from it as written, or `Sent back document: <title>, revision <n>`, his note and to revise the page at its path.
`detail` then keeps `settled`, `settled_at` and his `words`.

**Rule.** A new revision writes the detail afresh, so it drops `settled` and asks again, and lifts a dismissal; the same bytes written again ask nothing.

**Rule.** A step can end on it: `document_approved` in a workflow's `ends_on` holds once he approves the newest revision of any page the Job wrote.

## The page

| Part | Read from |
|---|---|
| Title | `<title>`, else the first `<h1>` |
| Conclusion | The first non-empty `<p>` after the `<h1>`; Delivery shows it with the title |
| Contents | Each `<h2>`, in order; an `<h2>` with no `id` gets one |
| Notable rows | Each `<tr>` in a `<tbody>` |

**Rule.** A page may link `armada-kit.css` and add its own `<style>`; its own rules win over the kit's.
The kit is Armada's type and colour tokens as plain styling for headings, paragraphs, tables, code, quotes and figures, in `apps/desktop/src/renderer/src/session-view/document-kit.ts`.

**Rule.** `<armada-sketch title="…">` is the kit's one element, and Bridge draws it as a live sketch.
Its scene is JSON in a `<script type="application/json">` child, in the format `sketch-pane.md` owns.

**Rule.** A scene that does not validate draws an empty frame, as in a `show_sketch` call.

## Sealing

**Rule.** A page is read with `DOMParser`, which runs no script and loads nothing, and rebuilt element by element from an allow list.
Why: the page is drawn inside Bridge's own page. The code is `document-page.ts`.

| Kept | Dropped |
|---|---|
| Headings, paragraphs, lists, tables, `code`, `pre`, `figure`, `details` | `iframe`, `object`, `embed`, `form`, `input`, `button` |
| Sectioning and inline text elements, `a` | `img`, `video`, `audio`, `canvas`, `math` |
| Inline `svg` of shapes and text | In an `svg`: `script`, `foreignObject`, `use`, `image`, `a` with an address |
| `colspan`, `rowspan`, `scope`, `headers`, `datetime`, `open`, `start`, `lang`, `title`, `abbr` | `script`, with what is inside it, and every event handler |
| `class` and `id`, renamed | The `style` attribute |
| A link to `#…` or `http(s)://…` | Any other link address |

An element in neither column is dropped and its contents kept.
An element inside an `svg` that is not allowed is dropped with its contents.
The page's `<style>` and the kit's link are read for CSS and are not drawn as elements.

**Rule.** An inline `svg` is rebuilt from `svg`, `g`, `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, `text`, `tspan`, `title`, `desc`, `defs` and `clipPath`, so a Job can draw a chart.
Its attributes are `viewBox`, `preserveAspectRatio`, `class`, `id`, `role`, `aria-label`, `aria-hidden`, the geometry of those shapes and the presentation attributes (`fill`, `stroke`, the font and text attributes, `opacity`, `clip-path`).
Why: a chart is shapes and text, and none of the dropped elements can draw one.
A value holding an address is dropped, except `url(#id)` to an id in the same page, which is renamed as ids are.
Gradients, filters and animation are not allowed.

**Rule.** The page's CSS is read rule by rule and put under the page's own root.
A rule holding an address (`url(`, `image-set(`, `@import`) is dropped, and so is any at-rule other than `@media`.

**Rule.** The page's classes and ids are renamed, so its CSS styles the page and Bridge's styles reach nothing in it.
`body`, `html` and `:root` in a selector are the page's root.

**Rule.** The kit is the only stylesheet a page may link.

## The reading sheet

**Rule.** A press on Delivery's document row opens the page in the floating sheet the plan review uses, about three quarters of the window wide, with the Job dimmed behind it.
Esc closes it and gives the Job back as it was.

| Part | Holds |
|---|---|
| Head | Title, Revision, Changes, Notes, Share, Close |
| Contents card | A floating card on the left, one entry per `<h2>`; ↑ and ↓ walk it; the entry for the section in view is marked |
| Notes card | A second card directly below Contents: notes held, then notes sent |
| Page | The sealed page, with each `<armada-sketch>` drawn live, taking the rest of the width |

| Control | Key | Does |
|---|---|---|
| Revision | R | A dropdown from the first revision, newest first; R swaps the newest and the one before |
| Changes | C | Marks what moved on from the revision before; absent on the first |
| Notes | M | Hides or shows the Notes card; the choice holds across opens until the window reloads |
| Share | S | Opens Share |
| Close | Esc | Closes the sheet |

**Rule.** The revision is always a dropdown, even with one revision.

## Notes

**Rule.** A note is made on the page itself and is the only feedback.

| Point at | With | The note is on |
|---|---|---|
| Words | Select, then right-click or Shift+F10 | The quoted words |
| A part of a sketch | Right-click the part | The part, by id |
| A row of a table | Right-click, or focus the row and Shift+F10 | The row's cells |

**Rule.** Enter holds a note and ⌘Enter, or Send to Job, sends every held note as one numbered message.
`notesMessage` writes the message, and `session/noting-what-you-read.md` owns its format.

**Rule.** Sent notes stay messages in the Job's thread, and the Notes card reads them back from it. There is no separate store of notes.

**Rule.** Every note is highlighted at its place in the page: held more strongly than sent.
Hovering a note in the Notes card lights its place, and hovering a place lights its note.

**Rule.** A press on a sent note's heading in the thread opens the page at that revision and place.

## Revisions and Changes

**Rule.** The Job answers by rewriting the same file, which becomes the next revision.

**Rule.** Changes compares two revisions as text, block by block and then word by word.
Words and blocks that came are green, and words and blocks that went are struck in red. A block edited in place is marked amber at its edge.

**Rule.** A sent note shows the change that answered it: in the note's section, the change nearest the words, the part or the row.
A press on it scrolls the page to that change.

**Rule.** Changes does not read a sketch's scene. A sketch redrawn against the revision before draws the difference as `sketch-pane.md` says.

## Sharing

| Share | Does |
|---|---|
| Save as web page | Writes one HTML file, sketches drawn into it |
| Save as PDF | Writes a PDF, sketches drawn into it |
| Add to the repo | Asks the Job, in its thread, to commit the page |

**Rule.** Save as web page writes one HTML file: the page as it is read, its CSS inline, each `<armada-sketch>` replaced by the sketch drawn as static markup by the sketch pane's own export, and the value of every token the page's rules read written into the file.
The file holds no script and no address, and a policy in it allows nothing to load.
Why: the person who opens it has neither Armada nor a network the Job could have used.

**Rule.** Save as PDF prints that same file in a window that is never shown, with scripts off and nothing but the file loadable.
Both ask where to save in the system's save dialog, named for the page title.

**Rule.** Add to the repo sends the Job one line naming the page's path and saying it is gitignored, so the Job adds it with `git add -f`.

**Rule.** The page leaves the Job only by the person's press. Adding to the repo is a request the Job carries out, so the commit is on its branch and in its diff.
