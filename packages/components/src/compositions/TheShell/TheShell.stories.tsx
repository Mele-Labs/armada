import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState, type ComponentProps } from "react";
import { expect, within } from "storybook/test";
import { Megaphone } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { ActiveJobsList } from "../ActiveJobsList/ActiveJobsList";
import { DockQuestions } from "../DockQuestions/DockQuestions";
import { JobRowStacked } from "../JobRowStacked/JobRowStacked";
import { TheShell } from "./TheShell";

/**
 * The title row with the window's places, the panel and the dock. There is no left column.
 *
 * The values are the drawing's own: pid 4417, port 7411, six jobs, one of them
 * waiting on you. `today ~$4.80` is left out — nothing measures spend.
 */
const meta: Meta<typeof TheShell> = {
  title: "Compositions/The shell",
  component: TheShell,
};
export default meta;

type Story = StoryObj<typeof TheShell>;

const shell: ComponentProps<typeof TheShell> = {
  onSearch: () => {},
  onDispatch: () => {},
  wordmark: "Armada",
  places: (
    <Tabs
      items={[
        { id: "cockpit", label: "Cockpit" },
        { id: "jobs", label: "Jobs" },
        { id: "sessions", label: "Sessions" },
        { id: "system", label: "System" },
      ]}
      value="cockpit"
    />
  ),
  children: <div className="armada-screen__mount">The list mounts here — 1d</div>,
  fleet: {
    state: "running",
    label: "Running",
    rows: [
      { label: "pid", value: "4417" },
      { label: "port", value: "7411" },
      { label: "protocol", value: "14.5" },
      { label: "up", value: "2h 14m" },
    ],
    doctor: { outcome: "pass", checked: "Fleet, SQLite, Manifest, System stats" },
    open: true,
    onOpenChange: () => {},
  },
};

/** The window the app mounts the shell into, which every story here draws it in. */
const inWindow = (args: ComponentProps<typeof TheShell>) => (
  <div className="armada-screen">
    <div className="armada-screen__window">
      <TheShell {...args} />
    </div>
  </div>
);

export const Shell: Story = { args: shell, render: inWindow };

/**
 * No left column and the content from edge to edge. What a column would have held, Fleet's panel, opens from the title row's dot.
 */
export const FleetInAPopover: Story = {
  name: "The content takes the full width and Fleet opens from the dot",
  args: shell,
  render: inWindow,
  play: async ({ canvas, canvasElement, userEvent }) => {
    await expect(canvas.queryByRole("navigation", { name: "Work" })).not.toBeInTheDocument();
    await expect(canvas.queryByRole("separator", { name: "Resize the left column" })).not.toBeInTheDocument();
    await expect(canvas.getByText("The list mounts here — 1d")).toBeVisible();
    // The content starts where the window does, less its own gutter.
    const window = canvasElement.querySelector(".armada-screen__window")!.getBoundingClientRect();
    const work = canvasElement.querySelector(".armada-shell__work")!.getBoundingClientRect();
    await expect(work.left - window.left).toBeLessThan(window.width / 20);
    // The dot opens Fleet's panel, with its Doctor row.
    await userEvent.click(canvas.getByRole("button", { name: "Fleet — Running" }));
    const panel = canvas.getByRole("dialog", { name: "Fleet" });
    await expect(within(panel).getByText("Doctor")).toBeVisible();
    // Below the dot, and inside the window.
    const dot = canvas.getByRole("button", { name: "Fleet — Running" }).getBoundingClientRect();
    const opened = panel.getBoundingClientRect();
    await expect(opened.top).toBeGreaterThanOrEqual(dot.bottom);
    await expect(opened.right).toBeLessThanOrEqual(window.right);
  },
};

/**
 * Fleet down. The Fleet panel says which of the two failures this is: a
 * missing runtime file and a live pid that does not answer call for
 * different things.
 */
export const FleetIsNotRunning: Story = {
  args: {
    ...shell,
    fleet: {
      state: "not-running",
      label: "Not running",
      detail: "no runtime file at ~/Library/Application Support/Armada/fleet.json",
      open: true,
      onOpenChange: () => {},
    },
  },
  render: Shell.render,
};

