// The Operator panel, in Helm's place, before any Fleet work is built (armada-ehhu.2). Two takes of one walk, for the owner to pick:
// `operatorWide` is the dock as wide as the window lets it be, `operatorFull` the dock over the whole work. Over `operator-wide` and
// `operator-full`.
//
// No wire carries any of this yet. Each shape is typed in `operator/model.ts`, and is a proposal for the protocol:
// - `OperatorLogRow`: id, at, from, item, did (answered, raised, delegated, existing, merged, in_touch, held, started, filed,
//   passed_on, released, returned, resumed, moved), why, the Line row a hold made, the other ways Fleet offered for Change, and the way the person changed it to.
// - `OperatorLineRow`: id, title, what it waits for (`OperatorWait`: a Job and pushes or ends with its plan group, a meter under a
//   percentage, or the person), why, and the Job it was dispatched as.
// - `StandingOrder`: one of three enforced forms, a role off in a repository, a hold on paths until a Job pushes or ends, or a kind
//   always raised.
// - `OperatorRead`: a Job's question being read, or raised with its read and its pick.
// - `OperatorTie`: what a merge or keep in touch ruling left on a live Job's row: the other Job, the shared paths, and the plan
//   group that waits on the other's push.
// - The switch, a position (on, suggest, off) per role, and the daily ceiling on calls.
// - The box's conversation: messages from the person and the Operator. The mock answers from a script (`OperatorTalk`).

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step, Target } from "../walk";

const panel = role("complementary", "Operator", {
  exact: true,
});
const log = inside(panel, region("Log"));
const line = inside(panel, region("Line"));
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
const message = inside(box, role("textbox", "Message to the Operator"));

function steps(full: boolean): Step[] {
  return [
    {
      press: button("Operator", {
        exact: true,
      }),
      say: full
        ? "The title row's button, Helm's renamed Operator, opens the panel over the whole work"
        : "The title row's button, Helm's renamed Operator, opens the panel as a dock as wide as the window allows",
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
      look: logRow("Home Draws Desktop States: Keep the old test name?"),
      say: "The Log: every act, where it came from, what it did, why and how long ago. Answered, since its group is not seen",
    },
    {
      look: logRow("Stop Silent Job Deaths: Which icon marks a dead process?"),
      say: "Raised: its group is seen, so it went to Needs you. No Change here",
    },
    {
      look: logRow("Bug report: A Job whose agent process died stays listed as idle"),
      say: "Existing Job: a live Job's plan group covers the report, so the words went to it and no Job started",
    },
    {
      look: logRow("Show build times in the Record: Time each build step"),
      say: "Merged: two Jobs on the same work. The one that had pushed nothing ended, and its brief, plan and branch went to the other",
    },
    {
      look: logRow("Store Reports Its Size: Plan recorded: the size group"),
      say: "A recorded plan is ruled group by group. Kept in touch, naming the other Job and the shared path",
    },
    {
      look: logRow("Cockpit Console: Fleet marked it Stopped with 3 tasks open"),
      say: "Delegated too: a nudge to a Job Fleet marked Stopped",
    },
    {
      look: inside(
        logRow("You dispatched: Show the slot pool on the Map"),
        button("Start now: Show the slot pool on the Map"),
      ),
      say: "Kept in touch, where the other Job builds first: its group waits on that push, with Start now",
    },
    {
      look: logRow("Bug report: The Logbook search drops the first letter typed"),
      say: "Filed, after a duplicate check",
    },
    {
      look: logRow("Rework the Map legend: Keep the legend collapsed by default?"),
      say: "When it goes wrong. Passed on: no ruling in time, so it went on as today",
    },
    {
      look: logRow("Line: Wire the Doctor's new checks"),
      say: "Released: the Job it waited on ended",
    },
    {
      look: logRow(
        "Home Draws Desktop States: A Job whose agent process died stays listed as idle",
      ),
      say: "Returned: the Job ended without touching it, and it was ruled again",
    },
    {
      look: logRow("Fleet restarted: Rename the slot pool setting?"),
      say: "Resumed: kept across a restart and ruled after it",
    },
    {
      press: inside(log, button("Change: Keep the old test name?")),
      say: "Change, on a row, lists the other ways Fleet offered. There is no Agree",
    },
    {
      press: role("menuitem", "Raise it to me"),
      say: "A change corrects it, and the next call reads it",
    },
    {
      look: inside(log, text("Changed by you: Raise it to me")),
      say: "The row says so",
    },
    {
      look: lineRow("Tests Build Only What They Test"),
      say: "The Line: what has not started, in the Operator's order, each row naming the Job or meter it waits for",
    },
    {
      look: inside(line, text("5 hour window under 80%")),
      say: "A meter holds this one",
    },
    {
      drag: lineRow("Idea: sounds for raised items"),
      onto: lineRow("Tests Build Only What They Test"),
      say: "Drag a row to override the order. ⌥↑ and ⌥↓ do it from the keyboard",
    },
    {
      look: logRow("You: Idea: sounds for raised items"),
      say: "The move goes in the Log as a correction",
    },
    {
      press: inside(line, button("Start now: Redraw the Map key")),
      say: "Start now on any row overrides the hold in one press",
    },
    {
      look: logRow("You: Redraw the Map key"),
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
      type: "Why did you hold the tests Job?\n",
      into: message,
      say: "The box is a short conversation",
    },
    {
      look: inside(box, text(/crates\/testkit\/src\/run\.rs/)),
      say: "Answered from the Log row: which plan, which paths, what it waits for",
    },
    {
      type: "Start it anyway\n",
      into: message,
      say: "A follow-up knows what it is about",
    },
    {
      look: logRow(/^You: Tests Build Only What They Test/),
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
      say: "A Job you dispatched that the Operator held. Its row says what it waits for",
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
      say: "Its start button is Dispatch now, since the press may delegate or hold",
    },
    {
      key: "Alt+Meta+a",
      on: jobs,
      say: "The layer off again",
    },
  ];
}

export const operatorWide = walk("operator-wide", steps(false));

export const operatorFull = walk("operator-full", steps(true));
