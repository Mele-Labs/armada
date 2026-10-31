// A Job's area in panes. Every Job opens on Default: its thread, with Done, Now and Open stacked on its right. ⌘D splits a pane to the right and ⌘⌥D below it, a new pane opens empty,
// and + adds a tab of any kind to it. While a Job is open the title bar's right side holds only the Layout control, named "Layout: Default · edited" and wears a mark once the area is rearranged. ⌘L opens its menu: Save as turns it into a name field,
// every layout row has a pin that makes it the one every Job opens on (filled on the one that is), Update overwrites the layout the Job is on. A page the Job opens by itself joins the pane that holds a browser and leaves the
// layout as it was. The Job's acts (Auto land, Pause, Judge, Checks, Subagents) are icons in the Thread's head beside the context meter, and the merge line a strip along the window's bottom edge. ⌘Z takes back the last change and ⇧⌘Z puts it back. A tab dragged onto the gap between two panes lights the gap and drops into a new pane there, and so does one dragged onto an outer edge of the area. A Changes tab picks where its diff comes from, a Ledger tab lists what is attached, Subagents, Checks and Workflow tabs draw those views, Custom builds a tab of its own from a description, and ⇧⌘↵ zooms a pane over the Job's own area, to the right of the Jobs list, and puts it back. Mock only: nothing is kept. Over the `job-layouts` scenario.

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
  { hover: inside(role("toolbar", "Job acts"), button("Auto land")), say: "The Job's acts are icons in the Thread's head, each with its key" },
  { look: inside(pane("Thread"), button("Context 47%")), say: "The context meter is in the Job's head" },
  { hover: inside(role("toolbar", "Job acts"), button("Checks")), say: "Checks and Subagents are acts too" },
  { look: layout("Default"), say: "The title bar holds only the Layout control" },
  { look: region("Merge queue", { exact: true }), say: "The merge line is a strip along the window's bottom edge" },
  { key: "Meta+Shift+Enter", on: thread, say: "⇧⌘↵ zooms the pane over the Job's own area" },
  { look: deckRow("Retry the slot lease"), say: "The Jobs list stays where it was" },
  { look: inside(pane("Thread"), button("Restore")), say: "The strip's Restore says it is zoomed" },
  { key: "Meta+Shift+Enter", on: thread, say: "⇧⌘↵ puts it back as it was" },
  { key: "Meta+d", on: thread, say: "⌘D splits the pane to the right" },
  { press: inside(empty, button("Add tab")), say: "+ adds a tab" },
  { press: item("Browser"), say: "Browser, named by its page" },
  { key: "Meta+Alt+d", on: thread, say: "⌘⌥D splits the thread downward" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Terminal"), say: "Terminal, named by its shell" },
  { look: layout("Default · edited"), say: "The Layout icon in the title bar is marked: this Job is rearranged" },
  { key: "Meta+z", on: thread, say: "⌘Z takes back the last change: the terminal's pane goes" },
  { key: "Meta+Shift+z", on: thread, say: "⇧⌘Z puts it back" },
  { drag: inside(pane("zsh"), tab("zsh", { exact: true })), onto: role("separator", "Resize the panes"), say: "Drag a tab onto the gap between two panes and the gap lights" },
  { look: inside(pane("zsh"), tab("zsh", { exact: true })), say: "The tab is a pane of its own in the gap, with the space shared evenly" },
  { key: "Meta+z", on: thread, say: "⌘Z takes it back" },
  { drag: inside(pane("zsh"), tab("zsh", { exact: true })), onto: role("separator", "Resize the left column"), say: "The window's edges take a tab too: onto the left edge, over the Job list's handle" },
  { look: inside(pane("zsh"), tab("zsh", { exact: true })), say: "The tab is a pane on the far left, and the pane it left is gone" },
  { key: "Meta+z", on: thread, say: "⌘Z takes it back" },
  { key: "Meta+l", on: places, say: "⌘L opens the Layout menu" },
  { press: item("Save as…"), say: "Save as becomes a name field" },
  { type: "Triage\n", into: role("searchbox", "Name"), say: "A name, Enter" },
  { look: layout("Triage"), say: "The Job is on the layout it saved" },
  { press: deckRow("Measure lease contention"), say: "Another Job opens on Default" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: item("Triage"), say: "Triage" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { press: inside(item(/^Triage/), role("img", "Use for all Jobs")), say: "The pin on a row makes that layout the one every Job opens on" },
  { later: inside(pane(host), tab(host, { exact: true })), say: "The Job opens a page itself" },
  { look: inside(pane("localhost:5174"), tab("localhost:5174", { exact: true })), say: "It joins the pane that holds a browser" },
  { look: layout(/^Layout: Triage$/), say: "The layout is not marked edited" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { look: inside(item(/^Triage/), role("img", "Used for all Jobs")), say: "A filled pin marks the layout all Jobs use" },
  { press: item("Default"), say: "Default" },
  { key: "Meta+d", on: thread, say: "⌘D" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Changes"), say: "Changes" },
  { type: "pr:2391", into: role("combobox", "Changes from"), say: "The picker lists the branch, the Job's pull requests and the Job it started. A pull request" },
  { press: inside(pane("Changes"), button("Add tab")), say: "+" },
  { press: item("Ledger"), say: "Ledger: everything attached to the Job, by kind" },
  { press: inside(pane("Ledger"), button("Add tab")), say: "+" },
  { press: item("Subagents"), say: "Subagents" },
  { press: inside(pane("Subagents"), button("Add tab")), say: "+" },
  { press: item("Checks"), say: "Checks" },
  { press: inside(pane("Checks"), button("Add tab")), say: "+" },
  { press: item("Workflow"), say: "Workflow draws the Job's lanes" },
  { press: inside(pane("Workflow"), button("Add tab")), say: "+" },
  { press: item("Custom"), say: "Custom is the last entry, under a rule" },
  { type: "Pull requests waiting on me", into: inside(pane("Custom"), role("textbox", "Custom")), say: "Describe the tab" },
  { press: inside(pane("Custom"), button("Send")), say: "Send" },
  { look: inside(pane("Custom"), role("img", /^Mod Pull requests/)), say: "A Mod Job is building it: its thread shows, and its glyph turns" },
  { later: inside(pane("Review queue"), tab("Review queue", { exact: true })), say: "The Job finishes, and the tab takes the mod's name" },
  { look: inside(pane("Review queue"), button(/^Pull request #812/)), say: "Pull requests waiting on him, across repositories" },
  { press: inside(pane("Review queue"), button("Add tab")), say: "+" },
  { look: item("Review queue"), say: "The tab is in the menu by name, above Custom" },
  { key: "Escape", on: item("Review queue"), say: "Esc" },
  { key: "Meta+l", on: places, say: "⌘L" },
  { look: item("Update Default"), say: "Update Default overwrites Default for every Job on it" },
];

const layouts = walk("job-layouts", steps);

export { layouts as jobLayouts };
