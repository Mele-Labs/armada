import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn } from "storybook/test";

import { ProposalPage } from "./ProposalPage";

/**
 * The proposal's own screen, which dispatching lands on. Every state here is
 * one a person meets: the call being read, what it became, and the three ways
 * it can end with nothing created.
 *
 * Why this is a screen and not a state of the composer, and why it is not the
 * Job page either, is the owner's decision of 30 Sep 2026, *the wait is a
 * destination*.
 */
const meta: Meta<typeof ProposalPage> = {
  title: "Compositions/Proposal page",
  component: ProposalPage,
  args: {
    request:
      "The board flickers every time an event lands. Find out why and stop it — it has been " +
      "doing it since the resync change.",
    onStop: fn(),
    onOpen: fn(),
    onApprove: fn(),
    onEdit: fn(),
    onAnother: fn(),
    slowAfterMs: 120_000,
  },
};
export default meta;

type Story = StoryObj<typeof ProposalPage>;

/** One Job, as the proposer drafted it. */
const ONE = {
  id: "job_2d90bb",
  title: "Stop the board flickering on every event",
  workflow: "bug",
  status: "awaiting_approval",
};

/**
 * Asked, and Fleet has said nothing about the call yet — an older Fleet, or the
 * moment before the first `proposal.moved`.
 *
 * **The stop is here anyway.** A screen somebody was sent to in order to watch
 * one call carries the only act on it from the first frame; the alternative was
 * a destination with nothing on it to press.
 */
export const ReadingWithNothingSaid: Story = {
  args: { proposal: { at: "reading" } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText(/answers once, whole/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Stop the proposer" }));
    await expect(args.onStop).toHaveBeenCalled();
  },
};

/**
 * The call has reached the vendor and is thinking. **What a wait is for**: the
 * reach, the elapsed figure against Fleet's ceiling, and how much thinking
 * there has been — none of which an elapsed count alone can say.
 *
 * Well inside `slowAfterMs`, so nothing says the wait is long.
 */
export const Reading: Story = {
  args: {
    proposal: {
      at: "reading",
      watch: {
        reached: "thinking",
        elapsedMs: 41_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 763,
      },
    },
  },
  /**
   * **The press is pressed once and goes dead**, which no rendering shows: a
   * stop sent twice would be two kills, and the second would arrive after the
   * call it names has gone.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("The model is thinking")).toBeVisible();
    await expect(canvas.getByText("41s")).toBeVisible();
    await expect(canvas.queryByText(/taking longer than expected/)).toBeNull();

    const stop = canvas.getByRole("button", { name: "Stop the proposer" });
    await userEvent.click(stop);
    await expect(args.onStop).toHaveBeenCalledOnce();

    // Pending, not disabled: it is the control being waited on, so it stays
    // focusable and sweeps a bar. #1117. Dispatched rather than clicked,
    // because the app's base styles take a dead control out of pointer reach.
    const stopping = canvas.getByRole("button", { name: "Stopping…" });
    await expect(stopping).toHaveAttribute("aria-busy", "true");
    fireEvent.click(stopping);
    await expect(args.onStop).toHaveBeenCalledOnce();
  },
};

/**
 * Past the mark, and the screen says so. **There is no `Keep waiting`
 * control** — waiting is what happens if nothing is pressed, and a button for
 * it would perform no act.
 */
export const ReadingAndSlow: Story = {
  args: {
    proposal: {
      at: "reading",
      watch: {
        reached: "thinking",
        elapsedMs: 142_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 4_210,
      },
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/taking longer than expected/)).toBeVisible();
    await expect(canvas.getByText("2m 22s")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Keep waiting/ })).toBeNull();
  },
};

/**
 * **The case worth telling apart from every other.** Two minutes in and the
 * harness has still not announced itself, so the call never reached the vendor
 * at all — a credential or a harness problem, which will not resolve by
 * waiting. Under an elapsed count alone this is indistinguishable from a model
 * thinking hard, and the two take opposite decisions.
 */
