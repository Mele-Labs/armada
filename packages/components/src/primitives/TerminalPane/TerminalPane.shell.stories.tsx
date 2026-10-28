import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { ShellDock, ShellPane, ShellPanelPane, ShellWindowPane, type ShellPaneProps, type TerminalPhase } from "./TerminalPane";
import { XtermScreen } from "./XtermScreen";

/**
 * The Job's own shell, one that stays open and that the person and the Job's agent both type into. It
 * has no exit to mark and is never run again: Hide puts the pane away and leaves the shell running, End
 * shell stops it, and once it has ended the pane offers a new one.
 */
const meta: Meta<typeof ShellPane> = {
  title: "Primitives/Terminal pane",
  component: ShellPane,
  args: { onHide: fn(), onEnd: fn(), onOpen: fn(), onClose: fn() },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", flexDirection: "column", width: "calc(var(--space-12) * 12)", height: "calc(var(--space-12) * 6)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ShellPane>;

/** An act on the shell's head drawn as a glyph alone: named, and described, by its tooltip. */
async function iconAct(pane: ReturnType<typeof within>, label: string): Promise<HTMLElement> {
  const button = pane.getByRole("button", { name: label });
  await expect(button).toHaveAccessibleDescription(label);
  await expect(button.textContent).toBe("");
  return button;
}

const body = (text: string): ShellPaneProps["children"] => <pre style={{ margin: 0, padding: "var(--space-2) var(--space-3)", fontFamily: "var(--font-mono)" }}>{text}</pre>;

export const ShellRunning: Story = {
  args: { phase: { state: "running" }, children: body("~/armada $ git status") },
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.getByRole("group", { name: "Terminal" })).toBeVisible();
    await expect(pane.getByText("Terminal", { selector: "header *" })).toBeVisible();
    // A shell is not a run: nothing marks how it exited and nothing runs it again.
    await expect(pane.queryAllByRole("img")).toEqual([]);
    await expect(pane.queryByRole("button", { name: "Run again" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Stop" })).toBeNull();

    // Hide puts the pane away and leaves the shell running.
    await userEvent.click(await iconAct(pane, "Hide"));
    await expect(args.onHide).toHaveBeenCalledTimes(1);
    await expect(args.onEnd).not.toHaveBeenCalled();

    await userEvent.click(await iconAct(pane, "End shell"));
    await expect(args.onEnd).toHaveBeenCalledTimes(1);
  },
};

export const ShellEnded: Story = {
  args: { phase: { state: "ended" }, children: body("~/armada $ exit") },
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.getByText("Shell ended")).toBeVisible();
    await expect(pane.queryAllByRole("img")).toEqual([]);
    await expect(pane.queryByRole("button", { name: "Hide" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "End shell" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Run again" })).toBeNull();

    await userEvent.click(await iconAct(pane, "New shell"));
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
    await userEvent.click(await iconAct(pane, "Close"));
    await expect(args.onClose).toHaveBeenCalledTimes(1);
  },
};

/** In the side panel, which names the shell and closes it: End shell is the pane's one key. */
export const ShellInThePanel: Story = {
  args: { phase: { state: "running" }, children: body("~/armada $ git status") },
  render: (args) => <ShellPanelPane {...args} />,
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.queryByText("Terminal", { selector: "header *" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Hide" })).toBeNull();
    await userEvent.click(await iconAct(pane, "End shell"));
    await expect(args.onEnd).toHaveBeenCalledTimes(1);
  },
};

/** Ended in the side panel: New shell, and the panel's own Close rather than one of the pane's. */
export const ShellEndedInThePanel: Story = {
  args: { phase: { state: "ended" }, children: body("~/armada $ exit") },
  render: (args) => <ShellPanelPane {...args} />,
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.getByText("Shell ended")).toBeVisible();
    await expect(pane.queryByRole("button", { name: "Close" })).toBeNull();
    await userEvent.click(await iconAct(pane, "New shell"));
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
  },
};

const toPanel = fn();
const toWindow = fn();
const toDock = fn();

/** Docked, the head offers the two other places: the side panel and a window of its own. */
export const ShellMovesFromTheDock: Story = {
  args: {
    phase: { state: "running" },
    moves: [
      { to: "side panel", onMove: toPanel },
      { to: "window", onMove: toWindow },
    ],
    children: body("~/armada $ git status"),
  },
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    toPanel.mockClear();
    toWindow.mockClear();
    await userEvent.click(await iconAct(pane, "Move to the side panel"));
    await expect(toPanel).toHaveBeenCalledTimes(1);
    await userEvent.click(await iconAct(pane, "Open in a window"));
    await expect(toWindow).toHaveBeenCalledTimes(1);
    await expect(toPanel).toHaveBeenCalledTimes(1);
    await expect(args.onHide).not.toHaveBeenCalled();
    await expect(pane.queryByRole("button", { name: "Dock" })).toBeNull();
  },
};

/** In the side panel, the head docks it again or opens it in a window. */
export const ShellMovesFromThePanel: Story = {
  args: {
    phase: { state: "running" },
    moves: [
      { to: "docked", onMove: toDock },
      { to: "window", onMove: toWindow },
    ],
    children: body("~/armada $ git status"),
  },
  render: (args) => <ShellPanelPane {...args} />,
  play: async ({ canvasElement, userEvent }) => {
    const pane = within(canvasElement);
    toDock.mockClear();
    await userEvent.click(await iconAct(pane, "Dock"));
    await expect(toDock).toHaveBeenCalledTimes(1);
    await expect(await iconAct(pane, "Open in a window")).toBeVisible();
    await expect(pane.queryByRole("button", { name: "Move to the side panel" })).toBeNull();
  },
};

/** In a window of its own, whose close stands for Hide and Close: End shell, and the moves back into Bridge. */
export const ShellInAWindow: Story = {
  args: {
    phase: { state: "running" },
    moves: [
      { to: "docked", onMove: toDock },
      { to: "side panel", onMove: toPanel },
    ],
    children: body("~/armada $ git status"),
  },
  render: (args) => <ShellWindowPane {...args} />,
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    toPanel.mockClear();
    await expect(pane.queryByText("Terminal", { selector: "header *" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Hide" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Open in a window" })).toBeNull();
    await userEvent.click(await iconAct(pane, "Move to the side panel"));
    await expect(toPanel).toHaveBeenCalledTimes(1);
    await expect(await iconAct(pane, "Dock")).toBeVisible();
    await userEvent.click(await iconAct(pane, "End shell"));
    await expect(args.onEnd).toHaveBeenCalledTimes(1);
  },
};

