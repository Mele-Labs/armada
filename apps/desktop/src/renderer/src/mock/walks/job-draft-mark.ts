// A Job an agent started is held on its start card: its first message sits in the box, and nothing runs until
// he presses Start. The Job wears the Draft mark on its row and in its header until then, and Working once
// it runs. Over the `job-draft-mark` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const draft = role("img", "Not started", { exact: true });

const draftMark = walk("job-draft-mark", [
  toSessions,
  { hover: inside(sessions, draft), say: "A Job an agent started. Its row wears the Draft mark" },
  { press: inside(sessions, button(/Settings loader and the cap/)), say: "He opens it" },
  { look: inside(region(/^Job (?!:)/), role("img", /^Not started · /)), say: "The header says Not started" },
  { look: role("textbox", "Message"), say: "The first message waits in the box" },
  { key: "Enter", on: role("textbox", "Message"), say: "Send reads Start" },
  { look: inside(region(/^Job (?!:)/), role("img", /^Working · /)), say: "The mark is Working" },
]);

export { draftMark as "job-draft-mark" };
