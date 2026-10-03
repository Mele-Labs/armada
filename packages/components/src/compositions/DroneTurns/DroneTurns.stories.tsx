import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { DroneTurns, type DroneTurn } from "./DroneTurns";
import { NOTHING_YET, thinking } from "./DroneTurns.fixtures";

/**
 * What a screen reader is told, leaving out the tooltip's bubble, which is
 * hidden from it. A thinking run's state is its mark's name and its tooltip,
 * never a word on the line (the owner, 2 Oct 2026).
 */
const SPOKEN = { ignore: "script, style, [aria-hidden], [aria-hidden] *" };

/**
 * One Drone's turns, read while it is still working.
 *
 * **A card per speaker.** Consecutive rows from one of the activity log's three
 * voices — Drone, Armada or Fleet — are one card headed with its name, and the
 * rows say what. A refusal says so in its body. The owner drew it, 29 Sep 2026,
 * replacing a column that named the speaker on every row.
 *
 * **A call that simply answered says nothing more.** Only an answer carrying
 * more than that — a failure — is drawn.
 *
 * **A call and its answer are one row**, joined on the call id by whoever
 * builds these. Fleet puts both on the wire because joining them there would
 * mean holding a call open until its result arrived.
 *
 * **A run of the Drone thinking is one line.** Measured on one real transcript:
 * 106 of 149 rows were kinds the decoder could not place, so three lines in
 * four described the plumbing rather than the work. They collapse to one row
 * that does not open, carrying the tokens they cost.
 *
 * **The step is a boundary, not a column.** One Drone works several steps, and
 * a name repeated down every row would be the same string forty times over. The
 * line is drawn where the step changed and every row beneath it is answered by
 * position — including a step that runs, stops and runs again.
 */
const meta: Meta<typeof DroneTurns> = {
  title: "Compositions/Drone turns",
  component: DroneTurns,
};
export default meta;

type Story = StoryObj<typeof DroneTurns>;

/**
 * The ordinary transcript: Armada's opening turn, then the Drone's session, some
 * prose, and calls with their answers — two speakers, so two cards.
 *
 * The third call carries no detail, which is the wire saying it had no name for
 * that tool's arguments rather than a field that failed to arrive — so the row
 * falls back to the call id, which is then the only thing telling it from the
 * next `Bash`.
 */
const turns: DroneTurn[] = [
  {
    id: "0",
    at: "09:14:01",
    who: "armada",
    kind: "instructed",
    said: "Split the settings reducer so the selectors can be tested alone.",
  },
  {
    id: "1",
    at: "09:14:02",
    who: "drone",
    kind: "started",
    // The model is whatever the Job named. A vendor spelling belongs in
    // `adapters` and nowhere else, so the fixture carries a placeholder.
    subject: "sess_01JB4 · the job's model · 2 mcp servers",
  },
  {
    id: "2",
    at: "09:14:03",
    who: "drone",
    kind: "said",
    said: "Reading the settings module before I split anything, so the public signature survives.",
  },
  {
    id: "3",
    at: "09:14:04",
    who: "drone",
    kind: "called",
    subject: "Read",
    detail: "src/settings.rs",
  },
  {
    id: "4",
    at: "09:14:09",
    who: "drone",
    kind: "called",
    subject: "Bash",
    detail: "cargo test -p settings --lib",
    answer: "Failed.",
  },
  {
    id: "5",
    at: "09:14:10",
    who: "drone",
    kind: "called",
    subject: "TodoWrite · call_7f23",
  },
  {
    id: "6",
    at: "09:14:11",
    who: "drone",
    kind: "called",
    subject: "Edit",
    detail: "src/settings.rs +42 -18",
    answer: "No answer yet.",
  },
];

export const ADroneWorking: Story = {
  args: { turns, emptyNote: NOTHING_YET },
  // A change of speaker starts a card, each card is named by its speaker, and
  // what does not happen: a call that answered and said nothing else draws no
  // word for it, and no row carries the wire's event word.
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("group")).toHaveLength(2);
    await expect(canvas.getByRole("group", { name: "Armada" })).toBeVisible();
    const drone = within(canvas.getByRole("group", { name: "Drone" }));
    await expect(drone.getByText("src/settings.rs")).toBeVisible();
    await expect(drone.queryByText(/answered/i)).toBeNull();
    await expect(drone.queryByText("called")).toBeNull();
  },
};

