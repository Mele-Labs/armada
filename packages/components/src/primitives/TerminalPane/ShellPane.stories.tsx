import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { ShellDock, ShellPane, type ShellPaneProps, type TerminalPhase } from "./TerminalPane";
import { XtermScreen } from "./XtermScreen";

/**
 * The Job's own shell, one that stays open and that the person and the Job's agent both type into. It
 * has no exit to mark and is never run again: Hide puts the pane away and leaves the shell running, End
 * shell stops it, and once it has ended the pane offers a new one.
 */
const meta: Meta<typeof ShellPane> = {
  title: "Primitives/Terminal pane/Shell",
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

const body = (text: string): ShellPaneProps["children"] => <pre style={{ margin: 0, padding: "var(--space-2) var(--space-3)", fontFamily: "var(--font-mono)" }}>{text}</pre>;

export const Running: Story = {
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
    await userEvent.click(pane.getByRole("button", { name: "Hide" }));
    await expect(args.onHide).toHaveBeenCalledTimes(1);
    await expect(args.onEnd).not.toHaveBeenCalled();

    await userEvent.click(pane.getByRole("button", { name: "End shell" }));
    await expect(args.onEnd).toHaveBeenCalledTimes(1);
  },
};

export const Ended: Story = {
  args: { phase: { state: "ended" }, children: body("~/armada $ exit") },
  play: async ({ canvasElement, args, userEvent }) => {
    const pane = within(canvasElement);
    await expect(pane.getByText("Shell ended")).toBeVisible();
    await expect(pane.queryAllByRole("img")).toEqual([]);
    await expect(pane.queryByRole("button", { name: "Hide" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "End shell" })).toBeNull();
    await expect(pane.queryByRole("button", { name: "Run again" })).toBeNull();

    await userEvent.click(pane.getByRole("button", { name: "New shell" }));
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
    await userEvent.click(pane.getByRole("button", { name: "Close" }));
    await expect(args.onClose).toHaveBeenCalledTimes(1);
  },
};

/** What the app puts in the body: xterm, the same screen a block's run draws on. */
export const OnTheTerminalScreen: Story = {
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
