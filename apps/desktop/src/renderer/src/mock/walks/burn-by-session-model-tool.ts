// The Usage page: open it by name, key down to the Session that burned most, pick it, and set its breakdown to tools.
// Keys on the page: ↓ j and ↑ k move down the Sessions, ↵ o picks one, Esc goes back to the week, m t d set the breakdown
// to model, tool or day.

import { inside, region, role, viaPalette, walk } from "../walk";

const SESSIONS = role("listbox", "Sessions", { exact: true });

export const burnBySessionModelTool = walk("cleanup/slots", [
  ...viaPalette("Usage", "Usage, by name"),
  { key: "ArrowDown", on: SESSIONS, say: "↓ moves the cursor to the top Session" },
  { key: "Enter", on: SESSIONS, say: "↵ picks it: the breakdown and the day chart are now its own" },
  { look: inside(region("Breakdown"), role("region", "By model")), say: "Its burn by model" },
  { key: "t", on: SESSIONS, say: "t sets the breakdown to tools" },
  { look: inside(region("Breakdown"), region("By tool")), say: "By tool, with what no listed tool carried as Other" },
]);