/**
 * Tool calls as a quiet block under the sentences.
 *
 * **Reading down the card gives what the Drone said.** Consecutive calls — a
 * refused one and a failed one included — gather into one block, smaller and
 * dimmer behind a rule, and a sentence between two calls splits them into two
 * blocks. The owner, 29 Sep 2026: *"the lines all just kind of blend
 * together."* A thinking run between two calls folds into the block as a line
 * of its own; the run's closing line stays out, since it is neither.
 */
export const ToolCallsReadAsAQuietBlock: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "05:12:20", who: "drone", kind: "said", said: "Starting on T1: Serve one read of everything running. Reading first." },
      { id: "2", at: "05:12:21", who: "drone", kind: "called", subject: "Read", detail: "crates/api/src/running.rs" },
      { id: "3", at: "05:12:22", who: "drone", kind: "called", subject: "Read", detail: "crates/ipc/operations.toml" },
      { id: "4", at: "05:14:02", who: "drone", kind: "said", said: "The change belongs in running.rs. Writing it now." },
      { id: "5", at: "05:14:10", who: "drone", kind: "called", subject: "Edit", detail: "crates/api/src/running.rs" },
      ...thinking(10, 4, "05:14:11"),
      { id: "6", at: "05:15:40", who: "drone", kind: "called", subject: "Bash", detail: "cargo test -p api", answer: "Failed." },
      { id: "7", at: "05:15:41", who: "drone", kind: "refused", subject: "Bash", said: "Refused: this command is not on the allowlist for this drone." },
      { id: "20", at: "05:16:30", who: "drone", kind: "said", said: "The read answers Drones, Checks and Judge calls in one." },
      { id: "21", at: "05:16:31", who: "drone", kind: "ended", subject: "34 turns · $2.40" },
    ],
  },
  play: async ({ canvas }) => {
    const blocks = canvas.getAllByRole("list", { name: "Tool calls" });
    await expect(blocks).toHaveLength(2);
    const [reads, edits] = blocks.map((block) => within(block));
    await expect(reads?.getAllByRole("listitem")).toHaveLength(2);
    // Three calls and the thinking between them, which does not split the block.
    await expect(edits?.getAllByRole("listitem")).toHaveLength(4);
    await expect(edits?.getByText("Thinking", SPOKEN)).toBeInTheDocument();
    await expect(edits?.getByText("Failed.")).toBeVisible();
    await expect(edits?.getByText(/^Refused:/)).toBeVisible();
    // Sentences and the closing line are never inside a block.
    await expect(reads?.queryByText(/Writing it now/)).toBeNull();
    await expect(edits?.queryByText("34 turns · $2.40")).toBeNull();
  },
};

/**
 * A thinking run is one row with the tokens its rows added (the owner, 29 Sep
 * 2026): no count of rows nobody can open, no control to open them, and never
 * the wire's kind. Its mark is named `Thinking`; while it is the live tail it
 * pulses and is named `Working` instead.
 */
export const ThinkingInWords: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "05:12:21", who: "drone", kind: "called", subject: "Read", detail: "crates/api/src/running.rs" },
      { id: "2", at: "05:12:22", who: "drone", kind: "unrecognised", quiet: true, thought: { of: "thinking", tokens: 400 } },
      { id: "3", at: "05:12:23", who: "drone", kind: "unrecognised", quiet: true, thought: { of: "thinking", tokens: 900 } },
      { id: "4", at: "05:12:24", who: "drone", kind: "unrecognised", quiet: true, subject: "the Drone's reasoning, not carried" },
      { id: "5", at: "05:14:10", who: "drone", kind: "called", subject: "Edit", detail: "crates/api/src/running.rs" },
      { id: "6", at: "05:14:11", who: "drone", kind: "unrecognised", quiet: true, thought: { of: "thinking", tokens: 830 } },
    ],
  },
  play: async ({ canvas }) => {
    const block = within(canvas.getByRole("list", { name: "Tool calls" }));
    await expect(block.getAllByRole("listitem")).toHaveLength(4); // two calls, two runs of one row each
    await expect(block.getAllByText("Thinking", SPOKEN)).toHaveLength(2);
    await expect(block.getByText("~1,300 tokens")).toBeVisible();
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.queryByText(/turns?\b/)).toBeNull();
    await expect(canvas.queryByText(/system\/thinking_tokens|reasoning/)).toBeNull();
  },
};