/**
 * **A screen taller than the window, mounted the way Bridge mounts one.**
 *
 * This is the story that was missing, and its absence is why the layout kept
 * reading right here and wrong in the app. Every other screen story mounts a
 * screen on its own, into a box the story sized. Bridge mounts it into the
 * shell, beside the left column, and the chain from the window down to the
 * screen is exactly the part that was broken.
 *
 * **Nothing outside a pane scrolls.** The left column holds, and the tall
 * content moves inside its own box. If this story ever scrolls the whole
 * shell, the chain is broken again, and it is broken in Bridge with it.
 */
export const AScreenTallerThanTheWindow: Story = {
  args: {
    ...shell,
    children: (
      <div className="armada-screen__mounted">
      <div className="armada-screen__pane">
        {Array.from({ length: 40 }, (_, at) => (
          <div key={at} style={{ flex: "none" }}>
            {`Row ${at + 1} of 40 — the pane scrolls, the frame does not`}
          </div>
        ))}
      </div>
      </div>
    ),
  },
  render: Shell.render,
};

/**
 * **One Job on the Board, in a window with room for twenty.**
 *
 * A row is a row whether there is one of them or forty. The frame fills the
 * pane — that is what lets it scroll — and a grid stretches its rows into the
 * space it is given unless told not to, so a Board holding one Job drew that
 * Job a window tall with its content floating in the middle of it.
 *
 * What the pane does with the room left over is nothing, which is the correct
 * answer and the one a person expects.
 */
export const OneRowInATallWindow: Story = {
  args: {
    ...shell,
    children: (
      <div className="armada-screen__mounted">
        <div className="armada-screen__stack">
          <ActiveJobsList summary="1 job needs you. 1 on the Board." selectable label="Job Board">
            <JobRowStacked
              status="escalated"
              statusIcon={Megaphone}
              statusLabel="went quiet"
              headline="Preserve job metadata during resource cleanup"
              jobId="01M21BKVPW002DC0ATD1X9T0VF"
              handle="9-preserve-job-metadata-cleanup"
              fields={[
                { label: "Workflow", value: "feature, 4 steps" },
                { label: "Progress", value: "scope" },
                { label: "Run time", value: "5h 40m", mono: true },
              ]}
              action={<Button variant="secondary" size="sm">Open</Button>}
            />
          </ActiveJobsList>
        </div>
      </div>
    ),
  },
  render: Shell.render,
};

function WithDock(args: ComponentProps<typeof TheShell>) {
  const [open, setOpen] = useState(args.dock?.open ?? false);
  const dock = args.dock === undefined ? undefined : { ...args.dock, open, onOpen: setOpen };
  return <TheShell {...args} dock={dock} />;
}

/** The dock's own width, so a story can drag and nudge what the app drags and nudges. */
function WithResizableDock(args: ComponentProps<typeof TheShell>) {
  const [open, setOpen] = useState(args.dock?.open ?? false);
  const [width, setWidth] = useState(args.dock?.width ?? 380);
  const dock = args.dock === undefined ? undefined : { ...args.dock, open, width, onOpen: setOpen, onResize: setWidth };
  return <TheShell {...args} dock={dock} />;
}

/** At `--layout-breakpoint` and wider: Helm's dock over the content, on every surface. */
export const DockOverTheContent: Story = {
  args: { ...shell, dock: { open: true, binding: "⌘J", onOpen: () => {} } },
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <WithDock {...args} />
      </div>
    </div>
  ),
};

/**
 * At `--layout-breakpoint` and wider, closed: no strip, no residue — #1094.
 * The title row's own Helm button is the only way back, and pressing it draws
 * the dock over the content. **This is how Bridge opens** (#1583).
 */
export const DockClosedAtWidth: Story = {
  args: { ...shell, dock: { open: false, binding: "⌘J", onOpen: () => {} } },
  render: DockOverTheContent.render,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByLabelText("Helm")).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: /Open Helm/ })).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Helm" }));
    await expect(canvas.getByLabelText("Helm")).toBeVisible();
  },
};

/**
 * **The whole of #1583, as an assertion**: the panel measures the same with
 * Helm shut and open. It was 380px narrower with the dock open, and three
 * screens grew a workaround around that in one day.
 *
 * A geometry claim, which `armada-components` says a `play` does not usually
 * earn — this one is the issue's own definition of done, and nothing a reader
 * looks at can tell 1192px from 812px without a second frame beside it.
 */
