// The Dashboard as one panel. Its top bar filters Your move, Active and Done, and reads them as a grid
// of tiles or a map of stars; a call that needs the owner comes forward over either with the context to answer it, and is answered from the
// keyboard, the calls behind it stacked like a deck. Over `dashboard-cockpit`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const cockpit = region("Dashboard");
const glass = role("listbox", "Tiles");
const map = role("listbox", "Map");
const behind = (kind: RegExp) => button(kind);
const call = (kind: RegExp) => region(kind);

const dashboardCockpit = walk("dashboard-cockpit", [
  { look: tab("Your move"), say: "One panel, three filters on its top bar. Your move is what needs him, and nothing does: the panel is empty" },
  { key: "]", on: cockpit, say: "] steps to the next filter, [ to the one before" },
  { look: tab("Active"), say: "Active: everything live, one tile each. What is active is the glyph, and a tile that asks is lit" },
  { hover: inside(glass, role("img", "Drone working")), say: "A glyph names its state on hover" },
  { key: "j", on: cockpit, say: "j and k move along the glass, the arrows too" },
  { look: inside(glass, button("Open")), say: "The one under the cursor carries its keys: Enter opens, x kills" },
  { key: "m", on: cockpit, say: "m flips the grid into a map: a star each, clustered by repository, joined by what connects them" },
  { look: map, say: "The same filter, as stars. A parent to its child, a Job to what it waits on, a Session to its Job, branches to main" },
  { hover: inside(map, role("option", /Cache the manifest/)), say: "A star names its state on hover; the one that needs him burns brightest" },
  { later: cockpit, say: "Time passes: a Drone reaches a question" },
  { look: call(/^Plan question/), say: "It comes forward over the map, as it would over the grid: the Job, what he asked for, who asked and where, what is active, then the question" },
  { key: "e", on: cockpit, say: "e opens the whole request" },
  { look: inside(call(/^Plan question/), button(/Less/)), say: "The request, in full" },
  { key: "2", on: cockpit, say: "A number picks an answer" },
  { look: inside(call(/^Plan question/), role("radio", "Wrap it in place")), say: "Picked, and not sent" },
  { later: cockpit, say: "A second call arrives while the first is open" },
  { look: behind(/^Check failed/), say: "It stands behind the open one, its title on its edge" },
  { key: "b", on: cockpit, say: "b is a standing answer: make the best decision" },
  { look: inside(call(/^Plan question/), role("radio", /Make the best decision/)), say: "Under the numbered ones, with what it tells the agent on hover" },
  { key: "Enter", on: cockpit, say: "Enter sends the answer" },
  { look: call(/^Check failed/), say: "The next call slides up from behind. A state Fleet raises, a failed Check, keeps only its own act" },
  { later: cockpit, say: "A Session stops to ask for a command" },
  { look: behind(/^Session/), say: "Behind it, in its turn" },
  { key: "l", on: cockpit, say: "l puts this one at the back of the stack" },
  { look: call(/^Session/), say: "The Session's call: a small view of the Session, and what it asks" },
  { look: text("The store test is flaky on CI"), say: "What he told it, and what it did since" },
  { key: "g", on: cockpit, say: "On a Session's ask the standing two hand the decision to the agent: g, just get it done" },
  { look: inside(call(/^Session/), role("radio", /Just get it done/)), say: "Picked" },
  { key: "1", on: cockpit, say: "1 is Allow, which he gives himself" },
  { key: "Enter", on: cockpit, say: "Enter sends it" },
  { look: behind(/^Check failed/), say: "The one put off waits at the back, dashed" },
  { key: "w", on: cockpit, say: "w brings it forward" },
  { look: call(/^Check failed/), say: "Back in front" },
  { key: "Escape", on: cockpit, say: "Esc puts it off again" },
  { look: map, say: "Nothing in front: the map, with the call still waiting at the bottom edge" },
  { key: "m", on: cockpit, say: "m flips back to the grid" },
  { look: glass, say: "The grid" },
  { key: "?", on: cockpit, say: "? lists every key" },
  { look: role("dialog", "Keys"), say: "The keys, by what they act on" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
  { key: "]", on: cockpit, say: "] steps to Done" },
  { look: tab("Done"), say: "Done: Jobs and Sessions that are over, the same tiles, the one picked opened beside them" },
  { key: "Alt+1", on: tab("Done"), say: "Option and a number picks a filter" },
  { look: inside(glass, role("option", /Shorten the reconnect wait/)), say: "Your move again: the call that was put off is a lit tile" },
]);

export { dashboardCockpit as "dashboard-cockpit" };
