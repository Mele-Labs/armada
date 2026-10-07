// A long Session's ledger with Open | All in the head of its Pull requests, Jobs and Subagents
// sections: Open by default, All adds the finished, and each section keeps its own choice.
// Over the `ledger-filter` scenario. Told twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, rail } = kit(narrow);
  const sessions = region("Sessions");
  const section = (name: string) => inside(ledger, region(name));
  const choice = (name: string, one: "Open" | "All") => inside(section(name), role("radio", one, { exact: true }));
  return [
    { press: inside(sessions, button("Retire the sleeps")), say: "A long Session: eleven merged pull requests, six open, and a crowd of subagents" },
    ...opened([
      { look: choice("Pull requests", "Open"), say: "Pull requests open on Open: the open and draft ones, and the merged are out of the way" },
      { look: choice("Subagents", "Open"), say: "Subagents on Open show the two still running" },
      { look: choice("Jobs", "Open"), say: "Jobs on Open show the one not yet ended" },
      { press: choice("Pull requests", "All"), say: "All on Pull requests adds the merged ones" },
      { look: inside(section("Pull requests"), role("button", /Open Pull request #1901/)), say: "A merged pull request, here under All" },
      { look: choice("Subagents", "Open"), say: "Subagents kept its own choice: still Open" },
      { press: choice("Subagents", "All"), say: "All on Subagents adds the finished ones" },
      { press: choice("Pull requests", "Open"), say: "Back to Open on Pull requests" },
    ]),
    { press: rail("Sessions"), say: "Away to the list" },
    { press: inside(sessions, button("Retire the sleeps")), say: "And back to the Session" },
    ...opened([{ look: choice("Subagents", "All"), say: "Subagents is still on All: each section remembers its own choice" }]),
    { press: rail("Sessions"), say: "Away to the list" },
    { press: inside(sessions, button("Settled work")), say: "A Session whose pull requests are all merged" },
    ...opened([{ look: choice("Pull requests", "Open"), say: "The section keeps its head and the toggle, so All is reachable, and nothing under it" }]),
  ];
}

const wide = walk("ledger-filter", steps(false));
const narrow = walk("ledger-filter", steps(true), NARROW);

export { wide as "ledger-filter", narrow as "ledger-filter-narrow" };
