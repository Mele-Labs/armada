// The Operator panel, in Helm's place over the whole work, before any Fleet work is built (armada-ehhu.2). Over `operator-full`.
//
// No wire carries any of this yet. Each shape is typed in `operator/model.ts`, and is a proposal for the protocol:
// - `OperatorLogRow`: id, at, what started it (`OperatorSource`: you, a Job, a bug report and who filed it, Fleet, or Waiting to
//   start), item, did (answered, raised, delegated, existing, merged, in_touch, held, started, filed, passed_on, released,
//   returned, resumed, moved, overruled), why, every Job the ruling names, the row a hold put in Waiting to start, the outcomes
//   Overrule offers, and the one the person picked.
// - `OperatorLineRow`, a row of Waiting to start: id, title, what it waits for (`OperatorWait`: a Job and pushes or ends with its
//   plan group, a meter under a percentage, or the person), why, and the Job it was dispatched as.
// - `StandingOrder`: one of three enforced forms, a role off in a repository, a hold on paths until a Job pushes or ends, or a kind
//   always raised.
// - `OperatorRead`: a Job's question being read, or raised with its read and its pick.
// - `OperatorTie`: what a merge or keep in touch ruling left on a live Job's row: the other Job, the shared paths, and the plan
//   group that waits on the other's push.
// - The switch, a position (on, suggest, off) per role, and the daily ceiling on calls.
// - The conversation in the Job composer: messages from the person and the Operator. The mock answers from a script (`OperatorTalk`).

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Target } from "../walk";

const panel = role("complementary", "Operator", {
  exact: true,
});
const log = inside(panel, region("Log"));
const line = inside(panel, region("Waiting to start"));
const permissions = inside(panel, region("Permissions"));
const box = inside(panel, region("Talk to the Operator"));
const jobs = region("Jobs", {
  exact: true,
});
const needs = region("Needs you");
const logRow = (name: string | RegExp): Target => inside(log, role("listitem", name));
const lineRow = (name: string): Target =>
  inside(
    line,
    role("listitem", name, {
      exact: true,
    }),
  );
const card = (title: string): Target =>
  button(`Job ${title}`, {
    exact: true,
  });
const message = inside(box, role("textbox", "Message to the Operator"));

const BUG = "Bug report from Home Draws Desktop States: Job says Idle, but its agent crashed";
const POOL = "Map shows Jobs, but not the slots they hold";
const TESTS = "Tests take minutes, but build the whole workspace";

