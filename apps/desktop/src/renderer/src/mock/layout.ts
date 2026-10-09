// Mock-only: the layout mods on this machine, held in memory until Fleet serves the mod folder. A
// mod is a `layout.json` as a Session would write it, checked by the same `parseLayout` Bridge
// will run on what Fleet returns.

import { createLayoutSource, parseLayout } from "@armada/shell";
import type { LayoutMod } from "@armada/shell";

/** No mods, nothing chosen: where every mock window starts. */
export const mockLayout = createLayoutSource();

/** Safe mode, `?nomods`: no layout mod is read, so every window draws the shipped layout. */
export const NO_LAYOUT_MODS = createLayoutSource();

/** What the first request writes: the merge line above the fleet, and Retros off the rail. */
export const TIDY_DASHBOARD = `{
  "version": 1,
  "dashboard.panels": { "order": ["merge-line", "fleet"] },
  "rail": { "hidden": ["lessons"] }
}
`;

/** What the second writes over it: Record before Plan in a Job, Pulse hidden, Plan first. */
export const TIDY_JOB = `{
  "version": 1,
  "dashboard.panels": { "order": ["merge-line", "fleet"] },
  "rail": { "hidden": ["lessons"] },
  "job.tabs": { "order": ["overview", "workflow", "record", "plan"], "hidden": ["pulse"], "first": "plan" }
}
`;

/** `text` checked and installed as the mod `tidy`, the way Fleet would read it from its folder. */
export function writeTidy(text: string): void {
  const { file, problems } = parseLayout(text);
  const mod: LayoutMod = { name: "tidy", title: "Tidy", enabled: true, file, ...(problems.length === 0 ? {} : { problem: problems[0]! }) };
  mockLayout.install(mod);
}
