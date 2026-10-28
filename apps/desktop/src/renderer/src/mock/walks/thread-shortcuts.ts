// The four keys on a Job's thread cards. The target card, the newest waiting or the one holding focus, wears a ring and draws its keys;
// ⌥↵ and ⌥X ask which card first, every time, and ⌥R opens the target's own note box. Over the `thread-shortcuts` scenario.

import { button, deckRow, dialog, inside, role, walk } from "../walk";

const plan = role("article", "Plan to approve");
const oldest = role("article", "Draft: Description for the resumable upload change", { exact: true });
const second = role("article", "Draft: Release note for resumable uploads", { exact: true });
const box = role("textbox", "Message");
const approving = dialog("Approve which?");
const withdrawing = dialog("Withdraw which?");

const threadShortcuts = walk("thread-shortcuts", [
  { press: deckRow("Resume large uploads"), say: "A Job with three drafts and a plan waiting in its thread" },
  { look: inside(plan, role("group", "Answers")), say: "The plan is the newest card waiting: it wears the ring, and Approve, Suggest and Not yet draw ⌥↵, ⌥E and ⌥R" },
  { look: inside(plan, button("Suggest", { exact: true })), say: "Suggest opens the plan in full to suggest in, as ⌥E does" },
  { key: "Alt+Enter", on: box, say: "⌥↵ from the message box" },
  { look: inside(approving, role("radio", "Plan · Keep the upload's parts on disk, Show where a resumed upload stands")), say: "Approve which? lists the four cards, numbered, with the plan picked" },
  { key: "2", on: approving, say: "2 picks the release note" },
  { look: inside(approving, role("radio", "Draft · Release note for resumable uploads")), say: "The release note is picked" },
  { key: "Alt+Enter", on: approving, say: "⌥↵ approves it" },
  { look: inside(second, role("img", "Approved")), say: "The release note is settled as approved where it stands" },
  { press: oldest, say: "A press on the oldest draft" },
  { look: inside(oldest, role("group", "Answers")), say: "The ring and the keys move to it: Approve ⌥↵, Edit ⌥E, Note ⌥R, Withdraw ⌥X" },
  { key: "Alt+r", on: oldest, say: "⌥R" },
  { look: inside(oldest, role("textbox", "Note")), say: "Its note box opens with the caret in it, and ⌘↵ there sends it back" },
  { key: "Alt+x", on: inside(oldest, role("textbox", "Note")), say: "⌥X" },
  { look: inside(withdrawing, role("radio", "Draft · Description for the resumable upload change")), say: "Withdraw which? lists the drafts that can be withdrawn, the one in focus picked" },
  { key: "Escape", on: withdrawing, say: "Esc puts it away, and nothing is withdrawn" },
  { look: inside(oldest, role("textbox", "Note")), say: "The note box is as it was" },
]);

export { threadShortcuts as "thread-shortcuts" };
