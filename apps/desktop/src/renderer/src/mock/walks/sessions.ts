// A Session: a raw conversation with a hosted agent, beside Helm, with Armada's
// features hooked in. This walk is the Session itself, from a blank start. The
// other is the surface that leads into one: `worktree-to-sessions`. Drawn in Bridge over a mock Fleet, ahead of
// any Fleet work: what Fleet would owe is listed in
// `packages/screens/src/draft/sessions.ts`. Told twice: wide, and below the
// breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, dialog, inside, role, region, tab, text, walk } from "../walk";
import type { Step } from "../walk";

const words = "Fix the flaky store test and have subagents read the CI history.";

function steps(narrow: boolean): Step[] {
  const { artifacts, key, thread, rail, sheet, close, opened, row } = kit(narrow);
  const bays = region("Bays", { exact: true });
  const sessions = region("Jobs", { exact: true });
  const mode = role("combobox", "Permission mode");
  const request = inside(dialog("Dispatch"), role("textbox", "Request"));
  const listed = (group: string, name: string | RegExp, say: string): Step => ({ look: inside(inside(artifacts, region(group, { exact: true })), button(name)), say });
  return [
    { press: inside(role("region", "Dispatch controls"), button("Dispatch")), say: "A new Job starts from Dispatch" },
    { press: inside(dialog("Dispatch"), tab("Job")), say: "Job" },
    { type: words, into: request, say: "A raw request, no workflow" },
    { press: role("radio", "Repository armada"), say: "In armada" },
    { key: "Enter", on: request, say: "The Job opens and the agent reads first" },
    { look: mode, say: "It runs in auto mode" },
    { look: region("Slot leased"), say: "Its first write leased a worktree slot and cut a branch, in the thread where it happened" },
    { press: key("Bays"), say: "Bays" },
    { look: inside(bays, button("Show slot-3 in Bays")), say: "The slot it leased" },
    { press: inside(bays, button("Back")), say: "Back to the thread" },
    { later: thread, say: "The agent carries on" },
    ...opened([
      listed("Subagents", "Read the CI history of store_flaky", "A subagent, running, then done"),
      listed("Studios", "Flaky store shapes", "A Studio it opened"),
      listed("Sketches", "CI history of store_flaky", "A sketch the agent published"),
    ]),
    { later: thread, say: "The agent opens a pull request" },
    ...opened([listed("Pull requests", /^#1843 /, "Checks ran and failed on it")]),
    ...row(/^#1843 /, "Every row opens what it names"),
    { look: sheet("Pull request #1843"), say: "The pull request, with its Checks" },
    { look: role("group", "Pull request acts"), say: "Red: no merge" },
    { press: close(sheet("Pull request #1843")), say: "Back to the Job" },
    { later: thread, say: "Another Job writes to this one" },
    { look: region("Message from Release notes script"), say: "Set apart, with the sender's band" },
    { look: text("Replied to s2"), say: "The message woke it: it took a turn and replied" },
    { look: region("Slot released"), say: "The slot given back, in the style of the lease: its number and the branch it was on" },
    { look: role("article", "Waiting on you"), say: "The permission opens in the Needs you panel. Auto asks only about a push, and this is one" },
    { press: button("Open Job Release notes script"), say: "The sender is a press" },
    { look: region("Job s2"), say: "It opens the Job that wrote" },
    ...row(/^#1847 /, "A pull request that has passed"),
    { press: inside(sheet("Pull request #1847"), button("Merge")), say: "Merge, without the browser" },
    { look: inside(sheet("Pull request #1847"), text("Merged")), say: "Merged, and no acts left" },
    { press: close(sheet("Pull request #1847")), say: "Back to the Job" },
    { press: rail("Sessions"), say: "Two more pull requests, in another Job" },
    { press: inside(sessions, button("Store migration spike")), say: "The Job holding them" },
    ...row(/^#1849 /, "A draft"),
    { press: inside(sheet("Pull request #1849"), button("Ready for review")), say: "Ready for review, from here" },
    { look: inside(sheet("Pull request #1849"), button("Enable auto-merge")), say: "Its Checks are still running, so auto-merge is offered" },
    { press: close(sheet("Pull request #1849")), say: "Back to the Job" },
    ...row(/^#1850 /, "Checks running"),
    { press: inside(sheet("Pull request #1850"), button("Enable auto-merge")), say: "Auto-merge asked for" },
    { look: inside(sheet("Pull request #1850"), button("Auto-merge on")), say: "It merges when every Check has passed" },
  ];
}

const sessionsWide = walk("sessions-on-a-workflow", [toSessions, ...steps(false)]);
const sessionsNarrow = walk("sessions-on-a-workflow", [toSessions, ...steps(true)], NARROW);

export { sessionsWide as "sessions", sessionsNarrow as "sessions-narrow" };
