// A file of a Session's pull request, opened from the Files tab as a diff in a panel to the left of the
// pull request's own. Over the third pull request in `session-pr-panel`, whose files each have a
// patch but one, a binary. The plates move into the panel while the diff takes their place.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, text, walk } from "../walk";

const { close, row, sheet } = kit(false);
const sessions = region("Jobs", { exact: true });
const pull = sheet("Pull request #2209");
const files = role("list", "Files");
const forge = sheet("apps/desktop/src/main/forge.ts");
const panel = sheet("apps/desktop/src/renderer/src/session-pull-request.tsx");
const binary = sheet("apps/desktop/resources/icon.png");

const prFileDiff = walk("session-pr-panel", [
  toSessions,
  { press: inside(sessions, button("Store migration spike")), say: "A Job holding pull requests" },
  ...row("Open Pull request #2209", "Its pull request opens"),
  { press: inside(pull, tab("Files")), say: "Files" },
  { look: inside(pull, files), say: "Each file is a row that can be pressed" },
  { press: inside(pull, button(/main\/forge\.ts/)), say: "A file" },
  { look: inside(forge, text(/openPullRequestsIn/)), say: "Its diff opens in a panel to the left of the pull request" },
  { look: inside(pull, button(/main\/forge\.ts/)), say: "Its row is marked" },
  { press: inside(pull, button(/src\/session-pull-request\.tsx/)), say: "Another file" },
  { look: inside(panel, text(/promptsOut\(one\.body\)/)), say: "The panel shows that file's diff instead" },
  { press: inside(pull, button(/resources\/icon\.png/)), say: "A binary file" },
  { look: inside(binary, text(/The forge sent no patch for this file/)), say: "The forge sent no patch, and the panel says so" },
  { press: close(binary), say: "Close" },
  { look: inside(pull, tab("Files")), say: "The pull request is as it was" },
]);

export { prFileDiff as "pr-file-diff" };
