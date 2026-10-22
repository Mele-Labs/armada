// A pull request's Files tab draws its changed files as a tree of folders, not one long path per row: a folder is drawn once,
// a folder holding only one folder is drawn with it, and a file is its name under them. Told on the `session-pr-panel-work` scenario's
// first pull request, folds and opens the folders, and is left on the tab.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, walk } from "../walk";

const { row, sheet } = kit(false);
const queued = sheet("Pull request #2209");
const files = role("dialog", "Files");

const filesAsATree = walk("session-pr-panel-work", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store migration spike")), say: "A Job holding a pull request" },
  ...row(/^#2209 /, "Its pull request"),
  { press: inside(queued, tab("Files")), say: "Files" },
  { look: inside(files, role("list", "Files")), say: "The changed files as a tree of folders, each file by its name with its lines added and removed" },
  { press: inside(files, button("Fold every folder")), say: "One press folds every folder" },
  { look: inside(files, button(/^packages\/components\/src\/compositions\//)), say: "Only the top folders are left, each says it is closed" },
  { press: inside(files, button(/^packages\/components\/src\/compositions\//)), say: "A folder opens by itself" },
  { look: inside(files, button(/PullRequestCard\//)), say: "Its folders are folded as they were left" },
]);

export { filesAsATree };
