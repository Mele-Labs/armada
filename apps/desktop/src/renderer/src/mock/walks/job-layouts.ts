// A Job's area in panes. Every Job opens on Default: its thread, with Done, Now and Open to its right. ⌘D splits a pane to the right and ⌘⌥D below it, a new pane opens empty,
// and + adds a tab of any kind to it. The Layout control in the Job's header reads "Default · edited" once the area is rearranged. ⌘L opens its menu: Save as keeps the arrangement
// under a name, Use for all Jobs makes a layout the one every Job opens on. A page the Job opens by itself joins the pane that holds a browser. Mock only: nothing is kept.
// Over the `job-layouts` scenario.

import { button, deckRow, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const empty = region("Empty pane", { exact: true });
const layout = (name: string | RegExp) => role("button", typeof name === "string" ? `Layout: ${name}` : name);
const item = (name: string | RegExp) => role("menuitem", name);
const saving = dialog("Save layout");
const places = role("navigation", "Places");

const steps: Step[] = [
  toSessions,
  { press: deckRow("Retry the slot lease"), say: "A Job opens on Default" },
  { look: inside(pane("Now"), tab("Now", { exact: true })), say: "Its thread on the left, Done, Now and Open as tabs on the right" },
  { key: "Meta+d", on: places, say: "⌘D splits the focused pane to the right" },
  { look: empty, say: "The new pane is empty" },
  { press: inside(empty, button("Add tab")), say: "+ adds a tab" },
  { press: item("Browser"), say: "Browser" },
  { press: inside(pane("Thread"), tab("Thread", { exact: true })), say: "Back to the thread" },
  { key: "Meta+Alt+d", on: places, say: "⌘⌥D splits it downward" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Terminal"), say: "Terminal" },
  { look: layout("Default · edited"), say: "The Layout control says this Job is rearranged" },
  { key: "Meta+l", on: places, say: "⌘L opens the Layout menu" },
  { press: item("Save as…"), say: "Save as" },
  { type: "Triage", into: inside(saving, role("textbox", "Name")), say: "A name" },
  { press: inside(saving, button("Save")), say: "Save" },
  { look: layout("Triage"), say: "The Job is on the layout it saved" },
  { press: deckRow("Measure lease contention"), say: "Another Job opens on Default" },
  { look: layout("Default"), say: "Rearranging one Job left the other as it was" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Triage"), say: "The saved layout" },
  { look: inside(pane("Browser"), tab("Browser", { exact: true })), say: "Thread over Terminal, Browser beside them, Done, Now and Open on the right" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Use for all Jobs"), say: "Use for all Jobs. The pin marks the layout every Job opens on" },
  { later: inside(pane("Browser"), tab("Browser", { exact: true })), say: "The Job opens a browser page itself" },
  { look: inside(pane("Browser"), role("tablist", "Tabs")), say: "It joins the pane that holds a browser" },
];

const layouts = walk("job-layouts", steps);

export { layouts as jobLayouts };
