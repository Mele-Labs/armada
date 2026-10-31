// A Job's area in panes. Every Job opens on Default: its thread, with Done, Now and Open stacked on its right. ⌘D splits a pane to the right and ⌘⌥D below it, a new pane opens empty,
// and + adds a tab of any kind to it. The Layout control, in the title bar beside the Job's views, reads "Default · edited" once the area is rearranged. ⌘L opens its menu: Save as turns it into a name field,
// Use for all Jobs makes a layout the one every Job opens on, Update overwrites the layout the Job is on. A page the Job opens by itself joins the pane that holds a browser and leaves the
// layout as it was. ⌘Z takes back the last change and ⇧⌘Z puts it back. A Changes tab picks where its diff comes from. Mock only: nothing is kept. Over the `job-layouts` scenario.

import { button, deckRow, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const empty = region("Empty pane", { exact: true });
const layout = (name: string | RegExp) => role("button", typeof name === "string" ? `Layout: ${name}` : name);
const item = (name: string | RegExp) => role("menuitem", name);
const thread = inside(pane("Thread"), tab("Thread", { exact: true }));
const places = role("navigation", "Places");
const host = "localhost:5173";

const steps: Step[] = [
  toSessions,
  { press: deckRow("Retry the slot lease"), say: "A Job opens on Default: its thread, and Done, Now and Open stacked beside it" },
  { key: "Meta+d", on: thread, say: "⌘D splits the pane to the right" },
  { press: inside(empty, button("Add tab")), say: "+ adds a tab" },
  { press: item("Browser"), say: "Browser, named by its page" },
  { key: "Meta+Alt+d", on: thread, say: "⌘⌥D splits the thread downward" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Terminal"), say: "Terminal, named by its shell" },
  { look: layout("Default · edited"), say: "The control, in the title bar, says this Job is rearranged" },
  { key: "Meta+z", on: thread, say: "⌘Z takes back the last change: the terminal's pane goes" },
  { key: "Meta+Shift+z", on: thread, say: "⇧⌘Z puts it back" },
  { key: "Meta+l", on: places, say: "⌘L opens the Layout menu" },
  { press: item("Save as…"), say: "Save as becomes a name field" },
  { type: "Triage\n", into: role("searchbox", "Name"), say: "A name, Enter" },
  { look: layout("Triage"), say: "The Job is on the layout it saved" },
  { press: deckRow("Measure lease contention"), say: "Another Job opens on Default" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Triage"), say: "Triage" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Use for all Jobs"), say: "Use for all Jobs. The pin marks the layout every Job opens on" },
  { later: inside(pane(host), tab(host, { exact: true })), say: "The Job opens a page itself" },
  { look: inside(pane("localhost:5174"), tab("localhost:5174", { exact: true })), say: "It joins the pane that holds a browser" },
  { look: layout(/^Layout: Triage$/), say: "The layout is not marked edited" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Default"), say: "Default" },
  { key: "Meta+d", on: thread, say: "⌘D" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Changes"), say: "Changes" },
  { type: "pr:2391", into: role("combobox", "Changes from"), say: "The picker lists the branch, the Job's pull requests and the Job it started. A pull request" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { look: item("Update Default"), say: "Update Default overwrites Default for every Job on it" },
];

const layouts = walk("job-layouts", steps);

export { layouts as jobLayouts };
