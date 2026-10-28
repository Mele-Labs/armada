import type { Meta, StoryObj } from "@storybook/react-vite";
import { FileCog } from "lucide-react";
import { expect, fn, screen, waitFor, within } from "storybook/test";
import { FLEET_DOT_TONE, type FleetState } from "../FleetPanel/FleetPanel";
import { Select } from "../../primitives/Select/Select";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { AccountProvider, type Account } from "../Account/Account";
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
 * Every slot filled: the picker, the one bar, the logo and
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
  /** One bar searches and dispatches, so the full row draws no Dispatch button (the owner, 10 Oct 2026). */
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /^Search, or describe work to dispatch/ })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Dispatch" })).not.toBeInTheDocument();
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

/** A live pid that has not answered yet: the Armada mark breathing in the dot's place, `Starting` on hover. */
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

    // The filter field is here at this width too, and typing narrows the rows to what matches.
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    const reopened = canvas.getByRole("dialog", { name: "Menu" });
    await userEvent.type(within(reopened).getByRole("searchbox", { name: "Find a repository" }), "arm");
    await expect(within(reopened).queryByRole("button", { name: "All repositories" })).toBeNull();
    await userEvent.type(within(reopened).getByRole("searchbox"), "zzz");
    await expect(within(reopened).getByRole("status")).toHaveTextContent("Nothing matches “armzzz”.");
    await userEvent.clear(within(reopened).getByRole("searchbox"));
    await userEvent.type(within(reopened).getByRole("searchbox"), "arm");
    await userEvent.click(within(reopened).getByRole("button", { name: "armada" }));
    await expect(args.repositoryMenu!.onSelect).toHaveBeenCalledWith("/code/armada");
    await expect(canvas.queryByRole("dialog", { name: "Menu" })).not.toBeInTheDocument();
  },
};

/** The menu's own entries, Sleep, Settings with its key and Doctor. Pressing Settings or Doctor opens it and puts the panel away. */
export const MenuEntries: Story = {
  args: {
    ...Full.args,
    sleep: { on: false, onToggle: fn() },
    menu: { settings: { onOpen: fn(), shortcut: "⌘," }, doctor: { onOpen: fn() } },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    const panel = canvas.getByRole("dialog", { name: "Menu" });
    await expect(within(panel).getByRole("button", { name: "Sleep mode" })).toBeVisible();
    const settings = within(panel).getByRole("button", { name: "Settings" });
    await expect(settings.querySelector("kbd")).toBeInTheDocument();
    await userEvent.click(settings);
    await expect(args.menu?.settings?.onOpen).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("dialog", { name: "Menu" })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await userEvent.click(within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("button", { name: "Doctor" }));
    await expect(args.menu?.doctor?.onOpen).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("dialog", { name: "Menu" })).not.toBeInTheDocument();
  },
};

/**
 * Sleep mode, an entry in the menu: the entry is pressed while it is on, and the moon rides on the menu button.
 */
export const SleepModeToggle: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    onDispatch: fn(),
    fleet: { state: "running", label: "Running" },
    sleep: { on: false, onToggle: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu");
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    const sleep = within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("button", { name: "Sleep mode" });
    await expect(sleep).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(sleep);
    await expect(args.sleep?.onToggle).toHaveBeenCalledTimes(1);
  },
};

/** Sleep is on: the menu button's tooltip says so, and its entry is pressed. */
export const SleepModeOn: Story = {
  args: { ...SleepModeToggle.args, sleep: { on: true, onToggle: fn() } },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu, sleep mode on");
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await expect(within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("button", { name: "Sleep mode" })).toHaveAttribute("aria-pressed", "true");
  },
};

const noAct = () => undefined;
/** An account a window is signed in to, or out of with no profiles; its acts do nothing. */
const account = (logins: readonly string[], signIn: () => void = noAct): Account => ({
  profiles: logins.map((login, at) => ({ account_id: 1001 + at, login, active: at === 0, chosen: false })),
  flow: { phase: "idle" },
  syncs: null,
  signIn,
  openDevicePage: noAct,
  addProfile: noAct,
  dismiss: noAct,
  switchTo: noAct,
  choose: noAct,
  closeSyncs: noAct,
  setKind: noAct,
  setScope: noAct,
  signOut: noAct,
  openSetting: noAct,
});

