// A pull request's Conversation holds every comment, the ones left on a line or a file too, in the order they were
// said: a thread headed by where it is, the lines it is about, and its replies under it. Evaluate on a line comment
// sends the Job that holds the pull request a message asking it to weigh the comment and bring a proposal; nothing is
// posted to the forge. Over the `session-pr-panel` scenario, on #2209.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { close, sheet, thread } = kit(false);
const sessions = region("Jobs", {
  exact: true,
});
const panel = sheet("Pull request #2209");
const onLine = role("article", "Thread on crates/config/settings.toml:16");
const onFile = role("article", "Thread on apps/desktop/src/main/forge.ts");
const evaluate = button("Evaluate: marta on crates/config/settings.toml:16", {
  exact: true,
});

const wide = walk("session-pr-panel", [
  toSessions,
  {
    press: inside(sessions, button("Store migration spike")),
    say: "A Job holding pull requests",
  },
  {
    press: inside(region("Merge queue"), button(/#2209/)),
    say: "Its pull request in the merge queue opens on Conversation",
  },
  {
    look: inside(panel, onLine),
    say: "A comment on a line is in the Conversation, where it was said, headed by its file and line",
  },
  {
    look: inside(onLine, text('+openPullRequestsIn = "armada"')),
    say: "The lines it is about, from the pull request's diff",
  },
  {
    look: inside(onLine, role("article", "nick")),
    say: "Its reply sits under it",
  },
  {
    hover: inside(onFile, role("img", "Resolved")),
    say: "A comment on a whole file is headed by the file. This one is resolved",
  },
  {
    look: inside(panel, button("Evaluate: marta")),
    say: "Every comment and review with words has Evaluate",
  },
  {
    press: inside(onLine, evaluate),
    say: "Evaluate on the line comment",
  },
  {
    look: inside(onLine, role("img", "Sent to the Job")),
    say: "It went to the Job that holds the pull request. Nothing was posted on it",
  },
  {
    press: close(panel),
    say: "Back to the Job",
  },
  {
    look: inside(thread, text(/Never reply on the pull request/)),
    say: "The Job has the comment, with its author, line and words, and is asked for a proposal in Needs you",
  },
]);

export { wide as "pr-conversation-evaluate" };