/** The same run while a Drone writes: only its tail is named `Working`. */
export const ThinkingWhileLive: Story = {
  args: { ...ThinkingInWords.args, live: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Thinking", SPOKEN)).toBeInTheDocument();
    const tail = canvas.getByText("Working", SPOKEN).closest("li");
    await expect(tail).toContainElement(canvas.getByText("~830 tokens"));
  },
};

/**
 * A Job nobody dispatched. **Ordinary, not an error** — the socket opens, says
 * nothing is writing, sends no rows and closes — so **nothing is drawn**.
 */
export const AJobWithNoTranscript: Story = {
  args: { turns: [] },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.textContent).toBe("");
  },
};

/**
 * The three rows that are never gaps: a refusal, a kind this build does not
 * know, and a line that did not decode. A view that hid any of them would
 * report a quiet stream where there was a broken one.
 *
 * The middle one is a run of length one and still collapses. Left alone it
 * would render the decoder's own words for a turn it could not place, which is
 * the reading the collapse exists to remove.
 */
export const RefusedUnrecognisedAndUnreadable: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      {
        id: "1",
        at: "09:15:40",
        who: "drone",
        kind: "refused",
        subject: "Bash · call_7f31",
        said: "Refused: this command is not on the allowlist for this drone.",
      },
      { id: "2", at: "09:15:41", who: "drone", kind: "unrecognised", subject: "thinking_delta", quiet: true },
      {
        id: "3",
        at: "09:15:42",
        who: "drone",
        kind: "unreadable",
        subject: '{"type":"assistant","message":{"content":[{"type":"to',
        said: "The line ended mid-object.",
      },
    ],
  },
};

/**
 * What a call did, as the wire carries it.
 *
 * **Not derived from the tool name, ever.** A row that has no detail shows the
 * tool and the opaque call id instead, which is what made twenty-two rows read
 * alike and is why the wire carries this at all. The last row is a value the
 * wire cut short — a `Write` argument is a whole file — and it renders as cut
 * rather than as the whole thing.
 */
export const WhatEachCallDid: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      {
        id: "1",
        at: "09:16:02",
        who: "drone",
        kind: "called",
        subject: "Read",
        detail: "src/settings.rs",
      },
      {
        id: "2",
        at: "09:16:04",
        who: "drone",
        kind: "called",
        subject: "Edit",
        detail: "reducer.rs +42 -18",
      },
      {
        id: "3",
        at: "09:16:09",
        who: "drone",
        kind: "called",
        subject: "Grep",
        detail: "fn observe\\( in crates/",
      },
      {
        id: "4",
        at: "09:16:20",
        who: "drone",
        kind: "called",
        subject: "Write",
        detail: "docs/practices/bridge.md, 412 lines starting # Bridge practices",
        truncated: true,
        answer: "No answer yet.",
      },
    ],
  },
};

/**
 * A Drone thinking now. The last run is the one that can still be happening, so
 * it is the one that says **Working** and the only mark that moves — a run with
 * rows after it already ended.
 */
export const ADroneThinking: Story = {
  args: {
    live: true,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", who: "drone", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", who: "drone", kind: "called", subject: "Read", detail: "src/settings.rs" },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18" },
      ...thinking(60, 6, "09:14:40"),
    ],
  },
};

/**
 * The same transcript, finished. **No mark moves and no line says Working** —
 * a live mark on a gap in the middle of a history that ended would claim work
 * that stopped.
 */
