// Armada Mods, tier 2: layout. A Session is asked to move the merge line to the top of the Dashboard
// and hide a page, then to arrange a Job's tabs; the mod it writes shows in Mods, and Settings →
// Layout holds every tab, panel and rail row, the owner's own moves over the mod, and Reset to defaults.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, tab, text, walk } from "../walk";

const { message, thread, rail } = kit(false);
const palette = role("combobox", "Search actions, jobs and settings");
const jobTabs = role("tablist", "Job detail");
const group = (name: string) => role("group", name);
const ask = (words: string) => ({ type: words, into: message, say: "The request" });
const tabsOf = (words: string) => ({ look: jobTabs, say: words });

export const modsLayout = walk("mods-layout", [
  { look: tab("Your move"), say: "The Cockpit as shipped" },
  { look: rail("Retros"), say: "Retros is on the rail" },
  toSessions,
  { press: inside(region("Sessions"), button(/A layout/)), say: "A Session Bridge hosts" },
  ask("Move Merge line to the top of my Dashboard and hide the Lessons tab.\n"),
  { look: inside(thread, text(/layout\.json/)), say: "layout.json written into the mod folder" },
  { press: rail("Cockpit"), say: "The Cockpit" },
  { look: tab("Your move"), say: "The Cockpit after the mod" },
  { press: rail("Mods"), say: "Mods, under Machine" },
  { look: role("img", "Layout"), say: "Each row names its kind" },
  { look: group("Tidy"), say: "Tidy" },
  { look: button("Promote Tidy"), say: "Promote, as for a theme" },
  toSessions,
  { press: inside(region("Sessions"), button(/A layout/)), say: "Back to the Session" },
  ask("In a Job, put Record before Plan, hide Pulse and open on Plan.\n"),
  { look: inside(thread, text(/Jobs open on Plan/)), say: "layout.json rewritten" },
  { press: button("Search jobs, commands, settings… ⌘ K"), say: "Open a Job" },
  { type: "pin the store\n", into: palette, say: "The running Job" },
  tabsOf("Record is before Plan, Pulse is gone, and the Job opened on Plan"),
  { press: rail("Settings"), say: "Settings" },
  { look: group("Job tabs"), say: "Layout lists every tab, in the order it is drawn" },
  { look: inside(group("Job tabs"), group("Pulse")), say: "Pulse is off, and a mod did it" },
  { look: inside(group("Job tabs"), role("img", "Hidden by Tidy")), say: "The mod's name on hover" },
  { hover: inside(group("Job tabs"), text("Overview")), say: "A tab a decision lives on has no switch" },
  { press: inside(group("Cockpit filters"), button("Move Done up")), say: "A move here is the owner's own" },
  { look: inside(group("Rail"), group("Retros")), say: "Retros is off the rail" },
  { press: button("Reset to defaults"), say: "Reset to defaults" },
  { look: inside(group("Rail"), group("Retros")), say: "Everything is back, and Tidy is switched off" },
  { press: rail("Mods"), say: "Mods" },
  { look: group("Tidy"), say: "Tidy is still here, off" },
]);
