# Noting what you read

**Part of:** [Job](../session.md).

**A person reading a sketch, a file or a picture in Bridge can note what he is pointing at and send it to a Job without closing the viewer.**
The code is `packages/components/src/compositions/ViewerNotes/`, mounted for Sessions by `apps/desktop/src/renderer/src/viewer-notes.tsx`; the walk is `job-notes`.

| Reading | Point at it with | Note on |
|---|---|---|
| A file, in the editor or the markdown preview | Select text, then right-click, the ContextMenu key or Shift+F10, and Add note | The lines selected |
| A sketch | Right-click a part, or Shift+drag to box several; a plain drag still pans | The parts picked |
| A picture, in the file panel or enlarged | Drag a region, or note the whole picture | The region, or the whole |
| A Job's document, in the reading layer | Select words, right-click a part of an embedded sketch, or right-click a table row | The words, the parts or the row, with the page's title, its revision and the section |

**Enter holds a note on its viewer**, numbered and marked on what it is about.
**⌘Enter sends** every held note plus the one being typed, as one numbered message.

**A note goes to the open Job; elsewhere the box's Job chip picks one.**
Held notes and the pick stay per sketch, file or picture until sent, in memory: closing the viewer and opening it again finds them, and a reload does not.

**The agent receives plain text**, one numbered block per note, what it is about on the number's line and the person's words beneath. `notesMessage` in `compose.ts` writes it.

| About | Heading |
|---|---|
| Sketch | Its title, and each part's id and label |
| File | `path:from-to`, and up to six lines quoted under it |
| Picture | Its path, the region in percent, and `attached as picture-N.png`, the crop riding along as an image |
| Document | `Document "title", revision N, section "heading": ` then `words "…"`, `row "…" of "table"`, or `sketch "title", part id (label)` |

**In the Job's thread each note's heading is a press that reopens what was noted:** the sketch with its parts picked, the file with its lines selected, the picture with its region drawn.
`notedOf` reads the heading back out of the message, and a message `notesMessage` did not write is drawn as any other.

**A Job's document is read in a layer over the whole Job, and the notes are made on the page itself.**
The Job writes the next revision as its reply: Changes marks the words that came and went, and each sent note in the margin shows the change that answered it.
The layer is `apps/desktop/src/renderer/src/session-view/DocumentLayer.tsx`; the walk is `job-document`.
`docs/concepts/document.md` owns what a document is, how it is written, sealed and shared.

**These notes are not the annotation layer.** The layer (⌥⌘A, `docs/practices/running-locally.md`, *Annotating Bridge*) is for notes on Armada itself and saves a file for `/annotations`; these go to a Job as a message.
