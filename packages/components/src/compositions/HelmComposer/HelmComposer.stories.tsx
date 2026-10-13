import type { ReactElement, ReactNode } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, userEvent, within } from "storybook/test";

import { HelmComposer, type HelmBox, type HelmRepositoryOption } from "./HelmComposer";

const repositories: HelmRepositoryOption[] = [
  { id: "01M2ARMADA", label: "armada" },
  { id: "01M2SHOP", label: "shop-01" },
];

/** The switch's own entry while Helm is pointed at nothing — the word the repository asks elsewhere use. */
const UNPOINTED = "Choose a repository";

/** What the switch is called with Helm pointed at nothing: there is no *different* repository yet. */
const POINT = "Point Helm at a repository";

/** And once it is pointed at one. */
const REPOINT = "Point Helm at a different repository";

/** The caret's own name on the record's split button — what is behind it, in words. */
const MORE = "More ways to report this answer";

/**
 * Pressing the switch and choosing a row, the way a person does. The menu is drawn in a layer of
 * its own over the page, so its rows are found on the screen and not inside the story's canvas.
 */
async function choose(
  switcher: HTMLElement,
  label: string,
  events: { click: (target: Element) => Promise<void> },
): Promise<void> {
  await events.click(switcher);
  await events.click(await screen.findByRole("menuitem", { name: label }));
}

/** The dock's own width and glass — every story draws inside it, so a story that
 *  types is measured at the width the composer really has. */
function InTheDock({ children }: { children: ReactNode }): ReactElement {
  return (
    <div
      className="armada-glass"
      style={{ width: "var(--w-dock)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)" }}
    >
      {children}
    </div>
  );
}

/** The composer under Helm's thread, drawn at the dock's own width, on the dock's own glass. */
const meta: Meta<typeof HelmComposer> = {
  title: "Compositions/Helm composer",
  component: HelmComposer,
  args: { onSend: fn(), location: "Job Board" },
  render: (args) => (
    <InTheDock>
      <HelmComposer {...args} />
    </InTheDock>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmComposer>;

/**
 * A single repository, and Helm pointed at it: no switch to draw, nothing to
 * switch to. **The switch offered here would be a switch to where Helm already
 * is**, so the line names the repository instead and the handler goes unused.
 */
export const AtRest: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onSwitch: fn(),
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /^Point Helm at a/ })).not.toBeInTheDocument();
    await expect(canvas.getByText(repositories[0]!.label)).toBeInTheDocument();
  },
};

/**
 * How a bad answer is carried to somebody who could fix it — `#1367`, as one
 * control since 18 Sep 2026. **The banner form, split**: *Copy debug info*
 * acts on one press from the face, and *Details* opens the same artifact to
 * read from behind the caret. Each act still calls its own handler and neither
 * calls the other's.
 */
export const RecordCopiedAndRead: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Copy debug info" }));
    await expect(args.onCopyRecord).toHaveBeenCalled();
    await expect(args.onOpenRecord).not.toHaveBeenCalled();

    // The reading is behind the caret, which is the whole of what the caret is
    // for: nothing else in this row discloses anything.
    await userEvent.click(canvas.getByRole("button", { name: MORE }));
    await userEvent.click(canvas.getByRole("menuitem", { name: "Details" }));
    await expect(args.onOpenRecord).toHaveBeenCalled();
    await expect(args.onCopyRecord).toHaveBeenCalledTimes(1);
  },
};

/**
 * One handler, and there is nothing to split. **A caret over an empty menu is
 * a control that does not answer**, so the act is drawn as the plain button it
 * already was — whichever of the two the dock could offer.
 */
export const OnlyOneActToReportWith: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onCopyRecord: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: MORE })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Copy debug info" }));
    await expect(args.onCopyRecord).toHaveBeenCalled();
  },
};

/**
 * The same, with the reading as the only act: the plain button carries
 * *Details* rather than a face with a dead caret beside it.
 */
export const OnlyTheReadingToReportWith: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onOpenRecord: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: MORE })).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Copy debug info" })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Details" }));
    await expect(args.onOpenRecord).toHaveBeenCalled();
  },
};

/**
 * **The row holds one line at the dock's own width** — the owner's note of
 * 18 Sep 2026, where three controls beside the repository wrapped it onto a
 * second line 72px tall. Every control this row can hold is present: the
 * switch, and the record's split button with both acts.
 *
 * A rendering cannot state this, and neither can a role: the fact is that
 * every control shares one line and none of them is pushed past the row's
 * trailing edge.
 */
