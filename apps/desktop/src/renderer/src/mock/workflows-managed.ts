// The workflows the manager is walked on: Armada's, a repository's copy of `feature` that differs in one step,
// and one of Yours. `feature` is the six steps the owner chose on 27 Oct 2026, in
// `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`.

import type { Drift, ManagedStep, ManagedWorkflow } from "@armada/components";

const propose: ManagedStep = {
  id: "propose",
  label: "Propose",
  glyph: "propose",
  ends: "you reply",
  words:
    "Say in a few lines what you took the request to mean and a first idea. Read only what that needs. Then stop and wait for the person's reply.",
};
const discuss: ManagedStep = {
  id: "discuss",
  label: "Discuss",
  glyph: "discuss",
  ends: "a plan is recorded",
  words:
    "Ask what is unclear, one question at a time. Show ideas as sketches. Where the idea is about how something looks, a rough walk may come now. Edit nothing on the branch but a mock.",
};
const approve: ManagedStep = {
  id: "approve_plan",
  label: "Approve plan",
  glyph: "approve",
  ends: "you approve the plan",
  words: "Record the plan: what the person gets, in his words, the groups in order, and a walk for each piece he would see. Wait for approval.",
};
const walk: ManagedStep = {
  id: "walk",
  label: "Walk",
  glyph: "walk",
  ends: "you approve the walk",
  passedOver: "nothing would be seen",
  words: "Build the mock in the real Bridge on fixtures and show it as a walk. Write no tests until it is approved. Notes come back as another take.",
};
const refine: ManagedStep = {
  id: "refine",
  label: "Refine",
  glyph: "fix",
  ends: "it is pushed",
  words: "Wire it to the real thing and write the tests now. Show a new walk only if the look changed.",
};
const deliver: ManagedStep = {
  id: "deliver",
  label: "Deliver",
  glyph: "deliver",
  ends: "it is merged",
  words: "Open the pull request and merge it on a green ci. Say what landed.",
};

const FEATURE = [propose, discuss, approve, walk, refine, deliver];

const BUG: ManagedStep[] = [
  {
    id: "find_the_cause",
    label: "Find the cause",
    glyph: "cause",
    ends: "the cause is named",
    words: "Reproduce the bug. Find why it happens, not only where it shows. Say the cause in a line.",
  },
  {
    id: "fix",
    label: "Fix",
    glyph: "fix",
    ends: "the fix is made",
    words: "Make the smallest change that addresses the cause. Write no tests yet.",
  },
  {
    id: "short_walk",
    label: "Short walk",
    glyph: "walk",
    ends: "you approve the walk",
    passedOver: "nothing would be seen",
    words: "Show the fix as a short walk straight to the place. Notes come back as another take.",
  },
  {
    id: "tests",
    label: "Tests",
    glyph: "tests",
    ends: "the tests pass",
    words: "Write a test that fails without the fix and passes with it.",
  },
  deliver,
];

const DESIGN_PLAN: ManagedStep[] = [
  { ...propose },
  {
    id: "sketch",
    label: "Sketch",
    glyph: "sketch",
    ends: "you pick a direction",
    words: "Show the options as sketches, one idea each. Build nothing.",
  },
  {
    id: "record_plan",
    label: "Record plan",
    glyph: "plan",
    ends: "a plan is recorded",
    words: "Record what was chosen and what it leaves out. Wait for approval.",
  },
];

const PROTOTYPE: ManagedStep[] = [
  { ...propose },
  {
    id: "build",
    label: "Build",
    glyph: "fix",
    ends: "the mock runs",
    words: "Build the idea in the real Bridge on fixtures. Wire nothing and write no tests.",
  },
  {
    id: "show",
    label: "Show",
    glyph: "walk",
    ends: "you approve the walk",
    words: "Show it as a walk. Notes come back as another take.",
  },
];

const DOCS_ONLY: ManagedStep[] = [
  {
    id: "draft",
    label: "Draft",
    glyph: "draft",
    ends: "the page reads right",
    words: "Write the change in the page's own voice. Touch no code.",
  },
  { ...deliver },
];

export const MANAGED_WORKFLOWS: readonly ManagedWorkflow[] = [
  { id: "feature", source: "armada", forRequests: "adding or changing something a person uses", steps: FEATURE },
  { id: "bug", source: "armada", forRequests: "something that worked and no longer does", steps: BUG },
  { id: "design_plan", source: "armada", forRequests: "a design to settle before anything is built", steps: DESIGN_PLAN },
  { id: "prototype", source: "armada", forRequests: "a throwaway build to try an idea", steps: PROTOTYPE },
  {
    id: "feature",
    source: "repository",
    forRequests: "adding or changing something a person uses",
    steps: FEATURE.map((one) =>
      one.id === "refine"
        ? { ...one, words: "Wire it to the real thing and write the tests now. Run the repository's checks before pushing. Show a new walk only if the look changed." }
        : one,
    ),
  },
  { id: "docs_only", source: "yours", forRequests: "a change to words only", steps: DOCS_ONLY },
];

/** The Jobs Fleet nudged, by workflow and step. */
export const MANAGED_DRIFT: Drift = {
  feature: {
    walk: [{ job: "Cockpit timeline rail", what: "a test before the walk was approved", when: "2h ago" }],
    approve_plan: [{ job: "Pause and resume", what: "edited before the plan was approved", when: "yesterday" }],
  },
};

/** What the agent comes back with for "split Refine into Wire and Tests". */
export const WIRE_AND_TESTS: ManagedStep[] = [
  {
    id: "wire",
    label: "Wire",
    glyph: "fix",
    ends: "it is wired to the real thing",
    words: "Wire it to the real thing. Show a new walk only if the look changed.",
  },
  {
    id: "tests",
    label: "Tests",
    glyph: "tests",
    ends: "it is pushed",
    words: "Write the tests now, against what the walk showed.",
  },
];