const signIn = fn();

const openAccount = fn();

/** Nothing signed in: the button's tooltip says so, and the first entry is Sign in, which opens the account where the window keeps it. */
export const SignedOut: Story = {
  args: { ...SleepModeToggle.args, menu: { account: { onOpen: openAccount } } },
  render: (args) => (
    <AccountProvider value={account([], signIn)}>
      <TitleBar {...args} />
    </AccountProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu, signed out");
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await userEvent.click(within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("button", { name: "Sign in" }));
    await expect(openAccount).toHaveBeenCalledTimes(1);
    await expect(signIn).not.toHaveBeenCalled();
  },
};

/** Signed in, the entry reads Account with no login in it, and the button says nothing more than Menu. */
export const SignedIn: Story = {
  args: { ...SleepModeToggle.args, menu: { account: { onOpen: openAccount } } },
  render: (args) => (
    <AccountProvider value={account(["nickmele"])}>
      <TitleBar {...args} />
    </AccountProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu");
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await expect(within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("button", { name: "Account" })).toBeVisible();
    await expect(canvas.queryByText("nickmele")).toBeNull();
  },
};

/** A sign-in waiting on the person: the button's title and the account row carry the turn, so the way back is a mark. */
export const SignInWaiting: Story = {
  args: { ...SleepModeToggle.args, menu: { account: { onOpen: openAccount } } },
  render: (args) => (
    <AccountProvider value={{ ...account([], signIn), flow: { phase: "code", code: "WDJB-MJHT" } }}>
      <TitleBar {...args} />
    </AccountProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu, signed out, waiting for GitHub");
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await expect(within(canvas.getByRole("dialog", { name: "Menu" })).getByRole("img", { name: "Waiting for GitHub" })).toBeVisible();
  },
};

/** Signed out with Sleep on: both marks ride on the button, and the tooltip names both. */
export const SignedOutAsleep: Story = {
  args: { ...SleepModeToggle.args, sleep: { on: true, onToggle: fn() }, menu: { account: { onOpen: openAccount } } },
  render: SignedOut.render!,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Menu" })).toHaveAttribute("title", "Menu, sleep mode on, signed out");
  },
};

const inTwoHours = () => new Date(Date.now() + (2 * 60 + 14) * 60_000 + 30_000).toISOString();

/**
 * The plan's limits as one ring beside Sleep, the 5-hour limit outside and the week inside. Hover or focus opens
 * one card of both, a press keeps it open, and Esc closes it. The ring is named for both and how full each is.
 */
export const PlanUsage: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
    planUsage: [
      { kind: "five_hour", percentUsed: 34, resetsAt: inTwoHours() },
      { kind: "seven_day", percentUsed: 61 },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const ring = canvas.getByRole("button", { name: "5-hour limit 34%, Weekly limit 61%" });
    await expect(canvas.queryByRole("img", { name: "Analyzing Usage" })).not.toBeInTheDocument();
    await expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    await userEvent.hover(ring);
    const card = await screen.findByRole("tooltip");
    const five = within(card).getByRole("region", { name: "5-hour limit" });
    await expect(five).toHaveTextContent("34%");
    await expect(five).toHaveTextContent("Resets in");
    await expect(five).toHaveTextContent("2h 14m");
    await expect(within(card).getByRole("region", { name: "Weekly limit" })).toHaveTextContent("61%");

    await userEvent.click(ring);
    await userEvent.unhover(ring);
    await expect(screen.getByRole("dialog", { name: "Plan limits" })).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  },
};

/** Warning from 80% and danger from 95%, by the figure as it is drawn. */
export const PlanUsageLevels: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
    planUsage: [
      { kind: "five_hour", percentUsed: 86, resetsAt: inTwoHours() },
      { kind: "seven_day", percentUsed: 97 },
    ],
  },
};

/** A limit with an offer standing: the bell on the ring, a fact under the reset, and the owner's two acts in the card. */
export const PlanUsageOffer: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
    planUsage: [
      { kind: "five_hour", percentUsed: 34, resetsAt: inTwoHours() },
      {
        kind: "seven_day",
        percentUsed: 83,
        resetsAt: inTwoHours(),
        offer: {
          attention: "Default to sonnet offered",
          facts: [["Default model", "opus"]],
          acts: [
            { label: "Default to sonnet", onPress: () => {}, primary: true, stays: true },
            { label: "Choose…", onPress: () => {} },
          ],
        },
      },
    ],
  },
};

