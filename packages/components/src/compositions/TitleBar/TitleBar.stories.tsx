import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import { FLEET_DOT_TONE, type FleetState } from "../FleetPanel/FleetPanel";
import { Select } from "../../primitives/Select/Select";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { TitleBar } from "./TitleBar";

/**
 * The title row #1087 put in place of macOS's grey bar. Every story below
 * sits in the same framed strip `TheShell`'s own stories use, at the
 * drawing's own screen width.
 */
const meta: Meta<typeof TitleBar> = {
  title: "Compositions/TitleBar",
  component: TitleBar,
  decorators: [
    // A column, matching `.armada-shell`'s own axis — that is what stretches
    // the bar to the frame's width there, and a row (`.armada-screen__window`)
    // would only shrink-wrap it to its content, which is a story artefact
    // nothing in Bridge ever draws.
    (Story) => (
      <div className="armada-screen" style={{ display: "flex", flexDirection: "column" }}>
        <div className="armada-screen__window" style={{ height: "auto", flexDirection: "column" }}>
          <Story />
        </div>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof TitleBar>;

const picker = (
  <Select aria-label="Project">
    <option>All repositories</option>
  </Select>
);

/**
 * Every slot filled: the picker, the search field, Dispatch, the logo and
 * Fleet's dot — Helm's dock is open, so its button is absent. The dot is not a
 * slot that can be left out: it is drawn at every width, so every story below
 * carries it.
 */
export const Full: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    onDispatch: fn(),
    fleet: { state: "running", label: "Running" },
  },
  /**
   * #1156: Dispatch is one button with no caret, and clicking it dispatches.
   * `SplitButton` drew a second, separately-named control for the menu even
   * with nothing behind it — `queryByRole` for that name is the regression
   * this guards.
   */
  play: async ({ args, canvas, userEvent }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(canvas.queryByRole("button", { name: "Dispatch a job" })).not.toBeInTheDocument();

    await userEvent.click(dispatch);
    await expect(args.onDispatch).toHaveBeenCalledTimes(1);
  },
};

/** The dock is closed: Helm's own reopen control appears before the logo, carrying what is waiting. */
export const HelmClosed: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
  },
};

/**
 * Nothing connected yet: no picker, no Dispatch — the bar still draws, with
 * the logo and nowhere else to go but search. Fleet is still read, because
 * "not read yet" is one of the dot's own four states.
 */
export const BeforeAnythingIsRead: Story = {
  args: {
    onSearch: () => {},
    fleet: { state: "unknown", label: "Reading" },
  },
};

/**
 * Fleet's dot on the bar's trailing edge — #1438, drawn at every width since
 * 18 Sep 2026 rather than only while the left column was away. It says
 * liveness and no more: pid, port, protocol and uptime are the panel's.
 */
export const FleetRunning: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
  },
};

/** Fleet is not running — `--status-escalated`, the panel's own hue for it, and the word in the name. */
export const FleetNotRunning: Story = {
  args: { ...FleetRunning.args, fleet: { state: "not-running", label: "Not running" } },
};

/** Alive and not answering — `--status-awaiting-review`, a state a dot alone could not tell from running. */
export const FleetUnreachable: Story = {
  args: { ...FleetRunning.args, fleet: { state: "unreachable", label: "Unreachable" } },
};

/** None of the three — reading, a refused runtime file — neutral, and the label names which. */
export const FleetUnknown: Story = {
  args: { ...FleetRunning.args, fleet: { state: "unknown", label: "Reading" } },
};

/** A live pid that has not answered yet: the `server` glyph breathing in the dot's place, `Starting` on hover. */
export const FleetStarting: Story = {
  args: { ...FleetRunning.args, fleet: { state: "starting", label: "Starting" } },
};

/**
 * What a rendering cannot show: that each state's dot carries the Fleet
 * panel's own tone rather than one chosen here, and that the state is
 * readable without sight. The tone is asserted against `FLEET_DOT_TONE`
 * itself — the map the panel draws from — so a hue that moved in one place
 * and not the other fails here rather than shipping two dots that disagree.
 *
 * The name and the tooltip are read as two properties because they are two
 * doors: a pointer gets `title`, a screen reader gets the accessible name,
 * and a dot that had only one of them would be silent for half its readers.
 */
export const FleetStates: Story = {
  args: FleetRunning.args,
  render: (args) => (
    <>
      {FLEET_SAID.map(({ state, label }) => (
        <TitleBar key={state} {...args} fleet={{ state, label }} />
      ))}
    </>
  ),
  play: async ({ canvas }) => {
    for (const { state, label } of FLEET_SAID) {
      const said = `Fleet — ${label}`;
      const dot = canvas.getByRole("img", { name: said });
      await expect(dot).toHaveAttribute("title", said);
      await expect(dot.firstElementChild).toHaveAttribute("data-tone", FLEET_DOT_TONE[state]);
    }
  },
};

