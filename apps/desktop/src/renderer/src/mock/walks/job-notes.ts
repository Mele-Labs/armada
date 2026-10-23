// Notes on what is being read, sent to a Job without closing it. A sketch in a Job: a part noted and held, two boxed
// with Shift and noted, both sent with ⌘Enter. A file opened outside any Job: lines noted from the keyboard, the Job
// picked on the box's chip. A picture the Job showed: a region dragged and noted, sent with its crop.
// Over the `job-notes` scenario.

import { button, deckRow, dialog, inside, place, role, text, toSystemPage, walk } from "../walk";
import type { Name } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";
import { FAILURE, NOTES_JOB } from "../scenarios/job-notes";

/** A sheet named this and nothing else: the note box's own name holds the file's. */
const sheet = (name: Name) => role("dialog", name, { exact: true });

function steps(): Step[] {
  const { thread, close, row } = kit(false);
  const sketch = dialog("Sketch Store clock reads");
  const part = (name: string) => inside(sketch, role("group", name, { exact: true }));
  const add = role("menuitem", "Add note");
  const note = role("textbox", "Note");
  const box = dialog(/^Note on/);
  const editor = inside(sheet("clock.rs"), role("group", "Editor clock.rs", { exact: true }));
  const picture = role("img", FAILURE, { exact: true });
  return [
    toSessions,
    { press: deckRow(NOTES_JOB), say: "A Job whose agent drew a sketch and showed a picture" },
    { press: inside(thread, button("Open sketch Store clock reads")), say: "The sketch opens beside the thread" },
    { context: part("Writer"), say: "Right-click a part" },
    { press: add, say: "Add note" },
    { type: "Why does the writer not hold the clock?\n", into: note, say: "Enter holds the note on the sketch" },
    { look: inside(part("Writer"), role("img", "Note 1")), say: "Its number stands on the part" },
    { drag: part("Clock"), by: { x: 0, y: 200 }, shift: true, say: "Shift drag boxes the clock and the test under it. A plain drag still pans" },
    { context: part("Clock"), say: "Right-click what was boxed" },
    { press: add, say: "Add note" },
    { type: "Pin both to one FixedClock", into: note, say: "A second note" },
    { look: inside(box, role("list", "Held notes")), say: "The box lists what is held, each with Remove" },
    { key: "Meta+Enter", on: note, say: "⌘Enter sends both as one message" },
    { press: close(sketch), say: "Close" },
    { look: inside(thread, text("Store clock reads: Writer")), say: "One numbered message in the Job's thread" },
    ...toSystemPage("Settings", "Away from any Job"),
    { key: "Meta+o", on: role("navigation", "Places"), say: "⌘O opens a file" },
    { select: "impl Clock for SystemClock {\n    fn now(&self) -> Instant {\n        Instant::now()", in: editor, say: "Lines selected" },
    { key: "Shift+F10", on: editor, say: "Shift F10, or the menu key, opens the menu at the selection" },
    { press: add, say: "Add note" },
    { type: "SystemClock still reads the wall clock", into: note, say: "The note" },
    { press: inside(box, button("Send to a Job")), say: "Outside a Job the chip picks where it goes" },
    { press: role("menuitem", NOTES_JOB), say: "The Job" },
    { look: editor, say: "The lines are marked with the note's number" },
    { key: "Meta+Enter", on: note, say: "⌘Enter" },
    { press: inside(sheet("clock.rs"), role("button", /^Close/)), say: "Close" },
    { press: place("Cockpit"), say: "Cockpit" },
    { press: deckRow(NOTES_JOB), say: "The Job" },
    { look: inside(thread, text(/clock\.rs:\d+-\d+/)), say: "The file's lines, quoted, and the note" },
    ...row(FAILURE, "The picture the agent showed"),
    { press: inside(sheet(FAILURE), button(FAILURE)), say: "Pressed, it opens large" },
    { drag: picture, by: { x: 180, y: 110 }, say: "A drag draws a region" },
    { context: picture, say: "Right-click inside it. Outside a region, the note is on the whole picture" },
    { press: add, say: "Add note" },
    { type: "This row is the second turning", into: note, say: "The note" },
    { key: "Meta+Enter", on: note, say: "⌘Enter sends it with the crop" },
    { key: "Escape", on: picture, say: "Esc" },
    { press: close(sheet(FAILURE)), say: "Close" },
    { look: inside(thread, text(/picture 1/)), say: "The message names the picture it carries" },
  ];
}

const notes = walk("job-notes", steps());

export { notes as "job-notes" };
