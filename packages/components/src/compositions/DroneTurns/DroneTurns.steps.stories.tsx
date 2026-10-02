import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { DroneTurns, type DroneTurn, type TurnStep } from "./DroneTurns";
import { NOTHING_YET, thinking } from "./DroneTurns.fixtures";

/**
 * Drone turns under the steps they ran in. Split from `DroneTurns.stories.tsx`
 * with the same `title`, so every story keeps its id.
 */
const meta: Meta<typeof DroneTurns> = {
  title: "Compositions/Drone turns",
  component: DroneTurns,
};
export default meta;

type Story = StoryObj<typeof DroneTurns>;

/**
 * The two steps of the transcript below. A `label` Fleet served, so both read
 * as names rather than as identifiers.
 */
const REPRO: TurnStep = { id: "repro", label: "Reproduce the bug" };
const FIX: TurnStep = { id: "fix", label: "Fix the root cause" };

const UNDER_TWO_STEPS: DroneTurn[] = [
  { id: "1", at: "09:14:02", step: REPRO, who: "drone", kind: "started", subject: "sess_01JB4 · the job's model · 2 mcp servers" },
  { id: "2", at: "09:14:03", step: REPRO, who: "drone", kind: "said", said: "Writing the failing test before I touch the reducer." },
  ...thinking(10, 5, "09:14:04").map((turn) => ({ ...turn, step: REPRO })),
  { id: "20", at: "09:14:22", step: REPRO, who: "drone", kind: "called", subject: "Write", detail: "tests/settings_split.rs" },
  { id: "21", at: "09:15:01", step: FIX, who: "drone", kind: "said", said: "The test reproduces it. Splitting the reducer now." },
  { id: "22", at: "09:15:09", step: FIX, who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18" },
  { id: "23", at: "09:15:40", step: FIX, who: "drone", kind: "called", subject: "Bash", detail: "cargo test -p settings --lib", answer: "Failed." },
  { id: "24", at: "09:18:02", step: REPRO, who: "drone", kind: "said", said: "The gate sent this back. Widening the reproduction first." },
  { id: "25", at: "09:18:30", step: REPRO, who: "drone", kind: "called", subject: "Edit", detail: "tests/settings_split.rs +11 -0" },
  { id: "26", at: "09:19:04", step: FIX, who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +6 -2", answer: "No answer yet." },
];

/**
 * One Drone across three runs of two steps.
 *
 * **The boundary is drawn where the step changed, and nowhere else.** A label on
 * every row would be the same string down forty consecutive lines, taking width
 * from the body that carries what the Drone actually did; the question a reader
 * asks is where one step stopped and the next began.
 *
 * **A step that runs twice draws two boundaries.** The transcript records the
 * step that was running when each row was written, not a range, so `fix` failing
 * its gate and being retried is two separate stretches — and a component that
 * marked only first appearances would fold the retry into the original.
 *
 * A boundary also breaks a run of quiet rows, because one collapsed line
 * spanning two steps would attribute the whole of it to whichever the reader
 * guessed at.
 */
export const TurnsUnderTheirSteps: Story = {
  args: { live: true, emptyNote: NOTHING_YET, turns: UNDER_TWO_STEPS },
  // Four boundaries, and each one breaks the Drone's card.
  play: async ({ canvas }) => {
    await expect(canvas.getAllByText("step", { exact: true })).toHaveLength(4);
    await expect(canvas.getAllByRole("group", { name: "Drone" })).toHaveLength(4);
  },
};

/**
 * The same transcript where the surface already names the step — a Drone's
 * sheet, whose head does, and whose Drone works one step.
 *
 * **No boundary, and no card broken at one.** A line repeating the head says
 * nothing, and a card split at a step nobody sees reads as a random break. The
 * owner, 29 Sep 2026: *"This is already represented in the header."*
 */
export const StepsNamedElsewhere: Story = {
  args: { live: true, emptyNote: NOTHING_YET, turns: UNDER_TWO_STEPS, steps: false },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("step", { exact: true })).toBeNull();
    await expect(canvas.getAllByRole("group", { name: "Drone" })).toHaveLength(1);
  },
};

/**
 * A step whose workflow declares no name of its own.
 *
 * **The `step_id` renders, in mono, and nothing composes a name from it.** That
 * is the rail's answer to the same substitution: Fleet never sends a blank
 * label, it sends the id, and mono is how a reader is told which arrived. See
 * `[workflow-step-human-label]` — no workflow in the repository declares a
 * label yet, so this is what most transcripts look like today.
 */
export const AStepWithNoNameOfItsOwn: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:22:01", step: { id: "implement", label: "implement", labelIsAnIdentifier: true }, who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18" },
      { id: "2", at: "09:24:40", step: { id: "regression_verify", label: "regression_verify", labelIsAnIdentifier: true }, who: "drone", kind: "called", subject: "Bash", detail: "cargo nextest run --workspace" },
      { id: "3", at: "09:26:12", step: { id: "write_up", label: "write_up", labelIsAnIdentifier: true }, who: "drone", kind: "said", said: "Submitting the evidence report." },
    ],
  },
};

/**
 * A transcript that begins before Fleet recorded the step.
 *
 * **The leading rows are unlabelled, never the first step.** That is the exact
 * falsehood the field was added to remove — a four-step Job whose whole
 * transcript claimed to have happened under step one. The step those rows ran
 * under cannot be recovered from anything on disk, so no migration invented one
 * and this pane does not either.
 *
 * A transcript where *no* row anywhere carries a step draws no boundary at all:
 * every row of it predates the field, so the line would contrast with nothing.
 * Every story above this one is that case.
 */
export const RowsWrittenBeforeTheStepWasRecorded: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "08:59:14", who: "drone", kind: "started", subject: "sess_01J9Z · the job's model · 2 mcp servers" },
      { id: "2", at: "08:59:20", who: "drone", kind: "called", subject: "Read", detail: "src/settings.rs" },
      { id: "3", at: "09:01:02", who: "drone", kind: "said", said: "Reading the reducer before I split it." },
      { id: "4", at: "09:12:41", step: FIX, who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18" },
      { id: "5", at: "09:13:10", step: FIX, who: "drone", kind: "called", subject: "Bash", detail: "cargo test -p settings --lib" },
    ],
  },
};
