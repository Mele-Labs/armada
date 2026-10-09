// Command Central as a cockpit. What is running is the glass, each Job and Session one tile; a call
// that needs the owner comes forward over it with the context to answer it, and is answered from the
// keyboard, the calls behind it stacked like a deck. Over `dashboard-cockpit`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const cockpit = region("Command Central");
const glass = role("listbox", "Running");
const behind = (kind: RegExp) => button(kind);
const call = (kind: RegExp) => region(kind);

const dashboardCockpit = walk("dashboard-cockpit", [
  { look: cockpit, say: "Everything running, one tile each. What is active is the glyph, and a tile that asks is lit" },
  { hover: inside(glass, role("img", "Drone working")), say: "A glyph names its state on hover" },
  { key: "j", on: cockpit, say: "j and k move along the glass, the arrows too" },
  { look: inside(glass, button("Open")), say: "The one under the cursor carries its keys: Enter opens, x kills" },
  { later: cockpit, say: "Time passes: a Drone reaches a question" },
  { look: call(/^Plan question/), say: "It comes forward over the glass: the Job, what he asked for, who asked and where, what is active, then the question" },
  { key: "e", on: cockpit, say: "e opens the whole request" },
  { look: inside(call(/^Plan question/), button(/Less/)), say: "The request, in full" },
  { key: "2", on: cockpit, say: "A number picks an answer" },
  { look: inside(call(/^Plan question/), role("radio", "Wrap it in place")), say: "Picked, and not sent" },
  { later: cockpit, say: "A second call arrives while the first is open" },
  { look: behind(/^Check failed/), say: "It stands behind the open one, its title on its edge" },
  { key: "b", on: cockpit, say: "b is a standing answer: make the best decision" },
  { look: inside(call(/^Plan question/), role("radio", /Make the best decision/)), say: "Under the numbered ones, with what it tells the agent on hover" },
  { key: "Enter", on: cockpit, say: "Enter sends the answer" },
  { look: call(/^Check failed/), say: "The next call slides up from behind" },
  { later: cockpit, say: "A Session stops to ask for a command" },
  { look: behind(/^Session/), say: "Behind it, in its turn" },
  { key: "l", on: cockpit, say: "l puts this one at the back of the stack" },
  { look: call(/^Session/), say: "The Session's call: a small view of the Session, and what it asks" },
  { look: text("The store test is flaky on CI"), say: "What he told it, and what it did since" },
  { key: "1", on: cockpit, say: "1 is Allow" },
  { key: "Enter", on: cockpit, say: "Enter sends it" },
  { look: behind(/^Check failed/), say: "The one put off waits at the back, dashed" },
  { key: "w", on: cockpit, say: "w brings it forward" },
  { look: call(/^Check failed/), say: "Back in front" },
  { key: "g", on: cockpit, say: "g is the other standing answer: just get it done" },
  { look: inside(call(/^Check failed/), role("radio", /Just get it done/)), say: "Picked" },
  { key: "Escape", on: cockpit, say: "Esc puts it off again" },
  { look: glass, say: "Nothing in front: the glass, with the call still waiting at the bottom edge" },
  { key: "?", on: cockpit, say: "? lists every key" },
  { look: role("dialog", "Keys"), say: "The keys, by what they act on" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
  { key: "]", on: cockpit, say: "] and [ switch the Dashboard's tabs" },
  { look: tab("Running"), say: "Running" },
  { key: "Alt+1", on: tab("Running"), say: "Option and a number picks a tab" },
  { look: cockpit, say: "Back on Command Central" },
]);

export { dashboardCockpit as "dashboard-cockpit" };
