// The start menu of an empty Cockpit, in two takes by `?menu=`: `map` (the default) has the Cockpit's own battlefield behind it, which previews the
// start the focus is on; `hangar` is three bays, the focused one open. Over the `first-run` scenario. A short walk: at rest, down through the starts, a pick.

import { inside, region, role, walk } from "../walk";

const menu = role("group", "Start from");

const firstRunMenu = walk("first-run", [
  { look: menu, say: "Dispatch to a repository is open, with what it does" },
  { key: "ArrowDown", on: menu, say: "↓ opens Launch a new project" },
  { key: "ArrowDown", on: menu, say: "↓ opens Bring in your Claude Code sessions" },
  { key: "3", on: menu, say: "3 picks it" },
  { look: inside(region("First Job"), role("checkbox", /scratch/)), say: "The folders Claude Code kept sessions for" },
]);

export { firstRunMenu as "first-run-menu" };
