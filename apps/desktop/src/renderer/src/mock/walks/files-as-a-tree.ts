// A pull request's Files tab draws its changed files as a tree of folders, not one long path per row: a folder is drawn once,
// a folder holding only one folder is drawn with it, and a file is its name under them. Told on the `session-pr-panel` scenario's
// first pull request, and left open on the tab.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, walk } from "../walk";

const { row, sheet } = kit(false);
const queued = sheet("Pull request #2209");
const files = role("dialog", "Files");

const filesAsATree = walk("session-pr-panel", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store migration spike")), say: "A Job holding a pull request" },
  ...row("Open Pull request #2209", "Its pull request"),
  { press: inside(queued, tab("Files")), say: "Files" },
  { look: inside(files, role("list", "Files")), say: "The changed files as a tree of folders, each file by its name with its lines added and removed" },
]);

export { filesAsATree };