export const DockTakesNoWidthFromTheContent: Story = {
  args: { ...shell, dock: { open: false, binding: "⌘J", onOpen: () => {} } },
  render: DockOverTheContent.render,
  play: async ({ canvas, canvasElement, userEvent }) => {
    const panel = (): number => {
      const element = canvasElement.querySelector(".armada-shell__panel");
      if (element === null) throw new Error("the panel is not drawn");
      return Math.round(element.getBoundingClientRect().width);
    };
    const shut = panel();
    await userEvent.click(canvas.getByRole("button", { name: "Helm" }));
    await expect(canvas.getByLabelText("Helm")).toBeVisible();
    await expect(panel()).toBe(shut);
    await userEvent.click(canvas.getByRole("button", { name: /^Close/ }));
    await expect(panel()).toBe(shut);
  },
};

/** Questions from two repositories in the dock's questions zone, above where Helm's conversation will sit. */
export const DockWithQuestions: Story = {
  args: {
    ...shell,
    dock: {
      open: true,
      binding: "⌘J",
      questions: 2,
      onOpen: () => {},
      children: (
        <DockQuestions
          questions={[
            {
              id: "b:judge",
              repository: "shop-01",
              job: "3",
              title: "Checkout total ignores the discount code",
              label: "Judge refused a criterion and is asking you",
              asked: "Does the fix address the cause the note names?",
              detail: "A customer with a valid code is still charged the full price.",
              waiting: "22m",
              answers: [
                { id: "agree", label: "Agree with the refusal" },
                { id: "disagree_once", label: "Disagree, just this step" },
                { id: "disagree_always", label: "Always disagree" },
              ],
              note: "Open job 3 to answer.",
            },
            {
              id: "a:command",
              repository: "armada",
              job: "12",
              title: "The drone count is wrong after a restart",
              label: "The drone wants to run a command it was not given",
              asked: <span className="mono">cargo nextest run -p store</span>,
              waiting: "1m",
              answers: [
                { id: "allow_for_job", label: "Allow for this job" },
                { id: "always_allow", label: "Always allow in this repository" },
                { id: "reject", label: "Reject" },
              ],
              note: "Open job 12 to answer.",
            },
          ]}
        />
      ),
    },
  },
  render: DockOverTheContent.render,
};

/**
 * The dock's leading edge, grabbable and keyboard-operable — dragged wider,
 * dragged to `--w-dock-min`, or nudged by the arrow keys the ARIA separator
 * pattern names. `onResize` absent draws no handle at all: `DockOverTheContent`
 * above is that case.
 */
export const DockResizable: Story = {
  args: { ...shell, dock: { open: true, binding: "⌘J", width: 380, onOpen: () => {}, onResize: () => {} } },
  render: (args) => (
    <div className="armada-screen">
      <div className="armada-screen__window">
        <WithResizableDock {...args} />
      </div>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    const handle = canvas.getByRole("separator", { name: "Resize Helm" });
    await expect(handle).toHaveAttribute("aria-valuenow", "380");
    await userEvent.click(handle);
    await expect(handle).toHaveFocus();
    await userEvent.keyboard("{ArrowLeft}");
    await expect(handle).toHaveAttribute("aria-valuenow", "396");
    await userEvent.keyboard("{ArrowRight}{ArrowRight}");
    await expect(handle).toHaveAttribute("aria-valuenow", "364");
  },
};

/**
 * Below the breakpoint: no edge strip — the owner dropped it on 30 Sep 2026.
 * The title row's Helm button carries the questions waiting and opens the dock
 * as a sheet, the same press as at every other width.
 */
export const DockFoldedToASheet: Story = {
  args: {
    ...shell,
    dock: { open: false, folded: true, questions: 3, binding: "⌘J", onOpen: () => {} },
  },
  render: (args) => (
    <div className="armada-screen" style={{ width: "var(--layout-breakpoint-narrow)" }}>
      <div className="armada-screen__window">
        <WithDock {...args} />
      </div>
    </div>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog", { name: "Helm" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: /Open Helm/ })).not.toBeInTheDocument();
    // At this width the title row is behind its menu, and Helm is in it.
    await userEvent.click(canvas.getByRole("button", { name: "Menu" }));
    await userEvent.click(canvas.getByRole("button", { name: /^Helm 3/ }));
    await expect(canvas.getByRole("dialog", { name: "Helm" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /^Close/ }));
    await expect(canvas.queryByRole("dialog", { name: "Helm" })).toBeNull();
  },
};