export const ReadingAndStuckStarting: Story = {
  args: {
    proposal: {
      at: "reading",
      watch: { reached: "starting", elapsedMs: 130_000, budgetMs: 600_000, model: "haiku" },
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Starting the proposer")).toBeVisible();
    // Nothing thought and nothing answered, so neither count is drawn. Absent
    // rather than zeroed: `0 tokens` reads as a model that thought about
    // nothing, which is a different and much less alarming fact.
    await expect(canvas.queryByText(/tokens of thinking/)).toBeNull();
  },
};

/**
 * The answer is arriving. **Nearly over** — stopping here would throw away work
 * about to land, which is what the reach is for.
 */
export const ReadingAndAnswering: Story = {
  args: {
    proposal: {
      at: "reading",
      watch: {
        reached: "answering",
        elapsedMs: 88_000,
        budgetMs: 600_000,
        model: "haiku",
        thinkingTokens: 2_100,
        answeredCharacters: 340,
      },
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The answer is arriving")).toBeVisible();
    await expect(canvas.getByText(/340 characters of answer/)).toBeVisible();
  },
};

/**
 * A window reopened while its own proposer is out. **Bridge and Fleet have
 * independent lifetimes**, so the call is known and what it was about is not —
 * and the screen draws no quote rather than a sentence standing in for one.
 */
export const ReadingSomethingThisWindowDidNotSend: Story = {
  args: {
    request: undefined,
    proposal: {
      at: "reading",
      watch: { reached: "requesting", elapsedMs: 12_000, budgetMs: 600_000, model: "sonnet" },
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Asking the model")).toBeVisible();
    await expect(canvas.queryByText(/board flickers/)).toBeNull();
  },
};

/**
 * One Job, which is the ordinary case — and the screen does not keep it. **A
 * list of one is not a list**, so the screen asks to be replaced by the Job it
 * drafted, which is what the owner asked for on 30 Sep 2026.
 *
 * **A `play`, because the whole claim is a call nobody pressed for.** A still
 * of this shows one row and says nothing about the handover.
 */
export const OneJob: Story = {
  args: { proposal: { at: "proposed", jobs: [ONE] } },
  play: async ({ args, canvas }) => {
    await expect(args.onOpen).toHaveBeenCalledWith("job_2d90bb");
    // And it is not asked for twice, which a re-render on the window's clock
    // would do — the effect is keyed on the Job's id, not on the callback.
    await expect(args.onOpen).toHaveBeenCalledOnce();
    await expect(canvas.getByText(ONE.title)).toBeVisible();
  },
};

/**
 * Several, and the order between them.
 *
 * **The order is the whole of the graph.** A proposal of several is a chain —
 * each member waits on the one before it reaching `completed_success` — so
 * position carries it and no second field restates it.
 *
 * **Only the first is approvable, and nothing here approves all three.**
 * Fleet's rule is strictly one by one, and the second is not at its gate until
 * the first completes.
 */
export const SeveralJobs: Story = {
  args: {
    request:
      "Move the runtime file to its own crate and make Bridge verify the pid before it " +
      "connects, so an unreachable Fleet stops reading as a missing one.",
    proposal: {
      at: "proposed",
      jobs: [
        { id: "job_11a0", title: "Move the runtime file into its own crate", workflow: "refactor", status: "awaiting_approval" },
        { id: "job_11a1", title: "Verify the pid before connecting", workflow: "feature", status: "awaiting_approval" },
        { id: "job_11a2", title: "Tell an unreachable Fleet from a missing one", workflow: "feature", status: "awaiting_approval" },
      ],
    },
  },
  /**
   * The third row opens the third Job. An implementation keyed on an index
   * rather than the row's own id looks identical here and sends the wrong id
   * the moment Fleet reorders anything.
   */
  play: async ({ args, canvas, userEvent }) => {
    // Nothing was opened for them: several is a list, and choosing one of three
    // for somebody is deciding on their behalf.
    await expect(args.onOpen).not.toHaveBeenCalled();

    await userEvent.click(
      canvas.getByRole("button", { name: "Review Tell an unreachable Fleet from a missing one" }),
    );
    await expect(args.onOpen).toHaveBeenCalledWith("job_11a2");

    // One gate on screen, and it is the head's. Three rows at
    // `awaiting_approval` is what the answer says; which of them a person may
    // release is not the same question.
    await expect(canvas.getAllByRole("button", { name: /^Approve/ })).toHaveLength(1);
    await userEvent.click(
      canvas.getByRole("button", { name: "Approve Move the runtime file into its own crate" }),
    );
    await expect(args.onApprove).toHaveBeenCalledWith("job_11a0");
  },
};

/**
 * The approval is out. **The control says so and waits** — approving twice does
 * not spawn twice, but a control that looks unpressed invites the second press
 * and then says nothing about the first.
 */
export const Approving: Story = {
  args: {
    approving: ["job_11a0"],
    proposal: {
      at: "proposed",
      jobs: [
        { id: "job_11a0", title: "Move the runtime file into its own crate", workflow: "refactor", status: "awaiting_approval" },
        { id: "job_11a1", title: "Verify the pid before connecting", workflow: "feature", status: "awaiting_approval" },
      ],
    },
  },
  play: async ({ args, canvas }) => {
    const approving = canvas.getByRole("button", { name: /Approving/ });
    await expect(approving).toHaveAttribute("aria-busy", "true");
    await expect(approving).not.toBeDisabled();
    // Dispatched rather than clicked: the app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(approving);
    await expect(args.onApprove).not.toHaveBeenCalled();
  },
};

/**
 * Released, and the row says so. **The badge is Fleet's**, so a Job that has
 * left its gate draws no second approval — by this press or by anybody else's.
 */
export const Approved: Story = {
  args: {
    proposal: {
      at: "proposed",
      jobs: [
        { id: "job_11a0", title: "Move the runtime file into its own crate", workflow: "refactor", status: "queued" },
        { id: "job_11a1", title: "Verify the pid before connecting", workflow: "feature", status: "awaiting_approval" },
      ],
    },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /^Approve/ })).toBeNull();
  },
};

