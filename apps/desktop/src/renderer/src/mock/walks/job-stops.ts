// How a row says a Job is working, running a command, on a subagent or stopped, what Now says of a stopped Job, and what Fleet did for it. Draft fields only. Over `jobStops`.

import { deckRow, inside, region, role, tab, text, walk } from "../walk";

const jobs = region("Jobs", {
  exact: true,
});
const now = role("tabpanel", "Now", {
  exact: true,
});
const open = role("tabpanel", "Open", {
  exact: true,
});
const done = region("Done", {
  exact: true,
});

const jobStops = walk("jobStops", [
  {
    hover: inside(jobs, role("img", "Running: Run the store tests")),
    say: "Its turn ended with a background command running. This row used to read as idle",
  },
  {
    hover: inside(jobs, role("img", "Subagent working")),
    say: "Its turn ended with a subagent at work",
  },
  {
    hover: inside(deckRow("Fix the merge queue retry"), role("img", /^Stopped: /)),
    say: "Stopped, and Fleet is still acting on it, so it is not amber",
  },
  {
    press: deckRow("Fix the merge queue retry"),
    say: "It ended its turn with two tasks open",
  },
  {
    look: inside(now, text("Stopped: ended its turn with tasks open")),
    say: "Now leads with the stop and names the tasks it left",
  },
  {
    look: inside(open, text("Nudge at")),
    say: "Open holds what Fleet will do next, with its time",
  },
  {
    press: tab("Done"),
    say: "Done",
  },
  {
    press: role("radio", "By kind"),
    say: "By kind",
  },
  {
    look: inside(
      done,
      region("Fleet", {
        exact: true,
      }),
    ),
    say: "Fleet is a kind of its own, newest first, each with its time. A reason shows on hover",
  },
  {
    press: deckRow("Phone review of everything"),
    say: "Fleet was down when its turn ended. Not amber, since Fleet is bringing it back",
  },
  {
    look: inside(now, text("Stopped: Fleet was down, 16:10 to 16:57")),
    say: "Now gives the stretch Fleet was down",
  },
  {
    press: deckRow(/^Tell the phone what holds a Job/),
    say: "Its process ended and the restart failed, so Fleet is repairing it. Nothing is asked of you",
  },
  {
    look: inside(now, text("Stopped: process ended")),
    say: "Now gives what the process said, and Fleet's acts stand beside it",
  },
  {
    look: inside(now, text("Repairing: its process died twice")),
    say: "The repair is no Job of its own: it is one line on this Job",
  },
  {
    hover: inside(deckRow("Store migration spike"), role("img", /^Stopped: /)),
    say: "Fleet is unhealthy, so it started no repair. Amber: this is the one that reaches you",
  },
  {
    press: deckRow("Theme token audit"),
    say: "Fleet told it once and it stopped again with its agent alive. Amber too: until a repair is asked for, only you move it",
  },
  {
    press: deckRow("Rewrite the importer"),
    say: "A turn in one call with nothing heard from it",
  },
  {
    look: inside(now, text(/^No word for \d+m$/)),
    say: "Now names the call and how long it has been silent",
  },
]);

export { jobStops };