export const HeadHoldsOneLineAtDockWidth: Story = {
  args: {
    current: repositories[0]!.id,
    repositories,
    onSwitch: fn(),
    onCopyRecord: fn(),
    onOpenRecord: fn(),
  },
  play: async ({ canvas, canvasElement }) => {
    async function onOneLine(): Promise<void> {
      const head = canvasElement.querySelector(".armada-helm-composer__head");
      if (!(head instanceof HTMLElement)) throw new Error("the composer drew no head row");
      const row = head.getBoundingClientRect();
      const controls = [
        canvas.getByRole("button", { name: REPOINT }),
        canvas.getByRole("button", { name: "Copy debug info" }),
        canvas.getByRole("button", { name: MORE }),
      ].map((one) => one.getBoundingClientRect());

      // One line: the row is no taller than the tallest control standing in it.
      const tallest = Math.max(...controls.map((one) => one.height));
      await expect(row.height).toBeLessThanOrEqual(tallest);
      // And every control is on it, within its edges — a row that does not wrap
      // is only an improvement if nothing has been pushed out of sight instead.
      for (const control of controls) {
        await expect(control.top).toBeGreaterThanOrEqual(row.top - 1);
        await expect(control.bottom).toBeLessThanOrEqual(row.bottom + 1);
        await expect(control.left).toBeGreaterThanOrEqual(row.left - 1);
        await expect(control.right).toBeLessThanOrEqual(row.right + 1);
      }
    }

    await onOneLine();

    // And again at the floor of the dock's drag range, which is where the
    // owner's own dock was standing when he wrote the note: 286px of head is
    // --w-dock-min, not --w-dock. A row that only holds at rest holds nowhere
    // a person actually drags to.
    const dock = canvasElement.querySelector(".armada-glass");
    if (!(dock instanceof HTMLElement)) throw new Error("the story drew no dock around the composer");
    dock.style.width = "var(--w-dock-min)";
    await onOneLine();
  },
};

/**
 * Pointed at nothing with exactly one repository set up — the moment that
 * once gave nobody anything to pick with: the count that governs the control
 * said one repository is nothing to switch between, which is true only while
 * Helm is pointed at it.
 *
 * **The switch is the way to point it**, so it is drawn here, standing at its
 * own entry with that one repository under it to choose.
 */
export const UnpointedWithOneRepository: Story = {
  args: { repositories: [repositories[0]!], onSwitch: fn(), disabled: true },
  play: async ({ args, canvas, userEvent }) => {
    const switcher = canvas.getByRole("button", { name: POINT });
    // Its own entry, not the one repository: Helm is not pointed at that yet,
    // and a switch displaying it would say it was.
    await expect(switcher).toHaveTextContent(UNPOINTED);
    await expect(switcher).not.toHaveTextContent(repositories[0]!.label);
    // Said once, as with two or more: the entry says it and says what to do.
    await expect(canvas.queryByText("No repository to ask yet")).not.toBeInTheDocument();

    // And the one repository is pickable — which is the whole of why the
    // control is drawn.
    await choose(switcher, repositories[0]!.label, userEvent);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[0]!.id);
  },
};

/** On All repositories, the dock's own switch — the rail's pick never moves for it. */
export const SwitchOnAll: Story = {
  args: { current: repositories[0]!.id, repositories, onSwitch: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const switcher = canvas.getByRole("button", { name: REPOINT });
    // Pointed at one, the switch stands on it and carries nothing else — the
    // unpointed entry is not a row a pointed switch offers.
    await expect(switcher).toHaveTextContent(repositories[0]!.label);
    await userEvent.click(switcher);
    await expect(screen.queryByRole("menuitem", { name: UNPOINTED })).not.toBeInTheDocument();
    // The switch searches: focus lands in the filter field, and typing leaves only what matches.
    await expect(await screen.findByRole("searchbox", { name: "Find a repository" })).toHaveFocus();
    await userEvent.keyboard("shop");
    await expect(screen.queryByRole("menuitem", { name: repositories[0]!.label })).not.toBeInTheDocument();
    await userEvent.click(await screen.findByRole("menuitem", { name: repositories[1]!.label }));
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[1]!.id);
  },
};

