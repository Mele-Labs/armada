import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { PlanTaskSheet } from "./PlanTaskSheet";

/**
 * One plan task's whole reading, moved off the 380px rail onto the layer that
 * can hold it. The content below is a real plan Fleet recorded — seven tasks
 * for *Show what's running in the Drones stat* — because the case this exists
 * for is a long one: 613 characters of note, ten paths and 237 of evidence,
 * under a 75-character title.
 *
 * The sheet lays out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof PlanTaskSheet> = {
  title: "Compositions/Plan task sheet",
  component: PlanTaskSheet,
  args: { open: true, onClose: fn() },
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "var(--palette-max-height)",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PlanTaskSheet>;

const LONG = {
  id: "T1",
  title: "Fleet: one read of every Drone, running Check and Judge call on the machine",
  scope: [
    "crates/ipc/operations.toml",
    "crates/ipc/src/drones.rs",
    "crates/ipc/src/lib.rs",
    "crates/fleet/src/underway.rs",
    "crates/fleet/src/judging.rs",
    "crates/fleet/src/judging/marking.rs",
    "crates/fleet/src/serving.rs",
    "crates/api/src/fleetwide.rs",
    "crates/api/src/daemon/queries.rs",
    "crates/api/src/routes/served.rs",
  ],
  expects:
    "A Fleet-side test driving a Job mid-Check and a Job mid-Judge-call, asserting the new query's answer carries both rows plus the Drone roster, in one call.",
};

/** The case the sheet exists for: everything a task can carry, at once. */
export const Working: Story = {
  args: { ...LONG, state: "working" },
};

/**
 * **The header is the task's mark alone, named on hover**, and the task's
 * Drones are listed as the Drones tab lists them: a mark, the Drone, what it
 * spent. A press opens that Drone there. Protocol 23.1's `JobDrone.task`.
 */
export const ItsDrones: Story = {
  args: {
    ...LONG,
    state: "done",
    drones: [
      { id: "01A", label: "Drone on T1", state: "failed", stateSays: "Failed", spent: "4 turns · ~$0.20", onOpen: fn() },
      { id: "01B", label: "Drone on T1", state: "done", stateSays: "Done", spent: "12 turns · ~$0.90", onOpen: fn() },
    ],
  },
  play: async ({ canvasElement, args }) => {
    const sheet = within(canvasElement);
    // The state is a mark and never the word: no badge spells it.
    await expect(canvasElement.querySelector(".armada-badge")).toBeNull();
    const listed = within(sheet.getByRole("list", { name: "Drones on this task" }));
    await expect(listed.getAllByRole("listitem")).toHaveLength(2);
    await expect(listed.getByText("12 turns · ~$0.90")).toBeVisible();
    await userEvent.click(listed.getAllByRole("button", { name: "Drone on T1" })[1]!);
    await expect(args.drones?.[1]?.onOpen).toHaveBeenCalled();
  },
};

/**
 * **The two ends of the evidence read side by side, and nothing reconciles
 * them.** The plan named one artifact and the work proved it with another;
 * that is the useful answer, not a wrong one.
 */
export const Done: Story = {
  args: {
    ...LONG,
    state: "done",
    shown: "left-column.test.ts asserts the reading, not the Fleet-side test the plan named.",
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("The plan asked for")).toBeVisible();
    await expect(sheet.getByText("The work showed")).toBeVisible();
  },
};

/**
 * Every field a model or a person wrote — the brief, why it stopped, and both
 * ends of the evidence — drawn as the markdown it was written in.
 */
export const InMarkdown: Story = {
  args: {
    ...LONG,
    state: "failed",
    failedReason: "`cargo test` failed **twice** on the same case",
    note: "Add the read to Fleet:\n\n- one query\n- one route",
    expects: "A test in `fleetwide.rs`",
    shown: "**Nothing** was run",
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    const field = (label: string) => within(sheet.getByRole("heading", { name: label }).closest("section")!);
    await expect(field("Why it stopped").getByRole("code")).toHaveTextContent("cargo test");
    await expect(field("Why it stopped").getByRole("strong")).toHaveTextContent("twice");
    await expect(field("Brief").getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "one query",
      "one route",
    ]);
    const evidence = field("Done when");
    await expect(evidence.getByRole("code")).toHaveTextContent("fleetwide.rs");
    await expect(evidence.getByRole("strong")).toHaveTextContent("Nothing");
  },
};

