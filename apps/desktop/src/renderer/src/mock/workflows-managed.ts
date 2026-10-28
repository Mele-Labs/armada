// The workflows the Workflows page is walked on, as the definitions Fleet would hold: Armada's, a repository's copy of
// `feature` that differs in one step, and one of Yours. `feature` is the six steps the owner chose on 27 Oct 2026, in
// The decision of 27 Oct, a feature Job walks before it tests. Only the mock Fleet in `workflows-fleet.ts`
// reads this; the page reads what that Fleet answers.

/** A step as its definition file writes it. */
type Written = { id: string; label: string; instructions: string };

/** What a definition file holds, as the loader reads it. */
export type Definition = { version: 1; workflow_id: string; name: string; for_requests: string; steps: Written[] };

const define = (id: string, forRequests: string, steps: Written[]): Definition => ({ version: 1, workflow_id: id, name: id, for_requests: forRequests, steps });

const propose: Written = {
  id: "propose",
  label: "Propose",
  instructions:
    "Say in a few lines what you took the request to mean and a first idea. Read only what that needs. Then stop and wait for the person's reply.",
};
const discuss: Written = {
  id: "discuss",
  label: "Discuss",
  instructions:
    "Ask what is unclear, one question at a time. Show ideas as sketches. Where the idea is about how something looks, a rough walk may come now. Edit nothing on the branch but a mock.",
};
const approve: Written = {
  id: "approve_plan",
  label: "Approve plan",
  instructions: "Record the plan: what the person gets, in his words, the groups in order, and a walk for each piece he would see. Wait for approval.",
};
const walk: Written = {
  id: "walk",
  label: "Walk",
  instructions: "Build the mock in the real Bridge on fixtures and show it as a walk. Write no tests until it is approved. Notes come back as another take.",
};
const refine: Written = {
  id: "refine",
  label: "Refine",
  instructions: "Wire it to the real thing and write the tests now. Show a new walk only if the look changed.",
};
const deliver: Written = {
  id: "deliver",
  label: "Deliver",
  instructions: "Open the pull request and merge it on a green ci. Say what landed.",
};

const FEATURE = [propose, discuss, approve, walk, refine, deliver];

const FOR_FEATURE = "adding or changing something a person uses";

/** Every definition the mock Fleet holds at the start, by the place that holds it. */
export const MANAGED_DEFINITIONS: readonly { source: "armada" | "repository" | "kit"; definition: Definition }[] = [
  { source: "armada", definition: define("feature", FOR_FEATURE, FEATURE) },
  {
    source: "armada",
    definition: define("bug", "something that worked and no longer does", [
      {
        id: "find_the_cause",
        label: "Find the cause",
        instructions: "Reproduce the bug. Find why it happens, not only where it shows. Say the cause in a line.",
      },
      { id: "fix", label: "Fix", instructions: "Make the smallest change that addresses the cause. Write no tests yet." },
      {
        id: "short_walk",
        label: "Short walk",
        instructions: "Show the fix as a short walk straight to the place. Notes come back as another take.",
      },
      { id: "tests", label: "Tests", instructions: "Write a test that fails without the fix and passes with it." },
      deliver,
    ]),
  },
  {
    source: "armada",
    definition: define("design_plan", "a design to settle before anything is built", [
      propose,
      { id: "sketch", label: "Sketch", instructions: "Show the options as sketches, one idea each. Build nothing." },
      { id: "record_plan", label: "Record plan", instructions: "Record what was chosen and what it leaves out. Wait for approval." },
    ]),
  },
  {
    source: "armada",
    definition: define("prototype", "a throwaway build to try an idea", [
      propose,
      { id: "build", label: "Build", instructions: "Build the idea in the real Bridge on fixtures. Wire nothing and write no tests." },
      { id: "show", label: "Show", instructions: "Show it as a walk. Notes come back as another take." },
    ]),
  },
  {
    source: "repository",
    definition: define(
      "feature",
      FOR_FEATURE,
      FEATURE.map((one) =>
        one.id === "refine"
          ? { ...one, instructions: "Wire it to the real thing and write the tests now. Run the repository's checks before pushing. Show a new walk only if the look changed." }
          : one,
      ),
    ),
  },
  {
    source: "kit",
    definition: define("docs_only", "a change to words only", [
      { id: "draft", label: "Draft", instructions: "Write the change in the page's own voice. Touch no code." },
      deliver,
    ]),
  },
];

/** What ends each step and when it is passed over: what Fleet's rail reads off the rules. By step id, and `workflow/step` where two differ. */
export const MANAGED_ENDS: Readonly<Record<string, { ends: string; passedOver?: string }>> = {
  propose: { ends: "you reply" },
  discuss: { ends: "a plan is recorded" },
  approve_plan: { ends: "you approve the plan" },
  walk: { ends: "you approve the walk", passedOver: "nothing would be seen" },
  refine: { ends: "it is pushed" },
  wire: { ends: "it is wired to the real thing" },
  "feature/tests": { ends: "it is pushed" },
  deliver: { ends: "it is merged" },
  find_the_cause: { ends: "the cause is named" },
  fix: { ends: "the fix is made" },
  short_walk: { ends: "you approve the walk", passedOver: "nothing would be seen" },
  tests: { ends: "the tests pass" },
  sketch: { ends: "you pick a direction" },
  record_plan: { ends: "a plan is recorded" },
  build: { ends: "the mock runs" },
  show: { ends: "you approve the walk" },
  draft: { ends: "the page reads right" },
};

/** A Job Fleet nudged at a step of a place's definition, `ago` before the Fleet started. */
export type MockNudge = { source: "armada" | "repository" | "kit"; workflow: string; step: string; job_id: string; job_title: string; fact: string; ago: number };

const HOUR = 3_600_000;

/** The Jobs Fleet nudged. The first two drifted at Walk in the repository's `feature`, and the Suggest a change there is drawn from them. */
export const MANAGED_NUDGES: readonly MockNudge[] = [
  { source: "repository", workflow: "feature", step: "walk", job_id: "timeline-rail", job_title: "Cockpit timeline rail", fact: "a test before the walk was approved", ago: 2 * HOUR },
  { source: "repository", workflow: "feature", step: "walk", job_id: "pause-mock", job_title: "Pause and resume", fact: "a test file written with the mock", ago: 72 * HOUR },
  { source: "repository", workflow: "feature", step: "approve_plan", job_id: "pause-plan", job_title: "Pause and resume", fact: "edited before the plan was approved", ago: 24 * HOUR },
];

/** The sentence an agent adds to a step's words when asked for a suggestion there, by step id. */
export const SUGGESTED_SENTENCE: Readonly<Record<string, string>> = {
  walk: "Test files wait for the approval too, including fixtures for the mock.",
  approve_plan: "Edit nothing before approval, including the mock.",
};

/** What Armada lacks for "Propose should end when a walk, a sketch or a document is approved, or last of all a question is answered". */
export const CANNOT_YET: readonly string[] = [
  "A step ends on one thing, not on any of several",
  "Nothing ends a step when a sketch is approved",
  "Nothing ends a step when a document is approved",
];

/** The sentence an agent adds to Walk's words for "Walk should say why when it skips the walk". */
export const SAY_WHY_SKIPPED = "Where nothing would be seen, say so and why before going on.";

/** The steps an agent comes back with for "split Refine into Wire and Tests". */
export const WIRE_AND_TESTS: readonly Written[] = [
  { id: "wire", label: "Wire", instructions: "Wire it to the real thing. Show a new walk only if the look changed." },
  { id: "tests", label: "Tests", instructions: "Write the tests now, against what the walk showed." },
];