/**
 * Pointed at nothing, with repositories to point at: the switch stands at its
 * own entry, not at whichever repository happens to be listed first.
 *
 * **Both halves of this are the defect.** A `<select>` whose `value` matches no
 * `<option>` falls back to displaying the first one, so the switch read
 * *armada* while the chip beside it read *No repository to ask yet* — and
 * because *armada* was already the displayed value, picking it fired no
 * `change` at all, which left the first repository in the list unchoosable. A
 * play that only picks the second repository passes either way.
 */
export const UnpointedStandsAtItsOwnEntry: Story = {
  args: { repositories, onSwitch: fn(), disabled: true },
  play: async ({ args, canvas, userEvent }) => {
    const switcher = canvas.getByRole("button", { name: POINT });
    await expect(switcher).toHaveTextContent(UNPOINTED);
    await expect(switcher).not.toHaveTextContent(repositories[0]!.label);
    // Said once: the entry says it and says what to do, so the line does not
    // repeat it — at the dock's width the two cut each other down to "No repo…".
    await expect(canvas.queryByText("No repository to ask yet")).not.toBeInTheDocument();

    // The first repository in the list, which is the one a native select could not choose.
    await choose(switcher, repositories[0]!.label, userEvent);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[0]!.id);
  },
};

/**
 * Nothing is servable yet — no repository has a Manifest for Helm to answer
 * for. **The handler is given and the switch is still not drawn**: pointing
 * Helm at nothing is a reason to offer a repository, not to offer none.
 */
export const NothingToAskYet: Story = {
  args: { repositories: [], onSwitch: fn(), disabled: true },
  play: async ({ args, canvas }) => {
    // No switch is drawn with nothing to point at, so the line is the only
    // place the state can be said, and it says it.
    await expect(canvas.queryByRole("button", { name: /^Point Helm at a/ })).not.toBeInTheDocument();
    await expect(canvas.getByText("No repository to ask yet")).toBeInTheDocument();
    await expect(canvas.getByRole("textbox")).toHaveAttribute("aria-disabled", "true");
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.click(send, { pointerEventsCheck: 0 });
    await expect(args.onSend).not.toHaveBeenCalled();
    // A disabled field takes no focus, so the press lands on the document —
    // and nothing else in the tree answers ⌘Enter.
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
    await expect(args.onSend).not.toHaveBeenCalled();
  },
};

/** A Job's detail is open: the chip names it, above the message box. #1075. */
export const ChipOnAJob: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "12", title: "Fix the poke loop" },
    onRemoveChip: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Remove Job 12 · Fix the poke loop" })).toBeInTheDocument();
  },
};

/** The chip's own `×` — dropped from the next ask's context, without closing the Job. #1075. */
export const ChipRemovedWithX: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "12", title: "Fix the poke loop" },
    onRemoveChip: fn(),
  },
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Remove Job 12 · Fix the poke loop" }));
    await expect(args.onRemoveChip).toHaveBeenCalled();
  },
};

/** Off a Job, nothing is chipped — the composer draws no differently than any other ask. #1075. */
export const NoChipOffAJob: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByText(/^Job \d/)).not.toBeInTheDocument();
  },
};

/** The footer names the screen and the cursor row — no lead-in. #1094. */
export const FooterNamesTheCursorRow: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    location: "Overview · cursor on Job 16",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Overview · cursor on Job 16")).toBeInTheDocument();
    await expect(canvas.queryByText(/Helm reads/i)).not.toBeInTheDocument();
  },
};

/** A Job's detail, chipped: the footer agrees with the chip above it. #1094. */
export const FooterNamesAJobsDetail: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    chip: { jobHandle: "16", title: "Preserve job metadata during resource cleanup" },
    onRemoveChip: fn(),
    location: "Job 16's detail (in context)",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Job 16's detail (in context)")).toBeInTheDocument();
  },
};

