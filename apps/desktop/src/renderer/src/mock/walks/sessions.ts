// A Session: a raw conversation with a hosted agent, beside Helm, with Armada's
// features hooked in. This walk is the Session itself, from a blank start. The
// other three are the surfaces that lead into one: `worktree-to-sessions`,
// `job-to-sessions` and `piloting`. Drawn in Bridge over a mock Fleet, ahead of
// any Fleet work: what Fleet would owe is listed in
// `packages/screens/src/draft/sessions.ts`. Told twice: wide, and below the
// breakpoint, where the ledger folds into a sheet.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, region, text, walk } from "../walk";
import type { Step } from "../walk";

const words = "Fix the flaky store test. Dispatch the sleep cleanup as Jobs and have subagents read the CI history.";

function steps(narrow: boolean): Step[] {
  const { ledger, message, thread, rail, sheet, close, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  const mode = role("combobox", "Permission mode");
  const inLedger = (name: string, say: string): Step => ({ look: inside(ledger, role("listitem", name)), say });
  return [
    { press: inside(sessions, button("New Session")), say: "A new Session starts blank" },
    ...opened([{ look: inside(ledger, role("img", "Nothing attached")), say: "Nothing on it yet: a small picture, and no sections" }]),
    { look: mode, say: narrow ? "The message box is one row: each select a glyph and its value" : "It runs in auto mode" },
    { hover: mode, say: "The mode is on the message box's one row, with what it does on hover" },
    ...(narrow ? [{ look: button("Attachments"), say: "The ledger is a button in the header" } as Step] : []),
    { type: "plan", into: mode, say: "Ask, auto, accept edits, plan: the terminal's modes" },
    { type: "auto", into: mode, say: "Back to auto" },
    { type: "opus", into: role("combobox", "Model"), say: "Model, from the same list Dispatch offers" },
    { type: "high", into: role("combobox", "Effort"), say: "Effort, as a Drone's settings have it" },
    { type: "/", into: message, say: "A slash opens the skills and commands" },
    { look: role("listbox", "Skills and commands"), say: "Skills first, then commands" },
    { press: role("option", "/simplify"), say: "One chosen" },
    { type: "@", into: message, say: "An at sign opens what can be tagged, grouped by kind" },
    { look: role("group", "Sessions"), say: "Other Sessions are in it, beside Jobs, pull requests and branches" },
    { press: role("option", "Release notes script"), say: "Tagged, so the agent can talk to it" },
    { press: button("Draw sketch"), say: "A sketch is drawn, on the pad Dispatch draws on" },
    { look: sheet("Draw a sketch"), say: "The pad opens over the Session" },
    { press: inside(sheet("Draw a sketch"), button("Add a box")), say: "A box" },
    { type: "pin the clock", into: role("textbox", "The words in this box"), say: "With words in it" },
    { press: inside(sheet("Draw a sketch"), button("Attach sketch")), say: "Attached to the message" },
    { paste: message, say: "A screenshot pasted in" },
    { look: inside(role("group", "Attached"), text("Screenshot")), say: "It waits in the row, which scrolls and takes nothing from the typing area" },
    { type: words, into: message, say: "A raw message, no workflow" },
    { press: button("Send"), say: "The agent reads first" },
    { look: inside(thread, role("img", /Screenshot/)), say: "What was sent goes with the message" },
    { look: region("Leased on first write"), say: "Its first write leased a worktree slot and cut a branch, in the thread where it happened" },
    ...opened([
      inLedger("Worktree slot 3", "The ledger took the slot"),
      inLedger("Branch fix/flaky-store", "And the branch"),
      inLedger("Sketch pin the clock", "The sketch that was drawn"),
    ]),
    { later: thread, say: "The agent carries on" },
    ...opened([
      inLedger("Job 52", "A Job dispatched from the Session, with its own slot"),
      inLedger("Job 53", "A second Job. A Session holds several of each"),
      inLedger("Subagent Read the CI history", "A subagent, running, then done"),
      inLedger("Studio Flaky store shapes", "A Studio it opened"),
      inLedger("Sketch CI history", "A sketch the agent published"),
    ]),
    { later: thread, say: "The agent opens a pull request" },
    ...opened([inLedger("Pull request #1843, checks failed", "Checks ran and failed on it")]),
    ...row("Open Pull request #1843", "Every row opens what it names"),
    { look: sheet("Pull request #1843"), say: "The pull request, with its Checks" },
    { look: role("group", "Pull request acts"), say: "Red: no merge, and a review is still there" },
    { press: close(sheet("Pull request #1843")), say: "Back to the Session" },
    { later: thread, say: "Another Session writes to this one" },
    { look: region("Message from Release notes script"), say: "Set apart, with the sender's band" },
    { look: text("Replied to s2"), say: "The message woke it: it took a turn and replied" },
    { look: role("article", "Waiting on you"), say: "Auto asks only about a push, and this is one" },
    { press: button("Open Session Release notes script"), say: "The sender is a press" },
    { look: region("Session s2"), say: "It opens the Session that wrote" },
    ...row("Open Pull request #1847", "A pull request that has passed"),
    { press: inside(sheet("Pull request #1847"), button("Merge")), say: "Merge, without the browser" },
    { look: inside(sheet("Pull request #1847"), text("Merged")), say: "Merged, and no acts left" },
    { press: close(sheet("Pull request #1847")), say: "Back to the Session" },
    { press: rail("Sessions"), say: "Two more pull requests, in another Session" },
    { press: inside(sessions, button("Store migration spike")), say: "The Session holding them" },
    ...row("Open Pull request #1849", "A draft"),
    { press: inside(sheet("Pull request #1849"), button("Ready for review")), say: "Ready for review, from here" },
    { look: inside(sheet("Pull request #1849"), button("Enable auto-merge")), say: "Its Checks are still running, so auto-merge is offered" },
    { press: close(sheet("Pull request #1849")), say: "Back to the Session" },
    ...row("Open Pull request #1850", "Checks running"),
    { press: inside(sheet("Pull request #1850"), button("Enable auto-merge")), say: "Auto-merge asked for" },
    { look: inside(sheet("Pull request #1850"), button("Auto-merge on")), say: "It merges when every Check has passed" },
  ];
}

const sessionsWide = walk("sessions", steps(false));
const sessionsNarrow = walk("sessions", steps(true), NARROW);

export { sessionsWide as "sessions", sessionsNarrow as "sessions-narrow" };