/** Fleet has answered with no limit the row knows, or only a window it does not draw: the rings loading, and nothing to open. */
export const PlanUsageAbsent: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
    planUsage: [{ kind: "spend_limit", percentUsed: 12 }],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: /limit/ })).not.toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Analyzing Usage" })).toBeVisible();
  },
};

/** Fleet answered, and no Session has reported a limit yet: the rings loading, named for what is happening, and not a control. */
export const PlanUsageLoading: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
    planUsage: [],
  },
  play: async ({ canvas, userEvent }) => {
    const loading = canvas.getByRole("img", { name: "Analyzing Usage" });
    await expect(loading).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Plan limits" })).not.toBeInTheDocument();

    await userEvent.hover(loading);
    await waitFor(() => expect(canvas.getByText("Analyzing Usage")).toBeVisible());
  },
};

/** No `planUsage` at all, as before Bridge has connected to Fleet: neither the rings nor the loading ring is drawn. */
export const PlanUsageUnset: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: "Analyzing Usage" })).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: /limit/ })).not.toBeInTheDocument();
  },
};

const PLACES = [
  { id: "cockpit", label: "Cockpit" },
  { id: "jobs", label: "Jobs" },
  { id: "sessions", label: "Sessions" },
  { id: "system", label: "System" },
];

/**
 * The places slot and the wordmark slot filled, as the Cockpit's own window draws them: the word at the
 * start in place of the lockup, the tabs in the centre, and the search trigger shrunk to an icon and its
 * key to their right. The picker, Helm and the plan's meters are not here: that window carries them elsewhere.
 */
export const WithPlaces: Story = {
  args: {
    wordmark: "Armada",
    places: <Tabs items={PLACES} value="jobs" />,
    onSearch: () => {},
    fleet: { state: "running", label: "Running" },
  },
  play: async ({ canvas }) => {
    const tabs = canvas.getByRole("tablist");
    const search = canvas.getByRole("button", { name: "Search, or describe work to dispatch (⌘K)" });
    await expect(canvas.getByRole("tab", { name: "Jobs" })).toHaveAttribute("aria-selected", "true");
    // The places come first, then the trigger, and the trigger's label is not drawn.
    await expect(tabs.compareDocumentPosition(search) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await expect(search).toHaveAttribute("title", "Search, or describe work to dispatch (⌘K)");
    await expect(canvas.queryByText("Search, or describe work to dispatch")).not.toBeVisible();
    // The word stands for the lockup and the mark.
    await expect(canvas.getByText("Armada")).toBeVisible();
    await expect(canvas.queryByRole("img", { name: "Armada" })).not.toBeInTheDocument();
  },
};

/** The same row under the narrow breakpoint: the places stay, where the picker's start and the lockup go. */
export const WithPlacesInASlimWindow: Story = {
  args: WithPlaces.args!,
  render: (args) => (
    <div style={{ width: "calc(var(--window-floor) - var(--space-12) * 5)" }}>
      <TitleBar {...args} />
    </div>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("tablist")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Menu" })).toBeVisible();
  },
};

/** A surface with no picker of its own: the row names the picked repository, as text, and cuts a long name. */
export const WithRepository: Story = {
  args: { ...WithPlaces.args!, repository: "armada" },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("armada")).toBeVisible();
    await expect(canvas.getByText("armada")).toHaveAttribute("title", "armada");
    await expect(canvas.queryByRole("combobox")).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "armada" })).not.toBeInTheDocument();
  },
};

/** Without the slots the row is what it was: the lockup, no tabs, the trigger with its label. */
export const WithoutPlaces: Story = {
  args: Full.args!,
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("tablist")).not.toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Armada" })).toBeInTheDocument();
    await expect(canvas.getByText("Search, or describe work to dispatch")).toBeVisible();
  },
};