export const operatorFull = walk("operator-full", [
  {
    press: button("Operator", {
      exact: true,
    }),
    say: "The title row's button, Helm's renamed Operator, opens the panel over the whole work. Each pane wears the Cockpit HUD's frame",
  },
  {
    press: inside(panel, button(/^Close/)),
    say: "Close shuts it",
  },
  {
    key: "Meta+j",
    on: jobs,
    say: "⌘J opens it from anywhere",
  },
  {
    look: logRow(BUG),
    say: "The Log: every ruling, newest first. Each row starts with what started it. This bug report names the Job that filed it, as its card",
  },
  {
    look: inside(logRow(BUG), card("Stop Silent Job Deaths")),
    say: "Existing Job: a live Job's plan group covers it, so the words went there and no Job started. That Job is named as its card",
  },
  {
    look: logRow("You dispatched: Record shows the build, but not each step's time"),
    say: "Merged: two Jobs on the same work. The new one had pushed nothing, so it ended and its brief, plan and branch went to the other. Both are named",
  },
  {
    look: inside(
      logRow("You dispatched: Record shows the build, but not each step's time"),
      card("Time each build step"),
    ),
    say: "The Job merged in, ended",
  },
  {
    look: logRow("Store Reports Its Size: Plan recorded: size group"),
    say: "A recorded plan is ruled group by group. Kept in touch, naming the other Job and the shared path",
  },
  {
    look: logRow("Fleet: Job says Stopped, but 3 tasks are open"),
    say: "Delegated: a nudge to a Job Fleet marked Stopped. Fleet started this one",
  },
  {
    look: inside(logRow(`You dispatched: ${POOL}`), button(`Start now: ${POOL}`)),
    say: "Kept in touch, where the other Job builds first: its group waits on that push, with Start now",
  },
  {
    look: logRow("Bug report from you: Logbook search shows results, but drops the first letter"),
    say: "Filed, after a duplicate check. You filed this report",
  },
  {
    look: logRow("Rework the Map legend: Legend opens collapsed. Keep it that way?"),
    say: "When it goes wrong. Passed on: no ruling in time, so it went on as today",
  },
  {
    look: logRow("Waiting to start: Doctor lists new checks, but runs none"),
    say: "Released: the Job it waited on ended",
  },
  {
    look: logRow("Home Draws Desktop States: Job says Idle, but its agent crashed"),
    say: "Returned: the Job ended without touching it, and it was ruled again",
  },
  {
    look: logRow("Fleet restarted: Setting says pool, but means slots. Rename it?"),
    say: "Resumed: kept across a restart and ruled after it",
  },
  {
    press: inside(log, button("Overrule: Job says Idle, but its agent crashed")),
    say: "Overrule, on a row, lists the other outcomes: New Job, Merge, Keep in touch, Raise to me",
  },
  {
    press: role("menuitem", "New Job"),
    say: "A pick overrules the ruling",
  },
  {
    look: inside(logRow(BUG), text("Overruled by you: New Job")),
    say: "The row says so",
  },
  {
    look: logRow("You: Job says Idle, but its agent crashed"),
    say: "And the Log keeps it as your correction, which the next call reads",
  },
  {
    press: inside(logRow(BUG), card("Stop Silent Job Deaths")),
    say: "A Job's card opens it, and the panel steps aside",
  },
  {
    look: inside(needs, text(/circle-x reads apart from it/)),
    say: "Stop Silent Job Deaths, open",
  },
  {
    key: "Meta+j",
    on: jobs,
    say: "⌘J brings the panel back",
  },
  {
    look: lineRow(TESTS),
    say: "Waiting to start: what has not started, in the Operator's order",
  },
  {
    look: inside(lineRow(TESTS), card("Stop Silent Job Deaths")),
    say: "Each row says what it waits on, and the Job it waits on as its card",
  },
  {
    look: inside(line, text("Starts when the 5 hour window is under 80%")),
    say: "A meter holds this one",
  },
  {
    drag: lineRow("Raised items make no sound"),
    onto: lineRow(TESTS),
    say: "Drag a row to override the order. ⌥↑ and ⌥↓ do it from the keyboard",
  },
  {
    look: logRow("You: Raised items make no sound"),
    say: "The move goes in the Log as a correction",
  },
  {
    press: inside(line, button("Start now: Map key lists colours the Map no longer uses")),
    say: "Start now on any row overrides the wait in one press",
  },
  {
    look: logRow("You: Map key lists colours the Map no longer uses"),
    say: "Started, and logged",
  },
  {
    look: inside(permissions, button("Operator: On")),
    say: "Permissions: the Operator switch on top. Off sends everything to you as today",
  },
  {
    press: inside(permissions, button("Answer: On")),
    say: "Each role a name, what it does, and its position in one column",
  },
  {
    look: role("menuitem", "Suggest"),
    say: "On, Suggest or Off",
  },
  {
    press: role("menuitem", "Suggest"),
    say: "Suggest shows what it would do on a raised row, for one press",
  },
  {
    look: inside(permissions, text("Off in billing")),
    say: "A repository that differs says so under its role",
  },
  {
    look: inside(permissions, text("Raise and the destructive lock are always on")),
    say: "Raise has no control, and the destructive lock never moves",
  },
  {
    look: inside(permissions, role("spinbutton", "Daily ceiling")),
    say: "The daily ceiling on calls",
  },
  {
    look: inside(permissions, role("list", "Standing orders")),
    say: "Standing orders, each in a form Fleet enforces, each removed in one press",
  },
  {
    look: message,
    say: "The Job composer, the same one a Job has: @ for Jobs and files, # for issues and pull requests, / for commands, files and sketches, model and effort",
  },
  {
    type: "Why did you hold the tests Job?\n",
    into: message,
    say: "A short conversation",
  },
  {
    look: inside(box, text(/crates\/testkit\/src\/run\.rs/)),
    say: "Answered from the Log row: which plan, which paths, what it waits on",
  },
  {
    type: "Start it anyway\n",
    into: message,
    say: "A follow-up knows what it is about",
  },
  {
    look: logRow(`You: ${TESTS}`),
    say: "It does what Start now does, and logs it as your correction",
  },
  {
    key: "Meta+j",
    on: message,
    say: "⌘J shuts the panel",
  },
  {
    hover: inside(
      deckRow("Show build times in the Record"),
      role("img", /^Took in Time each build step/),
    ),
    say: "The merged Job's row is the one that stayed. Its glyph names the Job it took in and the shared paths",
  },
  {
    hover: inside(
      deckRow("Doctor Checks the Store"),
      role("img", /^Kept in touch with Store Reports Its Size/),
    ),
    say: "Two Jobs kept in touch: this row names the other, the shared path, and the group that waits on its push",
  },
  {
    hover: inside(
      deckRow("Store Reports Its Size"),
      role("img", /^Kept in touch with Doctor Checks the Store/),
    ),
    say: "And the other row names this one",
  },
  {
    hover: inside(
      deckRow("Tidy the Logbook filters"),
      role("img", "The Operator is reading its question"),
    ),
    say: "Its Job's row shows the Operator's glyph meanwhile, and no ask hue",
  },
  {
    press: deckRow("Tidy the Logbook filters"),
    say: "A question the Operator is reading",
  },
  {
    hover: inside(needs, role("img", /The Operator is reading this/)),
    say: "Its glyph, with a tooltip. It does not pulse until it is ruled",
  },
  {
    press: inside(needs, role("radio", "Keep them apart")),
    say: "You can answer it yourself at any moment",
  },
  {
    press: inside(
      needs,
      button("Answer", {
        exact: true,
      }),
    ),
    say: "Your answer wins. The ruling is dropped when it returns",
  },
  {
    press: deckRow("Stop Silent Job Deaths"),
    say: "A question the Operator raised",
  },
  {
    look: inside(needs, text(/circle-x reads apart from it/)),
    say: "Its read on the row",
  },
  {
    look: inside(needs, role("radio", "circle-x")),
    say: "And its pick, already selected",
  },
  {
    key: "Enter",
    on: inside(
      needs,
      button("Answer", {
        exact: true,
      }),
    ),
    say: "So Enter takes it",
  },
  {
    hover: inside(deckRow("Show the slot pool on the Map"), role("img", /^Held until /)),
    say: "A Job you dispatched whose group waits on another Job. Its row says what it waits for",
  },
  {
    press: deckRow("Show the slot pool on the Map"),
    say: "Opened",
  },
  {
    look: inside(
      role("group", "Held by the Operator"),
      text(/Rework the Map legend pushes its map group/),
    ),
    say: "Its header says so in one line, naming the other Job and the shared path",
  },
  {
    look: inside(role("group", "Held by the Operator"), button("Start now")),
    say: "With Start now beside it",
  },
  {
    key: "Alt+Meta+a",
    on: jobs,
    say: "The annotation layer",
  },
  {
    look: inside(role("status"), button("Dispatch")),
    say: "Its start button is Dispatch now, since the press may go to another Job or wait",
  },
  {
    key: "Alt+Meta+a",
    on: jobs,
    say: "The layer off again",
  },
]);