/** A dropped task keeps its reason, and the reason leads. */
export const Dropped: Story = {
  args: {
    id: "T4",
    title: "Carry the reword everywhere else it is spelled",
    state: "dropped",
    reason: "`SettingsSurface.tsx` and `Board.tsx` were deleted in #1236 and #1235.",
    scope: ["packages/components/src/compositions/StatsPanel/StatsPanel.stories.tsx"],
    expects: 'StatsPanel.stories.tsx reads "1 running · 2 max".',
  },
  play: async ({ canvasElement }) => {
    const reason = within(within(canvasElement).getByRole("heading", { name: "Dropped because" }).closest("section")!);
    await expect(reason.getAllByRole("code").map((code) => code.textContent)).toEqual([
      "SettingsSurface.tsx",
      "Board.tsx",
    ]);
  },
};

/**
 * A task that names no files and no evidence. **An empty field is not
 * drawn**: no brief and nothing beside it leave no label with a sentence
 * standing in. What covers it is the one still said, because a task nothing
 * tests is the finding a person approves a plan by.
 */
export const NothingBeyondTheTitle: Story = {
  args: { id: "T7", title: "Cover the four states the definition of done names", state: "open" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.queryByRole("heading", { name: "Brief" })).toBeNull();
    await expect(sheet.queryByRole("heading", { name: "Runs beside" })).toBeNull();
    await expect(sheet.queryByRole("heading", { name: "Done when" })).toBeNull();
    await expect(sheet.getByText("No test covers this task yet.")).toBeVisible();
  },
};

/**
 * Everything the split added: the tier the planner gave it, what it runs
 * beside, and a case with no spec that reads **not covered** rather than
 * green. `#1535`.
 */
export const ItsOwnAgent: Story = {
  args: {
    id: "T6",
    title: "Open a Drone's Job from its row",
    state: "failed",
    scope: ["packages/screens/src/running-rows.tsx"],
    tier: "medium",
    model: "sonnet",
    beside: [
      {
        kind: "task",
        name: "Draw what is running, in four lists",
        activity: "advanced",
        said: "done",
        facts: [{ value: "T5" }, { value: "27 turns" }],
        onOpen: fn(),
      },
    ],
    failedReason: "The row's press opened the Board rather than the Job",
    tests: [
      { id: "c-panel", spec: "packages/screens/src/Running.test.tsx", reads: "owed" },
      { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
    ],
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("medium · sonnet")).toBeVisible();
    await expect(sheet.getByText("T5")).toBeVisible();
    await expect(sheet.getByText("not covered")).toBeVisible();
  },
};

/**
 * What only a task's own Drone can say: what it is doing now, the last file it
 * wrote, and a stop for it alone. `#1536`. **Mock-fed today** — Fleet runs one
 * Drone per Job until slices 1 and 5 of `docs/spikes/022`, and the screen draws
 * none of the three until then.
 */
export const ItsOwnDroneWorking: Story = {
  args: {
    id: "T5",
    title: "Draw what is running, in four lists",
    state: "working",
    note: "Keep the four lists in this order, and draw each row as the Board does.",
    scope: ["packages/screens/src/Running.tsx"],
    doing: "14 turns",
    lastEdit: { path: "packages/screens/src/Running.tsx", says: "+61 −4" },
    stop: {
      children: "Hold to stop this task",
      askLabel: "Stop this task",
      description: "Stops this task's drone once held until it fills. Letting go sooner stops nothing. The job stays open.",
      onCommit: fn(),
      onAsk: fn(),
    },
  },
};

/**
 * A task done with nothing recorded against it reads differently from one
 * still working: **"Nothing was recorded" is a gap and "Not yet" is a wait**,
 * and a reader acts on those differently.
 */
export const DoneAndNothingShown: Story = {
  args: { ...LONG, state: "done" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Nothing was recorded.")).toBeVisible();
  },
};