const REPOSITORY_ENTRIES = [
  { kind: "item", id: "", label: "All repositories", selected: true },
  { kind: "separator", id: "all-rule" },
  { kind: "item", id: "/code/armada", label: "armada" },
  { kind: "label", id: "not-set-up", label: "Not set up" },
  { kind: "item", id: "/code/scratch", label: "scratch" },
  { kind: "separator", id: "acts-rule" },
  { kind: "item", id: "new-project", label: "New project" },
  { kind: "item", id: "add-repository", label: "Add a repository" },
] as const;

/**
 * The name is the picker, on every screen, with the Manifest's chip after it. Its list leads with All repositories, then
 * the repositories, a repository that is not set up wears a tag on its own row, and the acts sit below a rule.
 */
export const RepositoryPicker: Story = {
  args: {
    ...Full.args,
    repositoryPicker: undefined,
    repository: "All repositories",
    repositoryMenu: {
      entries: [...REPOSITORY_ENTRIES],
      onSelect: fn(),
      manifest: { label: "Open the Manifest", icon: FileCog, onOpen: fn() },
    },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Repository" }));
    const items = screen.getAllByRole("menuitem").map((one) => one.textContent);
    await expect(items).toEqual(["All repositories", "armada", "scratchNot set up", "New project", "Add a repository"]);
    // The tag is on the row, not a heading of its own.
    await expect(screen.queryByText("Not set up", { selector: ".armada-dropdown-menu__label" })).toBeNull();
    await expect(screen.getByRole("menuitem", { name: /^scratch/ }).querySelector(".armada-dropdown-menu__detail")).toHaveTextContent("Not set up");
    // The acts are marked apart from the repositories and carry a glyph each.
    await expect(screen.getByRole("menuitem", { name: "New project" })).toHaveAttribute("data-act");
    await expect(screen.getByRole("menuitem", { name: "armada" })).not.toHaveAttribute("data-act");
    await expect(screen.getByRole("menuitem", { name: "Add a repository" }).querySelector("svg")).not.toBeNull();
    await userEvent.click(screen.getByRole("menuitem", { name: "armada" }));
    await expect(args.repositoryMenu!.onSelect).toHaveBeenCalledWith("/code/armada");
  },
};

/** The Manifest's chip presses through to `onOpen`, with a tooltip-name, and is disabled where nothing is served. */
export const ManifestChip: Story = {
  args: RepositoryPicker.args!,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open the Manifest" }));
    await expect(args.repositoryMenu!.manifest!.onOpen).toHaveBeenCalledTimes(1);
  },
};

export const ManifestChipNothingServed: Story = {
  args: {
    ...RepositoryPicker.args,
    repositoryMenu: { ...RepositoryPicker.args!.repositoryMenu!, manifest: { label: "Open the Manifest", icon: FileCog, disabled: true, onOpen: fn() } },
  },
  play: async ({ args, canvas }) => {
    const chip = canvas.getByRole("button", { name: "Open the Manifest" });
    await expect(chip).toBeDisabled();
    await expect(args.repositoryMenu!.manifest!.onOpen).not.toHaveBeenCalled();
  },
};

/** A repository's row can carry a mark the app draws; → on the focused row runs its act and puts the list away. */
export const RepositoryRowMark: Story = {
  args: {
    ...RepositoryPicker.args,
    repositoryMenu: {
      ...RepositoryPicker.args!.repositoryMenu!,
      trailOf: (id, label) => (id === "/code/armada" ? { node: <span role="button" aria-label={`Armada configuration for ${label}`} />, open: openConfig } : undefined),
    },
  },
  play: async ({ canvas, userEvent }) => {
    openConfig.mockClear();
    await userEvent.click(canvas.getByRole("button", { name: "Repository" }));
    await expect(screen.getByRole("menuitem", { name: /^armada/ }).querySelector('[aria-label="Armada configuration for armada"]')).not.toBeNull();
    await expect(screen.getByRole("menuitem", { name: /^scratch/ }).querySelector(".armada-dropdown-menu__trail")).toBeNull();
    screen.getByRole("menuitem", { name: /^armada/ }).focus();
    await userEvent.keyboard("{ArrowRight}");
    await expect(openConfig).toHaveBeenCalledTimes(1);
    await expect(screen.queryByRole("menu")).toBeNull();
  },
};
const openConfig = fn();
