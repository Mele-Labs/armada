// A changed file's row in a pull request's Files list carries the tests that reach it, as the test map answers them: measured, read
// by a model only, measured and stale, and reached by none. Told on the `test-map-reach` scenario's first pull request, and ends on
// a test file opened from under its file.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, walk } from "../walk";

const { row, sheet } = kit(false);
const queued = sheet("Pull request #2209");
const files = role("dialog", "Files");
/** The mark at the end of a file's row, by the file's name. */
const mark = (name: string) => inside(files, button(`Tests reaching ${name}`));
/** A file's row itself, not its mark. */
const file = (name: string) => inside(files, button(new RegExp(`^(Modified|Added)\\s+${name.replace(/\./g, "\\.")}`)));

const testMapReach = walk("test-map-reach", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store migration spike")), say: "A Job holding a pull request" },
  ...row("Open Pull request #2209", "Its pull request"),
  { press: inside(queued, tab("Files")), say: "Files" },
  { look: mark("session-pull-request.tsx"), say: "Measured" },
  { look: mark("settings.toml"), say: "A model's reading" },
  { look: mark("PullRequestPanel.tsx"), say: "Measured, and stale" },
  { key: "ArrowRight", on: file("PullRequestPanel.tsx"), say: "Right opens the tests under a file, left closes them" },
  { look: mark("forge.ts"), say: "No test reaches this file" },
  { key: "ArrowRight", on: file("forge.ts"), say: "Opened" },
  { key: "ArrowRight", on: file("session-pull-request.tsx"), say: "Opened" },
  { press: inside(files, button(/session-pull-request\.test\.tsx/)), say: "A test file opens" },
]);

export { testMapReach as "test-map-reach" };
