// A Session: a raw conversation with a hosted agent, beside Helm, with Armada's
// features hooked in. Drawn in Bridge itself over a mock Fleet, ahead of any
// Fleet work: what Fleet would owe is listed in
// `packages/screens/src/draft/sessions.ts`. The owner's ask, 6 Oct 2026.

import { button, inside, role, region, text, walk } from "../walk";

const ledger = region("Attachments");
const sessions = region("Sessions");
const owner = role("group", "Owned by Flaky store test");
const rail = (name: string) => role("button", name, { exact: true });
const message = "Fix the flaky store test. Dispatch the sleep cleanup as Jobs and have subagents read the CI history.";

export const sessionsWalk = walk("sessions", [
  { press: inside(sessions, button("New Session")), say: "A new Session starts blank" },
  { look: ledger, say: "No slot and no branch, and nothing attached" },
  { type: message, into: role("textbox", "Message"), say: "A raw message, no workflow" },
  { press: button("Send"), say: "The agent reads first" },
  { look: region("Leased on first write"), say: "Its first write leased slot 3 and cut a branch, in the thread where it happened" },
  { look: inside(ledger, role("listitem", "Slot 3")), say: "The ledger took the slot" },
  { look: inside(ledger, role("listitem", "Branch fix/flaky-store")), say: "And the branch" },
  { later: ledger, say: "The agent carries on" },
  { look: inside(ledger, role("listitem", "Job 52")), say: "A Job dispatched from the Session, with its own slot" },
  { look: inside(ledger, role("listitem", "Job 53")), say: "A second Job. A Session holds several of each" },
  { look: inside(ledger, role("listitem", "Subagent Read the CI history")), say: "A subagent, running, then done" },
  { look: inside(ledger, role("listitem", "Studio Flaky store shapes")), say: "A Studio it opened" },
  { later: ledger, say: "The agent opens a pull request" },
  { look: inside(ledger, role("listitem", "Pull request #1843, checks failed")), say: "Checks ran and failed on it" },
  { press: rail("Overview"), say: "Back to Overview" },
  { type: "#1843", into: role("searchbox", "Search Sessions"), say: "A pull request number" },
  { look: inside(sessions, role("listitem", "Flaky store test")), say: "It finds the Session that owns the pull request, and shows what matched" },
  { press: rail("Cleanup"), say: "Cleanup holds the slots" },
  { hover: button("Branch fix/52-pin-store-clock"), say: "A branch chip anywhere names its owner" },
  { press: button("Branch fix/52-pin-store-clock"), say: "A press keeps the card up" },
  { look: owner, say: "Title, slot, pull requests, Jobs and last turn" },
  { press: inside(owner, button("Open Session")), say: "It opens the Session" },
  { later: region("Thread"), say: "Another Session writes to this one" },
  { look: region("Message from Release notes script"), say: "The message names the Session that sent it" },
  { look: text("Replied to s2"), say: "The message woke it: it took a turn and replied" },
  { look: role("article", "Waiting on you"), say: "A permission it is held on, answered here" },
]);

// Named as the link spells it, `?walk=sessions`.
export { sessionsWalk as "sessions" };
