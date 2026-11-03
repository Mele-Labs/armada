// How Bridge says a Job is working, waiting or stopped, and what Fleet did for it. Job 1 stopped with two tasks open and has a nudge to come; Job 3 stopped
// because Fleet was down; Job 2's process ended and its restart failed; Job 5 is silent on a call. Draft fields only. Over `jobStops`.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";

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
    hover: inside(deckRow("Fix the merge queue retry"), role("img", /^Stopped: /)),
    say: "The mark on a stopped Job. Its card gives the reason",
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
    press: deckRow("Tell the phone what holds a Job"),
    say: "Its process ended and the restart failed, so it is amber",
  },
  {
    press: button("Needs you", {
      exact: true,
    }),
    say: "Needs you holds one answer. Carry on sends the Job a word",
  },
  {
    look: inside(now, text("Stopped: process ended")),
    say: "Now gives what the process said",
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
