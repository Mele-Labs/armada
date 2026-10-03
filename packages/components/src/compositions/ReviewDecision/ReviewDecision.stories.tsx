import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor } from "storybook/test";
import { ReviewDecision, type DecisionAct } from "./ReviewDecision";

/**
 * The answers to a job waiting at a human gate, and the note one of them
 * carries.
 *
 * The reply field is on the surface rather than behind a control, because
 * reviewing and replying is one loop: a design that puts the reply in a
 * separate route, tab or modal from the diff is the thing `bridge.md` says to
 * push back on before it is built.
 *
 * **Two split buttons, the owner's own arrangement of 30 Sep 2026.** Merge
 * with Approve behind its caret; Request changes with Reject behind its caret,
 * `danger` and last. Every one of the four says what it does on hover, because
 * two of them no longer have a face to be read from.
 *
 * **Merge is drawn only where there is a pull request**, and it takes the
 * primary fill when it is. A job holding one has a single ordinary ending, and
 * it is not "record this done and leave the branch on the forge".
 */
const meta: Meta<typeof ReviewDecision> = {
  title: "Compositions/Review decision",
  component: ReviewDecision,
  args: {
    note: "",
    onNote: () => {},
    onApprove: () => {},
    onRequestChanges: () => {},
    onReject: () => {},
  },
};
export default meta;

type Story = StoryObj<typeof ReviewDecision>;

/**
 * At rest. `Request changes` is off because the note is blank — refused before
 * the press, matching the 422 Fleet would give it rather than making a person
 * read a refusal to learn a field was empty.
 */