export const AFinishedRun: Story = {
  args: {
    live: false,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", who: "drone", kind: "said", said: "Starting on the settings split." },
      ...thinking(10, 9, "09:14:03"),
      { id: "20", at: "09:14:12", who: "drone", kind: "called", subject: "Read", detail: "src/settings.rs" },
      ...thinking(30, 14, "09:14:15"),
      { id: "50", at: "09:14:31", who: "drone", kind: "called", subject: "Edit", detail: "src/settings.rs +42 -18" },
      { id: "60", at: "09:14:44", who: "drone", kind: "said", said: "The public signature is unchanged. Submitting." },
      { id: "61", at: "09:14:45", who: "drone", kind: "ended", subject: "18 turns · ~$0.42 · no calls refused" },
    ],
  },
};

/**
 * What the run cost, on the run's own last row.
 *
 * **This is the only place a spend figure appears**, and that is the decision
 * rather than an omission. `ended` is one Drone's total, so a Job that retried
 * has one of these per Drone — a number on the Job would be a sum the wire
 * declines to compute, and the board is not where a figure nobody is deciding
 * on belongs. It also arrives on the Observe socket alone, so an outcome region
 * drawn with that socket closed could only carry a labelled blank.
 *
 * **The figure hedges and the counts do not.** P4 of the design contract:
 * spend is estimated and is spelled `~$1.53`, a turn count is measured and
 * speaks flatly. Rendering the two alike would destroy trust in both.
 *
 * The two rows are the pair #161 was filed over — a run that burned a dollar
 * over forty-one turns and one that gave up in four, which read identically
 * until this row existed.
 */
export const WhatTheRunCost: Story = {
  args: {
    live: false,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:14:02", who: "drone", kind: "said", said: "Starting on the settings split." },
      { id: "2", at: "09:55:10", who: "drone", kind: "ended", subject: "41 turns · ~$1.53 · 6 calls refused" },
      { id: "3", at: "10:02:11", who: "drone", kind: "said", said: "Retrying the step. Reading the refusal first." },
      { id: "4", at: "10:03:40", who: "drone", kind: "ended", subject: "4 turns · ~$0.0018 · no calls refused" },
    ],
  },
};

/** Nothing but tool calls. No run to collapse, so no line is added to the pane. */
export const NothingButToolCalls: Story = {
  args: {
    live: true,
    emptyNote: NOTHING_YET,
    turns: [
      { id: "1", at: "09:20:01", who: "drone", kind: "called", subject: "Bash", detail: "cargo xtask verify-foundations" },
      { id: "2", at: "09:20:31", who: "drone", kind: "called", subject: "Bash", detail: "cargo test -p ipc", answer: "Failed." },
      { id: "3", at: "09:20:48", who: "drone", kind: "called", subject: "Read", detail: "crates/ipc/src/turn.rs" },
      { id: "4", at: "09:20:52", who: "drone", kind: "called", subject: "Grep", detail: "Saw::Called in crates/", answer: "No answer yet." },
    ],
  },
};

/**
 * The Drone writes markdown, and a `said` row draws it: emphasis, a list, a
 * name in code.
 *
 * **Only the Drone's own sentence.** The refusal under it is the harness's
 * wording, and the underscores in a tool name are not emphasis — so it is
 * drawn as the characters it is.
 */
export const TheDroneWritesMarkdown: Story = {
  args: {
    emptyNote: NOTHING_YET,
    turns: [
      {
        id: "1",
        at: "09:14:02",
        who: "drone",
        kind: "said",
        said: "Splitting the reducer. **The public signature** stays put:\n\n- `selectSettings` keeps its name\n- the tests move with it",
      },
      {
        id: "2",
        at: "09:14:05",
        who: "drone",
        kind: "refused",
        subject: "mcp__fleet__read",
        said: "Refused: mcp__fleet__read is not on the allowlist for this drone.",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The public signature").tagName).toBe("STRONG");
    await expect(canvas.getByText("selectSettings").tagName).toBe("CODE");
    await expect(canvas.getByText("the tests move with it").tagName).toBe("LI");
    await expect(canvas.getByText("Refused: mcp__fleet__read is not on the allowlist for this drone.")).toBeVisible();
  },
};
