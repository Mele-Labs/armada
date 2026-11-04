// The Operator panel, in Helm's place over the whole work, before any Fleet work is built (armada-ehhu.2). Over `operator-full`.
// It stands on the panes a Job's area uses: tabs and splits he adds, closes, moves and resizes, kept as a Job's layout is.
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
// - The conversation in Talk: messages from the person and the Operator. The mock answers from a script (`OperatorTalk`).

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Target } from "../walk";

const panel = role("complementary", "Operator", {
  exact: true,
});
const tabOf = (name: string): Target =>
  inside(
    panel,
    role("tab", name, {
      exact: true,
    }),
  );
const body = (name: string): Target =>
  inside(
    panel,
    role("tabpanel", name, {
      exact: true,
    }),
  );
const log = body("Log");
const line = body("Waiting to start");
const permissions = body("Permissions");
const talk = body("Talk");
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
const chip = (title: string): Target =>
  button(`Open ${title}`, {
    exact: true,
  });
const message = inside(talk, role("textbox", "Message to the Operator"));

const BUG = "Bug report from Home Draws Desktop States: Job says Idle, but its agent crashed";
const POOL = "Map shows Jobs, but not the slots they hold";
const TESTS = "Tests take minutes, but build the whole workspace";

export const operatorFull = walk("operator-full", [
  {
    press: button("Operator", {
      exact: true,
    }),
    say: "The title row's button, Helm's renamed Operator, opens the panel over the whole work, on the panes a Job's area uses",
  },
  {
    look: inside(panel, role("tablist", "Tabs")),
    say: "Light to begin with: one pane, Talk and the Log as its tabs, nothing else open",
  },
  {
    // The dock's own Close, not a tab's "Close Talk".
    press: inside(panel, button(/^Close( ?⌘ ?J)?$/)),
    say: "Close shuts it",
  },
  {
    key: "Meta+j",
    on: jobs,
    say: "⌘J opens it from anywhere",
  },
  {
    context: tabOf("Log"),
    say: "Each tab has a Job tab's menu",
  },
  {
    press: role("menuitem", "Split down"),
    say: "Split down opens an empty pane under it",
  },
  {
    press: inside(
      region("Empty pane"),
      button("Add tab", {
        exact: true,
      }),
    ),
    say: "Its + offers the Operator's tabs not open yet, from the place a Job adds one",
  },
  {
    press: role("menuitem", "Waiting to start"),
    say: "Waiting to start, in the new pane",
  },
  {
    press: inside(
      region("Pane: Waiting to start"),
      button("Add tab", {
        exact: true,
      }),
    ),
    say: "The same + adds Permissions",
  },
  {
    press: role("menuitem", "Permissions"),
    say: "Permissions joins it as a tab",
  },
  {
    drag: tabOf("Permissions"),
    onto: inside(region("Pane: Log"), role("tablist", "Tabs")),
    say: "Dragged onto the first pane's strip, Permissions moves in as a tab beside Talk and the Log. Panes resize from the line between them",
  },
  {
    look: inside(panel, button(/^Layout: Default/)),
    say: "The layout is kept, as a Job's is. The Layout menu saves it under a name or resets it",
  },
  {
    press: tabOf("Log"),
    say: "The Log",
  },
  {
    look: logRow(BUG),
    say: "Every ruling, newest first, one line each: what started it, the item, the ruling, the Job it names, how long ago, and Overrule",
  },
  {
    look: inside(logRow(BUG), chip("Home Draws Desktop States")),
    say: "What started it: a bug report, and the Job that filed it as its chip",
  },
  {
    look: inside(logRow(BUG), chip("Stop Silent Job Deaths")),
    say: "Existing Job: a live Job's plan group covers it, so the words went there and no Job started. That Job, as its chip",
  },
  {
    hover: inside(logRow(BUG), text("Existing Job")),
    say: "Why, on the ruling",
  },
  {
    look: inside(
      logRow("You dispatched: Record shows the build, but not each step's time"),
      chip("Time each build step"),
    ),
    say: "Merged: two Jobs on the same work. The new one had pushed nothing, so it ended and its brief, plan and branch went to the other. Both are named",
  },
  {
    look: logRow("Store Reports Its Size: Plan recorded: size group"),
    say: "A recorded plan is ruled group by group. Kept in touch, naming the other Job",
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
    look: logRow("Fleet restarted: Setting says pool, but means slots. Rename it?"),
    say: "Resumed: kept across a restart and ruled after it",
  },
  {
    press: inside(log, button("Overrule: Job says Idle, but its agent crashed")),
    say: "Overrule lists the other outcomes: New Job, Merge, Keep in touch, Raise to me",
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
    press: inside(logRow(BUG), chip("Stop Silent Job Deaths")),
    say: "A Job's chip opens it, and the panel steps aside",
  },
  {
    look: inside(needs, text(/circle-x reads apart from it/)),
    say: "Stop Silent Job Deaths, open",
  },
  {
    key: "Meta+j",
    on: jobs,
    say: "⌘J brings the panel back as he left it",
  },
  {
    look: lineRow(TESTS),
    say: "Waiting to start: what has not started, in the Operator's order, one line each",
  },
  {
    look: inside(lineRow(TESTS), chip("Stop Silent Job Deaths")),
    say: "Each row says what it waits on, and the Job it waits on as its chip",
  },
  {
    look: inside(line, text("Starts when the 5 hour window is under 80%")),
    say: "A meter holds this one",
  },
  {
    drag: lineRow("Raised items make no sound"),
    onto: lineRow("Map key lists colours the Map no longer uses"),
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
    press: tabOf("Permissions"),
    say: "Permissions",
  },
  {
    look: inside(permissions, button("Operator: On")),
    say: "The Operator switch on top. Off sends everything to you as today",
  },
  {
    press: inside(permissions, button("Answer: On")),
    say: "Each role a name, what it does, and its position in one column",
  },
  {
    press: role("menuitem", "Suggest"),
    say: "On, Suggest or Off. Suggest shows what it would do on a raised row, for one press",
  },
  {
    look: inside(permissions, text("Off in billing")),
    say: "A repository that differs says so under its role",
  },
  {
    look: inside(permissions, role("spinbutton", "Daily ceiling")),
    say: "The daily ceiling, 100 calls",
  },
  {
    look: inside(permissions, role("list", "Standing orders")),
    say: "Standing orders, each in a form Fleet enforces, each removed in one press",
  },
  {
    press: tabOf("Talk"),
    say: "Talk",
  },
  {
    look: message,
    say: "The Job composer, the same one a Job has: @ for Jobs and files, # for issues and pull requests, / for commands, files and sketches, model and effort",
  },
  {
    type: "Why did you hold the tests Job?\n",
    into: message,
    say: "The Operator's replies stand over it, as a Job's thread does",
  },
  {
    look: inside(talk, text(/crates\/testkit\/src\/run\.rs/)),
    say: "Answered from the Log row: which plan, which paths, what it waits on",
  },
  {
    type: "Start it anyway\n",
    into: message,
    say: "A follow-up knows what it is about",
  },
  {
    press: tabOf("Log"),
    say: "Back to the Log",
  },
  {
    look: logRow(`You: ${TESTS}`),
    say: "It did what Start now does, and logged it as your correction",
  },
  {
    key: "Meta+j",
    on: tabOf("Log"),
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
