// One proposal per Job: feature's Propose lists what ends it, a plan carries its own sketch, an approved sketch moves the Job on with
// no plan to approve, an approved rough walk was the walk, and a Job that showed three walks waits for all three. Over the `step-ends-any` scenario.

import { button, deckRow, inside, region, role, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const ARMADA = role("group", "Shipped with Armada", {
  exact: true,
});
const propose = role("group", "Propose", {
  exact: true,
});
const ends = inside(propose, role("list", "Ends when"));
const planCard = role("article", "Plan to approve");
const sketchCard = role("article", "Sketch to approve: Pause on the row", {
  exact: true,
});

const stepEndsAny = walk("step-ends-any", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  {
    press: inside(
      ARMADA,
      button("feature", {
        exact: true,
      }),
    ),
    say: "feature is Propose, Walk, Refine, Deliver",
  },
  {
    look: ends,
    say: "Propose ends on one approval: a plan, a walk, a sketch, a draft, a document or a picked idea",
  },
  toSessions,
  {
    press: deckRow("Stop a Job from its row"),
    say: "A Job that needs a plan and a picture",
  },
  {
    look: planCard,
    say: "One card: the sketch is inside the plan",
  },
  {
    press: deckRow("Pause a Job from its row"),
    say: "A Job that only needed a sketch",
  },
  {
    press: inside(
      sketchCard,
      button("Approve", {
        exact: true,
      }),
    ),
    say: "Approve",
  },
  {
    look: role("article", "Plan", {
      exact: false,
    }),
    say: "The tasks were written and stand. No plan card came",
  },
  {
    look: region("Walk", {
      exact: true,
    }),
    say: "The Job is on Walk: a mock on fixtures, no wiring and no tests",
  },
  {
    press: deckRow("Resume a Job from its row"),
    say: "A Job whose proposal was a rough walk",
  },
  {
    look: region("Refine", {
      exact: true,
    }),
    say: "That rough walk was the walk. The Job is on Refine",
  },
  {
    press: deckRow("Mute a Job"),
    say: "A Job that showed three walks. You have approved one",
  },
  {
    look: region("Walk", {
      exact: true,
    }),
    say: "It stays on Walk until you have approved all three",
  },
]);

export { stepEndsAny as "step-ends-any" };
