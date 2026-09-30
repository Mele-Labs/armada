import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor, within } from "storybook/test";

import { JobProposal } from "./JobProposal";
import type { ProposalGateRow } from "./ProposalGates";

/**
 * Two states, and they are one screen a press apart: the proposal while it is
 * yours to change, and the same values frozen at the moment you approved.
 * Armada still reading the request is the third and is drawn on the dispatch
 * form, where the request is.
 */
const meta: Meta<typeof JobProposal> = {
  title: "Compositions/Job proposal",
  component: JobProposal,
};
export default meta;

type Story = StoryObj<typeof JobProposal>;

const STEPS: ProposalGateRow[] = [
  {
    id: "plan",
    label: "Plan the change",
    checks: false,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "It advances unless the Judge refuses it.",
  },
  {
    id: "implement",
    label: "Implement",
    checks: true,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "Its Checks have to pass and the Judge has to decline to refuse them.",
  },
  {
    id: "tests",
    label: "Write tests",
    checks: true,
    judge: true,
    you: false,
    advanceGate: "auto_if_judge_passes",
    does: "Its Checks have to pass and the Judge has to decline to refuse them.",
    // A tick moves the gate and never what the step declares, which is the one
    // reading on this screen a person cannot get from the boxes alone.
    unmeant:
      "This step declares nothing for a Judge to read, so asking for a Judge asks for a verdict on no criteria.",
  },
  {
    id: "handoff",
    label: "Review the change",
    checks: false,
    judge: false,
    you: false,
    repositoryDecides: "review_gate",
    overridden: false,
    advanceGate: "manifest_rule:review_gate",
    // What the repository's word resolves to, in the verb generated for it.
    // `tab-proposal-read.ts` composes this from the policy `armada.yml`
    // declares; a row handed no word says only that the repository decides.
    does:
      "Today that policy says a person answers. Editing the Manifest changes it, for this Job " +
      "as well.",
  },
];

/**
 * The address that makes a reference a link. **A forge is never named here**
 * — which one a repository uses is `armada.yml`'s business and the adapter's,
 * and a story is neither.
 */
const ISSUE_ADDRESS = "https://forge.example/armada/issues/1162";

const CRITERIA = [
  {
    id: "a1",
    text: "The rail's Drones stat reads one running beside the machine's most",
    origin: "From issue",
    // The address is what makes the reference a link. Nothing on the wire
    // carries one, so the frozen story below draws the same reference as text.
    issue: { ref: "armada/1162", url: ISSUE_ADDRESS },
    decidedBy: "A Check will decide it",
  },
  {
    id: "a2",
    text: "Pressing the stat lists the Drone's Job and step",
    origin: "From issue",
    issue: { ref: "armada/1162", url: ISSUE_ADDRESS },
    decidedBy: "The Judge will decide it",
  },
];

/** Every workflow this repository declares, as the picker offers them. */
const WORKFLOWS = [
  { id: "feature", name: "feature", steps: 4 },
  // Seven, as `workflow-samples/bug.json` declares — the reference sample.
  { id: "bug", name: "bug", steps: 7 },
];

const COMPLETE = [
  { value: "pr_merged", label: "Its pull request merges", served: false },
  { value: "delivered", label: "The step that delivers has delivered", served: true },
];

const COMMON = {
  title: "Show what is running in the Drones stat",
  request: {
    repository: "armada",
    said:
      "The Drones stat on the rail says \u201c1 of 2\u201d and I cannot tell a busy Fleet from a " +
      "stalled one. I want to see what is actually running, and have it stay live.",
    absent: "This job was given no context beyond its title.",
  },
  workflow: "feature",
  workflowChoices: WORKFLOWS,
  steps: STEPS,
  tiers: { difficult: "opus", medium: "sonnet", easy: null },
  models: ["haiku", "sonnet", "opus"],
  droneCap: 2,
  machineCap: 4,
  landing: {
    target: "main",
    from: "main",
    branching: "job" as const,
    completeWhen: "pr_merged",
    prMode: "ready" as const,
  },
  completeChoices: COMPLETE,
  criteria: CRITERIA,
};

/** Nothing frozen: every gate, every tier, both refs and every criterion is a person's. */
export const YoursToChange: Story = {
  args: {
    ...COMMON,
    onTitle: () => {},
    onRequest: () => {},
    onWorkflow: () => {},
    onGate: () => {},
    onOverride: () => {},
    onTiers: () => {},
    onDroneCap: () => {},
    onLanding: () => {},
    onCriterion: () => {},
    onAddCriterion: () => {},
    onRemoveCriterion: () => {},
    onOpenIssue: () => {},
  },
  /**
   * **What the fourth state offers, before anything is pressed.** The owner
   * read this row and could not tell the two apart (`rhxt`, 29 Sep): it said
   * *The repository decides — review_gate* over a button, and what the button
   * changed was written only after he had pressed it.
   */
  play: async ({ canvas, userEvent }) => {
    const deferred = canvas.getByRole("listitem", { name: "Review the change" });
    const boxes = canvas.getByRole("listitem", { name: "Implement" });

    // The two states, side by side: a step the repository decides offers no
    // box at all, because the answer is not this Job's to give.
    expect(within(deferred).queryAllByRole("checkbox")).toHaveLength(0);
    expect(within(boxes).getAllByRole("checkbox")).toHaveLength(3);

    // What pressing it would change, beside the button rather than after it.
    expect(
      within(deferred).getByText(/hands this step to the three boxes/),
    ).toBeVisible();

    // And the identifier says what it is. A hover is the whole of the claim:
    // the sentence is in the document while closed, so a rendering shows
    // nothing about whether a reader can reach it.
    const policy = within(deferred).getByText("review_gate");
    expect(within(deferred).getByText(/whether a person answers the step/)).not.toBeVisible();
    await userEvent.hover(policy);
    await waitFor(() =>
      expect(within(deferred).getByText(/whether a person answers the step/)).toBeVisible(),
    );
  },
};

/**
 * Approved, and every value on it is what the Job runs on.
 *
 * The first criterion's issue has been edited since — the Job keeps the words
 * it froze and says so, and nothing re-reads the issue.
 */
export const ApprovedAndFrozen: Story = {
  args: {
    ...COMMON,
    steps: STEPS.map((step) =>
      step.id !== "handoff"
        ? step
        : {
            ...step,
            you: true,
            overridden: true,
            advanceGate: "human_always",
            does: "It holds at awaiting_review for you to answer, with nothing run before you read it.",
          },
    ),
    criteria: [{ ...CRITERIA[0]!, movedSince: "22 Sep 2026 at 10:02" }, CRITERIA[1]!],
    frozenAt: "22 Sep 2026 at 09:14",
  },
  // **A rule about what does not happen.** Frozen is the absence of every
  // handler, so the claim is that no control on the screen can take an answer
  // — which a rendering of it cannot show.
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    expect(canvas.queryAllByRole("checkbox")).toHaveLength(0);
    expect(canvas.queryAllByRole("combobox")).toHaveLength(0);
    expect(canvas.queryAllByRole("textbox")).toHaveLength(0);
    expect(canvas.queryAllByRole("spinbutton")).toHaveLength(0);
    expect(canvas.getByText(/This Job decides this step for itself/)).toBeVisible();
    // Nothing here can be pressed, so nothing here says what pressing would
    // change: the line beside the button is the button's, not the row's.
    expect(canvas.queryAllByText(/hands this step to the three boxes/)).toHaveLength(0);
  },
};
