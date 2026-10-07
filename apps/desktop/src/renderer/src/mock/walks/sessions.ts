// A Session: a raw conversation with a hosted agent, beside Helm, with Armada's
// features hooked in. Drawn in Bridge itself over a mock Fleet, ahead of any
// Fleet work: what Fleet would owe is listed in
// `packages/screens/src/draft/sessions.ts`. The owner's ask, 6 Oct 2026, and
// his notes on the first walk, 7 Oct.

import { button, inside, role, region, text, walk } from "../walk";

const ledger = region("Attachments");
const sessions = region("Sessions");
const thread = region("Thread");
const message = role("textbox", "Message");
const owner = role("group", "Owned by Flaky store test");
const rail = (name: string) => role("button", name, { exact: true });
const words = "Fix the flaky store test. Dispatch the sleep cleanup as Jobs and have subagents read the CI history.";

export const sessionsWalk = walk("sessions", [
  { press: inside(sessions, button("New Session")), say: "A new Session starts blank" },
  { look: ledger, say: "Every section is there, dim, and empty" },
  { type: "opus", into: role("combobox", "Model"), say: "Model, from the same list Dispatch offers" },
  { type: "high", into: role("combobox", "Effort"), say: "Effort, as a Drone's settings have it" },
  { type: "/", into: message, say: "A slash opens the skills and commands" },
  { look: role("listbox", "Skills and commands"), say: "Skills first, then commands" },
  { press: role("option", "/simplify"), say: "One chosen" },
  { type: "@Rel", into: message, say: "An at sign opens the other Sessions" },
  { look: role("listbox", "Sessions"), say: "Narrowed as it is typed" },
  { press: role("option", "Release notes script"), say: "Tagged, so the agent can talk to it" },
  { press: button("Attach sketch"), say: "A sketch is shared from the ones Dispatch and Studios hold" },
  { press: role("option", "Store clock"), say: "Attached" },
  { paste: message, say: "A screenshot pasted in" },
  { look: inside(role("group", "Attached"), text("Screenshot")), say: "It waits as a chip until it is sent" },
  { type: words, into: message, say: "A raw message, no workflow" },
  { press: button("Send"), say: "The agent reads first" },
  { look: inside(thread, role("img", /Screenshot/)), say: "What was sent goes with the message" },
  { look: region("Leased on first write"), say: "Its first write leased a worktree slot and cut a branch, in the thread where it happened" },
  { look: inside(ledger, role("listitem", "Worktree slot 3")), say: "The ledger took the slot" },
  { look: inside(ledger, role("listitem", "Branch fix/flaky-store")), say: "And the branch" },
  { look: inside(ledger, role("listitem", "Sketch Store clock")), say: "The sketch that was shared" },
  { later: ledger, say: "The agent carries on" },
  { look: inside(ledger, role("listitem", "Job 52")), say: "A Job dispatched from the Session, with its own slot" },
  { look: inside(ledger, role("listitem", "Job 53")), say: "A second Job. A Session holds several of each" },
  { look: inside(ledger, role("listitem", "Subagent Read the CI history")), say: "A subagent, running, then done" },
  { look: inside(ledger, role("listitem", "Studio Flaky store shapes")), say: "A Studio it opened" },
  { look: inside(ledger, role("listitem", "Sketch CI history")), say: "A sketch the agent published" },
  { later: ledger, say: "The agent opens a pull request" },
  { look: inside(ledger, role("listitem", "Pull request #1843, checks failed")), say: "Checks ran and failed on it" },
  { press: button("Open Pull request #1843"), say: "Every row opens what it names" },
  { look: role("dialog", "Pull request #1843"), say: "The pull request, with its Checks" },
  { press: inside(role("dialog", "Pull request #1843"), button("Close")), say: "Back to the Session" },
  { press: rail("Overview"), say: "Back to Overview" },
  { type: "#1843", into: role("searchbox", "Search Sessions"), say: "A pull request number" },
  { look: inside(sessions, role("listitem", "Flaky store test")), say: "It finds the Session that owns it: slot, pull requests with their Checks, Jobs, and a mark" },
  { look: inside(sessions, text("Needs you")), say: "Headed as Overview's lists are" },
  { press: rail("Cleanup"), say: "Cleanup holds the slots" },
  { hover: button("Branch fix/52-pin-store-clock"), say: "A branch chip anywhere names its owner" },
  { press: button("Branch fix/52-pin-store-clock"), say: "A press keeps the card up" },
  { look: owner, say: "Title, state, slot, pull requests, Jobs and last turn" },
  { press: inside(owner, button("Open Session")), say: "It opens the Session" },
  { later: thread, say: "Another Session writes to this one" },
  { look: region("Message from Release notes script"), say: "Set apart, with the sender's band" },
  { look: text("Replied to s2"), say: "The message woke it: it took a turn and replied" },
  { look: role("article", "Waiting on you"), say: "A permission it is held on, answered here" },
  { press: button("Open Session Release notes script"), say: "The sender is a press" },
  { look: region("Session s2"), say: "It opens the Session that wrote" },
]);

// Named as the link spells it, `?walk=sessions`.
export { sessionsWalk as "sessions" };
