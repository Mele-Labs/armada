// One side panel for a pull request, an issue, a bay and a Check, each opened from two places. The pull request is the panel a
// Job opens, filled from the forge's read, so it is whole for one nobody owns and the Job's acts are drawn only where a Job holds
// it; its Review tab has the Reviews page's tools. A press inside the panel that opens another thing puts Back at its head.
// Esc closes and focus is back on what opened it. Over `shared-side-panel`.

import { button, inside, place, region, role, tab, text, viaPalette, walk } from "../walk";

const queue = region("Merge queue");
const bays = region("Bays");
const jobs = region("Jobs", { exact: true });
const ledger = region("Attachments");
const controls = region("Flight controls");
const pull = (number: number) => role("dialog", `Pull request #${number}`);
const issue = (number: number) => role("dialog", `Issue #${number}`);
const bay = (name: string) => role("dialog", `Bay ${name}`);
const check = (name: string) => role("dialog", `Check ${name}`);

const sharedSidePanel = walk("shared-side-panel", [
  { look: queue, say: "Enter opens the focused row. Esc closes and returns focus. [ and ] move between tabs. 1 to 9 run a review tool" },

  // A pull request nobody here owns, from the merge queue.
  { press: inside(queue, button(/^#1899/)), say: "A pull request in the merge queue" },
  { look: region("Status"), say: "Its status plate, from the forge" },
  { look: inside(pull(1899), role("textbox", "Comment")), say: "The comment box is the forge's" },
  { key: "[", on: pull(1899), say: "[ wraps to the last tab" },
  { look: inside(pull(1899), role("group", "Review tools")), say: "Review, with the tools the Reviews page has" },
  { look: inside(pull(1899), button("Risks")), say: "The tools" },
  { key: "1", on: pull(1899), say: "1 runs the first tool" },
  { press: inside(role("dialog", "Risks"), button("Run")), say: "It asks first, since it costs" },
  { look: inside(pull(1899), region("Risks results")), say: "Its findings, here" },
  { key: "Escape", on: pull(1899), say: "Esc closes the panel" },
  { look: inside(queue, button(/^#1899/)), say: "Focus is back on the chip" },

  // The same pull request, from his review list.
  { press: place("Reviews"), say: "Reviews" },
  { press: inside(region("Pull requests"), button(/#1899/)), say: "A row of his review list" },
  { look: inside(pull(1899), role("group", "Review tools")), say: "The same panel, on its Review tab" },
  { key: "Escape", on: pull(1899), say: "Esc" },
  { press: place("Cockpit"), say: "Cockpit" },

  // A pull request a Job holds, from the timeline.
  { press: inside(controls, button("Timeline")), say: "Timeline" },
  { press: tab("Lanes"), say: "Lanes" },
  { press: inside(region("Timeline"), button(/^Checks failed: desktop_test/)), say: "A mark on the timeline" },
  { look: region("Failing checks"), say: "A Job holds it, so the failing checks can be sent back" },
  { look: inside(region("Failing checks"), button("Notify Job")), say: "Notify Job is drawn only here" },
  { key: "Escape", on: pull(1896), say: "Esc" },
  { press: inside(controls, button("Timeline")), say: "Back to the map" },

  // A bay, from the Bays list and from a Job's ledger.
  { press: inside(bays, button(/^slot-1, armada, Held/)), say: "A bay in the list" },
  { look: inside(bay("slot-1"), text("fleet/gate-policy-every-run")), say: "Its facts" },
  { key: "]", on: bay("slot-1"), say: "] is History" },
  { look: inside(bay("slot-1"), role("list", "History")), say: "Who held it" },
  { key: "Escape", on: bay("slot-1"), say: "Esc" },
  { press: inside(jobs, button("Gate policy on every run")), say: "The Job holding it" },
  { press: inside(ledger, button("Open Branch fleet/gate-policy-every-run")), say: "Its branch in the ledger" },
  { look: inside(bay("slot-1"), role("tab", "Facts")), say: "The same panel" },
  { key: "Escape", on: bay("slot-1"), say: "Esc" },

  // An issue, from a Job's thread and from a pull request's description.
  { press: inside(jobs, button("Cache key")), say: "A Job whose thread names an issue" },
  { press: inside(region(/^Job: Cache key/), role("link", /#2214/)), say: "The reference" },
  { look: inside(issue(2214), role("tab", "Conversation")), say: "An issue, in the pull request panel's shape" },
  { key: "]", on: issue(2214), say: "]" },
  { look: inside(issue(2214), role("list", "Pull requests")), say: "The pull requests that name it" },
  { key: "Escape", on: issue(2214), say: "Esc" },
  { press: inside(queue, button(/^#1892/)), say: "A pull request" },
  { press: inside(pull(1892), role("link", /#2214/)), say: "Its description closes the issue" },
  { look: inside(issue(2214), button(/^Back to #1892/)), say: "The same panel, with the way back" },
  { press: inside(issue(2214), button(/^Back to #1892/)), say: "Back" },
  { key: "Escape", on: pull(1892), say: "Esc" },

  // A Check, from a pull request's Checks tab and from the merge line.
  { press: inside(queue, button(/^#1899/)), say: "A pull request" },
  { key: "]", on: pull(1899), say: "]" },
  { press: inside(pull(1899), button("Open the log of test · bridge")), say: "A check row" },
  { look: inside(check("test · bridge"), button(/^Back to #1899/)), say: "A Check, with the way back" },
  { key: "]", on: check("test · bridge"), say: "]" },
  { look: inside(check("test · bridge"), region(/^Log of/)), say: "Its log" },
  { press: inside(check("test · bridge"), button(/^Back to #1899/)), say: "Back" },
  { key: "Escape", on: pull(1899), say: "Esc" },
  { later: role("navigation", "Places"), say: "Time passes" },
  { later: role("navigation", "Places"), say: "Main goes red" },
  ...viaPalette("Merge line", "The merge line"),
  { press: inside(inside(region("Merge line, armada"), role("list", "Recently landed")), button("ci failed on main, open its log")), say: "A failed run on main" },
  { look: inside(check("desktop_test"), role("list", "Runs")), say: "The same Check panel" },
  { key: "Escape", on: check("desktop_test"), say: "Esc" },
]);

export { sharedSidePanel as "shared-side-panel" };