/** Ended in its window: New shell, and the window's own close rather than one of the pane's. */
export const ShellEndedInAWindow: Story = {
  args: { phase: { state: "ended" }, moves: [{ to: "docked", onMove: toDock }], children: body("~/armada $ exit") },
  render: (args) => <ShellWindowPane {...args} />,
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.getByText("Shell ended")).toBeVisible();
    await expect(pane.queryByRole("button", { name: "Close" })).toBeNull();
    await userEvent.click(await iconAct(pane, "New shell"));
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
  },
};

/** What the app puts in the body: xterm, the same screen a block's run draws on. */
export const ShellOnTheTerminalScreen: Story = {
  args: { phase: { state: "running" } },
  render: (args) => (
    <ShellPane {...args}>
      <XtermScreen out={"\u001b[32m~/armada\u001b[39m $ cargo build\r\n   Compiling armada v0.1.0\r\n"} run={1} running write={fn()} onSize={fn()} />
    </ShellPane>
  ),
  play: async ({ canvasElement }) => {
    const pane = within(canvasElement);
    await expect(await pane.findByText(/Compiling/)).toBeVisible();
    await expect(await pane.findByRole("textbox", { name: "Terminal input" })).toHaveFocus();
  },
};

/** A shell the Job's agent opens while the owner types in a box beside it: the keyboard stays in the box. */
export const ShellOpensBesideATypingBox: Story = {
  args: { phase: { state: "running" } },
  render: (args) => (
    <>
      <textarea aria-label="Message" autoFocus />
      <ShellPane {...args}>
        <XtermScreen out={"~/armada $ "} run={1} running write={fn()} onSize={fn()} />
      </ShellPane>
    </>
  ),
  play: async ({ canvasElement }) => {
    const pane = within(canvasElement);
    await expect(await pane.findByRole("textbox", { name: "Terminal input" })).toBeInTheDocument();
    await expect(pane.getByRole("textbox", { name: "Message" })).toHaveFocus();
  },
};

