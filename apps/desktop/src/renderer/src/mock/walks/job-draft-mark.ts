// A Job an agent started waits as a draft: its first message sits in the box, and nothing runs until
// he sends it. The Job wears the Draft mark on its row and in its header until then, and Working once
// it runs. Over the `job-draft-mark` scenario. `?take=a` draws `message-square-dashed`, `?take=b` draws `text-cursor-input`.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const draft = role("img", "Draft", { exact: true });

const draftMark = walk("job-draft-mark", [
  toSessions,
  { hover: inside(sessions, draft), say: "A Job an agent started. Its row wears the Draft mark" },
  { press: inside(sessions, button(/Settings loader and the cap/)), say: "He opens it" },
  { look: inside(region(/^Job (?!:)/), draft), say: "The header says Draft" },
  { look: role("textbox", "Message"), say: "The first message waits in the box" },
  { key: "Enter", on: role("textbox", "Message"), say: "He sends it" },
  { look: inside(region(/^Job (?!:)/), role("img", "Working", { exact: true })), say: "The mark is Working" },
]);

export { draftMark as "job-draft-mark" };