/** Every Fleet state, with the word the panel puts beside its own dot for each. */
const FLEET_SAID: { state: FleetState; label: string }[] = [
  { state: "running", label: "Running" },
  { state: "not-running", label: "Not running" },
  { state: "unreachable", label: "Unreachable" },
  { state: "unknown", label: "Reading" },
];

/**
 * The dot is unconditional. #1438 mounted it on a prop the shell set from the
 * window's width, so the row a person saw at 1512px had no dot at all and the
 * row at 1150px did; the owner rejected that and rejected removing the dot,
 * and chose it drawn always on 18 Sep 2026.
 *
 * The two arrangements that used to decide it: the dock open beside the
 * content, which is `Full`, and the dock closed with Helm's own button in the
 * row. **No `play`, because there is nothing here for one to catch** — the
 * condition was never in this component but in what `TheShell` passed it, so
 * the assertion lives in `The shell`'s own stories and in
 * `job-detail-width.test.tsx`, which drives a real window across the widths.
 */
export const FleetIsDrawnWhateverElseTheRowCarries: Story = {
  args: Full.args,
  render: (args) => (
    <>
      <TitleBar {...args} />
      <TitleBar {...args} helm={{ questions: 3, binding: "⌘J", onOpen: () => {} }} />
    </>
  ),
};

/**
 * Holding Cmd mounts Helm's own `⌘J` badge, growing the button to fit it —
 * shrinking it back and taking the badge away again on release — the row is
 * drawn from `Full` and never itself. `useShortcutReveal()` needs a provider
 * above it for the state to reach anywhere at all, which `TheShell` is in the
 * running app and this story stands in for here.
 */
export const RevealedShortcuts: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
  },
  render: (args) => (
    <ShortcutRevealProvider>
      <TitleBar {...args} />
    </ShortcutRevealProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    const findBadge = () =>
      canvas.queryByText((_, el) => el?.tagName === "KBD" && el.textContent === "⌘J");
    await expect(findBadge()).not.toBeInTheDocument();

    await userEvent.keyboard("{Meta>}");
    await expect(findBadge()).toBeVisible();

    await userEvent.keyboard("{/Meta}");
    await expect(findBadge()).not.toBeInTheDocument();
  },
};

/**
 * A slim window, with every slot filled. Under about 924px the row goes behind
 * one menu, leaving the mark and Fleet's dot beside it (the owner, 30 Sep
 * 2026). The side columns once had a zero minimum, so below ~1100px the picker
 * slid under search and Dispatch sat on Helm.
 */
export const InASlimWindow: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: fn() },
    repositoryMenu: {
      entries: [
        { kind: "item", id: "all", label: "All repositories", selected: true },
        { kind: "separator", id: "rule" },
        { kind: "item", id: "/code/armada", label: "armada" },
      ],
      onSelect: fn(),
    },
  },
  render: (args) => (
    <div style={{ width: "calc(var(--window-floor) - var(--space-12) * 5)" }}>
      <TitleBar {...args} />
    </div>
  ),
  play: async ({ args, canvas, canvasElement, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Dispatch", hidden: true })).not.toBeVisible();
    await expect(canvasElement.querySelector(".armada-title-bar__search")).not.toBeVisible();

    const bar = canvasElement.querySelector(".armada-title-bar")!.getBoundingClientRect();
    const menu = canvas.getByRole("button", { name: "Menu" });
    await expect(menu.getBoundingClientRect().right).toBeLessThanOrEqual(bar.right);
    await expect(menu.getBoundingClientRect().right).toBeGreaterThan(bar.right - bar.width / 4);

    await userEvent.click(menu);
    const panel = canvas.getByRole("dialog", { name: "Menu" });
    await expect(panel).toBeVisible();
    // The repositories are rows in the panel, not a second list opening in it.
    await expect(within(panel).queryByRole("button", { expanded: false })).toBeNull();
    await expect(within(panel).getByRole("button", { name: "All repositories", pressed: true })).toBeVisible();
    const dispatch = within(panel).getByRole("button", { name: "Dispatch" });
    await expect(dispatch.querySelector("kbd")).toHaveTextContent(/^n$/);
    await userEvent.click(canvas.getByRole("button", { name: /^Helm/ }));
    await expect(args.helm!.onOpen).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("dialog", { name: "Menu" })).not.toBeInTheDocument();

    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await userEvent.click(canvas.getByRole("button", { name: "armada" }));
    await expect(args.repositoryMenu!.onSelect).toHaveBeenCalledWith("/code/armada");
    await expect(canvas.queryByRole("dialog", { name: "Menu" })).not.toBeInTheDocument();
  },
};

/** Wide, the row is all there and the menu is not. */
export const WideHasNoMenu: Story = {
  args: InASlimWindow.args,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Dispatch" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Menu", hidden: true })).not.toBeVisible();
  },
};
