// Piloting a Job from a Session. A person takes over a Job that stopped at its
// gate: its Drone is already gone, and its worktree is handed to a new Session
// rather than leased. The Job reads piloted on the Board and on its detail, the
// agent and the person fix it, and the person ends the pilot through one of
// three exits, drawn on the Session's ledger row and on Job detail alike. The
// owner's decision of 7 Oct 2026: a piloted session is a Session, and the exits
// live in both places. `docs/concepts/pilot.md` says the exits are a panel on
// the Job only, and was written for a raw terminal.
//
// It is not "Open in a Session": that leaves the Drone working, this stops it.

import { putOff, pick } from "../walk-steps";
import { button, inside, role, region, text, walk } from "../walk";
import type { Step } from "../walk";

const rail = (name: string) => role("button", name, { exact: true });
const message = role("textbox", "Message");
const detail = (narrow: boolean) => (narrow ? role("dialog", "Attachments") : region("Attachments"));

/**
 * Both windows tell one story. Below the breakpoint the ledger is a sheet, so
 * each look at it is bracketed by opening and closing it.
 */
function pilotSteps(narrow: boolean): Step[] {
  const ledger = detail(narrow);
  const opened = (steps: Step[]): Step[] =>
    narrow
      ? [
          { press: button("Attachments"), say: "The ledger, opened" },
          ...steps,
          { press: inside(ledger, role("button", /^Close( panel)?( Esc)?$/)), say: "Back to the conversation" },
        ]
      : steps;
  const dialog = role("dialog", "Pilot this Job?");
  return [
    ...putOff(2),
    ...pick(/Cap the retry backoff/, "A Job stopped at its gate, picked on Active, its acts in the pane"),
    { look: rail("Pilot"), say: "On a Job that stopped at its gate, Pilot is the main act, and Redirect has stepped into the menu" },
    { press: rail("Pilot"), say: "Pilot stops the Drone and takes the worktree" },
    { look: dialog, say: "What each outcome does to the Drone and to the worktree" },
    { look: inside(dialog, role("radio", "Take Over")), say: "Take Over, chosen" },
    { look: inside(dialog, role("radio", "Restart Step")), say: "Restart Step gives the worktree to a new Drone afterwards" },
    { look: inside(dialog, role("radio", "Assist")), say: "Assist is drawn and off" },
    { look: inside(dialog, text("Coming soon.")), say: "Marked coming soon" },
    { press: inside(dialog, button("Pilot", { exact: true })), say: "It opens a Session on the Job" },
    { look: region("Handed over: Job 55"), say: "The Session starts with what Fleet knew when the Drone stopped, as fields" },
    { look: inside(region("Handed over: Job 55"), text("Judge refused")), say: "The step, the attempts, the Judge's refusals" },
    { look: inside(region("Handed over: Job 55"), text("Plan against diff")), say: "What was declared against what was written" },
    { look: inside(region("Handed over: Job 55"), text("Blocked by")), say: "And the Drone's own account: trying to, blocked by, tried" },
    ...opened([
      { look: inside(ledger, role("img", "Handed over with Job 55, not leased")), say: "The slot is the Job's own, handed over and not leased" },
      { look: inside(ledger, role("img", "Handed over with Job 55, not created here")), say: "And so is the branch" },
      { look: inside(ledger, role("img", /^Piloted/)), say: "The Job is marked piloted on the ledger" },
      { look: inside(ledger, role("group", "Ways out of the pilot")), say: "The three ways out, on the Job's row" },
    ]),
    { type: "cap the loop at five attempts", into: message, say: "The fix is worked with the agent" },
    { press: button("Send"), say: "It reads the loop and edits it" },
    { look: text("clippy is clean"), say: "Clippy is clean on the worktree" },
    { press: rail("Cockpit"), say: "Back to the Dashboard" },
    { press: role("tab", "Active"), say: "A piloted Job is still running" },
    { press: role("option", /Cap the retry backoff, Job/), say: "It reads piloted" },
    { hover: button("Job 55"), say: "The Session that has it is named, and its card is on hover" },
    { press: button("Job 55"), say: "A press keeps the card up" },
    { look: role("group", "Owned by Cap the retry backoff"), say: "Its title, slot, state and last turn" },
    { press: inside(role("group", "Owned by Cap the retry backoff"), button("Open Session")), say: "Back in the Session" },
    ...(narrow ? [{ press: button("Attachments"), say: "The ledger, opened" } as Step] : []),
    { press: inside(ledger, button("Open Job 55, piloted")), say: "The Job's row opens its detail" },
    { look: role("group", "Ways out of the pilot"), say: "Job detail has the same three exits" },
    { press: button("Submit for verification"), say: "The work still fits the plan, so it goes for verification" },
    { press: rail("Sessions"), say: "Back to the Sessions" },
    { press: button("Cap the retry backoff"), say: "The Session that piloted it" },
    { look: text("The slot is back with the Job"), say: "The Job went back to running, its gates ran, and the slot went back with it" },
    ...opened([{ look: inside(ledger, role("listitem", "Job 55")), say: "The ledger row settled: no pilot mark, no exits" }]),
    { press: rail("Cockpit"), say: "A running Job can be piloted too" },
    { later: rail("Cockpit"), say: "Jobs another Session dispatched start running" },
    { press: role("tab", "Active"), say: "Running" },
    { press: role("option", /Retire sleep calls in the store tests, Job/), say: "Picked" },
    { press: button("More for Retire sleep calls in the store tests"), say: "From its menu" },
    { look: role("menuitem", "Pilot"), say: "Pilot is secondary on a running Job: one slot, two fills" },
  ];
}

const sessionsPilot = walk("sessions", pilotSteps(false));
const sessionsPilotNarrow = walk("sessions", pilotSteps(true), { width: 900, height: 900 });

export { sessionsPilot as "piloting", sessionsPilotNarrow as "piloting-narrow" };