const RUN_PHASE: TerminalPhase = { state: "running" };

/**
 * The dock at the foot of a Job's view holding the shell and a block's run in the one place, switched as
 * tabs: Terminal, and the run's command. Each pane keeps its own keys.
 */
export const DockedBesideARun: Story = {
  args: { phase: { state: "running" } },
  render: function Render(args) {
    const [front, setFront] = useState<"shell" | "run">("shell");
    return (
      <ShellDock
        front={front}
        onFront={setFront}
        shell={{ phase: args.phase, onHide: args.onHide, onEnd: args.onEnd, onOpen: args.onOpen, onClose: args.onClose, children: body("~/armada $ ls") }}
        run={{ command: "make test", phase: RUN_PHASE, onStop: fn(), onRerun: fn(), onClose: fn(), children: body("running 12 tests") }}
      />
    );
  },
  play: async ({ canvasElement, userEvent }) => {
    const dock = within(canvasElement);
    const shellTab = dock.getByRole("tab", { name: "Terminal" });
    const runTab = dock.getByRole("tab", { name: "make test" });
    await expect(shellTab).toHaveAttribute("aria-selected", "true");
    await expect(dock.getByRole("button", { name: "Hide" })).toBeVisible();
    await expect(dock.getByText("~/armada $ ls")).toBeVisible();

    await userEvent.click(runTab);
    await expect(runTab).toHaveAttribute("aria-selected", "true");
    await expect(dock.getByRole("img", { name: "Running" })).toBeVisible();
    await expect(dock.getByRole("button", { name: "Stop" })).toBeVisible();
    await expect(dock.queryByRole("button", { name: "Hide" })).toBeNull();
    await expect(dock.getByText("running 12 tests")).toBeVisible();

    // The arrow keys switch panes, and the strip is not drawn again under the focus as the pane in front changes.
    await userEvent.keyboard("{ArrowLeft}");
    await expect(shellTab).toHaveAttribute("aria-selected", "true");
    await expect(dock.getByRole("tablist")).toContainElement(document.activeElement as HTMLElement);
    await expect(dock.getByRole("button", { name: "End shell" })).toBeVisible();
  },
};

const LONG_COMMAND = "cd ~/Development/armada/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy";

/** A long command's tab is cut short, so the strip stays one line; the whole command is its tooltip. */
export const DockedBesideALongRun: Story = {
  args: { phase: { state: "running" } },
  render: (args) => (
    <ShellDock
      front="run"
      onFront={fn()}
      shell={{ phase: args.phase, onHide: args.onHide, onEnd: args.onEnd, onOpen: args.onOpen, onClose: args.onClose, children: body("~/armada $") }}
      run={{ command: LONG_COMMAND, phase: RUN_PHASE, onStop: fn(), onRerun: fn(), onClose: fn(), children: body("Attempting to login via OAuth...") }}
    />
  ),
  play: async ({ canvasElement }) => {
    const dock = within(canvasElement);
    const runTab = dock.getByRole("tab", { name: /…$/ });
    await expect(runTab).toHaveAccessibleName("cd ~/Development/armada/apps/st…");
    await expect(runTab).toHaveAccessibleDescription(LONG_COMMAND);
  },
};

/** The dock with the shell alone draws no tabs: the pane's own name says what it is. */
export const DockedAlone: Story = {
  args: { phase: { state: "running" } },
  render: (args) => (
    <ShellDock
      front="shell"
      onFront={fn()}
      shell={{ phase: args.phase, onHide: args.onHide, onEnd: args.onEnd, onOpen: args.onOpen, onClose: args.onClose, children: body("~/armada $") }}
      run={null}
    />
  ),
  play: async ({ canvasElement }) => {
    const dock = within(canvasElement);
    await expect(dock.queryByRole("tablist")).toBeNull();
    await expect(dock.getByRole("separator", { name: "Resize terminal" })).toBeInTheDocument();
    await expect(dock.getByRole("button", { name: "End shell" })).toBeVisible();
  },
};
