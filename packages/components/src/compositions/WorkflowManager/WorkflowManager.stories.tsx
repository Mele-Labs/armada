import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { WorkflowManager } from "./WorkflowManager";
import type { WorkflowManagerProps } from "./WorkflowManager";
import type { ManagedStep, ManagedWorkflow } from "./types";

const step = (id: string, label: string, ends: string, words: string, over: Partial<ManagedStep> = {}): ManagedStep => ({
  id,
  label,
  glyph: id === "propose" ? "propose" : id === "walk" ? "walk" : "deliver",
  ends,
  words,
  ...over,
});

const FEATURE = [
  step("propose", "Propose", "you reply", "Say in a few lines what you took the request to mean. Then stop and wait for the person's reply."),
  step("walk", "Walk", "you approve the walk", "Build the mock on fixtures and show it as a walk. Write no tests until it is approved.", { passedOver: "nothing would be seen" }),
  step("deliver", "Deliver", "it is merged", "Open the pull request and merge it on a green ci."),
];

const WORKFLOWS: readonly ManagedWorkflow[] = [
  { id: "feature", source: "armada", forRequests: "adding or changing something a person uses", steps: FEATURE },
  { id: "bug", source: "armada", forRequests: "something that worked and no longer does", steps: [step("fix", "Fix", "the fix is made", "Make the smallest change that addresses the cause.")] },
  {
    id: "feature",
    source: "repository",
    forRequests: "adding or changing something a person uses",
    steps: FEATURE.map((one) => (one.id === "walk" ? { ...one, words: `${one.words} Run the repository's checks before pushing.` } : one)),
  },
  { id: "docs_only", source: "yours", forRequests: "a change to words only", steps: [step("draft", "Draft", "the page reads right", "Write the change in the page's own voice.", { glyph: "draft" })] },
  { id: "hotfix", source: "yours", forRequests: "", steps: [], leftOut: { said: "Names a check this repository does not declare", file: "~/.armada/workflows/hotfix.json" } },
];

const done = async () => undefined;

const meta: Meta<typeof WorkflowManager> = {
  title: "Compositions/Workflow manager",
  component: WorkflowManager,
  decorators: [
    (Story) => (
      <div style={{ background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
  args: {
    repository: "armada",
    workflows: WORKFLOWS,
    drift: {},
    picked: { id: "feature", source: "repository" },
    agent: { phase: "idle" },
    suggestions: {},
    suggesting: null,
    onPick: fn(),
    onMakeCopy: fn(done),
    onPropose: fn(done),
    onSaveWords: fn(done),
    onReset: fn(done),
    onDelete: fn(done),
    onAsk: fn(done),
    onApprove: fn(done),
    onDismiss: fn(done),
    onSendBack: fn(done),
    onOpenJob: fn(),
    onNotDrift: fn(done),
    onSuggest: fn(done),
    onApproveSuggestion: fn(done),
    onDismissSuggestion: fn(done),
  } satisfies WorkflowManagerProps,
};
export default meta;

type Story = StoryObj<typeof WorkflowManager>;

/** A repository's copy: its words are edited where they stand, and it compares with Armada's and resets to it. */
export const RepositoryCopy: Story = {};

/** Armada's own: read-only, with Make my copy and Propose to Armada. */
export const ArmadasOwn: Story = { args: { picked: { id: "feature", source: "armada" } } };

/** A Job Fleet nudged at a step, and a change drawn up from it waiting on Approve. */
export const DriftAndSuggestion: Story = {
  args: {
    drift: { feature: { walk: [{ id: "1", jobId: "job-1", job: "Cockpit timeline rail", what: "a test before the walk was approved", when: "2h ago" }] } },
    suggestions: { feature: { walk: { words: `${FEATURE[1]!.words} Test files wait for the approval too.`, from: ["1"] } } },
  },
};

/** A file Fleet left out: its reason, and nothing to edit. */
export const LeftOut: Story = { args: { picked: { id: "hotfix", source: "yours", file: "~/.armada/workflows/hotfix.json" } } };

/** A change typed to an agent, drawn up and waiting on Approve. */
export const ProposedChange: Story = {
  args: {
    agent: {
      phase: "proposed",
      asked: "add a Review step before Deliver",
      before: FEATURE,
      after: [...FEATURE.slice(0, 2), step("review", "Review", "", "Read the diff."), FEATURE[2]!],
    },
  },
};