/** The labelled control is one of the sheet's two exits. */
export const Closes: Story = {
  args: { ...LONG, state: "working" },
  play: async ({ args, canvasElement }) => {
    const sheet = within(canvasElement);
    await userEvent.click(sheet.getByRole("button", { name: /close/i }));
    await expect(args.onClose).toHaveBeenCalled();
  },
};

/**
 * **The reading the fields exist for.** The plan named ten files; the work
 * reached eight of them, never opened two, and changed one nobody planned.
 * Neither half is an error — `crates/fleet/src/scope.rs` refuses to fail a
 * step for either — so both read as a quiet note rather than a warning.
 */
export const AgainstWhatItTouched: Story = {
  args: {
    ...LONG,
    state: "done",
    shown: "The Fleet-side test drives a Job mid-Check and one mid-Judge-call.",
    touched: {
      declared: LONG.scope.map((path, at) => ({ path, touched: at > 1 })),
      unplanned: ["crates/ipc/src/activity.rs"],
    },
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getAllByText("not touched")).toHaveLength(2);
    await expect(sheet.getAllByText("not planned")).toHaveLength(1);
    await expect(sheet.getByText("crates/ipc/src/activity.rs")).toBeVisible();
  },
};

/**
 * A task whose work reached everything it named. **No row is marked** — a
 * badge on every row would say nothing at all.
 */
export const EverythingItNamed: Story = {
  args: {
    id: "T3",
    title: "Reword the Drones stat itself",
    state: "done",
    scope: ["packages/screens/src/overview.ts", "packages/screens/src/overview.test.ts"],
    touched: {
      declared: [
        { path: "packages/screens/src/overview.ts", touched: true },
        { path: "packages/screens/src/overview.test.ts", touched: true },
      ],
      unplanned: [],
    },
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("packages/screens/src/overview.test.ts")).toBeVisible();
    await expect(sheet.queryByText("not touched")).toBeNull();
  },
};

/**
 * The Job's turns were never read, so there is nothing to compare against.
 * **It draws the declared list plainly** rather than a comparison claiming
 * every file went untouched, which is what an absent reading would become if
 * it were treated as an empty one.
 */
export const NotReadAgainstAnything: Story = {
  args: { ...LONG, state: "working" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("crates/api/src/routes/served.rs")).toBeVisible();
    await expect(sheet.queryByText("not touched")).toBeNull();
  },
};

/**
 * The plan is still a question, so the panel can propose a change to this
 * task. **The field is the confirmation**: an empty one sends nothing, which
 * is why Send is off until something is typed. Verified against a sheet whose
 * `propose` is absent, where neither is drawn at all.
 */
export const ProposingAChange: Story = {
  args: {
    ...LONG,
    state: "open",
    propose: {
      label: "Propose a change",
      send: "Send to the Drone",
      onPropose: fn(),
    },
  },
  play: async ({ canvasElement, args }) => {
    const sheet = within(canvasElement);
    await userEvent.click(sheet.getByRole("button", { name: "Propose a change" }));
    const send = sheet.getByRole("button", { name: "Send to the Drone" });
    await expect(send).toBeDisabled();
    await userEvent.type(
      sheet.getByRole("textbox"),
      "Split the rows out, so the tests have something smaller to hold",
    );
    await expect(send).toBeEnabled();
    await userEvent.click(send);
    await expect(args.propose?.onPropose).toHaveBeenCalledWith(
      "Split the rows out, so the tests have something smaller to hold",
    );
  },
};

/** A change is out. The control refuses a second one until it is answered. */
export const TheProposalIsOut: Story = {
  args: {
    ...ProposingAChange.args,
    propose: { ...ProposingAChange.args!.propose!, pending: true },
  } as Story["args"],
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByRole("button", { name: "Propose a change" })).toBeDisabled();
  },
};

/**
 * Edit this task, filled from the task, sending only what changed. Save is
 * off until something differs.
 */