/**
 * Ending one: no workflow resolved.
 *
 * **Armada working, not Armada failing.** Fleet read the request, could not
 * resolve a workflow and returned it unchanged; no Job was created. So it takes
 * no red, no code chip and no solid fill — the rule on the left is
 * `--step-waiting`, which means needs you and not urgent.
 *
 * **Nothing is assigned by default**, because the resolved definition is frozen
 * into the Job and becomes the yardstick the work is judged against, so a
 * default would be the standard a Drone is held to rather than a guess somebody
 * could correct.
 */
export const NoWorkflowResolved: Story = {
  args: { proposal: { at: "unresolved" } },
  play: async ({ args, canvas, userEvent }) => {
    // The request is still on screen, which is the whole claim of this ending.
    await expect(canvas.getByText(/board flickers/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Edit the request" }));
    await expect(args.onEdit).toHaveBeenCalled();
    await expect(canvas.queryByRole("button", { name: "Dispatch another" })).toBeNull();
  },
};

/**
 * Ending two: somebody stopped it.
 *
 * **Nobody's failure, so it is not the error treatment.**
 * `crates/fleet/src/refusing.rs` gives `proposer_stopped` its own code for
 * exactly this reason: drawing it in red would tell a person Armada broke when
 * what happened is that they pressed a control Armada offered them.
 *
 * The sentence says the part that matters about a stop rather than an
 * abandonment — the call is dead, so nothing more is being spent.
 */
export const Stopped: Story = {
  args: { proposal: { at: "stopped" } },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/Nothing was created/)).toBeVisible();
    await expect(canvas.getByText(/nothing more is being spent/)).toBeVisible();
    // No code, which is what tells this apart from the fault below on the one
    // channel the design contract gives: an error always carries one.
    await expect(canvas.queryByText(/fleet\./)).toBeNull();
  },
};

/**
 * Ending three: the call could not be made.
 *
 * **Armada failing, so it is the error treatment.** It carries the code every
 * error carries, it is the one solid fill on this screen, and it renders inline
 * because blast radius picks the placement — a proposer that could not be
 * called stops this screen and reaches nothing else.
 *
 * **What to do about it is Fleet's own sentence**, because Fleet is what knows
 * whether a budget ran out, a key is missing or the provider was down.
 */
export const CallFaulted: Story = {
  args: {
    proposal: {
      at: "faulted",
      code: "fleet.model.budget_exhausted",
      message: "The proposer was not called: this manifest's model budget is spent for today.",
      payload: {
        code: "fleet.model.budget_exhausted",
        message: "The proposer was not called: this manifest's model budget is spent for today.",
        run_id: "run_8f21c0",
        fields: [{ key: "budget_window", value: "day" }],
        bridgeProtocol: "5.2",
        fleetProtocol: "5.2",
        at: "2026-09-02T22:14:03Z",
      },
    },
    onCopied: fn(),
  },
  play: async ({ canvas }) => {
    // The act is named beside the notice and performed by the footer: two
    // controls for one act a few lines apart is what that rule is about.
    await expect(canvas.getByText(/The request is not what failed/)).toBeVisible();
    await expect(canvas.getAllByRole("button", { name: "Edit the request" })).toHaveLength(1);
  },
};