export const NothingWrittenYet: Story = {
  args: { note: "", onReject: fn() },
  /**
   * **A blank note refuses the face and not the caret.** The two are different
   * controls now, and a rendering cannot show that the second one is still
   * live — so the press is made here. Reject's own sentence comes with it into
   * the menu.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Request changes" })).toBeDisabled();

    const caret = canvas.getByRole("button", { name: "The other way to end this review" });
    await expect(caret).toBeEnabled();
    await userEvent.click(caret);

    const reject = canvas.getByRole("menuitem", { name: "Reject the work" });
    await userEvent.hover(reject);
    await waitFor(() =>
      expect(
        canvas.getByText("A verdict on the work, and the job ends there.", { exact: false }),
      ).toBeVisible(),
    );

    await userEvent.click(reject);
    await expect(args.onReject).toHaveBeenCalled();
  },
};

/** A note written, so the reply is live and all three answers are available. */
export const ANoteWritten: Story = {
  args: {
    note:
      "The gate change is right, but AdvanceGate::HumanAlways is handled in gate.rs and not in " +
      "config's loader, so a workflow declaring it is still refused at load. Add the arm there " +
      "and a test that loads one.",
  },
};

/** What should change, listed from the review and View. A listed change is not a blank note. #907. */
export const ChangesListed: Story = {
  args: {
    note: "",
    onRemoveChange: fn(),
    onRequestChanges: fn(),
    changes: [
      {
        id: "small-fix-0",
        from: "Small fix",
        text: "The store module is over 500 lines: move the migration into its own file.",
      },
      {
        id: "view-0",
        from: "From View: A busy CPU no longer delays a Job",
        text: "Keep the CPU reading in Doctor, and say in the status bar that CPU never holds a Job.",
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    // No count beside the label — it read as part of the sentence.
    await expect(canvas.getByText("What should change")).toBeVisible();

    const send = canvas.getByRole("button", { name: "Request changes" });
    await expect(send).toBeEnabled();
    await userEvent.click(send);
    await expect(args.onRequestChanges).toHaveBeenCalled();

    await userEvent.click(canvas.getByRole("button", { name: /^Remove The store module/ }));
    await expect(args.onRemoveChange).toHaveBeenCalledWith("small-fix-0");
    // The field is `Notes` whether or not anything is listed above it: the
    // list carries its own label now. The owner's word, 30 Sep 2026.
    await expect(canvas.getByLabelText("Notes")).toBeVisible();
  },
};

/**
 * A job whose branch went out, so there is a pull request to merge. **Merge is
 * the face and Approve steps behind its caret**: approving here records the
 * job done and leaves the branch open on the forge, which is the ending the
 * merge control exists to stop being the easy one.
 *
 * Approving is still one press away, because a person may want the job closed
 * without landing the branch — a change somebody else will carry, or one that
 * is merging by another route — and its hover is where what that costs is
 * read now that it has no face of its own.
 *
 * **The press asks; it does not merge.** This is the confirmation's closed
 * state — the caller opens a dialog on `onMerge`, the way it does on
 * `onReject`, because merging writes into a repository Fleet did not make and
 * nothing in Bridge takes it back. The words are `Primitives/Dialog → Merge the
 * pull request`, and what the screen wires is proven in
 * `packages/screens/src/Decide.test.tsx`, where a screen can be mounted.
 */
export const APullRequestToMerge: Story = {
  args: {
    note: "",
    onMerge: fn(),
    onApprove: fn(),
  },
  /**
   * **Each of the four says what it does, and two of them only have a hover
   * to say it in.** A rendering shows the two faces; nothing about it shows
   * what is behind either caret, or that choosing from one sends the act.
   */
  play: async ({ args, canvas, userEvent }) => {
    const merge = canvas.getByRole("button", { name: "Merge pull request" });
    await userEvent.hover(merge);
    await waitFor(() =>
      expect(canvas.getByText("Merges the pull request on its code host", { exact: false })).toBeVisible(),
    );
    await userEvent.unhover(merge);

    await userEvent.click(canvas.getByRole("button", { name: "The other way to take this work" }));
    const approve = canvas.getByRole("menuitem", { name: "Approve the work" });
    await userEvent.hover(approve);
    await waitFor(() => expect(canvas.getByText("Takes the work as the drone left it.")).toBeVisible());
    await userEvent.click(approve);
    await expect(args.onApprove).toHaveBeenCalled();

    // Merge asks on the face, and the caller's dialog is what merges.
    await userEvent.click(merge);
    await expect(args.onMerge).toHaveBeenCalled();
  },
};

/**
 * `#663`: the branch conflicts with main, so Fleet would refuse the merge.
 * **The face goes off and the caret does not** — Approve is behind that caret
 * and it is the act a person who cannot merge most likely wants, so refusing
 * the whole control would refuse the answer as well as the one Fleet objects
 * to. The reason sits under the row, never folded into `disabledNote`, which
 * belongs to the whole group and is not in play here. `#1131`: Fleet sends a
 * Drone to clear it on its own, so the reason names what is happening rather
 * than something to press.
 */
export const TheBranchConflicts: Story = {
  args: {
    note: "",
    onMerge: () => {},
    onApprove: fn(),
    mergeBlockedReason: "This branch conflicts with main. Fleet sends it back for a Drone to clear the conflicts.",
  },
  play: async ({ args, canvas, canvasElement, userEvent }) => {
    const merge = canvas.getByRole("button", { name: "Merge pull request" });
    await expect(merge).toBeDisabled();

    // The reason sits under the row, not beside it, and Merge still names it.
    const reason = canvasElement.querySelector<HTMLElement>('[role="note"]')!;
    await expect(reason).toHaveTextContent("This branch conflicts with main.");
    await expect(reason.getBoundingClientRect().top).toBeGreaterThanOrEqual(
      merge.getBoundingClientRect().bottom,
    );
    await expect(merge.getAttribute("aria-describedby")?.split(" ")).toContain(reason.id);

    // The caret beside the dead face still opens, and Approve still sends.
    await userEvent.click(canvas.getByRole("button", { name: "The other way to take this work" }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "Approve the work" }));
    await expect(args.onApprove).toHaveBeenCalled();
  },
};

/**
 * The same, with a decision already in flight. Every control is off, including
 * the merge — the one act here that writes into a repository Armada does not
 * own, and the last one that should be pressable twice.
 */
export const AMergeAlreadySent: Story = {
  args: {
    note: "",
    onMerge: () => {},
    disabled: true,
    disabledNote: "A decision on this job is already in flight. It was not sent twice.",
  },
};

/**
 * A decision already in flight. Every control is off, and the sentence says
 * why — a disabled group with no reason is a surface that looks broken.
 */
export const ADecisionAlreadySent: Story = {
  args: {
    note: "Add the arm in config's loader and a test that loads one.",
    disabled: true,
    disabledNote: "A decision on this job is already in flight. It was not sent twice.",
  },
};

const NOTE = "Add the arm in config's loader and a test that loads one.";

/**
 * Request changes pressed, and Fleet has not answered. The pressed control
 * waits and says so; the others are off, with no sentence about a second press
 * because the control already shows the first. #1117.
 */
export const WaitingOnFleet: Story = {
  args: { note: NOTE, pending: "changes" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Approve the work" })).toBeDisabled();
    // Nothing else opens while a press is out, so Reject is not reachable
    // either — the caret that discloses it is off with the rest.
    await expect(canvas.getByRole("button", { name: "The other way to end this review" })).toBeDisabled();
    await expect(canvas.queryByRole("menuitem")).toBeNull();
    await expect(canvas.queryByRole("status")).toBeNull();
  },
};

/**
 * Approve was chosen behind Merge's caret and Fleet has not answered. **The
 * face says `Approving…`**, because once the menu has closed the face is the
 * only surface that control still has — `SplitButton`'s own `pendingLabel`.
 * A face still reading `Merge pull request` would name the act that was
 * not sent. #1117.
 */
export const WaitingOnAnActChosenBehindTheCaret: Story = {
  args: { note: "", onMerge: () => {}, pending: "approve" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Approving…" })).toHaveAttribute("aria-busy", "true");
    await expect(canvas.queryByRole("button", { name: "Merge pull request" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Request changes" })).toBeDisabled();
  },
};

/**
 * Fleet took the Approve that was chosen behind Merge's caret. **The face
 * still reads `Approve the work` while the answer is drawn**, and goes back to
 * `Merge pull request` when it clears.
 *
 * The line is drawn on that face. A line meaning accepted beside a face
 * reading `Merge pull request` would name the act that did not go out, at
 * the one moment a person is checking that the right one did — so the label
 * holds past the press, not only during it.
 */
export const AnsweredOnAnActChosenBehindTheCaret: Story = {
  args: { note: "", onMerge: () => {}, answered: { act: "approve", answer: "accepted" } },
  play: async ({ canvas }) => {
    const face = canvas.getByRole("button", { name: "Approve the work" });
    await expect(canvas.queryByRole("button", { name: "Merge pull request" })).toBeNull();
    // `data-answer` is read because no accessible property carries the line
    // along a control's edge — `Decide.answer.test.tsx`'s own reason. What
    // matters here is that it is on the face whose words name the act.
    await expect(face).toHaveAttribute("data-answer", "accepted");
  },
};

/** The same for Reject, behind Request changes' caret. Its dialog makes the mismatch likelier to be seen. */
export const AnsweredOnRejectChosenBehindTheCaret: Story = {
  args: { note: NOTE, answered: { act: "reject", answer: "accepted" } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Reject the work" })).toHaveAttribute(
      "data-answer",
      "accepted",
    );
    await expect(canvas.queryByRole("button", { name: "Request changes" })).toBeNull();
  },
};

/** Five seconds on and Fleet still has not answered, so the group says so. */
export const StillWaitingOnFleet: Story = {
  args: { note: NOTE, pending: "changes" },
  play: async ({ canvas }) => {
    const said = await canvas.findByRole("status", {}, { timeout: 7000 });
    await expect(said).toHaveTextContent("Still waiting on Fleet.");
  },
};

/** Stands in for the app: a press goes out, and Fleet answers or refuses it. */
function Pressing({ answer }: { answer: "answered" | "refused" }) {
  const [note, setNote] = useState(NOTE);
  const [pending, setPending] = useState<DecisionAct | undefined>(undefined);
  const [moved, setMoved] = useState(false);
  if (moved) return <p>Changes requested. The job is running again.</p>;
  return (
    <ReviewDecision
      note={note}
      onNote={setNote}
      onApprove={() => {}}
      onReject={() => {}}
      onRequestChanges={() => {
        setPending("changes");
        setTimeout(() => {
          setPending(undefined);
          setMoved(answer === "answered");
        }, 600);
      }}
      {...(pending === undefined ? {} : { pending })}
    />
  );
}

/** Fleet takes it. The Job moves on Fleet's word, so the decision goes with it. */
export const FleetAnswered: Story = {
  render: () => <Pressing answer="answered" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Request changes" }));
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(await canvas.findByText("Changes requested. The job is running again.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /Request/ })).toBeNull();
  },
};

/**
 * Fleet refuses it. Nothing moved, so nothing snaps back: the controls are
 * live again and the note is still there. The refusal itself is the app's
 * failure notice, not this block's.
 */
export const FleetRefused: Story = {
  render: () => <Pressing answer="refused" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Request changes" }));
    await expect(canvas.getByRole("button", { name: "Requesting changes…" })).toBeVisible();
    const again = await canvas.findByRole("button", { name: "Request changes" });
    await expect(again).toBeEnabled();
    await expect(again).not.toHaveAttribute("aria-busy");
    await expect(canvas.getByRole("textbox")).toHaveValue(NOTE);
  },
};

/**
 * Fleet is not connected. The same disabled treatment and a different sentence,
 * because "already sent" and "nothing to send it over" are different things to
 * do about it.
 */
export const NotConnectedToFleet: Story = {
  args: {
    note: "",
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing here can be sent.",
  },
};