export const EditingTheTask: Story = {
  args: {
    ...LONG,
    state: "open",
    edit: { models: ["opus", "sonnet", "haiku"], onEdit: fn(async () => false) },
  },
  play: async ({ canvasElement, args }) => {
    const sheet = within(canvasElement);
    await userEvent.click(sheet.getByRole("button", { name: "Edit this task" }));
    const save = sheet.getByRole("button", { name: "Save" });
    await expect(save).toBeDisabled();
    await userEvent.selectOptions(sheet.getByLabelText("Model"), "haiku");
    await userEvent.click(save);
    await expect(args.edit?.onEdit).toHaveBeenCalledWith({ model: "haiku" });
  },
};

/**
 * Twenty-four paths.
 *
 * **A break test, because a cap that stopped working looks like a long list.**
 * The claim is a number of rows and a control that changes it, not that files
 * are drawn — a sheet with the cap removed passes every other story here.
 * The owner asked what 20+ would do (28 Sep 2026).
 */
export const ManyFiles: Story = {
  args: {
    id: "T1",
    title: "Fleet: one read of every Drone",
    state: "done",
    scope: Array.from({ length: 24 }, (_, at) => `crates/fleet/src/file-${at + 1}.rs`),
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    const files = canvasElement.querySelector(".armada-task-sheet__files")!;
    await expect(files.children).toHaveLength(12);
    await expect(sheet.queryByText("crates/fleet/src/file-13.rs")).toBeNull();
    await userEvent.click(sheet.getByRole("button", { name: "Show 12 more" }));
    await expect(files.children).toHaveLength(24);
    await expect(sheet.queryByRole("button", { name: /Show \d+ more/ })).toBeNull();
  },
};

/**
 * A path the plan never named survives the cap.
 *
 * **It is the row worth stopping on**, so it is never what the twelve eat —
 * twelve declared paths plus every unplanned one.
 */
export const ManyFilesAndOneUnplanned: Story = {
  args: {
    ...ManyFiles.args,
    touched: {
      declared: Array.from({ length: 24 }, (_, at) => ({
        path: `crates/fleet/src/file-${at + 1}.rs`,
        touched: true,
      })),
      unplanned: ["crates/ipc/src/activity.rs"],
    },
  } as Story["args"],
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(canvasElement.querySelector(".armada-task-sheet__files")!.children).toHaveLength(13);
    await expect(sheet.getByText("crates/ipc/src/activity.rs")).toBeVisible();
  },
};

/**
 * Every label is a register over plain text, and no value is boxed.
 *
 * **A break test on a style, because this note came back.** The value sat in a
 * sunken box once, and the owner read it as a field he could type in (29 Sep
 * 2026): a panel that only reads must not look like a form. The label's small
 * caps in `--fg-subtle` over `--fg-default` is the separator, and a
 * stylesheet that put the box back would pass every other story here.
 */
export const LabelsAndValues: Story = {
  args: { ...LONG, state: "working", tier: "difficult", model: "opus" },
  play: async ({ canvasElement }) => {
    const labels = canvasElement.querySelectorAll(".armada-task-sheet__label");
    const values = canvasElement.querySelectorAll(".armada-task-sheet__value");
    await expect(labels.length).toBeGreaterThan(3);
    await expect(values).toHaveLength(labels.length);
    const model = [...values].find((one) => one.textContent === "difficult · opus")!;
    const box = getComputedStyle(model);
    await expect(parseFloat(box.borderBottomWidth)).toBe(0);
    await expect(box.backgroundColor).toBe("rgba(0, 0, 0, 0)");
    await expect(getComputedStyle(labels[0]!).color).not.toBe(box.color);
  },
};

/**
 * The id is left of the title, at the head's leading edge; the state is the
 * task's mark under it, named on hover and never spelled.
 *
 * **A break test on position**, which a text assertion cannot see: both are
 * in the head either way.
 */
export const TheIdLeads: Story = {
  args: { ...LONG, state: "done" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    const id = sheet.getByText("T1");
    const title = sheet.getByRole("heading", { name: LONG.title });
    await expect(id.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await expect(id.getBoundingClientRect().left).toBeLessThan(title.getBoundingClientRect().left);
    await expect(sheet.getByRole("img", { name: "Done" })).toBeVisible();
    await expect(canvasElement.querySelector(".armada-badge")).toBeNull();
  },
};