/** The box as a window builds it: what `/`, `@` and `#` offer, and the model and effort a conversation holds. */
const FULL: Partial<HelmBox> = {
  model: null,
  effort: null,
  models: ["haiku", "sonnet", "opus"],
  efforts: ["low", "medium", "high"],
  onTune: fn(),
  // Fleet names the commands and has no sentence for each, so none is drawn.
  commands: [
    { name: "review", says: "" },
    { name: "simplify", says: "" },
  ],
  taggable: [
    { kind: "session", id: "s2", title: "Release notes script" },
    { kind: "pull_request", id: "1843", title: "#1843 Pin the store clock" },
  ],
  numbered: {
    open: [
      { tag: { kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" } },
      { tag: { kind: "pull_request", id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" } },
    ],
    find: async (number) =>
      number === 2207
        ? { tag: { kind: "pull_request", id: "2207", title: "#2207 Stop the Drone count drifting", url: "https://forge.invalid/pull/2207" }, standing: "Merged" }
        : undefined,
  },
  drawn: [],
  onDraw: fn(),
  onRemoveDrawn: fn(),
};

const BOXED: Story["args"] = { current: repositories[0]!.id, repositories: [repositories[0]!], box: FULL };

/** The dock's box, which sends on Enter and breaks the line on Shift+Enter, as a Session's does. */
export const EnterSends: Story = {
  args: BOXED,
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Ask Helm" });
    await userEvent.type(field, "Why did job 12 stall?{Shift>}{Enter}{/Shift}It ran an hour ago");
    await expect(args.onSend).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({ text: "Why did job 12 stall?\nIt ran an hour ago", files: [], tags: [] });
    await expect(args.onSend).toHaveBeenCalledTimes(1);
    await expect(field).toHaveTextContent("");
  },
};

/** Blank never sends: Send stays off until there is something to say, and Enter on a blank box does nothing. */
export const BlankDoesNotSend: Story = {
  args: BOXED,
  play: async ({ args, canvas, userEvent }) => {
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox", { name: "Ask Helm" }), "   {Enter}");
    await expect(send).toBeDisabled();
    await expect(args.onSend).not.toHaveBeenCalled();
    await userEvent.type(canvas.getByRole("textbox", { name: "Ask Helm" }), "Why did job 12 stall?");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};

/**
 * Send is a glyph named for what it does, inside the box's own row, since the dock has no width for the word.
 * It stays inside the box's edges however much is typed.
 */
export const SendIsAGlyphInsideTheBox: Story = {
  args: BOXED,
  play: async ({ canvas, userEvent }) => {
    const send = () => canvas.getByRole("button", { name: "Send" });
    await expect(send()).toHaveTextContent("");
    await userEvent.type(canvas.getByRole("textbox", { name: "Ask Helm" }), "one{Shift>}{Enter}{/Shift}two{Shift>}{Enter}{/Shift}three");
    const form = send().closest("form")!.getBoundingClientRect();
    const at = send().getBoundingClientRect();
    await expect(at.left).toBeGreaterThanOrEqual(form.left);
    await expect(at.right).toBeLessThanOrEqual(form.right);
    await expect(at.top).toBeGreaterThanOrEqual(form.top);
    await expect(at.bottom).toBeLessThanOrEqual(form.bottom);
  },
};

function GrowingDock(args: Story["args"]): ReactElement {
  return (
    <div
      className="armada-glass"
      style={{ width: "var(--w-dock)", height: "calc(var(--space-12) * 8)", display: "flex", flexDirection: "column", justifyContent: "flex-end", boxSizing: "border-box", padding: "var(--space-4)", overflowY: "auto" }}
    >
      <HelmComposer {...args} onSend={fn()} />
    </div>
  );
}

/** The ask box grows with what is typed, stops at half the height of the dock it sits in, and scrolls past that. */
export const GrowsToHalf: Story = {
  args: BOXED,
  render: (args) => <GrowingDock {...args} />,
  play: async ({ canvas, canvasElement, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Ask Helm" });
    const dock = canvasElement.querySelector(".armada-glass")!;
    const rest = field.getBoundingClientRect().height;
    await userEvent.type(field, "one{Shift>}{Enter}{/Shift}two{Shift>}{Enter}{/Shift}three{Shift>}{Enter}{/Shift}four");
    const grown = field.getBoundingClientRect().height;
    await expect(grown).toBeGreaterThan(rest);
    await expect(grown).toBeLessThan(dock.clientHeight / 2);
    await userEvent.type(field, "{Shift>}{Enter}{/Shift}m".repeat(14));
    await expect(field.getBoundingClientRect().height).toBeLessThanOrEqual(dock.clientHeight / 2 + 1);
    await expect(field.scrollHeight).toBeGreaterThan(field.clientHeight);
  },
};

/** `/` lists the commands the host names, by name, and choosing one writes it in. No permission mode is offered beside the model and effort. */
export const SlashListsTheNames: Story = {
  args: BOXED,
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Ask Helm" });
    await userEvent.type(field, "/sim");
    await userEvent.click(await canvas.findByRole("option", { name: /\/simplify/ }));
    await expect(field).toHaveTextContent("/simplify");
    await expect(canvas.queryByRole("combobox", { name: "Permission mode" })).not.toBeInTheDocument();
  },
};

/** The model and effort are the conversation's, each with Auto, and a pick is handed up whole. */
export const ModelAndEffortAreTuned: Story = {
  args: { ...BOXED, box: { ...FULL, model: "sonnet" } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("combobox", { name: "Model" })).toHaveValue("sonnet");
    await userEvent.selectOptions(canvas.getByRole("combobox", { name: "Effort" }), "high");
    await expect(args.box!.onTune).toHaveBeenCalledWith({ model: "sonnet", effort: "high" });
  },
};

/**
 * `#` offers the repository's open issues and pull requests by number, and a chosen one is sent with its kind and
 * the forge's address, which the line in the box does not show.
 */
export const HashTagsAnIssueWithItsAddress: Story = {
  args: BOXED,
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Ask Helm" });
    await userEvent.type(field, "see #22");
    const list = await canvas.findByRole("listbox", { name: "Tag an issue or pull request" });
    await expect(within(list).getAllByRole("option")).toHaveLength(2);
    await userEvent.click(within(list).getByRole("option", { name: /Sessions list drops a quiet Session/ }));
    await userEvent.type(field, "is it this?{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith({
      text: "see @#2214 Sessions list drops a quiet Session is it this?",
      files: [],
      tags: [{ kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" }],
    });
  },
};

/** A number that is not open is looked up once it rests, offered with where it stands, and still tags. */
export const HashLooksANumberUp: Story = {
  args: BOXED,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Ask Helm" }), "#2207");
    const merged = await canvas.findByRole("option", { name: /Stop the Drone count drifting/ });
    await expect(merged).toHaveTextContent("Merged");
    await userEvent.click(merged);
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).toHaveBeenCalledWith(expect.objectContaining({ tags: [expect.objectContaining({ kind: "pull_request", id: "2207", url: "https://forge.invalid/pull/2207" })] }));
  },
};

/** A picture alone is a message: with no words and a file waiting, Send is on and the words go blank. */
export const AFileAloneSends: Story = {
  args: BOXED,
  play: async ({ args, canvas, userEvent }) => {
    const clipboard = new DataTransfer();
    clipboard.items.add(new File(["png"], "Screenshot.png", { type: "image/png" }));
    canvas.getByRole("textbox", { name: "Ask Helm" }).dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
    // In the narrow box what waits moves to a row of its own above the bar, so it is found by its name.
    await expect(await canvas.findByText("Screenshot.png")).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
    await userEvent.click(canvas.getByRole("button", { name: "Send" }));
    await expect(args.onSend).toHaveBeenCalledWith({ text: "", files: [expect.objectContaining({ name: "Screenshot.png" })], tags: [] });
  },
};

/**
 * A long list leads with the few picked last. **The recent row and the row in the full list are
 * one repository**, so the full list still has every repository and picking either calls
 * `onSwitch` with the plain id, never the recent row's prefixed one.
 */
export const RecentLeads: Story = {
  args: {
    current: "01M2ARMADA",
    repositories: [
      ...repositories,
      { id: "01M2BILLING", label: "billing-api" },
      { id: "01M2CHECKOUT", label: "checkout" },
    ],
    recent: ["01M2CHECKOUT", "01M2SHOP"],
    onSwitch: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: REPOINT }));
    await expect(await screen.findByText("Recent")).toBeVisible();

    // Newest first, then the full list with its own copy of each.
    const rows = (await screen.findAllByRole("menuitem")).map((one) => one.textContent);
    await expect(rows).toEqual(["checkout", "shop-01", "armada", "shop-01", "billing-api", "checkout"]);

    // A recent row picks the repository by its own id.
    await userEvent.click((await screen.findAllByRole("menuitem", { name: "checkout" }))[0]!);
    await expect(args.onSwitch).toHaveBeenCalledWith("01M2CHECKOUT");
  },
};

/** No recent list given: no heading, and the rows are just the repositories. */
export const NoRecentNoHeading: Story = {
  args: { current: "01M2ARMADA", repositories, onSwitch: fn() },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: REPOINT }));
    await expect(await screen.findAllByRole("menuitem")).toHaveLength(repositories.length);
    await expect(screen.queryByText("Recent")).not.toBeInTheDocument();
  },
};
