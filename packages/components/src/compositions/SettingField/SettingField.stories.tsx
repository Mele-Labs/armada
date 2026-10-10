import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { SettingField, SettingsFileBar } from "./SettingField";

/**
 * One setting from settings.json, drawn from its kind. A plain row rather than a layer: `Card` on
 * the Settings surface supplies the chrome.
 */
const meta: Meta<typeof SettingField> = {
  title: "Compositions/Setting field",
  component: SettingField,
};
export default meta;

type Story = StoryObj<typeof SettingField>;

const DRONES = {
  name: "limits.dronesAtOnce",
  title: "Drones at once",
  description: "How many drones Fleet runs at the same time. A job past this waits for one to finish.",
  shipped: "2",
  onSave: fn(),
  onReset: fn(),
};

/** As Armada ships it: no mark, nothing to go back to. */
export const AtRest: Story = {
  name: "At rest",
  args: { ...DRONES, control: { kind: "number", value: 2, min: 1, max: 8 }, modified: false },
  /** **What goes to Fleet is a number, never the text typed.** */
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByLabelText("Drones at once");
    await userEvent.clear(field);
    await userEvent.type(field, "4");
    await userEvent.click(canvas.getByRole("button", { name: "Save Drones at once" }));
    await expect(args.onSave).toHaveBeenCalledWith(4);
    await expect(canvas.queryByText("Modified")).toBeNull();
  },
};

/** settings.json holds a value for the key: Modified, and Reset removes it. */
export const Modified: Story = {
  args: { ...DRONES, control: { kind: "number", value: 3, min: 1, max: 8 }, modified: true },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Modified")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Reset: Drones at once" }));
    await expect(args.onReset).toHaveBeenCalled();
  },
};

/** A figure outside the bound is refused before anything is sent. */
export const OutOfRange: Story = {
  name: "Refused in the field",
  args: { ...DRONES, control: { kind: "number", value: 2, min: 1, max: 8 }, modified: false },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByLabelText("Drones at once");
    await userEvent.clear(field);
    await userEvent.type(field, "12");
    await expect(canvas.getByText("Between 1 and 8.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save Drones at once" })).toBeDisabled();
  },
};

/** Fleet refused the save, in its own words. */
export const RefusedByFleet: Story = {
  name: "Refused by Fleet",
  args: { ...DRONES, control: { kind: "number", value: 2, min: 1, max: 8 }, modified: false, refused: "limits.dronesAtOnce must be between 1 and 8, and 12 is not." },
};

/** A seconds value, read out in minutes beneath the field. Saved, and waiting on a restart. */
export const WaitingOnRestart: Story = {
  name: "Waiting on a restart",
  args: {
    name: "timeouts.commandSeconds",
    title: "Command time limit",
    description: "How long Fleet works on one command from Bridge before it answers that it gave up.",
    control: { kind: "number", value: 60, min: 5, max: 300, seconds: true },
    shipped: "15 seconds",
    modified: true,
    restart: true,
    onSave: fn(),
    onReset: fn(),
  },
};

/** An environment variable wins over settings.json on this machine. */
export const OverriddenByEnvironment: Story = {
  name: "Overridden by the environment",
  args: {
    name: "harness.binaryPath",
    title: "Agent program",
    description: "The agent's program, by name on PATH or by absolute path.",
    control: { kind: "text", value: "agent" },
    modified: false,
    overriddenBy: "ARMADA_AGENT_BINARY",
    onSave: fn(),
    onReset: fn(),
  },
};

/** A switch saves as it is pressed. */
export const OnOff: Story = {
  name: "Switch",
  args: {
    name: "features.draftPullRequests",
    title: "Draft pull requests",
    description: "A pull request is opened as a draft unless the repository, the workflow or the job says otherwise.",
    control: { kind: "switch", value: false },
    shipped: "off",
    modified: false,
    onSave: fn(),
    onReset: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByText("Draft pull requests"));
    await expect(args.onSave).toHaveBeenCalledWith(true);
  },
};

/** A choice saves as it is chosen. */
export const Choice: Story = {
  args: {
    name: "terminal.app",
    title: "Terminal",
    description: "The terminal Bridge opens a worktree in.",
    control: { kind: "choice", value: "Terminal", options: ["Terminal", "iTerm", "Ghostty", "Warp", "WezTerm", "Custom"] },
    modified: false,
    onSave: fn(),
    onReset: fn(),
  },
};

/** One entry a line. Blank lines are dropped on Save. */
export const List: Story = {
  args: {
    name: "models.roster",
    title: "Models",
    description: "The models a job or a step may choose from.",
    control: { kind: "list", value: ["opus", "sonnet", "haiku"] },
    modified: false,
    onSave: fn(),
    onReset: fn(),
  },
};

/** A prompt edited from what Armada ships: its way back says so. */
export const PromptChanged: Story = {
  name: "Prompt, changed",
  args: {
    name: "prompts.droneBaseline",
    title: "Drone baseline",
    description: "What every drone is told before its step's own brief.",
    control: { kind: "prompt", value: "You are working one step of a job.\nStay inside the worktree." },
    modified: true,
    onSave: fn(),
    onReset: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Reset to shipped: Drone baseline" }));
    await expect(args.onReset).toHaveBeenCalled();
  },
};

/** settings.json above every section, read cleanly: its path and the way to open it. */
export const FileBar: StoryObj<typeof SettingsFileBar> = {
  name: "File bar",
  render: (args) => <SettingsFileBar {...args} />,
  args: { path: "/Users/user/Library/Application Support/Armada/settings.json", refused: null, onOpen: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open settings.json" }));
    await expect(args.onOpen).toHaveBeenCalled();
  },
};

/** Fleet refused the file: the key and why, and that the last good settings stay in force. */
export const FileRefused: StoryObj<typeof SettingsFileBar> = {
  name: "File bar, refused",
  render: (args) => <SettingsFileBar {...args} />,
  args: {
    path: "/Users/user/Library/Application Support/Armada/settings.json",
    refused: { key: "limits.dronesAtOnce", reason: "12 is above the most this takes, 8." },
    onOpen: fn(),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/The last settings that read cleanly stay in force/)).toBeVisible();
    await expect(canvas.getByText("limits.dronesAtOnce")).toBeVisible();
  },
};
